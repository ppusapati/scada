/// Modbus TCP Server — Low-Level Implementation
///
/// Provides a standalone Modbus TCP server that can be used
/// independently of the gateway module. Supports:
///   - Multiple simultaneous connections (up to max_connections)
///   - MBAP header parsing and response framing
///   - Function codes 03, 04, 06, 16
///   - Connection timeout management
///   - Per-connection statistics

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

/// Connection statistics
#[derive(Debug, Default)]
pub struct ConnectionStats {
    pub total_requests: u64,
    pub total_errors: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
}

/// Modbus TCP server configuration
pub struct TcpServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_connections: usize,
    pub timeout_secs: u64,
    pub unit_id: u8,
}

impl Default for TcpServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 502,
            max_connections: 4,
            timeout_secs: 30,
            unit_id: 1,
        }
    }
}

/// MBAP (Modbus Application Protocol) header
#[derive(Debug, Clone)]
pub struct MbapHeader {
    pub transaction_id: u16,
    pub protocol_id: u16,
    pub length: u16,
    pub unit_id: u8,
}

impl MbapHeader {
    /// Parse MBAP header from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 7 {
            return None;
        }
        Some(Self {
            transaction_id: ((data[0] as u16) << 8) | data[1] as u16,
            protocol_id: ((data[2] as u16) << 8) | data[3] as u16,
            length: ((data[4] as u16) << 8) | data[5] as u16,
            unit_id: data[6],
        })
    }

    /// Serialize MBAP header to bytes
    pub fn to_bytes(&self) -> [u8; 7] {
        [
            (self.transaction_id >> 8) as u8,
            (self.transaction_id & 0xFF) as u8,
            (self.protocol_id >> 8) as u8,
            (self.protocol_id & 0xFF) as u8,
            (self.length >> 8) as u8,
            (self.length & 0xFF) as u8,
            self.unit_id,
        ]
    }
}
