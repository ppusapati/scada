//! SCADA Communication Protocol
//!
//! Wire protocol for telemetry, commands, and configuration.
//! Used across all communication links (Ethernet, WiFi, GSM, LoRa).

use heapless::Vec;

/// Protocol message types
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum MessageType {
    /// Periodic sensor data upload
    Telemetry = 0x01,
    /// Alarm/event notification
    Alarm = 0x02,
    /// Command from server to device
    Command = 0x10,
    /// Command acknowledgment
    CommandAck = 0x11,
    /// Configuration read request
    ConfigRead = 0x20,
    /// Configuration read response
    ConfigResponse = 0x21,
    /// Configuration write
    ConfigWrite = 0x22,
    /// Heartbeat / keep-alive
    Heartbeat = 0x30,
    /// Firmware update chunk
    FirmwareData = 0xF0,
}

/// Protocol frame header (8 bytes)
#[derive(Clone, Debug)]
pub struct FrameHeader {
    pub sync: u16,       // 0x5CAD (SCAD)
    pub version: u8,     // Protocol version
    pub msg_type: u8,    // MessageType
    pub sequence: u16,   // Sequence number
    pub payload_len: u16, // Payload length
}

impl FrameHeader {
    pub const SYNC_WORD: u16 = 0x5CAD;
    pub const SIZE: usize = 8;

    pub fn new(msg_type: MessageType, sequence: u16, payload_len: u16) -> Self {
        Self {
            sync: Self::SYNC_WORD,
            version: 1,
            msg_type: msg_type as u8,
            sequence,
            payload_len,
        }
    }

    pub fn serialize(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0..2].copy_from_slice(&self.sync.to_le_bytes());
        buf[2] = self.version;
        buf[3] = self.msg_type;
        buf[4..6].copy_from_slice(&self.sequence.to_le_bytes());
        buf[6..8].copy_from_slice(&self.payload_len.to_le_bytes());
        buf
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        let sync = u16::from_le_bytes([data[0], data[1]]);
        if sync != Self::SYNC_WORD {
            return None;
        }
        Some(Self {
            sync,
            version: data[2],
            msg_type: data[3],
            sequence: u16::from_le_bytes([data[4], data[5]]),
            payload_len: u16::from_le_bytes([data[6], data[7]]),
        })
    }
}

/// Alarm severity levels
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum AlarmSeverity {
    Info = 0,
    Warning = 1,
    Critical = 2,
    Emergency = 3,
}

/// Alarm sources
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum AlarmSource {
    AnalogChannel0 = 0,
    AnalogChannel1 = 1,
    AnalogChannel2 = 2,
    AnalogChannel3 = 3,
    DigitalInput = 4,
    InternalTemp = 5,
    PowerSupply = 6,
    Communication = 7,
    Watchdog = 8,
    Storage = 9,
}

/// Command types from server
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum CommandType {
    /// Set digital output state
    SetDigitalOutput = 0x01,
    /// Reset alarm
    ResetAlarm = 0x02,
    /// Trigger immediate telemetry report
    ForceTelemetry = 0x03,
    /// Restart device
    Reboot = 0x04,
    /// Enter firmware update mode
    EnterBootloader = 0x05,
    /// Set telemetry interval
    SetInterval = 0x10,
    /// Synchronize RTC
    SyncTime = 0x20,
}

/// CRC-32 for protocol frames (IEEE 802.3)
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
