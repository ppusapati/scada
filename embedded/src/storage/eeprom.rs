//! AT24C256C I2C EEPROM Driver
//!
//! 32KB EEPROM for device configuration, calibration data, and identity.
//! 64-byte page write, 1M write cycles, 100-year retention.
//!
//! Hardware: I2C1 (PB6=SCL, PB7=SDA)
//! Address: 0x50 (A0=A1=A2=GND)

use heapless::Vec;
use super::StorageError;

/// EEPROM I2C address (7-bit)
pub const EEPROM_ADDRESS: u8 = 0x50;

/// EEPROM geometry
pub const EEPROM_SIZE: u32 = 32 * 1024; // 32KB
pub const PAGE_SIZE: u32 = 64;
pub const PAGE_COUNT: u32 = EEPROM_SIZE / PAGE_SIZE;

/// Memory map for SCADA configuration storage
pub mod memory_map {
    /// Device identity (address 0x0000, 64 bytes)
    pub const DEVICE_ID_ADDR: u16 = 0x0000;
    pub const DEVICE_ID_SIZE: u16 = 64;

    /// Network configuration (address 0x0040, 256 bytes)
    pub const NETWORK_CONFIG_ADDR: u16 = 0x0040;
    pub const NETWORK_CONFIG_SIZE: u16 = 256;

    /// Analog calibration data (address 0x0140, 128 bytes)
    /// 4 channels x (offset_f32 + gain_f32) = 32 bytes + CRC
    pub const ANALOG_CAL_ADDR: u16 = 0x0140;
    pub const ANALOG_CAL_SIZE: u16 = 128;

    /// Modbus configuration (address 0x01C0, 64 bytes)
    pub const MODBUS_CONFIG_ADDR: u16 = 0x01C0;
    pub const MODBUS_CONFIG_SIZE: u16 = 64;

    /// LoRa configuration (address 0x0200, 64 bytes)
    pub const LORA_CONFIG_ADDR: u16 = 0x0200;
    pub const LORA_CONFIG_SIZE: u16 = 64;

    /// Alarm thresholds (address 0x0240, 256 bytes)
    pub const ALARM_CONFIG_ADDR: u16 = 0x0240;
    pub const ALARM_CONFIG_SIZE: u16 = 256;

    /// Boot counter and runtime stats (address 0x0340, 64 bytes)
    pub const STATS_ADDR: u16 = 0x0340;
    pub const STATS_SIZE: u16 = 64;

    /// User-defined area (address 0x0400, remaining ~31KB)
    pub const USER_AREA_ADDR: u16 = 0x0400;
}

/// Device identity stored in EEPROM
#[derive(Clone, Debug)]
pub struct DeviceIdentity {
    pub serial_number: [u8; 16],
    pub hardware_revision: u8,
    pub firmware_version: [u8; 3], // major.minor.patch
    pub manufacture_date: u32,     // Unix timestamp
    pub crc16: u16,
}

/// Analog channel calibration
#[derive(Clone, Copy, Debug)]
pub struct ChannelCalibration {
    pub offset: f32,
    pub gain: f32,
}

impl Default for ChannelCalibration {
    fn default() -> Self {
        Self {
            offset: 0.0,
            gain: 1.0,
        }
    }
}

/// AT24C256 EEPROM driver
pub struct EepromDriver {
    initialized: bool,
    boot_count: u32,
}

impl EepromDriver {
    pub fn new() -> Self {
        Self {
            initialized: false,
            boot_count: 0,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn boot_count(&self) -> u32 {
        self.boot_count
    }

    /// Build I2C write command for a 16-bit address EEPROM
    /// Returns [addr_high, addr_low] prefix for I2C transaction
    pub fn address_bytes(address: u16) -> [u8; 2] {
        [(address >> 8) as u8, address as u8]
    }

    /// Calculate CRC-16 for configuration data integrity
    pub fn crc16(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &byte in data {
            crc ^= byte as u16;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }

    /// Serialize calibration data for 4 channels into bytes
    pub fn serialize_calibration(cal: &[ChannelCalibration; 4]) -> Vec<u8, 36> {
        let mut buf = Vec::new();
        for ch in cal {
            let _ = buf.extend_from_slice(&ch.offset.to_le_bytes());
            let _ = buf.extend_from_slice(&ch.gain.to_le_bytes());
        }
        // Append CRC
        let crc = Self::crc16(&buf);
        let _ = buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }

    /// Deserialize calibration data, verifying CRC
    pub fn deserialize_calibration(data: &[u8]) -> Result<[ChannelCalibration; 4], StorageError> {
        if data.len() < 34 {
            return Err(StorageError::DataCorruption);
        }
        let payload = &data[..32];
        let stored_crc = u16::from_le_bytes([data[32], data[33]]);
        if Self::crc16(payload) != stored_crc {
            return Err(StorageError::DataCorruption);
        }
        let mut cal = [ChannelCalibration::default(); 4];
        for i in 0..4 {
            let base = i * 8;
            cal[i].offset = f32::from_le_bytes([
                payload[base],
                payload[base + 1],
                payload[base + 2],
                payload[base + 3],
            ]);
            cal[i].gain = f32::from_le_bytes([
                payload[base + 4],
                payload[base + 5],
                payload[base + 6],
                payload[base + 7],
            ]);
        }
        Ok(cal)
    }
}
