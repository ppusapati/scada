//! W25Q64JV SPI NOR Flash Driver
//!
//! 8MB (64Mbit) flash with 4KB sector erase, 64KB block erase.
//! Used as high-speed ring buffer for data logging.
//!
//! Hardware: SPI3 (PC10=SCK, PC11=MISO, PC12=MOSI, PA15=CS)
//! Specs: 133MHz max SPI, 100K erase cycles, 20-year retention

use heapless::Vec;
use super::StorageError;

/// W25Q64JV identification
pub const JEDEC_MANUFACTURER: u8 = 0xEF; // Winbond
pub const JEDEC_DEVICE_ID: u16 = 0x4017; // W25Q64JV

/// Flash geometry
pub const FLASH_SIZE: u32 = 8 * 1024 * 1024; // 8MB
pub const SECTOR_SIZE: u32 = 4096; // 4KB
pub const BLOCK_SIZE_32K: u32 = 32 * 1024;
pub const BLOCK_SIZE_64K: u32 = 64 * 1024;
pub const PAGE_SIZE: u32 = 256;
pub const SECTOR_COUNT: u32 = FLASH_SIZE / SECTOR_SIZE; // 2048 sectors

/// SPI commands
mod cmd {
    pub const WRITE_ENABLE: u8 = 0x06;
    pub const WRITE_DISABLE: u8 = 0x04;
    pub const READ_STATUS_1: u8 = 0x05;
    pub const READ_STATUS_2: u8 = 0x35;
    pub const WRITE_STATUS: u8 = 0x01;
    pub const READ_DATA: u8 = 0x03;
    pub const FAST_READ: u8 = 0x0B;
    pub const PAGE_PROGRAM: u8 = 0x02;
    pub const SECTOR_ERASE: u8 = 0x20;
    pub const BLOCK_ERASE_32K: u8 = 0x52;
    pub const BLOCK_ERASE_64K: u8 = 0xD8;
    pub const CHIP_ERASE: u8 = 0xC7;
    pub const POWER_DOWN: u8 = 0xB9;
    pub const RELEASE_POWER_DOWN: u8 = 0xAB;
    pub const JEDEC_ID: u8 = 0x9F;
    pub const UNIQUE_ID: u8 = 0x4B;
}

/// Status register 1 bits
pub const STATUS_BUSY: u8 = 0x01;
pub const STATUS_WEL: u8 = 0x02;

/// Ring buffer management for data logging
/// Divides flash into a circular buffer of fixed-size records
pub struct FlashRingBuffer {
    /// Start address of ring buffer region
    base_address: u32,
    /// Total size of ring buffer region
    region_size: u32,
    /// Size of each record (must be sector-aligned for erase)
    record_size: u32,
    /// Write pointer (next sector to write)
    write_index: u32,
    /// Number of valid records
    record_count: u32,
    /// Total capacity in records
    capacity: u32,
}

impl FlashRingBuffer {
    /// Create a ring buffer using the full flash
    pub fn new(record_size: u32) -> Self {
        let capacity = FLASH_SIZE / record_size;
        Self {
            base_address: 0,
            region_size: FLASH_SIZE,
            record_size,
            write_index: 0,
            record_count: 0,
            capacity,
        }
    }

    /// Create a ring buffer in a specific flash region
    pub fn new_region(base_address: u32, region_size: u32, record_size: u32) -> Self {
        let capacity = region_size / record_size;
        Self {
            base_address,
            region_size,
            record_size,
            write_index: 0,
            record_count: 0,
            capacity,
        }
    }

    pub fn is_full(&self) -> bool {
        self.record_count >= self.capacity
    }

    pub fn usage_percent(&self) -> u8 {
        if self.capacity == 0 {
            return 100;
        }
        ((self.record_count * 100) / self.capacity) as u8
    }

    /// Get the address for the next write
    pub fn next_write_address(&self) -> u32 {
        self.base_address + (self.write_index * self.record_size)
    }

    /// Advance write pointer (wraps around)
    pub fn advance_write(&mut self) {
        self.write_index = (self.write_index + 1) % self.capacity;
        if self.record_count < self.capacity {
            self.record_count += 1;
        }
    }
}

/// W25Q64JV flash driver
pub struct W25Q64Driver {
    ring_buffer: FlashRingBuffer,
    initialized: bool,
}

impl W25Q64Driver {
    pub fn new() -> Self {
        // Default: 4KB records = 2048 log entries
        Self {
            ring_buffer: FlashRingBuffer::new(SECTOR_SIZE),
            initialized: false,
        }
    }

    pub fn with_record_size(record_size: u32) -> Self {
        Self {
            ring_buffer: FlashRingBuffer::new(record_size),
            initialized: false,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn ring_buffer(&self) -> &FlashRingBuffer {
        &self.ring_buffer
    }

    /// Build JEDEC ID read command
    pub fn cmd_read_jedec_id() -> [u8; 4] {
        [cmd::JEDEC_ID, 0, 0, 0]
    }

    /// Build a page program command (address + up to 256 bytes data)
    pub fn cmd_page_program(address: u32) -> [u8; 4] {
        [
            cmd::PAGE_PROGRAM,
            (address >> 16) as u8,
            (address >> 8) as u8,
            address as u8,
        ]
    }

    /// Build a sector erase command (4KB)
    pub fn cmd_sector_erase(address: u32) -> [u8; 4] {
        [
            cmd::SECTOR_ERASE,
            (address >> 16) as u8,
            (address >> 8) as u8,
            address as u8,
        ]
    }

    /// Build a read command
    pub fn cmd_read(address: u32) -> [u8; 4] {
        [
            cmd::READ_DATA,
            (address >> 16) as u8,
            (address >> 8) as u8,
            address as u8,
        ]
    }

    /// Build write enable command
    pub fn cmd_write_enable() -> [u8; 1] {
        [cmd::WRITE_ENABLE]
    }

    /// Build read status register 1 command
    pub fn cmd_read_status() -> [u8; 2] {
        [cmd::READ_STATUS_1, 0]
    }

    /// Check if status byte indicates busy
    pub fn is_busy(status: u8) -> bool {
        status & STATUS_BUSY != 0
    }
}
