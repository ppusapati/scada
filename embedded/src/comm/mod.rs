//! Communication Manager
//!
//! Coordinates all communication interfaces with priority-based failover:
//! 1. Ethernet (W5500) - Primary, lowest latency
//! 2. WiFi (ATWINC1500) - Secondary
//! 3. GSM/LTE (SIM7600) - Tertiary, always-available WAN
//! 4. LoRa (SX1276) - Long-range, low-bandwidth telemetry
//! 5. BLE (RN4870) - Local configuration and diagnostics
//! 6. RS485 Modbus - Field device polling
//! 7. CAN FD - Industrial bus

pub mod protocol;
pub mod mqtt_client;

use heapless::{String, Vec};

/// Communication link priority (lower = higher priority)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinkPriority {
    Ethernet = 0,
    WiFi = 1,
    Gsm = 2,
    LoRa = 3,
}

/// Communication link status
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LinkStatus {
    Down,
    Connecting,
    Up,
    Degraded,
}

/// Active communication link info
#[derive(Clone, Debug)]
pub struct LinkInfo {
    pub priority: LinkPriority,
    pub status: LinkStatus,
    pub signal_quality: i8,
    pub bytes_sent: u32,
    pub bytes_received: u32,
    pub error_count: u16,
}

/// Communication manager state
pub struct CommManager {
    links: [LinkInfo; 4],
    active_link: LinkPriority,
    failover_enabled: bool,
    mqtt_connected: bool,
}

impl CommManager {
    pub fn new() -> Self {
        let make_link = |priority: LinkPriority| LinkInfo {
            priority,
            status: LinkStatus::Down,
            signal_quality: 0,
            bytes_sent: 0,
            bytes_received: 0,
            error_count: 0,
        };
        Self {
            links: [
                make_link(LinkPriority::Ethernet),
                make_link(LinkPriority::WiFi),
                make_link(LinkPriority::Gsm),
                make_link(LinkPriority::LoRa),
            ],
            active_link: LinkPriority::Ethernet,
            failover_enabled: true,
            mqtt_connected: false,
        }
    }

    pub fn active_link(&self) -> LinkPriority {
        self.active_link
    }

    pub fn is_any_link_up(&self) -> bool {
        self.links.iter().any(|l| l.status == LinkStatus::Up)
    }

    pub fn is_mqtt_connected(&self) -> bool {
        self.mqtt_connected
    }

    /// Select the highest-priority available link
    pub fn select_best_link(&mut self) -> LinkPriority {
        for link in &self.links {
            if link.status == LinkStatus::Up {
                self.active_link = link.priority;
                return link.priority;
            }
        }
        self.active_link
    }

    /// Update link status
    pub fn update_link_status(&mut self, priority: LinkPriority, status: LinkStatus) {
        if let Some(link) = self.links.iter_mut().find(|l| l.priority == priority) {
            link.status = status;
        }
        if self.failover_enabled {
            self.select_best_link();
        }
    }

    /// Get link info for a specific interface
    pub fn link_info(&self, priority: LinkPriority) -> Option<&LinkInfo> {
        self.links.iter().find(|l| l.priority == priority)
    }

    /// Get summary of all link statuses
    pub fn all_links(&self) -> &[LinkInfo; 4] {
        &self.links
    }
}

/// SCADA telemetry message format
#[derive(Clone, Debug)]
pub struct TelemetryMessage {
    pub device_id: String<16>,
    pub timestamp: u64,
    pub sequence: u32,
    pub analog_values: [f32; 4],
    pub digital_inputs: u8,
    pub digital_outputs: u8,
    pub internal_temp: f32,
    pub battery_voltage: f32,
}

impl TelemetryMessage {
    /// Serialize to compact binary (fixed 44 bytes)
    pub fn to_binary(&self) -> Vec<u8, 48> {
        let mut buf = Vec::new();
        // Header
        let _ = buf.push(0x5C); // SCADA data marker
        let _ = buf.push(0x01); // Version
        // Sequence (4 bytes)
        let _ = buf.extend_from_slice(&self.sequence.to_le_bytes());
        // Timestamp (8 bytes)
        let _ = buf.extend_from_slice(&self.timestamp.to_le_bytes());
        // Analog values (16 bytes, 4x f32)
        for &val in &self.analog_values {
            let _ = buf.extend_from_slice(&val.to_le_bytes());
        }
        // Digital I/O (2 bytes)
        let _ = buf.push(self.digital_inputs);
        let _ = buf.push(self.digital_outputs);
        // Temperature (4 bytes)
        let _ = buf.extend_from_slice(&self.internal_temp.to_le_bytes());
        // Battery (4 bytes)
        let _ = buf.extend_from_slice(&self.battery_voltage.to_le_bytes());
        buf
    }
}
