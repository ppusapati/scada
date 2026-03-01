/// Micro XRCE-DDS Client Implementation
///
/// Implements the DDS-XRCE (eXtremely Resource Constrained Environments) protocol
/// for communicating with a DDS-XRCE Agent running on the gateway/edge device.
///
/// The XRCE protocol is defined in the OMG DDS-XRCE specification (v1.0).
/// It provides a lightweight bridge between resource-constrained devices and
/// a full DDS (Data Distribution Service) network.
///
/// Architecture:
///   [STM32 MCU] ---XRCE---> [DDS-XRCE Agent] ---DDS---> [ROS2/DDS Network]
///       ^                        ^
///       |                        |
///   This module            Runs on gateway
///   (XRCE Client)          (e.g., STM32MP157 A7 core)
///
/// Transport options:
///   1. RPMsg (shared memory) for STM32MP157 M4<->A7 communication
///   2. Serial/UART for standalone nodes
///   3. UDP over W5500 Ethernet
///
/// XRCE Protocol Overview:
///   - Session: maintains state between client and agent
///   - Streams: reliable (with ACK) and best-effort (no ACK)
///   - Objects: DDS entities (participant, topic, publisher, subscriber, datawriter, datareader)
///   - Operations: create, delete, write, read
///
/// CDR (Common Data Representation) serialization is used for data types.

use defmt::{info, warn, error, Format};
use embassy_time::{Timer, Instant, Duration};
use heapless::Vec;

// ============================================================================
// XRCE Protocol Constants
// ============================================================================

/// XRCE message header size (session info + stream + sequence)
const XRCE_HEADER_SIZE: usize = 8;
/// XRCE submessage header size (ID + flags + length)
const XRCE_SUBMSG_HEADER_SIZE: usize = 4;
/// Maximum XRCE message payload (fits in single UDP packet or serial frame)
const XRCE_MAX_MSG_SIZE: usize = 512;
/// Maximum CDR-serialized data payload
const XRCE_MAX_DATA_SIZE: usize = 256;

/// XRCE client key (unique per client)
const XRCE_CLIENT_KEY: [u8; 4] = [0xAA, 0xBB, 0xCC, 0x01];

// ============================================================================
// XRCE Submessage IDs
// ============================================================================

#[allow(dead_code)]
pub mod submsg_id {
    pub const CREATE_CLIENT: u8 = 0x00;
    pub const CREATE: u8 = 0x01;
    pub const GET_INFO: u8 = 0x02;
    pub const DELETE: u8 = 0x03;
    pub const STATUS_AGENT: u8 = 0x04;
    pub const STATUS: u8 = 0x05;
    pub const INFO: u8 = 0x06;
    pub const WRITE_DATA: u8 = 0x07;
    pub const READ_DATA: u8 = 0x08;
    pub const DATA: u8 = 0x09;
    pub const ACKNACK: u8 = 0x0A;
    pub const HEARTBEAT: u8 = 0x0B;
    pub const RESET: u8 = 0x0C;
    pub const FRAGMENT: u8 = 0x0D;
    pub const TIMESTAMP: u8 = 0x0E;
    pub const TIMESTAMP_REPLY: u8 = 0x0F;
}

// ============================================================================
// XRCE Object Types
// ============================================================================

#[derive(Clone, Copy, PartialEq, Format)]
pub enum ObjectKind {
    Participant = 0x01,
    Topic = 0x02,
    Publisher = 0x03,
    Subscriber = 0x04,
    DataWriter = 0x05,
    DataReader = 0x06,
}

// ============================================================================
// XRCE Object IDs
// ============================================================================

/// Object ID (2 bytes) used to reference DDS entities
#[derive(Clone, Copy, PartialEq)]
pub struct ObjectId(pub u16);

impl ObjectId {
    pub const PARTICIPANT: Self = ObjectId(0x0001);
    pub const fn topic(n: u16) -> Self { ObjectId(0x0100 | n) }
    pub const fn publisher(n: u16) -> Self { ObjectId(0x0200 | n) }
    pub const fn subscriber(n: u16) -> Self { ObjectId(0x0300 | n) }
    pub const fn datawriter(n: u16) -> Self { ObjectId(0x0400 | n) }
    pub const fn datareader(n: u16) -> Self { ObjectId(0x0500 | n) }
}

// ============================================================================
// XRCE QoS Profiles
// ============================================================================

/// Quality of Service profile for DDS communication
#[derive(Clone, Copy, PartialEq, Format)]
pub enum QosProfile {
    /// Best-effort delivery (no acknowledgment, lowest latency)
    /// Used for: periodic sensor telemetry, heartbeats
    BestEffort,
    /// Reliable delivery (acknowledged, retransmitted on loss)
    /// Used for: alarms, commands, configuration changes
    Reliable,
    /// Transient-local durability (late-joiners get last value)
    /// Used for: device status, configuration state
    TransientLocal,
}

// ============================================================================
// XRCE Session state
// ============================================================================

/// XRCE session state machine
#[derive(Clone, Copy, PartialEq, Format)]
pub enum SessionState {
    /// No session established
    Disconnected,
    /// CREATE_CLIENT sent, waiting for STATUS_AGENT
    Connecting,
    /// Session is active
    Connected,
    /// Session error, needs reconnect
    Error,
}

// ============================================================================
// CDR-serializable sensor data types
// ============================================================================

/// CDR serialization helper
pub struct CdrSerializer<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> CdrSerializer<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Write CDR encapsulation header (LE, no options)
    pub fn write_encapsulation(&mut self) {
        if self.pos + 4 <= self.buf.len() {
            self.buf[self.pos] = 0x00; // CDR_LE
            self.buf[self.pos + 1] = 0x01;
            self.buf[self.pos + 2] = 0x00; // options
            self.buf[self.pos + 3] = 0x00;
            self.pos += 4;
        }
    }

    /// Align to n-byte boundary (2 or 4)
    pub fn align(&mut self, n: usize) {
        let rem = self.pos % n;
        if rem != 0 {
            let pad = n - rem;
            for _ in 0..pad {
                if self.pos < self.buf.len() {
                    self.buf[self.pos] = 0;
                    self.pos += 1;
                }
            }
        }
    }

    pub fn write_u8(&mut self, v: u8) {
        if self.pos < self.buf.len() {
            self.buf[self.pos] = v;
            self.pos += 1;
        }
    }

    pub fn write_u16(&mut self, v: u16) {
        self.align(2);
        if self.pos + 2 <= self.buf.len() {
            let bytes = v.to_le_bytes();
            self.buf[self.pos..self.pos + 2].copy_from_slice(&bytes);
            self.pos += 2;
        }
    }

    pub fn write_u32(&mut self, v: u32) {
        self.align(4);
        if self.pos + 4 <= self.buf.len() {
            let bytes = v.to_le_bytes();
            self.buf[self.pos..self.pos + 4].copy_from_slice(&bytes);
            self.pos += 4;
        }
    }

    pub fn write_u64(&mut self, v: u64) {
        self.align(4); // CDR aligns to 8 for u64, but XCDR1 uses 4
        if self.pos + 8 <= self.buf.len() {
            let bytes = v.to_le_bytes();
            self.buf[self.pos..self.pos + 8].copy_from_slice(&bytes);
            self.pos += 8;
        }
    }

    pub fn write_i32(&mut self, v: i32) {
        self.write_u32(v as u32);
    }

    pub fn write_f32(&mut self, v: f32) {
        self.write_u32(v.to_bits());
    }

    pub fn write_f64(&mut self, v: f64) {
        self.write_u64(v.to_bits());
    }

    pub fn write_bool(&mut self, v: bool) {
        self.write_u8(if v { 1 } else { 0 });
    }

    /// Write a CDR string (length-prefixed, null-terminated)
    pub fn write_string(&mut self, s: &str) {
        let len = s.len() + 1; // Include null terminator
        self.write_u32(len as u32);
        let bytes = s.as_bytes();
        let copy_len = bytes.len().min(self.buf.len().saturating_sub(self.pos));
        self.buf[self.pos..self.pos + copy_len].copy_from_slice(&bytes[..copy_len]);
        self.pos += copy_len;
        if self.pos < self.buf.len() {
            self.buf[self.pos] = 0; // Null terminator
            self.pos += 1;
        }
    }

    /// Get the number of bytes written
    pub fn len(&self) -> usize {
        self.pos
    }
}

// ── Sensor Data Types (CDR-serializable) ──

/// Solar string voltage measurement
#[derive(Clone, Copy)]
pub struct StringVoltageData {
    pub string_id: u16,
    pub voltage_v: f32,
    pub timestamp_ns: u64,
    pub quality: u8,
}

impl StringVoltageData {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let mut cdr = CdrSerializer::new(buf);
        cdr.write_encapsulation();
        cdr.write_u16(self.string_id);
        cdr.write_f32(self.voltage_v);
        cdr.write_u64(self.timestamp_ns);
        cdr.write_u8(self.quality);
        cdr.len()
    }
}

/// Solar string current measurement
#[derive(Clone, Copy)]
pub struct StringCurrentData {
    pub string_id: u16,
    pub current_a: f32,
    pub timestamp_ns: u64,
    pub quality: u8,
}

impl StringCurrentData {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let mut cdr = CdrSerializer::new(buf);
        cdr.write_encapsulation();
        cdr.write_u16(self.string_id);
        cdr.write_f32(self.current_a);
        cdr.write_u64(self.timestamp_ns);
        cdr.write_u8(self.quality);
        cdr.len()
    }
}

/// Bus power measurement (aggregated)
#[derive(Clone, Copy)]
pub struct BusPowerData {
    pub bus_id: u16,
    pub voltage_v: f32,
    pub current_a: f32,
    pub power_w: f32,
    pub energy_wh: f32,
    pub timestamp_ns: u64,
    pub quality: u8,
}

impl BusPowerData {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let mut cdr = CdrSerializer::new(buf);
        cdr.write_encapsulation();
        cdr.write_u16(self.bus_id);
        cdr.write_f32(self.voltage_v);
        cdr.write_f32(self.current_a);
        cdr.write_f32(self.power_w);
        cdr.write_f32(self.energy_wh);
        cdr.write_u64(self.timestamp_ns);
        cdr.write_u8(self.quality);
        cdr.len()
    }
}

/// Water level measurement
#[derive(Clone, Copy)]
pub struct WaterLevelData {
    pub tank_id: u16,
    pub level_pct: f32,
    pub level_m: f32,
    pub timestamp_ns: u64,
    pub quality: u8,
    pub overflow: bool,
}

impl WaterLevelData {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let mut cdr = CdrSerializer::new(buf);
        cdr.write_encapsulation();
        cdr.write_u16(self.tank_id);
        cdr.write_f32(self.level_pct);
        cdr.write_f32(self.level_m);
        cdr.write_u64(self.timestamp_ns);
        cdr.write_u8(self.quality);
        cdr.write_bool(self.overflow);
        cdr.len()
    }
}

/// Flow rate measurement
#[derive(Clone, Copy)]
pub struct FlowRateData {
    pub sensor_id: u16,
    pub flow_lpm: f32,
    pub total_volume_l: f32,
    pub timestamp_ns: u64,
    pub quality: u8,
}

impl FlowRateData {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let mut cdr = CdrSerializer::new(buf);
        cdr.write_encapsulation();
        cdr.write_u16(self.sensor_id);
        cdr.write_f32(self.flow_lpm);
        cdr.write_f32(self.total_volume_l);
        cdr.write_u64(self.timestamp_ns);
        cdr.write_u8(self.quality);
        cdr.len()
    }
}

/// Pressure measurement
#[derive(Clone, Copy)]
pub struct PressureData {
    pub sensor_id: u16,
    pub pressure_bar: f32,
    pub timestamp_ns: u64,
    pub quality: u8,
}

impl PressureData {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let mut cdr = CdrSerializer::new(buf);
        cdr.write_encapsulation();
        cdr.write_u16(self.sensor_id);
        cdr.write_f32(self.pressure_bar);
        cdr.write_u64(self.timestamp_ns);
        cdr.write_u8(self.quality);
        cdr.len()
    }
}

/// Alarm event
#[derive(Clone, Copy)]
pub struct AlarmEvent {
    pub alarm_id: u32,
    pub severity: u8,      // 0=info, 1=warning, 2=critical, 3=emergency
    pub source_id: u16,
    pub alarm_code: u16,
    pub value: f32,
    pub threshold: f32,
    pub timestamp_ns: u64,
    pub message: [u8; 64],  // Fixed-size message buffer
    pub message_len: u8,
}

impl AlarmEvent {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let mut cdr = CdrSerializer::new(buf);
        cdr.write_encapsulation();
        cdr.write_u32(self.alarm_id);
        cdr.write_u8(self.severity);
        cdr.write_u16(self.source_id);
        cdr.write_u16(self.alarm_code);
        cdr.write_f32(self.value);
        cdr.write_f32(self.threshold);
        cdr.write_u64(self.timestamp_ns);
        // Write message as CDR string
        let msg_str = core::str::from_utf8(&self.message[..self.message_len as usize])
            .unwrap_or("unknown");
        cdr.write_string(msg_str);
        cdr.len()
    }
}

/// Heartbeat / device status
#[derive(Clone, Copy)]
pub struct HeartbeatData {
    pub device_id: u16,
    pub uptime_s: u32,
    pub cpu_temp_c: f32,
    pub vcc_v: f32,
    pub sensor_ok: bool,
    pub network_ok: bool,
    pub sd_ok: bool,
    pub fault_count: u16,
    pub timestamp_ns: u64,
}

impl HeartbeatData {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let mut cdr = CdrSerializer::new(buf);
        cdr.write_encapsulation();
        cdr.write_u16(self.device_id);
        cdr.write_u32(self.uptime_s);
        cdr.write_f32(self.cpu_temp_c);
        cdr.write_f32(self.vcc_v);
        cdr.write_bool(self.sensor_ok);
        cdr.write_bool(self.network_ok);
        cdr.write_bool(self.sd_ok);
        cdr.write_u16(self.fault_count);
        cdr.write_u64(self.timestamp_ns);
        cdr.len()
    }
}

// ============================================================================
// XRCE Transport abstraction
// ============================================================================

/// XRCE transport error
#[derive(Clone, Copy, Debug, Format)]
pub enum XrceError {
    /// Transport layer send error
    SendError,
    /// Transport layer receive error
    RecvError,
    /// Session not established
    NotConnected,
    /// Agent returned error status
    AgentError(u8),
    /// Timeout waiting for response
    Timeout,
    /// Message too large for transport
    MessageTooLarge,
    /// Object creation failed
    CreateFailed,
    /// Serialization error
    SerializationError,
    /// Invalid response from agent
    InvalidResponse,
}

/// Transport type selection
#[derive(Clone, Copy, PartialEq, Format)]
pub enum TransportKind {
    /// RPMsg shared memory (STM32MP157 M4 <-> A7)
    RPMsg,
    /// Serial UART
    Serial,
    /// UDP over network (W5500 or native Ethernet)
    Udp,
}

/// Transport configuration
pub struct TransportConfig {
    pub kind: TransportKind,
    /// Agent IP address (for UDP transport)
    pub agent_ip: [u8; 4],
    /// Agent port (for UDP transport)
    pub agent_port: u16,
    /// Local port (for UDP transport)
    pub local_port: u16,
    /// RPMsg channel ID (for RPMsg transport)
    pub rpmsg_channel: u32,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            kind: TransportKind::Udp,
            agent_ip: [192, 168, 1, 1],
            agent_port: 2019,  // Default XRCE-DDS Agent port
            local_port: 2020,
            rpmsg_channel: 0,
        }
    }
}

// ============================================================================
// XRCE Message Builder
// ============================================================================

/// XRCE message buffer with header/submessage construction
struct XrceMessage {
    buf: [u8; XRCE_MAX_MSG_SIZE],
    len: usize,
}

impl XrceMessage {
    fn new() -> Self {
        Self {
            buf: [0u8; XRCE_MAX_MSG_SIZE],
            len: 0,
        }
    }

    /// Write the XRCE session header
    /// Format: [session_id, stream_id, seq_num_hi, seq_num_lo, client_key(4)]
    fn write_header(&mut self, session_id: u8, stream_id: u8, seq_num: u16) {
        self.buf[0] = session_id;
        self.buf[1] = stream_id;
        self.buf[2] = (seq_num >> 8) as u8;
        self.buf[3] = (seq_num & 0xFF) as u8;
        self.buf[4..8].copy_from_slice(&XRCE_CLIENT_KEY);
        self.len = XRCE_HEADER_SIZE;
    }

    /// Append a submessage header
    /// Format: [submsg_id, flags, length_hi, length_lo]
    fn append_submsg_header(&mut self, submsg_id: u8, flags: u8, payload_len: u16) {
        if self.len + XRCE_SUBMSG_HEADER_SIZE <= XRCE_MAX_MSG_SIZE {
            self.buf[self.len] = submsg_id;
            self.buf[self.len + 1] = flags;
            self.buf[self.len + 2] = (payload_len >> 8) as u8;
            self.buf[self.len + 3] = (payload_len & 0xFF) as u8;
            self.len += XRCE_SUBMSG_HEADER_SIZE;
        }
    }

    /// Append raw bytes to the message
    fn append_bytes(&mut self, data: &[u8]) {
        let copy_len = data.len().min(XRCE_MAX_MSG_SIZE - self.len);
        self.buf[self.len..self.len + copy_len].copy_from_slice(&data[..copy_len]);
        self.len += copy_len;
    }

    /// Append a u16 in big-endian
    fn append_u16_be(&mut self, v: u16) {
        let bytes = v.to_be_bytes();
        self.append_bytes(&bytes);
    }

    /// Append a u32 in big-endian
    fn append_u32_be(&mut self, v: u32) {
        let bytes = v.to_be_bytes();
        self.append_bytes(&bytes);
    }

    /// Get the constructed message
    fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

// ============================================================================
// XRCE-DDS Client
// ============================================================================

/// Micro XRCE-DDS client
///
/// Manages a session with a DDS-XRCE Agent and provides methods to:
///   - Create DDS entities (participant, topic, publisher, datawriter)
///   - Publish sensor data with CDR serialization
///   - Subscribe to configuration/command topics
///   - Manage QoS per-topic
pub struct XrceClient {
    /// Current session state
    state: SessionState,
    /// Session ID assigned by the agent
    session_id: u8,
    /// Sequence number for reliable stream
    reliable_seq: u16,
    /// Sequence number for best-effort stream
    besteffort_seq: u16,
    /// Request ID counter
    request_id: u16,
    /// Transport configuration
    transport_config: TransportConfig,
    /// Transmit buffer
    tx_buf: [u8; XRCE_MAX_MSG_SIZE],
    /// Receive buffer
    rx_buf: [u8; XRCE_MAX_MSG_SIZE],
    /// Number of created objects (for cleanup)
    object_count: u8,
}

/// Stream IDs
const STREAM_NONE: u8 = 0x00;
const STREAM_BESTEFFORT: u8 = 0x01;
const STREAM_RELIABLE: u8 = 0x80;

impl XrceClient {
    /// Create a new XRCE-DDS client
    pub fn new(transport_config: TransportConfig) -> Self {
        Self {
            state: SessionState::Disconnected,
            session_id: 0x01,
            reliable_seq: 0,
            besteffort_seq: 0,
            request_id: 0,
            transport_config,
            tx_buf: [0u8; XRCE_MAX_MSG_SIZE],
            rx_buf: [0u8; XRCE_MAX_MSG_SIZE],
            object_count: 0,
        }
    }

    fn next_request_id(&mut self) -> u16 {
        self.request_id = self.request_id.wrapping_add(1);
        self.request_id
    }

    fn next_reliable_seq(&mut self) -> u16 {
        let seq = self.reliable_seq;
        self.reliable_seq = self.reliable_seq.wrapping_add(1);
        seq
    }

    fn next_besteffort_seq(&mut self) -> u16 {
        let seq = self.besteffort_seq;
        self.besteffort_seq = self.besteffort_seq.wrapping_add(1);
        seq
    }

    /// Create a session with the DDS-XRCE Agent
    ///
    /// Sends a CREATE_CLIENT submessage and waits for STATUS_AGENT response.
    /// The `send_fn` and `recv_fn` are transport-specific callbacks.
    pub async fn create_session<S, R>(
        &mut self,
        send_fn: &mut S,
        recv_fn: &mut R,
    ) -> Result<(), XrceError>
    where
        S: FnMut(&[u8]) -> Result<(), XrceError>,
        R: FnMut(&mut [u8], u32) -> Result<usize, XrceError>,
    {
        self.state = SessionState::Connecting;

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_NONE, 0);

        // CREATE_CLIENT submessage
        // Payload: [client_key(4), session_id, properties...]
        let payload_len = 7u16;
        msg.append_submsg_header(submsg_id::CREATE_CLIENT, 0x00, payload_len);
        msg.append_bytes(&XRCE_CLIENT_KEY);
        msg.append_bytes(&[self.session_id]);
        msg.append_u16_be(0x0000); // No properties

        send_fn(msg.as_bytes())?;

        // Wait for STATUS_AGENT response
        let rx_len = recv_fn(&mut self.rx_buf, 5000)?;
        if rx_len < XRCE_HEADER_SIZE + XRCE_SUBMSG_HEADER_SIZE + 2 {
            return Err(XrceError::InvalidResponse);
        }

        // Parse response: check for STATUS_AGENT submessage
        let submsg_id_byte = self.rx_buf[XRCE_HEADER_SIZE];
        if submsg_id_byte != submsg_id::STATUS_AGENT {
            warn!("XRCE: expected STATUS_AGENT, got 0x{:02X}", submsg_id_byte);
            self.state = SessionState::Error;
            return Err(XrceError::InvalidResponse);
        }

        // Check status code (0x00 = OK)
        let status_offset = XRCE_HEADER_SIZE + XRCE_SUBMSG_HEADER_SIZE;
        let status_code = self.rx_buf[status_offset];
        if status_code != 0x00 {
            error!("XRCE: session creation failed, status=0x{:02X}", status_code);
            self.state = SessionState::Error;
            return Err(XrceError::AgentError(status_code));
        }

        self.state = SessionState::Connected;
        self.reliable_seq = 0;
        self.besteffort_seq = 0;
        info!("XRCE: session created (id={})", self.session_id);

        Ok(())
    }

    /// Delete the current session
    pub async fn delete_session<S>(
        &mut self,
        send_fn: &mut S,
    ) -> Result<(), XrceError>
    where
        S: FnMut(&[u8]) -> Result<(), XrceError>,
    {
        if self.state != SessionState::Connected {
            return Ok(());
        }

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_NONE, 0);
        msg.append_submsg_header(submsg_id::DELETE, 0x00, 6);
        msg.append_u16_be(self.next_request_id());
        msg.append_u16_be(0x0000); // Object ID = client itself
        msg.append_u16_be(0x0000); // Padding

        send_fn(msg.as_bytes())?;

        self.state = SessionState::Disconnected;
        self.object_count = 0;
        info!("XRCE: session deleted");
        Ok(())
    }

    /// Create a DDS Participant entity
    pub async fn create_participant<S, R>(
        &mut self,
        send_fn: &mut S,
        recv_fn: &mut R,
        domain_id: u16,
    ) -> Result<ObjectId, XrceError>
    where
        S: FnMut(&[u8]) -> Result<(), XrceError>,
        R: FnMut(&mut [u8], u32) -> Result<usize, XrceError>,
    {
        self.check_connected()?;

        let object_id = ObjectId::PARTICIPANT;
        let req_id = self.next_request_id();
        let seq = self.next_reliable_seq();

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_RELIABLE, seq);

        // CREATE submessage for Participant
        // Payload: [request_id(2), object_id(2), object_kind(1), representation...]
        let mut payload = [0u8; 32];
        let mut pos = 0;
        // Request ID
        payload[pos] = (req_id >> 8) as u8; payload[pos + 1] = (req_id & 0xFF) as u8;
        pos += 2;
        // Object ID
        payload[pos] = (object_id.0 >> 8) as u8; payload[pos + 1] = (object_id.0 & 0xFF) as u8;
        pos += 2;
        // Object kind (Participant = 0x01)
        payload[pos] = ObjectKind::Participant as u8;
        pos += 1;
        // Domain ID
        payload[pos] = (domain_id >> 8) as u8; payload[pos + 1] = (domain_id & 0xFF) as u8;
        pos += 2;
        // Participant reference (BY_REFERENCE, XML string)
        payload[pos] = 0x00; // BY_REFERENCE
        pos += 1;

        msg.append_submsg_header(submsg_id::CREATE, 0x00, pos as u16);
        msg.append_bytes(&payload[..pos]);

        send_fn(msg.as_bytes())?;

        // Wait for STATUS response
        self.wait_for_status(recv_fn, req_id, 3000).await?;

        self.object_count += 1;
        info!("XRCE: participant created (domain={})", domain_id);
        Ok(object_id)
    }

    /// Create a DDS Topic
    pub async fn create_topic<S, R>(
        &mut self,
        send_fn: &mut S,
        recv_fn: &mut R,
        topic_id: ObjectId,
        topic_name: &str,
        type_name: &str,
    ) -> Result<ObjectId, XrceError>
    where
        S: FnMut(&[u8]) -> Result<(), XrceError>,
        R: FnMut(&mut [u8], u32) -> Result<usize, XrceError>,
    {
        self.check_connected()?;

        let req_id = self.next_request_id();
        let seq = self.next_reliable_seq();

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_RELIABLE, seq);

        // Build CREATE for Topic
        let mut payload = [0u8; 128];
        let mut pos = 0;
        // Request ID
        payload[pos] = (req_id >> 8) as u8; payload[pos + 1] = (req_id & 0xFF) as u8;
        pos += 2;
        // Object ID
        payload[pos] = (topic_id.0 >> 8) as u8; payload[pos + 1] = (topic_id.0 & 0xFF) as u8;
        pos += 2;
        // Object kind
        payload[pos] = ObjectKind::Topic as u8;
        pos += 1;
        // Participant ID reference
        payload[pos] = (ObjectId::PARTICIPANT.0 >> 8) as u8;
        payload[pos + 1] = (ObjectId::PARTICIPANT.0 & 0xFF) as u8;
        pos += 2;
        // Topic name (length-prefixed)
        let name_bytes = topic_name.as_bytes();
        payload[pos] = (name_bytes.len() >> 8) as u8;
        payload[pos + 1] = (name_bytes.len() & 0xFF) as u8;
        pos += 2;
        let copy_len = name_bytes.len().min(128 - pos);
        payload[pos..pos + copy_len].copy_from_slice(&name_bytes[..copy_len]);
        pos += copy_len;
        // Type name
        let type_bytes = type_name.as_bytes();
        payload[pos] = (type_bytes.len() >> 8) as u8;
        payload[pos + 1] = (type_bytes.len() & 0xFF) as u8;
        pos += 2;
        let copy_len = type_bytes.len().min(128 - pos);
        payload[pos..pos + copy_len].copy_from_slice(&type_bytes[..copy_len]);
        pos += copy_len;

        msg.append_submsg_header(submsg_id::CREATE, 0x00, pos as u16);
        msg.append_bytes(&payload[..pos]);

        send_fn(msg.as_bytes())?;
        self.wait_for_status(recv_fn, req_id, 3000).await?;

        self.object_count += 1;
        info!("XRCE: topic created ({})", topic_name);
        Ok(topic_id)
    }

    /// Create a Publisher + DataWriter pair
    pub async fn create_datawriter<S, R>(
        &mut self,
        send_fn: &mut S,
        recv_fn: &mut R,
        writer_id: ObjectId,
        topic_id: ObjectId,
        qos: QosProfile,
    ) -> Result<ObjectId, XrceError>
    where
        S: FnMut(&[u8]) -> Result<(), XrceError>,
        R: FnMut(&mut [u8], u32) -> Result<usize, XrceError>,
    {
        self.check_connected()?;

        // Create publisher first
        let pub_id = ObjectId::publisher(writer_id.0 & 0xFF);
        let req_id = self.next_request_id();
        let seq = self.next_reliable_seq();

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_RELIABLE, seq);

        let mut payload = [0u8; 16];
        let mut pos = 0;
        payload[pos] = (req_id >> 8) as u8; payload[pos + 1] = (req_id & 0xFF) as u8;
        pos += 2;
        payload[pos] = (pub_id.0 >> 8) as u8; payload[pos + 1] = (pub_id.0 & 0xFF) as u8;
        pos += 2;
        payload[pos] = ObjectKind::Publisher as u8;
        pos += 1;
        // Participant reference
        payload[pos] = (ObjectId::PARTICIPANT.0 >> 8) as u8;
        payload[pos + 1] = (ObjectId::PARTICIPANT.0 & 0xFF) as u8;
        pos += 2;

        msg.append_submsg_header(submsg_id::CREATE, 0x00, pos as u16);
        msg.append_bytes(&payload[..pos]);
        send_fn(msg.as_bytes())?;
        self.wait_for_status(recv_fn, req_id, 3000).await?;
        self.object_count += 1;

        // Create datawriter
        let req_id = self.next_request_id();
        let seq = self.next_reliable_seq();

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_RELIABLE, seq);

        let mut payload = [0u8; 24];
        let mut pos = 0;
        payload[pos] = (req_id >> 8) as u8; payload[pos + 1] = (req_id & 0xFF) as u8;
        pos += 2;
        payload[pos] = (writer_id.0 >> 8) as u8; payload[pos + 1] = (writer_id.0 & 0xFF) as u8;
        pos += 2;
        payload[pos] = ObjectKind::DataWriter as u8;
        pos += 1;
        // Publisher reference
        payload[pos] = (pub_id.0 >> 8) as u8; payload[pos + 1] = (pub_id.0 & 0xFF) as u8;
        pos += 2;
        // Topic reference
        payload[pos] = (topic_id.0 >> 8) as u8; payload[pos + 1] = (topic_id.0 & 0xFF) as u8;
        pos += 2;
        // QoS profile encoding
        payload[pos] = match qos {
            QosProfile::BestEffort => 0x01,
            QosProfile::Reliable => 0x02,
            QosProfile::TransientLocal => 0x03,
        };
        pos += 1;

        msg.append_submsg_header(submsg_id::CREATE, 0x00, pos as u16);
        msg.append_bytes(&payload[..pos]);
        send_fn(msg.as_bytes())?;
        self.wait_for_status(recv_fn, req_id, 3000).await?;
        self.object_count += 1;

        info!("XRCE: datawriter created (qos={})", qos);
        Ok(writer_id)
    }

    /// Create a Subscriber + DataReader pair
    pub async fn create_datareader<S, R>(
        &mut self,
        send_fn: &mut S,
        recv_fn: &mut R,
        reader_id: ObjectId,
        topic_id: ObjectId,
        qos: QosProfile,
    ) -> Result<ObjectId, XrceError>
    where
        S: FnMut(&[u8]) -> Result<(), XrceError>,
        R: FnMut(&mut [u8], u32) -> Result<usize, XrceError>,
    {
        self.check_connected()?;

        // Create subscriber
        let sub_id = ObjectId::subscriber(reader_id.0 & 0xFF);
        let req_id = self.next_request_id();
        let seq = self.next_reliable_seq();

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_RELIABLE, seq);

        let mut payload = [0u8; 16];
        let mut pos = 0;
        payload[pos] = (req_id >> 8) as u8; payload[pos + 1] = (req_id & 0xFF) as u8;
        pos += 2;
        payload[pos] = (sub_id.0 >> 8) as u8; payload[pos + 1] = (sub_id.0 & 0xFF) as u8;
        pos += 2;
        payload[pos] = ObjectKind::Subscriber as u8;
        pos += 1;
        payload[pos] = (ObjectId::PARTICIPANT.0 >> 8) as u8;
        payload[pos + 1] = (ObjectId::PARTICIPANT.0 & 0xFF) as u8;
        pos += 2;

        msg.append_submsg_header(submsg_id::CREATE, 0x00, pos as u16);
        msg.append_bytes(&payload[..pos]);
        send_fn(msg.as_bytes())?;
        self.wait_for_status(recv_fn, req_id, 3000).await?;
        self.object_count += 1;

        // Create datareader
        let req_id = self.next_request_id();
        let seq = self.next_reliable_seq();

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_RELIABLE, seq);

        let mut payload = [0u8; 24];
        let mut pos = 0;
        payload[pos] = (req_id >> 8) as u8; payload[pos + 1] = (req_id & 0xFF) as u8;
        pos += 2;
        payload[pos] = (reader_id.0 >> 8) as u8; payload[pos + 1] = (reader_id.0 & 0xFF) as u8;
        pos += 2;
        payload[pos] = ObjectKind::DataReader as u8;
        pos += 1;
        payload[pos] = (sub_id.0 >> 8) as u8; payload[pos + 1] = (sub_id.0 & 0xFF) as u8;
        pos += 2;
        payload[pos] = (topic_id.0 >> 8) as u8; payload[pos + 1] = (topic_id.0 & 0xFF) as u8;
        pos += 2;
        payload[pos] = match qos {
            QosProfile::BestEffort => 0x01,
            QosProfile::Reliable => 0x02,
            QosProfile::TransientLocal => 0x03,
        };
        pos += 1;

        msg.append_submsg_header(submsg_id::CREATE, 0x00, pos as u16);
        msg.append_bytes(&payload[..pos]);
        send_fn(msg.as_bytes())?;
        self.wait_for_status(recv_fn, req_id, 3000).await?;
        self.object_count += 1;

        info!("XRCE: datareader created");
        Ok(reader_id)
    }

    /// Write (publish) serialized data to a DataWriter
    ///
    /// The `cdr_data` must be CDR-serialized (including encapsulation header).
    /// The stream is chosen based on QoS:
    ///   - BestEffort: uses best-effort stream (no ACK)
    ///   - Reliable: uses reliable stream (with ACK)
    pub fn write_data<S>(
        &mut self,
        send_fn: &mut S,
        writer_id: ObjectId,
        cdr_data: &[u8],
        qos: QosProfile,
    ) -> Result<(), XrceError>
    where
        S: FnMut(&[u8]) -> Result<(), XrceError>,
    {
        self.check_connected()?;

        if cdr_data.len() > XRCE_MAX_DATA_SIZE {
            return Err(XrceError::MessageTooLarge);
        }

        let (stream_id, seq) = match qos {
            QosProfile::BestEffort => (STREAM_BESTEFFORT, self.next_besteffort_seq()),
            QosProfile::Reliable | QosProfile::TransientLocal => {
                (STREAM_RELIABLE, self.next_reliable_seq())
            }
        };

        let req_id = self.next_request_id();

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, stream_id, seq);

        // WRITE_DATA submessage
        // Payload: [request_id(2), writer_object_id(2), data_format(1), data...]
        let payload_len = (5 + cdr_data.len()) as u16;
        msg.append_submsg_header(submsg_id::WRITE_DATA, 0x00, payload_len);
        msg.append_u16_be(req_id);
        msg.append_u16_be(writer_id.0);
        msg.append_bytes(&[0x00]); // DATA_FORMAT_DATA (inline CDR)
        msg.append_bytes(cdr_data);

        send_fn(msg.as_bytes())
    }

    /// Request data from a DataReader (subscribe pull)
    pub fn read_data<S>(
        &mut self,
        send_fn: &mut S,
        reader_id: ObjectId,
    ) -> Result<u16, XrceError>
    where
        S: FnMut(&[u8]) -> Result<(), XrceError>,
    {
        self.check_connected()?;

        let req_id = self.next_request_id();
        let seq = self.next_reliable_seq();

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_RELIABLE, seq);

        // READ_DATA submessage
        // Payload: [request_id(2), reader_object_id(2), max_samples(2)]
        msg.append_submsg_header(submsg_id::READ_DATA, 0x00, 6);
        msg.append_u16_be(req_id);
        msg.append_u16_be(reader_id.0);
        msg.append_u16_be(1); // max_samples = 1

        send_fn(msg.as_bytes())?;
        Ok(req_id)
    }

    /// Process incoming data from the agent
    ///
    /// Parses a received XRCE message and returns any data payload.
    /// Returns (object_id, data_offset, data_len) if a DATA submessage is found.
    pub fn process_incoming(
        &mut self,
        rx_data: &[u8],
    ) -> Option<(ObjectId, usize, usize)> {
        if rx_data.len() < XRCE_HEADER_SIZE + XRCE_SUBMSG_HEADER_SIZE {
            return None;
        }

        let mut offset = XRCE_HEADER_SIZE;

        while offset + XRCE_SUBMSG_HEADER_SIZE <= rx_data.len() {
            let sub_id = rx_data[offset];
            let _flags = rx_data[offset + 1];
            let sub_len = u16::from_be_bytes([
                rx_data[offset + 2],
                rx_data[offset + 3],
            ]) as usize;
            offset += XRCE_SUBMSG_HEADER_SIZE;

            if sub_id == submsg_id::DATA && offset + 4 <= rx_data.len() {
                // DATA submessage: [request_id(2), reader_id(2), data...]
                let reader_id = u16::from_be_bytes([rx_data[offset + 2], rx_data[offset + 3]]);
                let data_start = offset + 4;
                let data_len = sub_len.saturating_sub(4);

                if data_start + data_len <= rx_data.len() {
                    return Some((ObjectId(reader_id), data_start, data_len));
                }
            }

            offset += sub_len;
        }

        None
    }

    /// Delete a previously created object
    pub async fn delete_object<S, R>(
        &mut self,
        send_fn: &mut S,
        recv_fn: &mut R,
        object_id: ObjectId,
    ) -> Result<(), XrceError>
    where
        S: FnMut(&[u8]) -> Result<(), XrceError>,
        R: FnMut(&mut [u8], u32) -> Result<usize, XrceError>,
    {
        self.check_connected()?;

        let req_id = self.next_request_id();
        let seq = self.next_reliable_seq();

        let mut msg = XrceMessage::new();
        msg.write_header(self.session_id, STREAM_RELIABLE, seq);

        msg.append_submsg_header(submsg_id::DELETE, 0x00, 4);
        msg.append_u16_be(req_id);
        msg.append_u16_be(object_id.0);

        send_fn(msg.as_bytes())?;
        self.wait_for_status(recv_fn, req_id, 3000).await?;

        if self.object_count > 0 {
            self.object_count -= 1;
        }
        Ok(())
    }

    // ── Internal helpers ──

    fn check_connected(&self) -> Result<(), XrceError> {
        if self.state != SessionState::Connected {
            Err(XrceError::NotConnected)
        } else {
            Ok(())
        }
    }

    /// Wait for a STATUS submessage matching the given request ID
    async fn wait_for_status<R>(
        &mut self,
        recv_fn: &mut R,
        expected_req_id: u16,
        timeout_ms: u32,
    ) -> Result<(), XrceError>
    where
        R: FnMut(&mut [u8], u32) -> Result<usize, XrceError>,
    {
        let rx_len = recv_fn(&mut self.rx_buf, timeout_ms)?;
        if rx_len < XRCE_HEADER_SIZE + XRCE_SUBMSG_HEADER_SIZE + 4 {
            return Err(XrceError::InvalidResponse);
        }

        // Find STATUS submessage
        let mut offset = XRCE_HEADER_SIZE;
        while offset + XRCE_SUBMSG_HEADER_SIZE <= rx_len {
            let sub_id = self.rx_buf[offset];
            let sub_len = u16::from_be_bytes([
                self.rx_buf[offset + 2],
                self.rx_buf[offset + 3],
            ]) as usize;
            offset += XRCE_SUBMSG_HEADER_SIZE;

            if sub_id == submsg_id::STATUS && offset + 4 <= rx_len {
                let req_id = u16::from_be_bytes([
                    self.rx_buf[offset],
                    self.rx_buf[offset + 1],
                ]);
                let status_code = self.rx_buf[offset + 2];

                if req_id == expected_req_id {
                    if status_code == 0x00 {
                        return Ok(());
                    } else {
                        error!("XRCE: operation failed, status=0x{:02X}", status_code);
                        return Err(XrceError::AgentError(status_code));
                    }
                }
            }

            offset += sub_len;
        }

        Err(XrceError::Timeout)
    }

    // ── Public status ──

    /// Get the current session state
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Check if the session is connected
    pub fn is_connected(&self) -> bool {
        self.state == SessionState::Connected
    }

    /// Get the number of created DDS objects
    pub fn object_count(&self) -> u8 {
        self.object_count
    }
}
