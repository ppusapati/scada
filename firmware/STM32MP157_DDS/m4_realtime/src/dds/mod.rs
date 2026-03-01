/// XRCE-DDS Client for STM32MP157 Cortex-M4
///
/// This module implements a lightweight XRCE-DDS (DDS for eXtremely
/// Resource Constrained Environments) client that communicates with
/// a full DDS agent running on the A7 Linux core.
///
/// The XRCE protocol (OMG DDS-XRCE standard) allows this no_std
/// firmware to participate in the DDS data space without requiring
/// a full DDS stack.
///
/// Architecture:
///   M4 XRCE Client ──RPMsg──→ A7 XRCE Agent ──DDS──→ Network
///
/// Entity hierarchy (XRCE object model):
///   Session
///   └── Participant (Domain 0)
///       ├── Publisher
///       │   ├── DataWriter<StringVoltageData>
///       │   ├── DataWriter<StringCurrentData>
///       │   ├── DataWriter<BusPowerData>
///       │   ├── DataWriter<AlarmEvent>
///       │   └── DataWriter<HeartbeatData>
///       └── Subscriber
///           ├── DataReader<ConfigCommand>
///           └── DataReader<CalibrationUpdate>

pub mod types;
pub mod qos;

use defmt::{info, warn, debug};
use heapless::Vec;

use crate::rpmsg::RpmsgEndpoint;
use crate::rpmsg::transport::{XrceTransport, StreamId};

/// XRCE Object ID type
pub type ObjectId = u16;

/// XRCE submessage types
mod submsg_id {
    pub const CREATE_CLIENT: u8 = 0x00;
    pub const CREATE: u8 = 0x01;
    pub const DELETE: u8 = 0x02;
    pub const WRITE_DATA: u8 = 0x03;
    pub const READ_DATA: u8 = 0x04;
    pub const STATUS: u8 = 0x05;
    pub const DATA: u8 = 0x06;
    pub const HEARTBEAT: u8 = 0x07;
    pub const ACKNACK: u8 = 0x08;
    pub const TIMESTAMP: u8 = 0x09;
}

/// XRCE object types
mod object_kind {
    pub const PARTICIPANT: u8 = 0x01;
    pub const TOPIC: u8 = 0x02;
    pub const PUBLISHER: u8 = 0x03;
    pub const SUBSCRIBER: u8 = 0x04;
    pub const DATAWRITER: u8 = 0x05;
    pub const DATAREADER: u8 = 0x06;
}

/// XRCE status codes
mod status_code {
    pub const OK: u8 = 0x00;
    pub const ERR_DDS_ERROR: u8 = 0x01;
    pub const ERR_MISMATCH: u8 = 0x02;
    pub const ERR_ALREADY_EXISTS: u8 = 0x03;
    pub const ERR_DENIED: u8 = 0x04;
    pub const ERR_UNKNOWN_REFERENCE: u8 = 0x05;
    pub const ERR_INVALID_DATA: u8 = 0x06;
    pub const ERR_INCOMPATIBLE: u8 = 0x07;
    pub const ERR_RESOURCES: u8 = 0x08;
}

/// Pre-defined Object IDs for the SCADA entities
pub mod entity_id {
    use super::ObjectId;

    pub const PARTICIPANT: ObjectId = 0x0001;
    pub const PUBLISHER: ObjectId = 0x0002;
    pub const SUBSCRIBER: ObjectId = 0x0003;

    // Solar SMU topics and writers
    pub const TOPIC_STRING_VOLTAGE: ObjectId = 0x0100;
    pub const TOPIC_STRING_CURRENT: ObjectId = 0x0101;
    pub const TOPIC_BUS_POWER: ObjectId = 0x0102;
    pub const TOPIC_ALARM: ObjectId = 0x0103;
    pub const TOPIC_HEARTBEAT: ObjectId = 0x0104;

    pub const WRITER_STRING_VOLTAGE: ObjectId = 0x0200;
    pub const WRITER_STRING_CURRENT: ObjectId = 0x0201;
    pub const WRITER_BUS_POWER: ObjectId = 0x0202;
    pub const WRITER_ALARM: ObjectId = 0x0203;
    pub const WRITER_HEARTBEAT: ObjectId = 0x0204;

    // Water RTU topics and writers
    pub const TOPIC_WATER_LEVEL: ObjectId = 0x0110;
    pub const TOPIC_FLOW_RATE: ObjectId = 0x0111;
    pub const TOPIC_PRESSURE: ObjectId = 0x0112;
    pub const TOPIC_WATER_QUALITY: ObjectId = 0x0113;

    pub const WRITER_WATER_LEVEL: ObjectId = 0x0210;
    pub const WRITER_FLOW_RATE: ObjectId = 0x0211;
    pub const WRITER_PRESSURE: ObjectId = 0x0212;
    pub const WRITER_WATER_QUALITY: ObjectId = 0x0213;

    // Command/config readers
    pub const TOPIC_CONFIG_COMMAND: ObjectId = 0x0300;
    pub const READER_CONFIG_COMMAND: ObjectId = 0x0400;
}

/// XRCE-DDS Client
///
/// Manages the session with the A7 XRCE agent and provides
/// methods to create DDS entities and write/read data.
pub struct XrceClient {
    transport: XrceTransport,
    session_active: bool,
    next_request_id: u16,
}

impl XrceClient {
    pub const fn new() -> Self {
        Self {
            transport: XrceTransport::new(0x01), // Session ID 1
            session_active: false,
            next_request_id: 1,
        }
    }

    /// Establish a session with the XRCE agent
    pub fn create_session(&mut self, rpmsg: &RpmsgEndpoint) -> bool {
        info!("XRCE: Creating session with agent...");

        // Build CREATE_CLIENT submessage
        let mut payload: Vec<u8, 64> = Vec::new();
        let _ = payload.push(submsg_id::CREATE_CLIENT);
        let _ = payload.push(0x00); // Flags
        let req_id = self.next_request_id();
        let _ = payload.extend_from_slice(&req_id.to_be_bytes());
        // Client key (unique identifier for this M4 instance)
        let _ = payload.extend_from_slice(&[0x50, 0x39, 0x65, 0x01]); // "P9e\x01"

        let ok = self.transport.send(rpmsg, StreamId::Session, &payload);
        if ok {
            self.session_active = true;
            info!("XRCE: Session created");
        }
        ok
    }

    /// Create a DDS domain participant
    pub fn create_participant(
        &mut self,
        rpmsg: &RpmsgEndpoint,
        domain_id: u16,
    ) -> bool {
        info!("XRCE: Creating participant (domain={})", domain_id);

        let mut payload: Vec<u8, 128> = Vec::new();
        let _ = payload.push(submsg_id::CREATE);
        let _ = payload.push(0x00); // Flags
        let req_id = self.next_request_id();
        let _ = payload.extend_from_slice(&req_id.to_be_bytes());
        let _ = payload.extend_from_slice(&entity_id::PARTICIPANT.to_be_bytes());
        let _ = payload.push(object_kind::PARTICIPANT);
        let _ = payload.push(0x00); // Representation: BY_REFERENCE
        // Domain ID
        let _ = payload.extend_from_slice(&domain_id.to_be_bytes());

        self.transport.send(rpmsg, StreamId::ReliableOutput, &payload)
    }

    /// Create a topic with the given name and type
    pub fn create_topic(
        &mut self,
        rpmsg: &RpmsgEndpoint,
        topic_id: ObjectId,
        topic_name: &str,
        type_name: &str,
    ) -> bool {
        debug!("XRCE: Creating topic '{}' (type='{}')", topic_name, type_name);

        let mut payload: Vec<u8, 256> = Vec::new();
        let _ = payload.push(submsg_id::CREATE);
        let _ = payload.push(0x00);
        let req_id = self.next_request_id();
        let _ = payload.extend_from_slice(&req_id.to_be_bytes());
        let _ = payload.extend_from_slice(&topic_id.to_be_bytes());
        let _ = payload.push(object_kind::TOPIC);
        let _ = payload.push(0x01); // Representation: BY_XML (simplified)
        // Participant reference
        let _ = payload.extend_from_slice(&entity_id::PARTICIPANT.to_be_bytes());
        // Topic name length + string
        let name_len = topic_name.len() as u16;
        let _ = payload.extend_from_slice(&name_len.to_be_bytes());
        let _ = payload.extend_from_slice(topic_name.as_bytes());
        // Type name length + string
        let type_len = type_name.len() as u16;
        let _ = payload.extend_from_slice(&type_len.to_be_bytes());
        let _ = payload.extend_from_slice(type_name.as_bytes());

        self.transport.send(rpmsg, StreamId::ReliableOutput, &payload)
    }

    /// Create a publisher
    pub fn create_publisher(&mut self, rpmsg: &RpmsgEndpoint) -> bool {
        debug!("XRCE: Creating publisher");

        let mut payload: Vec<u8, 64> = Vec::new();
        let _ = payload.push(submsg_id::CREATE);
        let _ = payload.push(0x00);
        let req_id = self.next_request_id();
        let _ = payload.extend_from_slice(&req_id.to_be_bytes());
        let _ = payload.extend_from_slice(&entity_id::PUBLISHER.to_be_bytes());
        let _ = payload.push(object_kind::PUBLISHER);
        let _ = payload.push(0x00);
        let _ = payload.extend_from_slice(&entity_id::PARTICIPANT.to_be_bytes());

        self.transport.send(rpmsg, StreamId::ReliableOutput, &payload)
    }

    /// Create a subscriber
    pub fn create_subscriber(&mut self, rpmsg: &RpmsgEndpoint) -> bool {
        debug!("XRCE: Creating subscriber");

        let mut payload: Vec<u8, 64> = Vec::new();
        let _ = payload.push(submsg_id::CREATE);
        let _ = payload.push(0x00);
        let req_id = self.next_request_id();
        let _ = payload.extend_from_slice(&req_id.to_be_bytes());
        let _ = payload.extend_from_slice(&entity_id::SUBSCRIBER.to_be_bytes());
        let _ = payload.push(object_kind::SUBSCRIBER);
        let _ = payload.push(0x00);
        let _ = payload.extend_from_slice(&entity_id::PARTICIPANT.to_be_bytes());

        self.transport.send(rpmsg, StreamId::ReliableOutput, &payload)
    }

    /// Create a data writer for a topic with specified QoS
    pub fn create_datawriter(
        &mut self,
        rpmsg: &RpmsgEndpoint,
        writer_id: ObjectId,
        topic_id: ObjectId,
        qos: &qos::TopicQos,
    ) -> bool {
        debug!("XRCE: Creating data writer (id=0x{:04X}, topic=0x{:04X})", writer_id, topic_id);

        let mut payload: Vec<u8, 128> = Vec::new();
        let _ = payload.push(submsg_id::CREATE);
        let _ = payload.push(0x00);
        let req_id = self.next_request_id();
        let _ = payload.extend_from_slice(&req_id.to_be_bytes());
        let _ = payload.extend_from_slice(&writer_id.to_be_bytes());
        let _ = payload.push(object_kind::DATAWRITER);
        let _ = payload.push(0x00);
        // Publisher reference
        let _ = payload.extend_from_slice(&entity_id::PUBLISHER.to_be_bytes());
        // Topic reference
        let _ = payload.extend_from_slice(&topic_id.to_be_bytes());
        // QoS profile (serialized)
        let _ = payload.push(qos.reliability as u8);
        let _ = payload.extend_from_slice(&qos.deadline_ms.to_be_bytes());
        let _ = payload.extend_from_slice(&qos.lifespan_ms.to_be_bytes());
        let _ = payload.extend_from_slice(&qos.history_depth.to_be_bytes());

        self.transport.send(rpmsg, StreamId::ReliableOutput, &payload)
    }

    /// Create a data reader for a topic
    pub fn create_datareader(
        &mut self,
        rpmsg: &RpmsgEndpoint,
        reader_id: ObjectId,
        topic_id: ObjectId,
    ) -> bool {
        debug!("XRCE: Creating data reader (id=0x{:04X}, topic=0x{:04X})", reader_id, topic_id);

        let mut payload: Vec<u8, 128> = Vec::new();
        let _ = payload.push(submsg_id::CREATE);
        let _ = payload.push(0x00);
        let req_id = self.next_request_id();
        let _ = payload.extend_from_slice(&req_id.to_be_bytes());
        let _ = payload.extend_from_slice(&reader_id.to_be_bytes());
        let _ = payload.push(object_kind::DATAREADER);
        let _ = payload.push(0x00);
        let _ = payload.extend_from_slice(&entity_id::SUBSCRIBER.to_be_bytes());
        let _ = payload.extend_from_slice(&topic_id.to_be_bytes());

        self.transport.send(rpmsg, StreamId::ReliableOutput, &payload)
    }

    /// Write data to a data writer
    ///
    /// The `cdr_data` must be pre-serialized in CDR format.
    /// The stream is selected based on the QoS (reliable or best-effort).
    pub fn write_data(
        &self,
        rpmsg: &RpmsgEndpoint,
        writer_id: ObjectId,
        cdr_data: &[u8],
        reliable: bool,
    ) -> bool {
        let mut payload: Vec<u8, 512> = Vec::new();
        let _ = payload.push(submsg_id::WRITE_DATA);
        let _ = payload.push(0x00); // Flags
        let _ = payload.extend_from_slice(&writer_id.to_be_bytes());
        // CDR data inline
        let data_len = cdr_data.len() as u16;
        let _ = payload.extend_from_slice(&data_len.to_be_bytes());
        let _ = payload.extend_from_slice(cdr_data);

        let stream = if reliable {
            StreamId::ReliableOutput
        } else {
            StreamId::BestEffortOutput
        };

        self.transport.send(rpmsg, stream, &payload)
    }

    /// Read data from a data reader (non-blocking)
    pub fn read_data(
        &mut self,
        rpmsg: &RpmsgEndpoint,
        buf: &mut [u8],
    ) -> Option<(ObjectId, usize)> {
        let mut recv_buf = [0u8; 512];
        let (stream_id, len) = self.transport.recv(rpmsg, &mut recv_buf)?;

        if len < 5 {
            return None;
        }

        // Check if this is a DATA submessage
        if recv_buf[0] != submsg_id::DATA {
            return None;
        }

        let reader_id = ((recv_buf[2] as u16) << 8) | recv_buf[3] as u16;
        let data_len = ((recv_buf[4] as u16) << 8) | recv_buf[5] as u16;
        let data_start = 6;
        let data_end = data_start + data_len as usize;

        if data_end > len || data_len as usize > buf.len() {
            return None;
        }

        buf[..data_len as usize].copy_from_slice(&recv_buf[data_start..data_end]);
        Some((reader_id, data_len as usize))
    }

    /// Delete an XRCE object
    pub fn delete_object(&mut self, rpmsg: &RpmsgEndpoint, object_id: ObjectId) -> bool {
        let mut payload: Vec<u8, 16> = Vec::new();
        let _ = payload.push(submsg_id::DELETE);
        let _ = payload.push(0x00);
        let req_id = self.next_request_id();
        let _ = payload.extend_from_slice(&req_id.to_be_bytes());
        let _ = payload.extend_from_slice(&object_id.to_be_bytes());

        self.transport.send(rpmsg, StreamId::ReliableOutput, &payload)
    }

    /// Check if the session is active
    pub fn is_active(&self) -> bool {
        self.session_active
    }

    fn next_request_id(&mut self) -> u16 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        if self.next_request_id == 0 {
            self.next_request_id = 1;
        }
        id
    }
}
