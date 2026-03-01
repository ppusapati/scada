package dds

import (
	"context"
	"encoding/binary"
	"fmt"
	"math/rand"
	"net"
	"sync"
	"time"

	"github.com/rs/zerolog"
)

// ═══════════════════════════════════════════════
// RTPS Protocol Constants
// ═══════════════════════════════════════════════

var rtpsMagic = [4]byte{'R', 'T', 'P', 'S'}

const (
	rtpsProtocolVersion2_2 = 0x0202

	// RTPS Vendor ID for our implementation (custom / non-standard)
	vendorIDScada = 0xFF01

	// Built-in endpoints for SPDP and SEDP
	builtinParticipantWriterEntityID = 0x000100C2
	builtinParticipantReaderEntityID = 0x000100C7
	builtinPublicationWriterEntityID = 0x000003C2
	builtinPublicationReaderEntityID = 0x000003C7
	builtinSubscriptionWriterEntityID = 0x000004C2
	builtinSubscriptionReaderEntityID = 0x000004C7

	// Submessage IDs
	submsgDATA    byte = 0x15
	submsgHEARTBEAT byte = 0x07
	submsgACKNACK   byte = 0x06
	submsgINFO_DST  byte = 0x0E
	submsgINFO_TS   byte = 0x09
	submsgPAD       byte = 0x01

	// Submessage flags
	flagEndianness byte = 0x01 // bit 0: 0=big-endian, 1=little-endian
	flagDataFlag   byte = 0x04 // bit 2: DATA submessage contains serialized data
	flagInlineQoS  byte = 0x02 // bit 1: inline QoS present

	// Default SPDP multicast port base
	spdpWellKnownPortBase  = 7400
	spdpPortDomainGain     = 250
	spdpPortParticipantGain = 2
	// PB + DG * domainID + d0  (d0=0 for SPDP multicast, d1=10 for SPDP unicast)
	spdpMulticastOffset    = 0
	spdpUnicastOffset      = 10

	// Default SPDP multicast address
	spdpDefaultMulticastAddr = "239.255.0.1"

	// Discovery announcement interval
	spdpResendPeriod = 30 * time.Second
	spdpLeaseDuration = 100 * time.Second

	// Participant stale timeout
	participantStaleTimeout = 120 * time.Second
)

// ═══════════════════════════════════════════════
// RTPS Header and GUID
// ═══════════════════════════════════════════════

// GUIDPrefix is a 12-byte prefix identifying a DDS participant
type GUIDPrefix [12]byte

// EntityID is a 4-byte entity identifier within a participant
type EntityID [4]byte

// GUID uniquely identifies a DDS entity (participant + entity)
type GUID struct {
	Prefix   GUIDPrefix `json:"prefix"`
	EntityID EntityID   `json:"entity_id"`
}

func (g GUID) String() string {
	return fmt.Sprintf("%x.%x", g.Prefix, g.EntityID)
}

// RTPSHeader is the 20-byte header at the start of every RTPS message
type RTPSHeader struct {
	Magic           [4]byte
	ProtocolVersion uint16
	VendorID        uint16
	GUIDPrefix      GUIDPrefix
}

// generateGUIDPrefix creates a unique 12-byte GUID prefix from host+process+random
func generateGUIDPrefix() GUIDPrefix {
	var prefix GUIDPrefix
	// Use a combination of host IP hash, PID-like value, and random bytes
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	for i := range prefix {
		prefix[i] = byte(r.Intn(256))
	}
	// Set first 4 bytes to a recognizable pattern for debugging
	prefix[0] = 0x53 // 'S'
	prefix[1] = 0x43 // 'C'
	prefix[2] = 0x41 // 'A'
	prefix[3] = 0x44 // 'D'  (SCAD)
	return prefix
}

// encodeRTPSHeader writes the 20-byte RTPS header
func encodeRTPSHeader(prefix GUIDPrefix) []byte {
	header := make([]byte, 20)
	copy(header[0:4], rtpsMagic[:])
	binary.BigEndian.PutUint16(header[4:6], rtpsProtocolVersion2_2)
	binary.BigEndian.PutUint16(header[6:8], vendorIDScada)
	copy(header[8:20], prefix[:])
	return header
}

// parseRTPSHeader reads the 20-byte RTPS header
func parseRTPSHeader(data []byte) (*RTPSHeader, error) {
	if len(data) < 20 {
		return nil, fmt.Errorf("RTPS header too short: %d bytes", len(data))
	}
	h := &RTPSHeader{}
	copy(h.Magic[:], data[0:4])
	if h.Magic != rtpsMagic {
		return nil, fmt.Errorf("invalid RTPS magic: %v", h.Magic)
	}
	h.ProtocolVersion = binary.BigEndian.Uint16(data[4:6])
	h.VendorID = binary.BigEndian.Uint16(data[6:8])
	copy(h.GUIDPrefix[:], data[8:20])
	return h, nil
}

// ═══════════════════════════════════════════════
// SPDP Data Structures
// ═══════════════════════════════════════════════

// DiscoveredParticipant represents a remote DDS domain participant
type DiscoveredParticipant struct {
	GUIDPrefix     GUIDPrefix `json:"guid_prefix"`
	ProtocolVer    uint16     `json:"protocol_version"`
	VendorID       uint16     `json:"vendor_id"`
	ParticipantName string    `json:"participant_name"`
	UnicastLocator  string    `json:"unicast_locator"`
	MulticastLocator string   `json:"multicast_locator"`
	LeaseDuration  time.Duration `json:"lease_duration_sec"`
	DiscoveredAt   time.Time  `json:"discovered_at"`
	LastSeen       time.Time  `json:"last_seen"`
	Endpoints      []DiscoveredEndpoint `json:"endpoints"`
}

// IsStale returns true if the participant has not been seen recently
func (p *DiscoveredParticipant) IsStale() bool {
	return time.Since(p.LastSeen) > p.LeaseDuration
}

// DiscoveredEndpoint represents a remote reader or writer discovered via SEDP
type DiscoveredEndpoint struct {
	EntityID  EntityID   `json:"entity_id"`
	TopicName string     `json:"topic_name"`
	TypeName  string     `json:"type_name"`
	IsWriter  bool       `json:"is_writer"`
	QoS       QoSProfile `json:"qos"`
}

// ═══════════════════════════════════════════════
// Discovery Service
// ═══════════════════════════════════════════════

// DiscoveryService implements Simple Participant Discovery Protocol (SPDP)
// and Simple Endpoint Discovery Protocol (SEDP) for DDS
type DiscoveryService struct {
	domainID       uint32
	guidPrefix     GUIDPrefix
	participantName string
	multicastAddr  string
	networkIface   string

	// Connections
	spdpMulticastConn *net.UDPConn
	spdpUnicastConn   *net.UDPConn

	// Discovery state
	participants  map[GUIDPrefix]*DiscoveredParticipant
	participantMu sync.RWMutex

	// Tracked local topics for SEDP announcements
	localTopics   map[DDSTopic]DiscoveredEndpoint
	localTopicsMu sync.RWMutex

	logger zerolog.Logger

	// Metrics
	spdpSent     int64
	spdpReceived int64
	sedpReceived int64
	metricsMu    sync.Mutex
}

// NewDiscoveryService creates a new SPDP/SEDP discovery service
func NewDiscoveryService(domainID uint32, participantName, multicastAddr, networkIface string, logger zerolog.Logger) *DiscoveryService {
	if multicastAddr == "" {
		multicastAddr = spdpDefaultMulticastAddr
	}
	return &DiscoveryService{
		domainID:        domainID,
		guidPrefix:      generateGUIDPrefix(),
		participantName: participantName,
		multicastAddr:   multicastAddr,
		networkIface:    networkIface,
		participants:    make(map[GUIDPrefix]*DiscoveredParticipant),
		localTopics:     make(map[DDSTopic]DiscoveredEndpoint),
		logger:          logger.With().Str("component", "dds-discovery").Logger(),
	}
}

// GUIDPrefixValue returns the local GUID prefix
func (ds *DiscoveryService) GUIDPrefixValue() GUIDPrefix {
	return ds.guidPrefix
}

// spdpMulticastPort computes the SPDP well-known multicast port for a domain
func spdpMulticastPort(domainID uint32) int {
	return spdpWellKnownPortBase + int(domainID)*spdpPortDomainGain + spdpMulticastOffset
}

// spdpUnicastPort computes the SPDP well-known unicast port for a domain + participant index
func spdpUnicastPort(domainID uint32, participantIndex int) int {
	return spdpWellKnownPortBase + int(domainID)*spdpPortDomainGain + spdpUnicastOffset + participantIndex*spdpPortParticipantGain
}

// Start begins the SPDP/SEDP discovery process
func (ds *DiscoveryService) Start(ctx context.Context) error {
	port := spdpMulticastPort(ds.domainID)

	// Resolve multicast address
	multicastGroup := net.ParseIP(ds.multicastAddr)
	if multicastGroup == nil {
		return fmt.Errorf("invalid multicast address: %s", ds.multicastAddr)
	}

	// Join multicast group
	multicastUDP := &net.UDPAddr{
		IP:   multicastGroup,
		Port: port,
	}

	var iface *net.Interface
	if ds.networkIface != "" {
		var err error
		iface, err = net.InterfaceByName(ds.networkIface)
		if err != nil {
			ds.logger.Warn().Err(err).Str("interface", ds.networkIface).Msg("Failed to find network interface, using default")
		}
	}

	var err error
	ds.spdpMulticastConn, err = net.ListenMulticastUDP("udp4", iface, multicastUDP)
	if err != nil {
		ds.logger.Warn().Err(err).Msg("Failed to join SPDP multicast group, falling back to unicast-only discovery")
		// Fallback: listen on unicast port
		unicastPort := spdpUnicastPort(ds.domainID, 0)
		unicastAddr := &net.UDPAddr{IP: net.IPv4zero, Port: unicastPort}
		ds.spdpUnicastConn, err = net.ListenUDP("udp4", unicastAddr)
		if err != nil {
			return fmt.Errorf("failed to bind SPDP unicast port %d: %w", unicastPort, err)
		}
	} else {
		ds.spdpMulticastConn.SetReadBuffer(65536)
	}

	// Also listen on a unicast port for directed SPDP messages
	if ds.spdpUnicastConn == nil {
		unicastPort := spdpUnicastPort(ds.domainID, 0)
		unicastAddr := &net.UDPAddr{IP: net.IPv4zero, Port: unicastPort}
		ds.spdpUnicastConn, err = net.ListenUDP("udp4", unicastAddr)
		if err != nil {
			ds.logger.Warn().Err(err).Int("port", unicastPort).Msg("Failed to bind SPDP unicast port")
			// Non-fatal; multicast discovery still works
		}
	}

	ds.logger.Info().
		Uint32("domain_id", ds.domainID).
		Str("multicast", fmt.Sprintf("%s:%d", ds.multicastAddr, port)).
		Str("guid_prefix", fmt.Sprintf("%x", ds.guidPrefix)).
		Msg("Discovery service started")

	// Start receiver goroutines
	if ds.spdpMulticastConn != nil {
		go ds.receiveLoop(ctx, ds.spdpMulticastConn, "multicast")
	}
	if ds.spdpUnicastConn != nil {
		go ds.receiveLoop(ctx, ds.spdpUnicastConn, "unicast")
	}

	// Start SPDP announcement goroutine
	go ds.announceLoop(ctx)

	// Start stale participant cleanup
	go ds.cleanupLoop(ctx)

	return nil
}

// Stop shuts down the discovery service
func (ds *DiscoveryService) Stop() {
	if ds.spdpMulticastConn != nil {
		ds.spdpMulticastConn.Close()
	}
	if ds.spdpUnicastConn != nil {
		ds.spdpUnicastConn.Close()
	}
	ds.logger.Info().Msg("Discovery service stopped")
}

// RegisterLocalTopic registers a local topic for SEDP announcement
func (ds *DiscoveryService) RegisterLocalTopic(topic DDSTopic, typeName string, isWriter bool) {
	ds.localTopicsMu.Lock()
	defer ds.localTopicsMu.Unlock()
	ds.localTopics[topic] = DiscoveredEndpoint{
		TopicName: string(topic),
		TypeName:  typeName,
		IsWriter:  isWriter,
		QoS:       QoSForTopic(topic),
	}
}

// GetParticipants returns all currently discovered (non-stale) participants
func (ds *DiscoveryService) GetParticipants() []DiscoveredParticipant {
	ds.participantMu.RLock()
	defer ds.participantMu.RUnlock()

	result := make([]DiscoveredParticipant, 0, len(ds.participants))
	for _, p := range ds.participants {
		if !p.IsStale() {
			result = append(result, *p)
		}
	}
	return result
}

// GetParticipantCount returns the number of active participants
func (ds *DiscoveryService) GetParticipantCount() int {
	ds.participantMu.RLock()
	defer ds.participantMu.RUnlock()

	count := 0
	for _, p := range ds.participants {
		if !p.IsStale() {
			count++
		}
	}
	return count
}

// GetAllTopics returns all discovered topics across all participants
func (ds *DiscoveryService) GetAllTopics() map[string][]string {
	ds.participantMu.RLock()
	defer ds.participantMu.RUnlock()

	topics := make(map[string][]string) // topic -> []participant_name
	for _, p := range ds.participants {
		if p.IsStale() {
			continue
		}
		for _, ep := range p.Endpoints {
			topics[ep.TopicName] = append(topics[ep.TopicName], p.ParticipantName)
		}
	}
	return topics
}

// GetMetrics returns discovery service metrics
func (ds *DiscoveryService) GetMetrics() DiscoveryMetrics {
	ds.metricsMu.Lock()
	defer ds.metricsMu.Unlock()
	return DiscoveryMetrics{
		SPDPSent:           ds.spdpSent,
		SPDPReceived:       ds.spdpReceived,
		SEDPReceived:       ds.sedpReceived,
		ActiveParticipants: ds.GetParticipantCount(),
	}
}

// DiscoveryMetrics holds discovery service statistics
type DiscoveryMetrics struct {
	SPDPSent           int64 `json:"spdp_sent"`
	SPDPReceived       int64 `json:"spdp_received"`
	SEDPReceived       int64 `json:"sedp_received"`
	ActiveParticipants int   `json:"active_participants"`
}

// ═══════════════════════════════════════════════
// SPDP Announcement
// ═══════════════════════════════════════════════

// buildSPDPAnnouncement creates an RTPS message containing SPDP data
func (ds *DiscoveryService) buildSPDPAnnouncement() []byte {
	// Build SPDP data payload using CDR
	enc := NewCDREncoder()
	enc.WriteString(ds.participantName) // participant name

	// Encode GUID prefix
	for _, b := range ds.guidPrefix {
		enc.WriteUint8(b)
	}

	// Lease duration
	enc.WriteInt32(int32(spdpLeaseDuration.Seconds()))
	enc.WriteUint32(0) // fraction

	// Builtin endpoint set (bitmask of available built-in endpoints)
	// Bits: 0=participantWriter, 1=participantReader, 2=pubWriter, 3=pubReader, etc.
	builtinEndpoints := uint32(0x3F) // We support all six standard built-in endpoints
	enc.WriteUint32(builtinEndpoints)

	// Domain ID
	enc.WriteUint32(ds.domainID)

	// Default unicast locator info
	if ds.spdpUnicastConn != nil {
		addr := ds.spdpUnicastConn.LocalAddr().(*net.UDPAddr)
		enc.WriteUint32(1) // locator kind: UDPv4
		enc.WriteUint32(uint32(addr.Port))
		if addr.IP != nil && len(addr.IP) >= 4 {
			ip := addr.IP.To4()
			if ip != nil {
				for _, b := range ip {
					enc.WriteUint8(b)
				}
			} else {
				enc.WriteUint32(0)
			}
		} else {
			enc.WriteUint32(0)
		}
	} else {
		enc.WriteUint32(0) // no unicast locator
		enc.WriteUint32(0)
		enc.WriteUint32(0)
	}

	cdrPayload := enc.Bytes()

	// Build RTPS message: header + DATA submessage
	rtpsHeader := encodeRTPSHeader(ds.guidPrefix)

	// DATA submessage for SPDP
	dataSubmsg := ds.buildDataSubmessage(
		EntityID{0x00, 0x01, 0x00, 0xC2}, // built-in participant writer
		EntityID{0x00, 0x01, 0x00, 0xC7}, // built-in participant reader
		1, // writerSN
		cdrPayload,
	)

	msg := make([]byte, 0, len(rtpsHeader)+len(dataSubmsg))
	msg = append(msg, rtpsHeader...)
	msg = append(msg, dataSubmsg...)
	return msg
}

// buildDataSubmessage creates a DATA submessage
func (ds *DiscoveryService) buildDataSubmessage(writerEID, readerEID EntityID, writerSN int64, payload []byte) []byte {
	endian := binary.LittleEndian

	// Flags: E=1 (little-endian), D=1 (data present)
	flags := flagEndianness | flagDataFlag

	// Extra flags (2 bytes) + octets to inline QoS (2 bytes) = 4 bytes
	// Reader/Writer EntityID = 4+4 = 8 bytes
	// Writer Sequence Number = 8 bytes
	// Total fixed: 4 + 8 + 8 = 20 bytes before payload
	fixedPartLen := 20

	submsgLen := fixedPartLen + len(payload)

	// Submessage header: 4 bytes (id, flags, length)
	buf := make([]byte, 4+submsgLen)
	buf[0] = submsgDATA
	buf[1] = flags
	endian.PutUint16(buf[2:4], uint16(submsgLen))

	// Extra flags (usually 0)
	endian.PutUint16(buf[4:6], 0)
	// Octets to inline QoS (16 = readerEID + writerEID + writerSN)
	endian.PutUint16(buf[6:8], 16)

	// Reader EntityID
	copy(buf[8:12], readerEID[:])
	// Writer EntityID
	copy(buf[12:16], writerEID[:])

	// Writer Sequence Number (high 32 bits, low 32 bits)
	endian.PutUint32(buf[16:20], uint32(writerSN>>32))
	endian.PutUint32(buf[20:24], uint32(writerSN&0xFFFFFFFF))

	// Serialized data
	copy(buf[24:], payload)

	return buf
}

// announceLoop periodically sends SPDP announcements
func (ds *DiscoveryService) announceLoop(ctx context.Context) {
	// Send immediately on start
	ds.sendSPDPAnnouncement()

	ticker := time.NewTicker(spdpResendPeriod)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			ds.sendSPDPAnnouncement()
		}
	}
}

func (ds *DiscoveryService) sendSPDPAnnouncement() {
	msg := ds.buildSPDPAnnouncement()
	port := spdpMulticastPort(ds.domainID)

	// Send to multicast
	destAddr := &net.UDPAddr{
		IP:   net.ParseIP(ds.multicastAddr),
		Port: port,
	}

	var conn *net.UDPConn
	if ds.spdpMulticastConn != nil {
		conn = ds.spdpMulticastConn
	} else if ds.spdpUnicastConn != nil {
		conn = ds.spdpUnicastConn
	}

	if conn != nil {
		_, err := conn.WriteTo(msg, destAddr)
		if err != nil {
			ds.logger.Debug().Err(err).Msg("Failed to send SPDP announcement")
			return
		}
		ds.metricsMu.Lock()
		ds.spdpSent++
		ds.metricsMu.Unlock()
		ds.logger.Debug().Msg("Sent SPDP announcement")
	}
}

// ═══════════════════════════════════════════════
// SPDP Receive and Parse
// ═══════════════════════════════════════════════

// receiveLoop reads incoming RTPS messages and processes them
func (ds *DiscoveryService) receiveLoop(ctx context.Context, conn *net.UDPConn, kind string) {
	buf := make([]byte, 65536)

	for {
		select {
		case <-ctx.Done():
			return
		default:
		}

		conn.SetReadDeadline(time.Now().Add(5 * time.Second))
		n, remoteAddr, err := conn.ReadFromUDP(buf)
		if err != nil {
			if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
				continue // timeout is expected, just retry
			}
			if ctx.Err() != nil {
				return
			}
			ds.logger.Debug().Err(err).Str("kind", kind).Msg("SPDP receive error")
			continue
		}

		ds.processRTPSMessage(buf[:n], remoteAddr)
	}
}

// processRTPSMessage parses an incoming RTPS message
func (ds *DiscoveryService) processRTPSMessage(data []byte, from *net.UDPAddr) {
	header, err := parseRTPSHeader(data)
	if err != nil {
		ds.logger.Debug().Err(err).Msg("Invalid RTPS header")
		return
	}

	// Ignore our own messages
	if header.GUIDPrefix == ds.guidPrefix {
		return
	}

	// Parse submessages
	offset := 20
	for offset+4 <= len(data) {
		submsgID := data[offset]
		flags := data[offset+1]
		var submsgLen uint16
		if flags&flagEndianness != 0 {
			submsgLen = binary.LittleEndian.Uint16(data[offset+2 : offset+4])
		} else {
			submsgLen = binary.BigEndian.Uint16(data[offset+2 : offset+4])
		}

		submsgStart := offset + 4
		submsgEnd := submsgStart + int(submsgLen)
		if submsgEnd > len(data) {
			break
		}

		switch submsgID {
		case submsgDATA:
			ds.processDataSubmessage(header, data[submsgStart:submsgEnd], flags, from)
		case submsgHEARTBEAT:
			// Process RTPS heartbeat (writer liveness)
			ds.logger.Debug().
				Str("from", fmt.Sprintf("%x", header.GUIDPrefix)).
				Msg("Received RTPS heartbeat")
		}

		offset = submsgEnd
		// Align to 4-byte boundary
		if offset%4 != 0 {
			offset += 4 - (offset % 4)
		}
	}
}

// processDataSubmessage handles a DATA submessage (SPDP or SEDP)
func (ds *DiscoveryService) processDataSubmessage(header *RTPSHeader, data []byte, flags byte, from *net.UDPAddr) {
	if len(data) < 20 {
		return
	}

	var endian binary.ByteOrder
	if flags&flagEndianness != 0 {
		endian = binary.LittleEndian
	} else {
		endian = binary.BigEndian
	}
	_ = endian

	// Extra flags + octets to inline QoS
	// extraFlags := binary.LittleEndian.Uint16(data[0:2])
	// octetToInlineQoS := binary.LittleEndian.Uint16(data[2:4])

	// Reader EntityID
	var readerEID EntityID
	copy(readerEID[:], data[4:8])

	// Writer EntityID
	var writerEID EntityID
	copy(writerEID[:], data[8:12])

	// Determine if this is SPDP or SEDP based on writer EntityID
	writerEIDu32 := binary.BigEndian.Uint32(writerEID[:])

	switch writerEIDu32 {
	case builtinParticipantWriterEntityID:
		// SPDP data
		ds.processSPDPData(header, data[20:], from)
	case builtinPublicationWriterEntityID:
		// SEDP publication data
		ds.processSEDPData(header, data[20:], from, true)
	case builtinSubscriptionWriterEntityID:
		// SEDP subscription data
		ds.processSEDPData(header, data[20:], from, false)
	default:
		// User data (not discovery) - handled elsewhere
	}
}

// processSPDPData parses SPDP participant data
func (ds *DiscoveryService) processSPDPData(header *RTPSHeader, payload []byte, from *net.UDPAddr) {
	ds.metricsMu.Lock()
	ds.spdpReceived++
	ds.metricsMu.Unlock()

	dec, err := NewCDRDecoder(payload)
	if err != nil {
		ds.logger.Debug().Err(err).Msg("Failed to create CDR decoder for SPDP data")
		return
	}

	participantName, err := dec.ReadString()
	if err != nil {
		participantName = fmt.Sprintf("unknown-%x", header.GUIDPrefix[:4])
	}

	ds.participantMu.Lock()
	defer ds.participantMu.Unlock()

	existing, found := ds.participants[header.GUIDPrefix]
	if found {
		existing.LastSeen = time.Now()
		existing.ParticipantName = participantName
		existing.UnicastLocator = from.String()
		ds.logger.Debug().
			Str("name", participantName).
			Str("from", from.String()).
			Msg("Updated existing participant")
	} else {
		participant := &DiscoveredParticipant{
			GUIDPrefix:      header.GUIDPrefix,
			ProtocolVer:     header.ProtocolVersion,
			VendorID:        header.VendorID,
			ParticipantName: participantName,
			UnicastLocator:  from.String(),
			MulticastLocator: fmt.Sprintf("%s:%d", ds.multicastAddr, spdpMulticastPort(ds.domainID)),
			LeaseDuration:   spdpLeaseDuration,
			DiscoveredAt:    time.Now(),
			LastSeen:        time.Now(),
			Endpoints:       make([]DiscoveredEndpoint, 0),
		}
		ds.participants[header.GUIDPrefix] = participant
		ds.logger.Info().
			Str("name", participantName).
			Str("from", from.String()).
			Str("guid", fmt.Sprintf("%x", header.GUIDPrefix)).
			Msg("Discovered new participant")
	}
}

// processSEDPData parses SEDP endpoint data (publication or subscription)
func (ds *DiscoveryService) processSEDPData(header *RTPSHeader, payload []byte, from *net.UDPAddr, isPublication bool) {
	ds.metricsMu.Lock()
	ds.sedpReceived++
	ds.metricsMu.Unlock()

	dec, err := NewCDRDecoder(payload)
	if err != nil {
		return
	}

	topicName, err := dec.ReadString()
	if err != nil {
		return
	}

	typeName, err := dec.ReadString()
	if err != nil {
		typeName = "unknown"
	}

	endpoint := DiscoveredEndpoint{
		TopicName: topicName,
		TypeName:  typeName,
		IsWriter:  isPublication,
		QoS:       QoSSensorData(), // default QoS
	}

	ds.participantMu.Lock()
	defer ds.participantMu.Unlock()

	participant, found := ds.participants[header.GUIDPrefix]
	if !found {
		// Create a placeholder participant entry
		participant = &DiscoveredParticipant{
			GUIDPrefix:      header.GUIDPrefix,
			ProtocolVer:     header.ProtocolVersion,
			VendorID:        header.VendorID,
			ParticipantName: fmt.Sprintf("unknown-%x", header.GUIDPrefix[:4]),
			UnicastLocator:  from.String(),
			LeaseDuration:   spdpLeaseDuration,
			DiscoveredAt:    time.Now(),
			LastSeen:        time.Now(),
			Endpoints:       make([]DiscoveredEndpoint, 0),
		}
		ds.participants[header.GUIDPrefix] = participant
	}

	// Add endpoint if not already known
	found = false
	for _, ep := range participant.Endpoints {
		if ep.TopicName == topicName && ep.IsWriter == isPublication {
			found = true
			break
		}
	}
	if !found {
		participant.Endpoints = append(participant.Endpoints, endpoint)
		kind := "subscriber"
		if isPublication {
			kind = "publisher"
		}
		ds.logger.Info().
			Str("participant", participant.ParticipantName).
			Str("topic", topicName).
			Str("type", typeName).
			Str("kind", kind).
			Msg("Discovered endpoint")
	}

	participant.LastSeen = time.Now()
}

// cleanupLoop removes stale participants periodically
func (ds *DiscoveryService) cleanupLoop(ctx context.Context) {
	ticker := time.NewTicker(30 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			ds.cleanupStaleParticipants()
		}
	}
}

func (ds *DiscoveryService) cleanupStaleParticipants() {
	ds.participantMu.Lock()
	defer ds.participantMu.Unlock()

	for prefix, p := range ds.participants {
		if time.Since(p.LastSeen) > participantStaleTimeout {
			ds.logger.Info().
				Str("name", p.ParticipantName).
				Str("guid", fmt.Sprintf("%x", prefix)).
				Msg("Removing stale participant")
			delete(ds.participants, prefix)
		}
	}
}
