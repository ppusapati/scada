//! Lightweight MQTT Client for no_std
//!
//! Minimal MQTT 3.1.1 implementation for SCADA telemetry.
//! Supports QoS 0 and QoS 1 publish, subscribe, and keep-alive.

use heapless::{String, Vec};

/// MQTT QoS levels
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
}

/// MQTT connection state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MqttState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// MQTT client configuration
#[derive(Clone)]
pub struct MqttConfig {
    pub client_id: String<32>,
    pub broker_host: String<64>,
    pub broker_port: u16,
    pub username: String<32>,
    pub password: String<64>,
    pub keep_alive_secs: u16,
    pub clean_session: bool,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            broker_host: String::new(),
            broker_port: 1883,
            username: String::new(),
            password: String::new(),
            keep_alive_secs: 60,
            clean_session: true,
        }
    }
}

/// MQTT packet types
#[repr(u8)]
enum PacketType {
    Connect = 1,
    ConnAck = 2,
    Publish = 3,
    PubAck = 4,
    Subscribe = 8,
    SubAck = 9,
    PingReq = 12,
    PingResp = 13,
    Disconnect = 14,
}

/// MQTT topic paths for SCADA
pub mod topics {
    /// Telemetry data: scada/{device_id}/telemetry
    pub const TELEMETRY_PREFIX: &str = "scada/";
    pub const TELEMETRY_SUFFIX: &str = "/telemetry";

    /// Alarm notifications: scada/{device_id}/alarm
    pub const ALARM_SUFFIX: &str = "/alarm";

    /// Commands (subscribe): scada/{device_id}/cmd
    pub const CMD_SUFFIX: &str = "/cmd";

    /// Configuration: scada/{device_id}/config
    pub const CONFIG_SUFFIX: &str = "/config";

    /// Status/LWT: scada/{device_id}/status
    pub const STATUS_SUFFIX: &str = "/status";
}

/// Lightweight MQTT client
pub struct MqttClient {
    state: MqttState,
    config: MqttConfig,
    packet_id: u16,
    pub_count: u32,
}

impl MqttClient {
    pub fn new(config: MqttConfig) -> Self {
        Self {
            state: MqttState::Disconnected,
            config,
            packet_id: 1,
            pub_count: 0,
        }
    }

    pub fn state(&self) -> MqttState {
        self.state
    }

    fn next_packet_id(&mut self) -> u16 {
        let id = self.packet_id;
        self.packet_id = self.packet_id.wrapping_add(1);
        if self.packet_id == 0 {
            self.packet_id = 1;
        }
        id
    }

    /// Build MQTT CONNECT packet
    pub fn build_connect_packet(&self) -> Vec<u8, 256> {
        let mut pkt = Vec::new();
        // Variable header
        let mut var_header = Vec::<u8, 16>::new();
        // Protocol name "MQTT"
        let _ = var_header.extend_from_slice(&[0x00, 0x04, b'M', b'Q', b'T', b'T']);
        // Protocol level 4 (MQTT 3.1.1)
        let _ = var_header.push(0x04);
        // Connect flags
        let mut flags: u8 = 0x02; // Clean session
        if !self.config.username.is_empty() {
            flags |= 0x80; // Username
        }
        if !self.config.password.is_empty() {
            flags |= 0x40; // Password
        }
        let _ = var_header.push(flags);
        // Keep alive
        let _ = var_header.extend_from_slice(&self.config.keep_alive_secs.to_be_bytes());

        // Payload
        let mut payload = Vec::<u8, 200>::new();
        // Client ID
        let _ = payload.extend_from_slice(&(self.config.client_id.len() as u16).to_be_bytes());
        let _ = payload.extend_from_slice(self.config.client_id.as_bytes());
        // Username
        if !self.config.username.is_empty() {
            let _ = payload.extend_from_slice(&(self.config.username.len() as u16).to_be_bytes());
            let _ = payload.extend_from_slice(self.config.username.as_bytes());
        }
        // Password
        if !self.config.password.is_empty() {
            let _ = payload.extend_from_slice(&(self.config.password.len() as u16).to_be_bytes());
            let _ = payload.extend_from_slice(self.config.password.as_bytes());
        }

        let remaining_len = var_header.len() + payload.len();
        // Fixed header
        let _ = pkt.push((PacketType::Connect as u8) << 4);
        encode_remaining_length(&mut pkt, remaining_len);
        let _ = pkt.extend_from_slice(&var_header);
        let _ = pkt.extend_from_slice(&payload);
        pkt
    }

    /// Build MQTT PUBLISH packet
    pub fn build_publish_packet(
        &mut self,
        topic: &str,
        payload: &[u8],
        qos: QoS,
    ) -> Vec<u8, 512> {
        let mut pkt = Vec::new();
        let packet_id = if qos == QoS::AtLeastOnce {
            Some(self.next_packet_id())
        } else {
            None
        };

        // Variable header: topic + optional packet ID
        let topic_len = topic.len();
        let var_header_len = 2 + topic_len + if packet_id.is_some() { 2 } else { 0 };
        let remaining_len = var_header_len + payload.len();

        // Fixed header
        let mut flags = (PacketType::Publish as u8) << 4;
        flags |= (qos as u8) << 1;
        let _ = pkt.push(flags);
        encode_remaining_length(&mut pkt, remaining_len);

        // Topic
        let _ = pkt.extend_from_slice(&(topic_len as u16).to_be_bytes());
        let _ = pkt.extend_from_slice(topic.as_bytes());

        // Packet ID (QoS 1)
        if let Some(id) = packet_id {
            let _ = pkt.extend_from_slice(&id.to_be_bytes());
        }

        // Payload
        let _ = pkt.extend_from_slice(payload);
        self.pub_count += 1;
        pkt
    }

    /// Build MQTT PINGREQ packet
    pub fn build_ping_packet() -> [u8; 2] {
        [(PacketType::PingReq as u8) << 4, 0x00]
    }

    /// Build MQTT DISCONNECT packet
    pub fn build_disconnect_packet() -> [u8; 2] {
        [(PacketType::Disconnect as u8) << 4, 0x00]
    }
}

/// Encode MQTT remaining length (variable-length encoding)
fn encode_remaining_length<const N: usize>(buf: &mut Vec<u8, N>, mut len: usize) {
    loop {
        let mut byte = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            byte |= 0x80;
        }
        let _ = buf.push(byte);
        if len == 0 {
            break;
        }
    }
}
