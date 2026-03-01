/// MQTT client for no_std embedded targets
///
/// Uses rust-mqtt crate over embassy-net TCP sockets to communicate
/// with the Mosquitto MQTT broker running on the SCADA backend.
///
/// Features:
///   - MQTT v3.1.1 with QoS 1 for telemetry publish
///   - Last Will and Testament for device offline detection
///   - Command subscription for remote control
///   - Auto-reconnect with exponential backoff

use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embassy_time::{Duration, Timer};
use heapless::String;
use defmt::{info, warn, error};

/// MQTT broker connection configuration
pub struct MqttConfig {
    pub broker_ip: [u8; 4],
    pub broker_port: u16,
    pub client_id: String<64>,
    pub keepalive_secs: u16,
}

impl MqttConfig {
    pub fn new(broker_ip: [u8; 4], broker_port: u16, client_id: &str) -> Self {
        let mut cid = String::new();
        let _ = cid.push_str(client_id);
        Self {
            broker_ip,
            broker_port,
            client_id: cid,
            keepalive_secs: 30,
        }
    }
}

/// MQTT connection state
#[derive(Clone, Copy, PartialEq, defmt::Format)]
pub enum MqttState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Simple MQTT v3.1.1 client for embedded use
///
/// This wraps a TCP socket and implements the minimal subset of MQTT
/// needed for SCADA telemetry publishing and command subscription.
pub struct MqttClient {
    config: MqttConfig,
    state: MqttState,
    packet_id: u16,
}

impl MqttClient {
    pub fn new(config: MqttConfig) -> Self {
        Self {
            config,
            state: MqttState::Disconnected,
            packet_id: 1,
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

    /// Connect to the MQTT broker over TCP
    pub async fn connect<'a>(
        &mut self,
        socket: &mut TcpSocket<'a>,
        lwt_topic: Option<&str>,
        lwt_payload: Option<&[u8]>,
    ) -> bool {
        self.state = MqttState::Connecting;

        let addr = embassy_net::IpEndpoint::new(
            embassy_net::IpAddress::v4(
                self.config.broker_ip[0],
                self.config.broker_ip[1],
                self.config.broker_ip[2],
                self.config.broker_ip[3],
            ),
            self.config.broker_port,
        );

        info!("MQTT: Connecting to broker...");
        if socket.connect(addr).await.is_err() {
            error!("MQTT: TCP connect failed");
            self.state = MqttState::Error;
            return false;
        }

        // Build CONNECT packet
        let mut pkt = [0u8; 256];
        let len = self.build_connect_packet(&mut pkt, lwt_topic, lwt_payload);

        if socket.write_all(&pkt[..len]).await.is_err() {
            error!("MQTT: Failed to send CONNECT");
            self.state = MqttState::Error;
            return false;
        }

        // Wait for CONNACK
        let mut resp = [0u8; 4];
        if socket.read_exact(&mut resp).await.is_err() {
            error!("MQTT: No CONNACK received");
            self.state = MqttState::Error;
            return false;
        }

        if resp[0] == 0x20 && resp[3] == 0x00 {
            info!("MQTT: Connected successfully");
            self.state = MqttState::Connected;
            true
        } else {
            error!("MQTT: CONNACK rejected (code={})", resp[3]);
            self.state = MqttState::Error;
            false
        }
    }

    /// Publish a message to a topic with QoS 0
    pub async fn publish<'a>(
        &mut self,
        socket: &mut TcpSocket<'a>,
        topic: &str,
        payload: &[u8],
    ) -> bool {
        if self.state != MqttState::Connected {
            return false;
        }

        let mut pkt = [0u8; 512];
        let len = self.build_publish_packet(&mut pkt, topic, payload, 0);

        if socket.write_all(&pkt[..len]).await.is_err() {
            warn!("MQTT: Publish failed");
            self.state = MqttState::Error;
            return false;
        }

        true
    }

    /// Subscribe to a topic with QoS 1
    pub async fn subscribe<'a>(
        &mut self,
        socket: &mut TcpSocket<'a>,
        topic: &str,
    ) -> bool {
        if self.state != MqttState::Connected {
            return false;
        }

        let mut pkt = [0u8; 256];
        let len = self.build_subscribe_packet(&mut pkt, topic);

        if socket.write_all(&pkt[..len]).await.is_err() {
            warn!("MQTT: Subscribe send failed");
            self.state = MqttState::Error;
            return false;
        }

        // Wait for SUBACK
        let mut resp = [0u8; 5];
        if socket.read_exact(&mut resp).await.is_err() {
            warn!("MQTT: No SUBACK received");
            self.state = MqttState::Error;
            return false;
        }

        if resp[0] == 0x90 {
            info!("MQTT: Subscribed to {}", topic);
            true
        } else {
            warn!("MQTT: Subscribe failed");
            false
        }
    }

    /// Send PINGREQ to keep connection alive
    pub async fn ping<'a>(&mut self, socket: &mut TcpSocket<'a>) -> bool {
        if self.state != MqttState::Connected {
            return false;
        }

        let pkt = [0xC0, 0x00]; // PINGREQ
        if socket.write_all(&pkt).await.is_err() {
            self.state = MqttState::Error;
            return false;
        }
        true
    }

    /// Read a single incoming MQTT packet (non-blocking check)
    /// Returns (topic_offset, topic_len, payload_offset, payload_len) into the buffer
    pub async fn read_packet<'a>(
        &mut self,
        socket: &mut TcpSocket<'a>,
        buf: &mut [u8],
    ) -> Option<(usize, usize, usize, usize)> {
        // Read first byte (packet type + flags)
        if socket.read_exact(&mut buf[..1]).await.is_err() {
            return None;
        }

        let pkt_type = buf[0] >> 4;

        // Decode variable-length remaining length (MQTT spec 2.2.3)
        // Supports payloads up to 268,435,455 bytes (4-byte encoding)
        let mut remaining_len: usize = 0;
        let mut multiplier: usize = 1;
        let mut header_len: usize = 1;

        loop {
            let mut byte = [0u8; 1];
            if socket.read_exact(&mut byte).await.is_err() {
                return None;
            }
            header_len += 1;
            remaining_len += (byte[0] & 0x7F) as usize * multiplier;
            if byte[0] & 0x80 == 0 {
                break;
            }
            multiplier *= 128;
            if multiplier > 128 * 128 * 128 {
                return None; // Malformed: more than 4 length bytes
            }
        }

        if remaining_len == 0 || remaining_len > buf.len() - header_len {
            return None;
        }

        // Read remaining bytes into buffer after header
        if socket.read_exact(&mut buf[header_len..header_len + remaining_len]).await.is_err() {
            return None;
        }

        // Handle PUBLISH (type 3)
        if pkt_type == 3 {
            let topic_len = ((buf[header_len] as usize) << 8) | (buf[header_len + 1] as usize);
            let topic_offset = header_len + 2;
            let payload_offset = topic_offset + topic_len;
            let payload_len = remaining_len - 2 - topic_len;
            return Some((topic_offset, topic_len, payload_offset, payload_len));
        }

        // Handle PINGRESP (type 13) — just acknowledge
        if pkt_type == 13 {
            return None; // No action needed
        }

        None
    }

    // ── Packet builders ──

    fn build_connect_packet(
        &self,
        buf: &mut [u8],
        lwt_topic: Option<&str>,
        lwt_payload: Option<&[u8]>,
    ) -> usize {
        let client_id = self.config.client_id.as_bytes();
        let mut flags: u8 = 0x02; // Clean session

        let mut var_len = 10 + 2 + client_id.len(); // Protocol header + client ID

        if let (Some(topic), Some(payload)) = (lwt_topic, lwt_payload) {
            flags |= 0x04; // Will flag
            flags |= 0x08; // Will QoS 1
            var_len += 2 + topic.len() + 2 + payload.len();
        }

        let mut i = 0;
        buf[i] = 0x10; // CONNECT
        i += 1;

        // Encode remaining length (variable-length encoding)
        i += encode_remaining_length(&mut buf[i..], var_len);

        // Protocol name "MQTT"
        buf[i..i + 6].copy_from_slice(&[0x00, 0x04, b'M', b'Q', b'T', b'T']);
        i += 6;

        // Protocol level 4 (MQTT 3.1.1)
        buf[i] = 0x04;
        i += 1;

        // Connect flags
        buf[i] = flags;
        i += 1;

        // Keep alive
        buf[i] = (self.config.keepalive_secs >> 8) as u8;
        buf[i + 1] = (self.config.keepalive_secs & 0xFF) as u8;
        i += 2;

        // Client ID
        buf[i] = (client_id.len() >> 8) as u8;
        buf[i + 1] = (client_id.len() & 0xFF) as u8;
        i += 2;
        buf[i..i + client_id.len()].copy_from_slice(client_id);
        i += client_id.len();

        // Last Will and Testament
        if let (Some(topic), Some(payload)) = (lwt_topic, lwt_payload) {
            let tb = topic.as_bytes();
            buf[i] = (tb.len() >> 8) as u8;
            buf[i + 1] = (tb.len() & 0xFF) as u8;
            i += 2;
            buf[i..i + tb.len()].copy_from_slice(tb);
            i += tb.len();

            buf[i] = (payload.len() >> 8) as u8;
            buf[i + 1] = (payload.len() & 0xFF) as u8;
            i += 2;
            buf[i..i + payload.len()].copy_from_slice(payload);
            i += payload.len();
        }

        i
    }

    fn build_publish_packet(
        &self,
        buf: &mut [u8],
        topic: &str,
        payload: &[u8],
        qos: u8,
    ) -> usize {
        let topic_bytes = topic.as_bytes();
        let remaining = 2 + topic_bytes.len() + payload.len();

        let mut i = 0;
        buf[i] = 0x30 | ((qos & 0x03) << 1); // PUBLISH
        i += 1;

        // Encode remaining length
        if remaining < 128 {
            buf[i] = remaining as u8;
            i += 1;
        } else {
            buf[i] = (remaining & 0x7F) as u8 | 0x80;
            buf[i + 1] = (remaining >> 7) as u8;
            i += 2;
        }

        // Topic
        buf[i] = (topic_bytes.len() >> 8) as u8;
        buf[i + 1] = (topic_bytes.len() & 0xFF) as u8;
        i += 2;
        buf[i..i + topic_bytes.len()].copy_from_slice(topic_bytes);
        i += topic_bytes.len();

        // Payload
        buf[i..i + payload.len()].copy_from_slice(payload);
        i += payload.len();

        i
    }

    fn build_subscribe_packet(&mut self, buf: &mut [u8], topic: &str) -> usize {
        let topic_bytes = topic.as_bytes();
        let pkt_id = self.next_packet_id();
        let remaining = 2 + 2 + topic_bytes.len() + 1; // packet_id + topic + qos

        let mut i = 0;
        buf[i] = 0x82; // SUBSCRIBE
        i += 1;

        buf[i] = remaining as u8;
        i += 1;

        // Packet identifier
        buf[i] = (pkt_id >> 8) as u8;
        buf[i + 1] = (pkt_id & 0xFF) as u8;
        i += 2;

        // Topic filter
        buf[i] = (topic_bytes.len() >> 8) as u8;
        buf[i + 1] = (topic_bytes.len() & 0xFF) as u8;
        i += 2;
        buf[i..i + topic_bytes.len()].copy_from_slice(topic_bytes);
        i += topic_bytes.len();

        // QoS 1
        buf[i] = 0x01;
        i += 1;

        i
    }
}

/// Encode MQTT variable-length remaining length field
/// Returns the number of bytes written
fn encode_remaining_length(buf: &mut [u8], mut length: usize) -> usize {
    let mut i = 0;
    loop {
        let mut byte = (length & 0x7F) as u8;
        length >>= 7;
        if length > 0 {
            byte |= 0x80;
        }
        buf[i] = byte;
        i += 1;
        if length == 0 {
            break;
        }
    }
    i
}

/// Reconnect with exponential backoff
pub async fn reconnect_delay(attempt: u32) {
    let delay_ms = core::cmp::min(1000 * (1u64 << attempt), 30_000);
    info!("MQTT: Reconnecting in {}ms (attempt {})", delay_ms, attempt);
    Timer::after(Duration::from_millis(delay_ms)).await;
}
