//! Storage Subsystem
//!
//! Three-tier storage architecture:
//! - W25Q64JV SPI Flash (8MB): High-speed data logging ring buffer
//! - microSD card (SPI mode): Long-term archival, CSV/binary logs
//! - AT24C256C I2C EEPROM (32KB): Configuration, calibration, device identity

pub mod w25q64_flash;
pub mod microsd;
pub mod eeprom;

/// Storage error types
#[derive(Clone, Copy, Debug)]
pub enum StorageError {
    /// SPI/I2C communication failure
    BusError,
    /// Device not responding
    DeviceNotFound,
    /// Write protection active
    WriteProtected,
    /// Address out of range
    AddressOutOfRange,
    /// Media not present (SD card removed)
    MediaNotPresent,
    /// Filesystem error
    FilesystemError,
    /// Buffer overflow
    BufferFull,
    /// CRC/checksum mismatch
    DataCorruption,
    /// Erase timeout
    EraseTimeout,
}

/// Storage health status
#[derive(Clone, Copy, Debug, Default)]
pub struct StorageHealth {
    pub flash_available: bool,
    pub sd_card_present: bool,
    pub eeprom_available: bool,
    pub flash_usage_percent: u8,
    pub sd_free_mb: u32,
}
