//! CAN FD Bus Driver
//!
//! ISO1042 isolated CAN FD transceiver interface via FDCAN1.
//! Supports Classic CAN 2.0 and CAN FD with bit rate switching.
//!
//! Hardware: FDCAN1 (PD0=RX, PD1=TX)
//! Isolation: ISO1042BQDWRQ1 (5kVrms)
//! ESD: PESD2CAN on bus lines

use heapless::Vec;

/// CAN bus speed configuration
#[derive(Clone, Copy, Debug)]
pub struct CanBitTiming {
    /// Nominal bit rate (arbitration phase)
    pub nominal_bps: u32,
    /// Data bit rate (CAN FD data phase, 0 = classic CAN only)
    pub data_bps: u32,
}

impl Default for CanBitTiming {
    fn default() -> Self {
        Self {
            nominal_bps: 500_000,
            data_bps: 2_000_000,
        }
    }
}

/// CAN FD driver configuration
#[derive(Clone)]
pub struct CanFdConfig {
    pub bit_timing: CanBitTiming,
    /// Enable CAN FD mode (vs classic CAN 2.0)
    pub fd_enabled: bool,
    /// Enable bit rate switching in FD mode
    pub brs_enabled: bool,
    /// Enable automatic retransmission
    pub auto_retransmit: bool,
    /// Bus-off automatic recovery
    pub auto_bus_off_recovery: bool,
    /// 120 ohm termination resistor enabled (on-board jumper)
    pub termination: bool,
}

impl Default for CanFdConfig {
    fn default() -> Self {
        Self {
            bit_timing: CanBitTiming::default(),
            fd_enabled: true,
            brs_enabled: true,
            auto_retransmit: true,
            auto_bus_off_recovery: true,
            termination: false,
        }
    }
}

/// CAN bus state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanState {
    Off,
    ErrorActive,
    ErrorWarning,
    ErrorPassive,
    BusOff,
}

/// CAN frame type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrameType {
    /// Classic CAN 2.0 (max 8 bytes)
    Classic,
    /// CAN FD without bit rate switching (max 64 bytes)
    Fd,
    /// CAN FD with bit rate switching (max 64 bytes)
    FdBrs,
}

/// CAN frame identifier
#[derive(Clone, Copy, Debug)]
pub enum CanId {
    /// Standard 11-bit identifier
    Standard(u16),
    /// Extended 29-bit identifier
    Extended(u32),
}

impl CanId {
    pub fn raw(&self) -> u32 {
        match self {
            Self::Standard(id) => *id as u32,
            Self::Extended(id) => *id,
        }
    }

    pub fn is_extended(&self) -> bool {
        matches!(self, Self::Extended(_))
    }
}

/// CAN FD frame (up to 64 bytes data)
pub struct CanFrame {
    pub id: CanId,
    pub frame_type: FrameType,
    pub data: Vec<u8, 64>,
    pub timestamp: u16,
}

impl CanFrame {
    pub fn new_standard(id: u16, data: &[u8]) -> Self {
        let mut d = Vec::new();
        let _ = d.extend_from_slice(data);
        Self {
            id: CanId::Standard(id & 0x7FF),
            frame_type: FrameType::Classic,
            data: d,
            timestamp: 0,
        }
    }

    pub fn new_fd(id: u32, data: &[u8], brs: bool) -> Self {
        let mut d = Vec::new();
        let _ = d.extend_from_slice(data);
        Self {
            id: CanId::Extended(id & 0x1FFF_FFFF),
            frame_type: if brs { FrameType::FdBrs } else { FrameType::Fd },
            data: d,
            timestamp: 0,
        }
    }
}

/// CAN bus error counters
#[derive(Clone, Copy, Debug, Default)]
pub struct CanErrorCounters {
    pub tx_error_count: u8,
    pub rx_error_count: u8,
}

/// CAN FD driver
pub struct CanFdDriver {
    state: CanState,
    config: CanFdConfig,
    errors: CanErrorCounters,
    tx_count: u32,
    rx_count: u32,
}

impl CanFdDriver {
    pub fn new(config: CanFdConfig) -> Self {
        Self {
            state: CanState::Off,
            config,
            errors: CanErrorCounters::default(),
            tx_count: 0,
            rx_count: 0,
        }
    }

    pub fn state(&self) -> CanState {
        self.state
    }

    pub fn error_counters(&self) -> CanErrorCounters {
        self.errors
    }

    pub fn tx_count(&self) -> u32 {
        self.tx_count
    }

    pub fn rx_count(&self) -> u32 {
        self.rx_count
    }

    /// Map CAN FD DLC code to actual data length
    pub fn dlc_to_len(dlc: u8) -> usize {
        match dlc {
            0..=8 => dlc as usize,
            9 => 12,
            10 => 16,
            11 => 20,
            12 => 24,
            13 => 32,
            14 => 48,
            15 => 64,
            _ => 0,
        }
    }

    /// Map data length to CAN FD DLC code
    pub fn len_to_dlc(len: usize) -> u8 {
        match len {
            0..=8 => len as u8,
            9..=12 => 9,
            13..=16 => 10,
            17..=20 => 11,
            21..=24 => 12,
            25..=32 => 13,
            33..=48 => 14,
            49..=64 => 15,
            _ => 15,
        }
    }
}

/// SCADA CAN message IDs (application layer)
pub mod scada_can_ids {
    /// Heartbeat broadcast (1 Hz)
    pub const HEARTBEAT: u16 = 0x100;
    /// Analog input values (4 channels, 16-bit each)
    pub const ANALOG_DATA: u16 = 0x200;
    /// Digital I/O status
    pub const DIGITAL_IO: u16 = 0x201;
    /// Alarm/event notification
    pub const ALARM: u16 = 0x300;
    /// Configuration request
    pub const CONFIG_REQ: u16 = 0x400;
    /// Configuration response
    pub const CONFIG_RESP: u16 = 0x401;
    /// Firmware update (CAN FD, 64 bytes)
    pub const FW_UPDATE: u16 = 0x7F0;
}
