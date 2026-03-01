package dds

import (
	"context"
	"encoding/binary"
	"fmt"
	"net"
	"sync"
	"sync/atomic"
	"time"

	"github.com/rs/zerolog"

	"scada-system/infrastructure/circuitbreaker"
	"scada-system/internal/config"
)

// ═══════════════════════════════════════════════
// DDS Publisher for Control Commands
// ═══════════════════════════════════════════════

// Publisher sends CDR-encoded commands to DDS topics via RTPS/UDP.
// It uses reliable QoS for control messages to ensure delivery
// to the embedded side.
type Publisher struct {
	cfg       config.DDSConfig
	discovery *DiscoveryService
	cb        *circuitbreaker.CircuitBreaker

	conn     *net.UDPConn
	connMu   sync.Mutex

	// Writer sequence numbers per topic (monotonically increasing)
	seqNums  map[DDSTopic]*atomic.Int64
	seqMu    sync.RWMutex

	// Command ID generator
	nextCmdID atomic.Uint32

	running atomic.Bool
	logger  zerolog.Logger

	// Metrics
	messagesSent  atomic.Int64
	encodeErrors  atomic.Int64
	sendErrors    atomic.Int64
	ackReceived   atomic.Int64
}

// NewPublisher creates a new DDS publisher for outbound commands
func NewPublisher(
	cfg config.DDSConfig,
	discovery *DiscoveryService,
	cb *circuitbreaker.CircuitBreaker,
	logger zerolog.Logger,
) *Publisher {
	p := &Publisher{
		cfg:       cfg,
		discovery: discovery,
		cb:        cb,
		seqNums:   make(map[DDSTopic]*atomic.Int64),
		logger:    logger.With().Str("component", "dds-publisher").Logger(),
	}

	// Pre-allocate sequence numbers for publish topics
	for _, topic := range AllPublishTopics() {
		p.seqNums[topic] = &atomic.Int64{}
	}

	return p
}

// Start initializes the publisher's UDP connection
func (p *Publisher) Start(ctx context.Context) error {
	if !p.cfg.Enabled {
		p.logger.Warn().Msg("DDS is disabled, publisher will not start")
		return nil
	}

	// Open a UDP socket for sending data
	localAddr := &net.UDPAddr{IP: net.IPv4zero, Port: 0}
	var err error
	p.conn, err = net.ListenUDP("udp4", localAddr)
	if err != nil {
		return fmt.Errorf("failed to open DDS publisher socket: %w", err)
	}

	p.running.Store(true)

	// Register publish topics for SEDP
	for _, topic := range AllPublishTopics() {
		p.discovery.RegisterLocalTopic(topic, topicTypeName(topic), true)
	}

	p.logger.Info().
		Str("local_addr", p.conn.LocalAddr().String()).
		Int("topic_count", len(AllPublishTopics())).
		Msg("DDS publisher started")

	// Start ACK listener for reliable QoS
	go p.ackListener(ctx)

	return nil
}

// Stop shuts down the publisher
func (p *Publisher) Stop() {
	p.running.Store(false)
	p.connMu.Lock()
	if p.conn != nil {
		p.conn.Close()
	}
	p.connMu.Unlock()
	p.logger.Info().Msg("DDS publisher stopped")
}

// GetMetrics returns publisher statistics
func (p *Publisher) GetMetrics() PublisherMetrics {
	return PublisherMetrics{
		Running:      p.running.Load(),
		MessagesSent: p.messagesSent.Load(),
		EncodeErrors: p.encodeErrors.Load(),
		SendErrors:   p.sendErrors.Load(),
		AckReceived:  p.ackReceived.Load(),
	}
}

// PublisherMetrics holds publisher statistics
type PublisherMetrics struct {
	Running      bool  `json:"running"`
	MessagesSent int64 `json:"messages_sent"`
	EncodeErrors int64 `json:"encode_errors"`
	SendErrors   int64 `json:"send_errors"`
	AckReceived  int64 `json:"ack_received"`
}

// ═══════════════════════════════════════════════
// Publish Commands
// ═══════════════════════════════════════════════

// PublishSolarConfig sends a configuration/control command to the solar subsystem via DDS.
func (p *Publisher) PublishSolarConfig(ctx context.Context, deviceID, action string, params map[string]string, issuedBy string) error {
	cmd := p.buildConfigCommand(deviceID, "solar", action, params, issuedBy, 2) // priority: high
	return p.publishCommand(ctx, TopicSolarConfig, &cmd)
}

// PublishWaterConfig sends a configuration/control command to the water subsystem via DDS.
func (p *Publisher) PublishWaterConfig(ctx context.Context, deviceID, action string, params map[string]string, issuedBy string) error {
	cmd := p.buildConfigCommand(deviceID, "water", action, params, issuedBy, 2) // priority: high
	return p.publishCommand(ctx, TopicWaterConfig, &cmd)
}

// buildConfigCommand creates a ConfigCommand struct ready for CDR serialization
func (p *Publisher) buildConfigCommand(deviceID, subsystem, action string, params map[string]string, issuedBy string, priority uint8) ConfigCommand {
	keys := make([]string, 0, len(params))
	vals := make([]string, 0, len(params))
	for k, v := range params {
		keys = append(keys, k)
		vals = append(vals, v)
	}

	return ConfigCommand{
		Device: DeviceIdentity{
			DeviceID:  deviceID,
			Subsystem: subsystem,
			Location:  "", // filled by the receiver if needed
		},
		Timestamp:   TimestampFromTime(time.Now()),
		CommandID:   p.nextCmdID.Add(1),
		Action:      action,
		ParamCount:  uint32(len(params)),
		ParamKeys:   keys,
		ParamValues: vals,
		IssuedBy:    issuedBy,
		Priority:    priority,
	}
}

// publishCommand serializes a ConfigCommand to CDR and sends it as an RTPS DATA message.
func (p *Publisher) publishCommand(ctx context.Context, topic DDSTopic, cmd *ConfigCommand) error {
	if !p.running.Load() {
		return fmt.Errorf("DDS publisher is not running")
	}

	return p.cb.Execute(ctx, func(cbCtx context.Context) error {
		// CDR-encode the command
		enc := NewCDREncoder()
		cmd.Encode(enc)
		cdrPayload := enc.Bytes()

		// Get and increment sequence number
		p.seqMu.RLock()
		seqCounter, ok := p.seqNums[topic]
		p.seqMu.RUnlock()
		if !ok {
			p.encodeErrors.Add(1)
			return fmt.Errorf("unknown publish topic: %s", topic)
		}
		seqNum := seqCounter.Add(1)

		// Build RTPS message
		msg := p.buildRTPSDataMessage(topic, seqNum, cdrPayload)

		// Determine destination address
		destAddr := p.resolveDestination(topic)
		if destAddr == nil {
			p.sendErrors.Add(1)
			return fmt.Errorf("no destination for topic %s", topic)
		}

		// Send the message
		p.connMu.Lock()
		conn := p.conn
		p.connMu.Unlock()

		if conn == nil {
			p.sendErrors.Add(1)
			return fmt.Errorf("DDS publisher connection is closed")
		}

		_, err := conn.WriteTo(msg, destAddr)
		if err != nil {
			p.sendErrors.Add(1)
			p.logger.Error().Err(err).
				Str("topic", string(topic)).
				Str("device", cmd.Device.DeviceID).
				Str("action", cmd.Action).
				Msg("Failed to send DDS command")
			return fmt.Errorf("send DDS command: %w", err)
		}

		p.messagesSent.Add(1)

		p.logger.Info().
			Str("topic", string(topic)).
			Str("device", cmd.Device.DeviceID).
			Str("action", cmd.Action).
			Uint32("cmd_id", cmd.CommandID).
			Int64("seq_num", seqNum).
			Msg("Published DDS command")

		// For reliable QoS, send a HEARTBEAT submessage so the reader knows we have data
		if QoSForTopic(topic).Reliability.Kind == ReliabilityReliable {
			heartbeatMsg := p.buildHeartbeatSubmessage(topic, seqNum)
			conn.WriteTo(heartbeatMsg, destAddr)
		}

		return nil
	})
}

// buildRTPSDataMessage creates a complete RTPS message with a DATA submessage
func (p *Publisher) buildRTPSDataMessage(topic DDSTopic, seqNum int64, cdrPayload []byte) []byte {
	guidPrefix := p.discovery.GUIDPrefixValue()
	endian := binary.LittleEndian

	// RTPS header
	rtpsHeader := encodeRTPSHeader(guidPrefix)

	// Build inline QoS with topic name
	inlineQoS := p.buildInlineQoS(topic, endian)

	// Writer and reader entity IDs
	writerEID := p.topicToWriterEntityID(topic)
	readerEID := EntityID{0x00, 0x00, 0x00, 0x00} // unknown reader

	// DATA submessage
	flags := flagEndianness | flagDataFlag | flagInlineQoS

	// Fixed fields: extraFlags(2) + octetsToInlineQoS(2) + readerEID(4) + writerEID(4) + seqNum(8) = 20
	fixedLen := 20
	submsgPayloadLen := fixedLen + len(inlineQoS) + len(cdrPayload)

	submsg := make([]byte, 4+submsgPayloadLen)
	submsg[0] = submsgDATA
	submsg[1] = flags
	endian.PutUint16(submsg[2:4], uint16(submsgPayloadLen))

	// Extra flags
	endian.PutUint16(submsg[4:6], 0)
	// Octets to inline QoS (reader+writer EID + seqnum = 16)
	endian.PutUint16(submsg[6:8], 16)

	// Reader EntityID
	copy(submsg[8:12], readerEID[:])
	// Writer EntityID
	copy(submsg[12:16], writerEID[:])

	// Sequence number (high, low)
	endian.PutUint32(submsg[16:20], uint32(seqNum>>32))
	endian.PutUint32(submsg[20:24], uint32(seqNum&0xFFFFFFFF))

	// Inline QoS
	copy(submsg[24:24+len(inlineQoS)], inlineQoS)

	// Serialized payload
	copy(submsg[24+len(inlineQoS):], cdrPayload)

	// Complete RTPS message
	msg := make([]byte, 0, len(rtpsHeader)+len(submsg))
	msg = append(msg, rtpsHeader...)
	msg = append(msg, submsg...)
	return msg
}

// buildInlineQoS creates inline QoS parameters including the topic name
func (p *Publisher) buildInlineQoS(topic DDSTopic, endian binary.ByteOrder) []byte {
	buf := make([]byte, 0, 64)

	// Topic name parameter
	topicNameBytes := []byte(string(topic))
	topicNameLen := uint32(len(topicNameBytes) + 1) // include null terminator
	topicParamDataLen := 4 + len(topicNameBytes) + 1 // length field + string + null
	paddedLen := (topicParamDataLen + 3) & ^3          // align to 4

	header := make([]byte, 4)
	endian.PutUint16(header[0:2], PIDTopicName)
	endian.PutUint16(header[2:4], uint16(paddedLen))
	buf = append(buf, header...)

	data := make([]byte, paddedLen)
	endian.PutUint32(data[0:4], topicNameLen)
	copy(data[4:], topicNameBytes)
	data[4+len(topicNameBytes)] = 0x00 // null terminator
	buf = append(buf, data...)

	// Sentinel
	sentinel := make([]byte, 4)
	endian.PutUint16(sentinel[0:2], PIDSentinel)
	endian.PutUint16(sentinel[2:4], 0)
	buf = append(buf, sentinel...)

	return buf
}

// buildHeartbeatSubmessage creates an RTPS HEARTBEAT submessage for reliable delivery
func (p *Publisher) buildHeartbeatSubmessage(topic DDSTopic, lastSeqNum int64) []byte {
	guidPrefix := p.discovery.GUIDPrefixValue()
	endian := binary.LittleEndian

	rtpsHeader := encodeRTPSHeader(guidPrefix)
	writerEID := p.topicToWriterEntityID(topic)
	readerEID := EntityID{0x00, 0x00, 0x00, 0x00}

	// HEARTBEAT submessage: 28 bytes payload
	// readerEID(4) + writerEID(4) + firstSN(8) + lastSN(8) + count(4) = 28
	submsgLen := uint16(28)
	submsg := make([]byte, 4+submsgLen)
	submsg[0] = submsgHEARTBEAT
	submsg[1] = flagEndianness // little-endian
	endian.PutUint16(submsg[2:4], submsgLen)

	// Reader EntityID
	copy(submsg[4:8], readerEID[:])
	// Writer EntityID
	copy(submsg[8:12], writerEID[:])

	// First available sequence number (high, low)
	endian.PutUint32(submsg[12:16], 0)
	endian.PutUint32(submsg[16:20], 1)

	// Last sequence number (high, low)
	endian.PutUint32(submsg[20:24], uint32(lastSeqNum>>32))
	endian.PutUint32(submsg[24:28], uint32(lastSeqNum&0xFFFFFFFF))

	// Count (incremented each time a heartbeat is sent)
	endian.PutUint32(submsg[28:32], uint32(lastSeqNum))

	msg := make([]byte, 0, len(rtpsHeader)+len(submsg))
	msg = append(msg, rtpsHeader...)
	msg = append(msg, submsg...)
	return msg
}

// topicToWriterEntityID maps a publish topic to its RTPS writer entity ID
func (p *Publisher) topicToWriterEntityID(topic DDSTopic) EntityID {
	switch topic {
	case TopicSolarConfig:
		return EntityID{0x00, 0x01, 0x08, 0x02} // solar config writer
	case TopicWaterConfig:
		return EntityID{0x00, 0x02, 0x08, 0x02} // water config writer
	default:
		return EntityID{0x00, 0x00, 0x00, 0x02} // generic writer
	}
}

// resolveDestination determines the UDP address to send commands to.
// For multicast, uses the configured multicast address. For unicast,
// looks up discovered participants.
func (p *Publisher) resolveDestination(topic DDSTopic) *net.UDPAddr {
	// First, try to find a specific subscriber for this topic via discovery
	participants := p.discovery.GetParticipants()
	for _, participant := range participants {
		for _, ep := range participant.Endpoints {
			if DDSTopic(ep.TopicName) == topic && !ep.IsWriter {
				// Found a subscriber for this topic
				addr, err := net.ResolveUDPAddr("udp4", participant.UnicastLocator)
				if err == nil {
					return addr
				}
			}
		}
	}

	// Fall back to multicast
	dataPort := spdpWellKnownPortBase + p.cfg.DomainID*spdpPortDomainGain + 1
	return &net.UDPAddr{
		IP:   net.ParseIP(p.cfg.MulticastAddr),
		Port: dataPort,
	}
}

// ackListener listens for ACKNACK messages from reliable readers
func (p *Publisher) ackListener(ctx context.Context) {
	if p.conn == nil {
		return
	}

	buf := make([]byte, 4096)
	for p.running.Load() {
		select {
		case <-ctx.Done():
			return
		default:
		}

		p.connMu.Lock()
		conn := p.conn
		p.connMu.Unlock()
		if conn == nil {
			return
		}

		conn.SetReadDeadline(time.Now().Add(5 * time.Second))
		n, _, err := conn.ReadFromUDP(buf)
		if err != nil {
			if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
				continue
			}
			if ctx.Err() != nil {
				return
			}
			continue
		}

		// Check for RTPS ACKNACK submessages
		if n < 24 {
			continue
		}
		_, err = parseRTPSHeader(buf[:n])
		if err != nil {
			continue
		}

		offset := 20
		for offset+4 <= n {
			submsgID := buf[offset]
			submsgFlags := buf[offset+1]
			var endian binary.ByteOrder
			if submsgFlags&flagEndianness != 0 {
				endian = binary.LittleEndian
			} else {
				endian = binary.BigEndian
			}
			submsgLen := endian.Uint16(buf[offset+2 : offset+4])
			if submsgID == submsgACKNACK {
				p.ackReceived.Add(1)
				p.logger.Debug().Msg("Received ACKNACK from reliable reader")
			}
			offset += 4 + int(submsgLen)
			if offset%4 != 0 {
				offset += 4 - (offset % 4)
			}
		}
	}
}
