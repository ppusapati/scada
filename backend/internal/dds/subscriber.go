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
	"scada-system/internal/db/models"
	"scada-system/internal/services"
	"scada-system/internal/websocket"
)

// ═══════════════════════════════════════════════
// DDS Subscriber Service
// ═══════════════════════════════════════════════

// Subscriber connects to the DDS domain via RTPS/UDP and routes
// incoming data to the appropriate backend services. It operates
// alongside the existing MQTT bridge in hybrid mode and falls back
// to MQTT when DDS is unavailable.
type Subscriber struct {
	cfg       config.DDSConfig
	discovery *DiscoveryService
	cb        *circuitbreaker.CircuitBreaker

	// Service dependencies (same as MQTT bridge)
	readings *services.ReadingService
	devices  *services.DeviceService
	water    *services.WaterService
	solar    *services.SolarService
	alarms   *services.AlarmService
	wsHub    *websocket.Hub

	// UDP connections per topic (multicast or unicast)
	dataConn    *net.UDPConn
	dataConnMu  sync.Mutex

	// Topic handlers
	handlers   map[DDSTopic]topicHandler
	handlersMu sync.RWMutex

	// State
	running   atomic.Bool
	connected atomic.Bool
	logger    zerolog.Logger

	// Metrics
	messagesReceived atomic.Int64
	messagesDecoded  atomic.Int64
	decodeErrors     atomic.Int64
	messagesRouted   atomic.Int64
	fallbackToMQTT   atomic.Int64
}

// topicHandler processes decoded data for a specific DDS topic
type topicHandler func(ctx context.Context, payload []byte, from *net.UDPAddr)

// NewSubscriber creates a new DDS subscriber that integrates with the existing
// service layer (reading_service, alarm_service, etc.)
func NewSubscriber(
	cfg config.DDSConfig,
	discovery *DiscoveryService,
	cb *circuitbreaker.CircuitBreaker,
	readings *services.ReadingService,
	devices *services.DeviceService,
	water *services.WaterService,
	solar *services.SolarService,
	alarms *services.AlarmService,
	wsHub *websocket.Hub,
	logger zerolog.Logger,
) *Subscriber {
	s := &Subscriber{
		cfg:       cfg,
		discovery: discovery,
		cb:        cb,
		readings:  readings,
		devices:   devices,
		water:     water,
		solar:     solar,
		alarms:    alarms,
		wsHub:     wsHub,
		handlers:  make(map[DDSTopic]topicHandler),
		logger:    logger.With().Str("component", "dds-subscriber").Logger(),
	}

	// Register handlers for all subscribe topics
	s.registerHandlers()
	return s
}

// registerHandlers sets up the deserialization and routing for each DDS topic
func (s *Subscriber) registerHandlers() {
	// Solar topics
	s.handlers[TopicSolarStringVoltage] = s.handleSolarStringVoltage
	s.handlers[TopicSolarStringCurrent] = s.handleSolarStringCurrent
	s.handlers[TopicSolarBusPower] = s.handleSolarBusPower
	s.handlers[TopicSolarIrradiance] = s.handleSolarIrradiance
	s.handlers[TopicSolarModuleTemp] = s.handleSolarModuleTemp
	s.handlers[TopicSolarAlarm] = s.handleAlarm
	s.handlers[TopicSolarHeartbeat] = s.handleHeartbeat

	// Water topics
	s.handlers[TopicWaterLevel] = s.handleWaterLevel
	s.handlers[TopicWaterFlow] = s.handleWaterFlow
	s.handlers[TopicWaterPressure] = s.handleWaterPressure
	s.handlers[TopicWaterQuality] = s.handleWaterQuality
	s.handlers[TopicWaterPumpStatus] = s.handleWaterPumpStatus
	s.handlers[TopicWaterAlarm] = s.handleAlarm
	s.handlers[TopicWaterHeartbeat] = s.handleHeartbeat
}

// Start begins the DDS subscriber, listening for RTPS data on the configured
// multicast/unicast address.
func (s *Subscriber) Start(ctx context.Context) error {
	if s.running.Load() {
		return fmt.Errorf("DDS subscriber already running")
	}

	s.logger.Info().
		Uint32("domain_id", uint32(s.cfg.DomainID)).
		Str("multicast", s.cfg.MulticastAddr).
		Bool("enabled", s.cfg.Enabled).
		Msg("Starting DDS subscriber")

	if !s.cfg.Enabled {
		s.logger.Warn().Msg("DDS is disabled in configuration, subscriber will not start")
		return nil
	}

	// Bind to the data multicast port (different from SPDP discovery port)
	dataPort := spdpWellKnownPortBase + s.cfg.DomainID*spdpPortDomainGain + 1
	multicastIP := net.ParseIP(s.cfg.MulticastAddr)
	if multicastIP == nil {
		return fmt.Errorf("invalid DDS multicast address: %s", s.cfg.MulticastAddr)
	}

	multicastAddr := &net.UDPAddr{
		IP:   multicastIP,
		Port: dataPort,
	}

	var iface *net.Interface
	if s.cfg.Interface != "" {
		var err error
		iface, err = net.InterfaceByName(s.cfg.Interface)
		if err != nil {
			s.logger.Warn().Err(err).Str("interface", s.cfg.Interface).Msg("Interface not found, using default")
		}
	}

	var err error
	s.dataConn, err = net.ListenMulticastUDP("udp4", iface, multicastAddr)
	if err != nil {
		// Fallback to unicast
		s.logger.Warn().Err(err).Msg("Failed to join data multicast, trying unicast")
		unicastAddr := &net.UDPAddr{IP: net.IPv4zero, Port: dataPort}
		s.dataConn, err = net.ListenUDP("udp4", unicastAddr)
		if err != nil {
			return fmt.Errorf("failed to bind DDS data port %d: %w", dataPort, err)
		}
	}

	s.dataConn.SetReadBuffer(1024 * 1024) // 1MB read buffer for high-throughput
	s.running.Store(true)
	s.connected.Store(true)

	// Start the receive loop
	go s.receiveLoop(ctx)

	// Register local topics for SEDP announcement
	for _, topic := range AllSubscribeTopics() {
		s.discovery.RegisterLocalTopic(topic, topicTypeName(topic), false)
	}

	s.logger.Info().
		Int("data_port", dataPort).
		Int("topic_count", len(s.handlers)).
		Msg("DDS subscriber started")

	return nil
}

// Stop shuts down the subscriber
func (s *Subscriber) Stop() {
	s.running.Store(false)
	s.connected.Store(false)
	s.dataConnMu.Lock()
	if s.dataConn != nil {
		s.dataConn.Close()
	}
	s.dataConnMu.Unlock()
	s.logger.Info().Msg("DDS subscriber stopped")
}

// IsConnected returns whether the subscriber has an active DDS connection
func (s *Subscriber) IsConnected() bool {
	return s.connected.Load()
}

// GetMetrics returns subscriber metrics
func (s *Subscriber) GetMetrics() SubscriberMetrics {
	return SubscriberMetrics{
		Connected:        s.connected.Load(),
		Running:          s.running.Load(),
		MessagesReceived: s.messagesReceived.Load(),
		MessagesDecoded:  s.messagesDecoded.Load(),
		DecodeErrors:     s.decodeErrors.Load(),
		MessagesRouted:   s.messagesRouted.Load(),
		FallbackToMQTT:   s.fallbackToMQTT.Load(),
	}
}

// SubscriberMetrics holds subscriber statistics
type SubscriberMetrics struct {
	Connected        bool  `json:"connected"`
	Running          bool  `json:"running"`
	MessagesReceived int64 `json:"messages_received"`
	MessagesDecoded  int64 `json:"messages_decoded"`
	DecodeErrors     int64 `json:"decode_errors"`
	MessagesRouted   int64 `json:"messages_routed"`
	FallbackToMQTT   int64 `json:"fallback_to_mqtt"`
}

// ═══════════════════════════════════════════════
// RTPS Data Receive Loop
// ═══════════════════════════════════════════════

func (s *Subscriber) receiveLoop(ctx context.Context) {
	buf := make([]byte, 65536)

	for s.running.Load() {
		select {
		case <-ctx.Done():
			s.running.Store(false)
			return
		default:
		}

		s.dataConnMu.Lock()
		conn := s.dataConn
		s.dataConnMu.Unlock()

		if conn == nil {
			time.Sleep(100 * time.Millisecond)
			continue
		}

		conn.SetReadDeadline(time.Now().Add(5 * time.Second))
		n, remoteAddr, err := conn.ReadFromUDP(buf)
		if err != nil {
			if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
				continue
			}
			if ctx.Err() != nil {
				return
			}
			s.logger.Debug().Err(err).Msg("DDS data receive error")
			s.connected.Store(false)
			time.Sleep(1 * time.Second)
			continue
		}

		s.connected.Store(true)
		s.messagesReceived.Add(1)

		// Process through circuit breaker
		_ = s.cb.Execute(ctx, func(cbCtx context.Context) error {
			s.processRTPSDataMessage(cbCtx, buf[:n], remoteAddr)
			return nil
		})
	}
}

// processRTPSDataMessage parses an RTPS message and dispatches to topic handlers
func (s *Subscriber) processRTPSDataMessage(ctx context.Context, data []byte, from *net.UDPAddr) {
	header, err := parseRTPSHeader(data)
	if err != nil {
		return
	}

	// Ignore our own messages
	if header.GUIDPrefix == s.discovery.GUIDPrefixValue() {
		return
	}

	// Parse submessages to extract user data
	offset := 20
	for offset+4 <= len(data) {
		submsgID := data[offset]
		flags := data[offset+1]

		var endian binary.ByteOrder
		if flags&flagEndianness != 0 {
			endian = binary.LittleEndian
		} else {
			endian = binary.BigEndian
		}

		submsgLen := endian.Uint16(data[offset+2 : offset+4])
		submsgStart := offset + 4
		submsgEnd := submsgStart + int(submsgLen)
		if submsgEnd > len(data) {
			break
		}

		if submsgID == submsgDATA {
			s.processUserDataSubmessage(ctx, data[submsgStart:submsgEnd], flags, from)
		}

		offset = submsgEnd
		// Align to 4-byte boundary
		if offset%4 != 0 {
			offset += 4 - (offset % 4)
		}
	}
}

// processUserDataSubmessage extracts topic name and payload from a DATA submessage,
// then dispatches to the appropriate handler
func (s *Subscriber) processUserDataSubmessage(ctx context.Context, data []byte, flags byte, from *net.UDPAddr) {
	if len(data) < 20 {
		return
	}

	var endian binary.ByteOrder
	if flags&flagEndianness != 0 {
		endian = binary.LittleEndian
	} else {
		endian = binary.BigEndian
	}

	// Parse DATA submessage header
	// extraFlags := endian.Uint16(data[0:2])
	octetsToInlineQoS := endian.Uint16(data[2:4])

	// Skip reader/writer entity IDs and sequence number
	payloadStart := 4 + int(octetsToInlineQoS)

	// If inline QoS is present, we need to parse it to find the topic name
	hasInlineQoS := flags&flagInlineQoS != 0
	hasData := flags&flagDataFlag != 0

	topicName := ""

	if hasInlineQoS && payloadStart < len(data) {
		// Parse inline QoS parameters to extract topic name
		qosOffset := payloadStart
		for qosOffset+4 <= len(data) {
			pid := endian.Uint16(data[qosOffset : qosOffset+2])
			plen := endian.Uint16(data[qosOffset+2 : qosOffset+4])
			qosOffset += 4

			if pid == PIDSentinel {
				break
			}

			paramEnd := qosOffset + int(plen)
			if paramEnd > len(data) {
				break
			}

			if pid == PIDTopicName && int(plen) > 4 {
				strLen := endian.Uint32(data[qosOffset : qosOffset+4])
				if strLen > 0 && qosOffset+4+int(strLen) <= paramEnd {
					topicName = string(data[qosOffset+4 : qosOffset+4+int(strLen)-1]) // exclude null terminator
				}
			}

			qosOffset = paramEnd
		}
		payloadStart = qosOffset
	}

	if !hasData || payloadStart >= len(data) {
		return
	}

	payload := data[payloadStart:]

	// If we got a topic name from inline QoS, use it to route
	if topicName != "" {
		s.dispatchToHandler(ctx, DDSTopic(topicName), payload, from)
		return
	}

	// If no inline QoS, try to determine the topic from the writer entity ID
	if len(data) >= 16 {
		var writerEID EntityID
		copy(writerEID[:], data[8:12])
		if topic, ok := s.entityIDToTopic(writerEID); ok {
			s.dispatchToHandler(ctx, topic, payload, from)
		}
	}
}

// entityIDToTopic maps well-known writer entity IDs to topic names.
// In a full DDS implementation this mapping comes from SEDP; here we use
// a simple convention-based mapping for our SCADA system.
func (s *Subscriber) entityIDToTopic(eid EntityID) (DDSTopic, bool) {
	eidVal := binary.BigEndian.Uint32(eid[:])
	// Entity IDs follow a convention: 0xAABBCCKK where KK is the kind
	// We map the upper 3 bytes to topic names
	topicMap := map[uint32]DDSTopic{
		0x00010102: TopicSolarStringVoltage,
		0x00010202: TopicSolarStringCurrent,
		0x00010302: TopicSolarBusPower,
		0x00010402: TopicSolarIrradiance,
		0x00010502: TopicSolarModuleTemp,
		0x00010602: TopicSolarAlarm,
		0x00010702: TopicSolarHeartbeat,
		0x00020102: TopicWaterLevel,
		0x00020202: TopicWaterFlow,
		0x00020302: TopicWaterPressure,
		0x00020402: TopicWaterQuality,
		0x00020502: TopicWaterPumpStatus,
		0x00020602: TopicWaterAlarm,
		0x00020702: TopicWaterHeartbeat,
	}
	topic, ok := topicMap[eidVal]
	return topic, ok
}

// dispatchToHandler routes a payload to the correct topic handler
func (s *Subscriber) dispatchToHandler(ctx context.Context, topic DDSTopic, payload []byte, from *net.UDPAddr) {
	s.handlersMu.RLock()
	handler, ok := s.handlers[topic]
	s.handlersMu.RUnlock()

	if !ok {
		s.logger.Debug().Str("topic", string(topic)).Msg("No handler for DDS topic")
		return
	}

	handler(ctx, payload, from)
}

// ═══════════════════════════════════════════════
// Solar Topic Handlers
// ═══════════════════════════════════════════════

func (s *Subscriber) handleSolarStringVoltage(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data StringVoltageData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode StringVoltageData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	// Store as sensor reading
	reading := &models.SensorReading{
		DeviceID:  deviceID,
		Timestamp: ts,
		Metric:    "voltage",
		Value:     float64(data.Voltage),
		Unit:      "V",
		Quality:   int(data.Quality),
	}
	s.insertReadingAndRoute(ctx, reading, "solar", deviceID, map[string]interface{}{
		"grid_voltage": float64(data.Voltage),
	})
}

func (s *Subscriber) handleSolarStringCurrent(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data StringCurrentData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode StringCurrentData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	reading := &models.SensorReading{
		DeviceID:  deviceID,
		Timestamp: ts,
		Metric:    "current",
		Value:     float64(data.Current),
		Unit:      "A",
		Quality:   int(data.Quality),
	}
	s.insertReadingAndRoute(ctx, reading, "solar", deviceID, map[string]interface{}{
		"panel_current": float64(data.Current),
	})
}

func (s *Subscriber) handleSolarBusPower(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data BusPowerData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode BusPowerData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	// Insert multiple readings for the various metrics
	metrics := []struct {
		name  string
		value float64
		unit  string
	}{
		{"power", float64(data.Power), "kW"},
		{"daily_energy_kwh", data.DailyEnergy, "kWh"},
		{"total_energy_mwh", data.TotalEnergy, "MWh"},
		{"efficiency", float64(data.Efficiency), "%"},
	}

	for _, m := range metrics {
		reading := &models.SensorReading{
			DeviceID:  deviceID,
			Timestamp: ts,
			Metric:    m.name,
			Value:     m.value,
			Unit:      m.unit,
			Quality:   int(data.Quality),
		}
		if err := s.readings.Insert(ctx, reading); err != nil {
			s.logger.Debug().Err(err).Str("metric", m.name).Msg("Failed to insert DDS reading")
		}
	}

	// Update solar system state
	updates := map[string]interface{}{
		"current_output_kw": float64(data.Power),
		"daily_energy_kwh":  data.DailyEnergy,
		"total_energy_mwh":  data.TotalEnergy,
		"efficiency":        float64(data.Efficiency),
	}
	if err := s.solar.UpdateFromMQTT(ctx, deviceID, updates); err != nil {
		s.logger.Debug().Err(err).Msg("Failed to update solar state from DDS")
	}

	s.devices.UpdateStatus(ctx, deviceID, "online")
	s.messagesRouted.Add(1)

	s.wsHub.Broadcast(websocket.Message{
		Type:      "telemetry",
		Subsystem: "solar",
		DeviceID:  deviceID,
		Data: map[string]interface{}{
			"source":           "dds",
			"power":            data.Power,
			"daily_energy_kwh": data.DailyEnergy,
			"total_energy_mwh": data.TotalEnergy,
			"efficiency":       data.Efficiency,
		},
		Timestamp: ts,
	})
}

func (s *Subscriber) handleSolarIrradiance(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data IrradianceData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode IrradianceData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	reading := &models.SensorReading{
		DeviceID:  deviceID,
		Timestamp: ts,
		Metric:    "irradiance",
		Value:     float64(data.Irradiance),
		Unit:      "W/m\u00b2",
		Quality:   int(data.Quality),
	}
	s.insertReadingAndRoute(ctx, reading, "solar", deviceID, map[string]interface{}{
		"irradiance": float64(data.Irradiance),
	})
}

func (s *Subscriber) handleSolarModuleTemp(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data TemperatureData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode TemperatureData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	reading := &models.SensorReading{
		DeviceID:  deviceID,
		Timestamp: ts,
		Metric:    "panel_temperature",
		Value:     float64(data.Temperature),
		Unit:      "\u00b0C",
		Quality:   int(data.Quality),
	}
	s.insertReadingAndRoute(ctx, reading, "solar", deviceID, map[string]interface{}{
		"panel_temperature": float64(data.Temperature),
	})
}

// ═══════════════════════════════════════════════
// Water Topic Handlers
// ═══════════════════════════════════════════════

func (s *Subscriber) handleWaterLevel(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data WaterLevelData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode WaterLevelData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	reading := &models.SensorReading{
		DeviceID:  deviceID,
		Timestamp: ts,
		Metric:    "tank_level",
		Value:     float64(data.Level),
		Unit:      "%",
		Quality:   int(data.Quality),
	}
	s.insertReadingAndRoute(ctx, reading, "water", deviceID, map[string]interface{}{
		"tank_level": float64(data.Level),
	})
}

func (s *Subscriber) handleWaterFlow(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data FlowRateData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode FlowRateData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	metric := "flow_rate_in"
	if data.Direction == 1 {
		metric = "flow_rate_out"
	}

	reading := &models.SensorReading{
		DeviceID:  deviceID,
		Timestamp: ts,
		Metric:    metric,
		Value:     float64(data.FlowRate),
		Unit:      "L/min",
		Quality:   int(data.Quality),
	}
	s.insertReadingAndRoute(ctx, reading, "water", deviceID, map[string]interface{}{
		metric: float64(data.FlowRate),
	})
}

func (s *Subscriber) handleWaterPressure(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data PressureData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode PressureData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	reading := &models.SensorReading{
		DeviceID:  deviceID,
		Timestamp: ts,
		Metric:    "pressure",
		Value:     float64(data.Pressure),
		Unit:      "bar",
		Quality:   int(data.Quality),
	}
	s.insertReadingAndRoute(ctx, reading, "water", deviceID, map[string]interface{}{
		"pressure": float64(data.Pressure),
	})
}

func (s *Subscriber) handleWaterQuality(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data WaterQualityData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode WaterQualityData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	// Insert multiple quality metrics
	metrics := []struct {
		name  string
		value float64
		unit  string
	}{
		{"ph_level", float64(data.PHLevel), "pH"},
		{"turbidity", float64(data.Turbidity), "NTU"},
		{"chlorine_level", float64(data.ChlorineLevel), "mg/L"},
	}

	updates := make(map[string]interface{})
	for _, m := range metrics {
		reading := &models.SensorReading{
			DeviceID:  deviceID,
			Timestamp: ts,
			Metric:    m.name,
			Value:     m.value,
			Unit:      m.unit,
			Quality:   int(data.Quality),
		}
		if err := s.readings.Insert(ctx, reading); err != nil {
			s.logger.Debug().Err(err).Str("metric", m.name).Msg("Failed to insert DDS reading")
		}
		updates[m.name] = m.value
	}

	if err := s.water.UpdateFromMQTT(ctx, deviceID, updates); err != nil {
		s.logger.Debug().Err(err).Msg("Failed to update water state from DDS")
	}
	s.devices.UpdateStatus(ctx, deviceID, "online")
	s.messagesRouted.Add(1)

	s.wsHub.Broadcast(websocket.Message{
		Type:      "telemetry",
		Subsystem: "water",
		DeviceID:  deviceID,
		Data: map[string]interface{}{
			"source":         "dds",
			"ph_level":       data.PHLevel,
			"turbidity":      data.Turbidity,
			"chlorine_level": data.ChlorineLevel,
		},
		Timestamp: ts,
	})
}

func (s *Subscriber) handleWaterPumpStatus(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data PumpStatusData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode PumpStatusData")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID

	statusStr := data.Status.String()

	// Update pump status in water service
	if err := s.water.SetPumpStatus(ctx, deviceID, statusStr); err != nil {
		s.logger.Debug().Err(err).Msg("Failed to update pump status from DDS")
	}
	s.devices.UpdateStatus(ctx, deviceID, "online")
	s.messagesRouted.Add(1)

	// If pump is in fault, raise an alarm
	if data.Status == PumpStatusFault {
		alarm := &models.Alarm{
			DeviceID:  deviceID,
			Subsystem: "water",
			Severity:  "critical",
			Category:  "equipment",
			Message:   fmt.Sprintf("Pump %d fault detected via DDS (current: %.1fA)", data.PumpID, data.Current),
		}
		if err := s.alarms.Create(ctx, alarm); err != nil {
			s.logger.Debug().Err(err).Msg("Failed to create pump fault alarm from DDS")
		}
		s.wsHub.Broadcast(websocket.Message{
			Type:      "alarm",
			Subsystem: "water",
			DeviceID:  deviceID,
			Data:      alarm,
			Timestamp: ts,
		})
	}

	s.wsHub.Broadcast(websocket.Message{
		Type:      "device_status",
		Subsystem: "water",
		DeviceID:  deviceID,
		Data: map[string]interface{}{
			"source":      "dds",
			"pump_status": statusStr,
			"pump_id":     data.PumpID,
			"rpm":         data.RPM,
			"current":     data.Current,
		},
		Timestamp: ts,
	})
}

// ═══════════════════════════════════════════════
// Shared Handlers (Alarm + Heartbeat)
// ═══════════════════════════════════════════════

func (s *Subscriber) handleAlarm(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data AlarmEvent
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode AlarmEvent")
		return
	}
	s.messagesDecoded.Add(1)

	ts := data.Timestamp.ToTime()
	deviceID := data.Device.DeviceID
	subsystem := data.Device.Subsystem

	if data.Active {
		alarm := &models.Alarm{
			DeviceID:  deviceID,
			Subsystem: subsystem,
			Severity:  data.Severity.String(),
			Category:  data.Category,
			Message:   fmt.Sprintf("[DDS] %s", data.Message),
			Value:     data.Value,
			Threshold: data.Threshold,
		}
		if err := s.alarms.Create(ctx, alarm); err != nil {
			s.logger.Debug().Err(err).Msg("Failed to create DDS alarm")
			return
		}
		s.messagesRouted.Add(1)

		s.wsHub.Broadcast(websocket.Message{
			Type:      "alarm",
			Subsystem: subsystem,
			DeviceID:  deviceID,
			Data:      alarm,
			Timestamp: ts,
		})

		s.logger.Warn().
			Str("severity", data.Severity.String()).
			Str("device", deviceID).
			Str("subsystem", subsystem).
			Str("message", data.Message).
			Msg("DDS alarm received")
	}
}

func (s *Subscriber) handleHeartbeat(ctx context.Context, payload []byte, from *net.UDPAddr) {
	var data HeartbeatData
	dec, err := NewCDRDecoder(payload)
	if err != nil {
		s.decodeErrors.Add(1)
		return
	}
	if err := data.Decode(dec); err != nil {
		s.decodeErrors.Add(1)
		s.logger.Debug().Err(err).Msg("Failed to decode HeartbeatData")
		return
	}
	s.messagesDecoded.Add(1)

	deviceID := data.Device.DeviceID
	subsystem := data.Device.Subsystem

	// Update device status to online
	s.devices.UpdateStatus(ctx, deviceID, "online")
	s.messagesRouted.Add(1)

	// Check for degraded status
	statusStr := "online"
	if data.Status == 1 {
		statusStr = "degraded"
	} else if data.Status == 2 {
		statusStr = "fault"
		// Raise alarm for faulted device
		alarm := &models.Alarm{
			DeviceID:  deviceID,
			Subsystem: subsystem,
			Severity:  "critical",
			Category:  "equipment",
			Message:   fmt.Sprintf("[DDS] Device %s heartbeat reports error (uptime: %ds, CPU: %.1f%%)", deviceID, data.UptimeSec, data.CPUUsage),
		}
		_ = s.alarms.Create(ctx, alarm)
	}

	s.wsHub.Broadcast(websocket.Message{
		Type:      "heartbeat",
		Subsystem: subsystem,
		DeviceID:  deviceID,
		Data: map[string]interface{}{
			"source":       "dds",
			"status":       statusStr,
			"sequence":     data.SequenceNum,
			"uptime_sec":   data.UptimeSec,
			"cpu_usage":    data.CPUUsage,
			"mem_usage":    data.MemUsage,
		},
		Timestamp: data.Timestamp.ToTime(),
	})
}

// ═══════════════════════════════════════════════
// Common Helpers
// ═══════════════════════════════════════════════

// insertReadingAndRoute is a helper that inserts a sensor reading into the DB,
// updates the subsystem state, marks the device online, and broadcasts via WS.
func (s *Subscriber) insertReadingAndRoute(
	ctx context.Context,
	reading *models.SensorReading,
	subsystem string,
	deviceID string,
	updates map[string]interface{},
) {
	if err := s.readings.Insert(ctx, reading); err != nil {
		s.logger.Debug().Err(err).
			Str("metric", reading.Metric).
			Str("device", deviceID).
			Msg("Failed to insert DDS reading")
	}

	switch subsystem {
	case "water":
		if err := s.water.UpdateFromMQTT(ctx, deviceID, updates); err != nil {
			s.logger.Debug().Err(err).Msg("Failed to update water state from DDS")
		}
	case "solar":
		if err := s.solar.UpdateFromMQTT(ctx, deviceID, updates); err != nil {
			s.logger.Debug().Err(err).Msg("Failed to update solar state from DDS")
		}
	}

	s.devices.UpdateStatus(ctx, deviceID, "online")
	s.messagesRouted.Add(1)

	s.wsHub.Broadcast(websocket.Message{
		Type:      "telemetry",
		Subsystem: subsystem,
		DeviceID:  deviceID,
		Data: map[string]interface{}{
			"source":  "dds",
			"metric":  reading.Metric,
			"value":   reading.Value,
			"unit":    reading.Unit,
			"quality": reading.Quality,
		},
		Timestamp: reading.Timestamp,
	})
}

// topicTypeName returns the DDS type name for a given topic (used in SEDP)
func topicTypeName(topic DDSTopic) string {
	typeNames := map[DDSTopic]string{
		TopicSolarStringVoltage: "scada::solar::StringVoltageData",
		TopicSolarStringCurrent: "scada::solar::StringCurrentData",
		TopicSolarBusPower:      "scada::solar::BusPowerData",
		TopicSolarIrradiance:    "scada::solar::IrradianceData",
		TopicSolarModuleTemp:    "scada::solar::TemperatureData",
		TopicSolarAlarm:         "scada::AlarmEvent",
		TopicSolarHeartbeat:     "scada::HeartbeatData",
		TopicSolarConfig:        "scada::ConfigCommand",
		TopicWaterLevel:         "scada::water::WaterLevelData",
		TopicWaterFlow:          "scada::water::FlowRateData",
		TopicWaterPressure:      "scada::water::PressureData",
		TopicWaterQuality:       "scada::water::WaterQualityData",
		TopicWaterPumpStatus:    "scada::water::PumpStatusData",
		TopicWaterAlarm:         "scada::AlarmEvent",
		TopicWaterHeartbeat:     "scada::HeartbeatData",
		TopicWaterConfig:        "scada::ConfigCommand",
	}
	if name, ok := typeNames[topic]; ok {
		return name
	}
	return "scada::Unknown"
}
