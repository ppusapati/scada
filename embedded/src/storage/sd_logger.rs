//! SD Card Data Logger
//!
//! High-level data logging to microSD card with:
//! - Automatic file rotation by size
//! - CSV format with configurable columns
//! - Write buffering (batch writes for SD card longevity)
//! - Safe shutdown with buffer flush
//! - Error recovery and card re-initialization
//!
//! Storage layout on SD card:
//!   /SCADA_LOG/
//!     LOG_0000.CSV
//!     LOG_0001.CSV
//!     ...
//!     LOG_0999.CSV  (wraps around, overwrites oldest)
//!   /SCADA_CFG/
//!     CONFIG.BIN    (device configuration backup)
//!     CALIB.BIN     (calibration data backup)
//!   /SCADA_FW/
//!     (firmware update staging area)

use heapless::{String, Vec};
use super::StorageError;
use super::microsd::{MicroSdDriver, SdState, SdLogConfig, SdStats};
use crate::datalogger::DataRecord;

/// SD logger operational mode
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SdLoggerMode {
    /// Not active (card not present or not initialized)
    Inactive,
    /// Logging actively to CSV files
    Logging,
    /// Paused (card present but logging suspended)
    Paused,
    /// Error state (will attempt recovery)
    Error,
}

/// SD card directory names
pub mod directories {
    pub const LOG_DIR: &str = "SCADA_LOG";
    pub const CONFIG_DIR: &str = "SCADA_CFG";
    pub const FIRMWARE_DIR: &str = "SCADA_FW";
}

/// SD card logger - manages file creation, writing, and rotation
pub struct SdLogger {
    mode: SdLoggerMode,
    sd: MicroSdDriver,
    /// CSV write buffer (accumulates lines before flushing to card)
    csv_buffer: Vec<u8, 8192>,
    /// Number of records in current buffer
    buffered_records: u16,
    /// Current file size in bytes
    current_file_size: u32,
    /// Current log file index (0-999)
    file_index: u16,
    /// Max file size before rotation
    max_file_size: u32,
    /// Records written since last flush
    records_since_flush: u16,
    /// Flush interval (records)
    flush_interval: u16,
    /// Total records written to SD
    total_records: u64,
    /// Total write errors
    error_count: u32,
    /// Flag: current file has header written
    header_written: bool,
}

impl SdLogger {
    pub fn new(config: SdLogConfig) -> Self {
        let flush_interval = config.buffer_flush_count;
        let max_file_size = config.max_file_size;
        Self {
            mode: SdLoggerMode::Inactive,
            sd: MicroSdDriver::new(config),
            csv_buffer: Vec::new(),
            buffered_records: 0,
            current_file_size: 0,
            file_index: 0,
            max_file_size,
            records_since_flush: 0,
            flush_interval,
            total_records: 0,
            error_count: 0,
            header_written: false,
        }
    }

    pub fn mode(&self) -> SdLoggerMode {
        self.mode
    }

    pub fn file_index(&self) -> u16 {
        self.file_index
    }

    pub fn total_records(&self) -> u64 {
        self.total_records
    }

    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    pub fn current_file_size(&self) -> u32 {
        self.current_file_size
    }

    pub fn sd_driver(&self) -> &MicroSdDriver {
        &self.sd
    }

    pub fn sd_driver_mut(&mut self) -> &mut MicroSdDriver {
        &mut self.sd
    }

    /// Start logging (call after SD card initialization succeeds)
    pub fn start_logging(&mut self) {
        self.mode = SdLoggerMode::Logging;
        self.header_written = false;
    }

    /// Pause logging (keeps card mounted)
    pub fn pause(&mut self) {
        if self.mode == SdLoggerMode::Logging {
            self.mode = SdLoggerMode::Paused;
        }
    }

    /// Resume logging after pause
    pub fn resume(&mut self) {
        if self.mode == SdLoggerMode::Paused {
            self.mode = SdLoggerMode::Logging;
        }
    }

    /// Stop logging completely
    pub fn stop(&mut self) {
        self.mode = SdLoggerMode::Inactive;
    }

    /// Get CSV header bytes (returned once per new file)
    pub fn get_header_if_needed(&mut self) -> Option<&'static [u8]> {
        if !self.header_written {
            self.header_written = true;
            Some(MicroSdDriver::csv_header().as_bytes())
        } else {
            None
        }
    }

    /// Add a data record to the CSV buffer
    /// Returns the buffer contents when ready to flush, or None if still accumulating
    pub fn log_record(&mut self, record: &DataRecord) -> Option<&[u8]> {
        if self.mode != SdLoggerMode::Logging {
            return None;
        }

        // Format record as CSV line
        let csv_line = record.to_csv_line();
        let line_bytes = csv_line.as_bytes();

        // Check if buffer has room
        if self.csv_buffer.len() + line_bytes.len() > self.csv_buffer.capacity() {
            // Buffer is full, return it for flushing before adding new data
            return Some(&self.csv_buffer);
        }

        let _ = self.csv_buffer.extend_from_slice(line_bytes);
        self.buffered_records += 1;
        self.records_since_flush += 1;
        self.total_records += 1;

        // Check if we should flush
        if self.records_since_flush >= self.flush_interval {
            Some(&self.csv_buffer)
        } else {
            None
        }
    }

    /// Get current buffer contents for writing (without clearing)
    pub fn pending_data(&self) -> &[u8] {
        &self.csv_buffer
    }

    /// Returns true if there's data waiting to be written
    pub fn has_pending_data(&self) -> bool {
        !self.csv_buffer.is_empty()
    }

    /// Number of bytes currently buffered
    pub fn pending_bytes(&self) -> usize {
        self.csv_buffer.len()
    }

    /// Confirm that buffered data was successfully written to SD card
    pub fn confirm_flush(&mut self) {
        let bytes = self.csv_buffer.len() as u32;
        self.current_file_size += bytes;
        self.csv_buffer.clear();
        self.buffered_records = 0;
        self.records_since_flush = 0;
    }

    /// Record a write failure
    pub fn record_write_error(&mut self) {
        self.error_count += 1;
        // Don't clear buffer on error - retry on next cycle
        if self.error_count > 10 {
            self.mode = SdLoggerMode::Error;
        }
    }

    /// Check if the current file needs rotation (size limit reached)
    pub fn needs_file_rotation(&self) -> bool {
        self.current_file_size >= self.max_file_size
    }

    /// Rotate to the next log file
    pub fn rotate_file(&mut self, max_files: u16) {
        self.file_index = (self.file_index + 1) % max_files;
        self.current_file_size = 0;
        self.header_written = false;
    }

    /// Get current log filename (8.3 format for FAT32)
    pub fn current_filename(&self) -> String<16> {
        let mut name = String::new();
        let _ = core::fmt::write(
            &mut name,
            format_args!("LOG_{:04}.CSV", self.file_index),
        );
        name
    }

    /// Attempt error recovery (re-initialize card)
    pub fn attempt_recovery(&mut self) {
        if self.mode == SdLoggerMode::Error {
            self.error_count = 0;
            self.csv_buffer.clear();
            self.buffered_records = 0;
            self.records_since_flush = 0;
            self.mode = SdLoggerMode::Inactive;
            // Caller should re-run SD card init sequence
        }
    }

    /// Get a diagnostic summary
    pub fn diagnostics(&self) -> SdLoggerDiagnostics {
        SdLoggerDiagnostics {
            mode: self.mode,
            file_index: self.file_index,
            file_size: self.current_file_size,
            total_records: self.total_records,
            buffer_used: self.csv_buffer.len() as u16,
            buffer_capacity: self.csv_buffer.capacity() as u16,
            error_count: self.error_count,
        }
    }
}

/// Diagnostic info for BLE/telemetry reporting
#[derive(Clone, Debug)]
pub struct SdLoggerDiagnostics {
    pub mode: SdLoggerMode,
    pub file_index: u16,
    pub file_size: u32,
    pub total_records: u64,
    pub buffer_used: u16,
    pub buffer_capacity: u16,
    pub error_count: u32,
}

/// Configuration file operations
pub struct ConfigBackup;

impl ConfigBackup {
    /// Config backup filename
    pub const CONFIG_FILE: &'static str = "CONFIG.BIN";
    /// Calibration backup filename
    pub const CALIB_FILE: &'static str = "CALIB.BIN";

    /// File header magic bytes for validation
    pub const CONFIG_MAGIC: [u8; 4] = [0x53, 0x43, 0x46, 0x47]; // "SCFG"
    pub const CALIB_MAGIC: [u8; 4] = [0x53, 0x43, 0x41, 0x4C]; // "SCAL"

    /// Build a config backup block with header and CRC
    pub fn build_config_block(data: &[u8]) -> Vec<u8, 512> {
        let mut block = Vec::new();
        // Magic header
        let _ = block.extend_from_slice(&Self::CONFIG_MAGIC);
        // Version
        let _ = block.push(0x01);
        // Data length
        let _ = block.extend_from_slice(&(data.len() as u16).to_le_bytes());
        // Payload
        let _ = block.extend_from_slice(data);
        // CRC-16 over everything before CRC
        let crc = crc16(&block);
        let _ = block.extend_from_slice(&crc.to_le_bytes());
        block
    }

    /// Validate a config backup block
    pub fn validate_config_block(data: &[u8]) -> Result<&[u8], StorageError> {
        if data.len() < 9 {
            return Err(StorageError::DataCorruption);
        }
        if data[0..4] != Self::CONFIG_MAGIC {
            return Err(StorageError::DataCorruption);
        }
        let payload_len = u16::from_le_bytes([data[5], data[6]]) as usize;
        let total_len = 7 + payload_len + 2;
        if data.len() < total_len {
            return Err(StorageError::DataCorruption);
        }
        let stored_crc = u16::from_le_bytes([data[total_len - 2], data[total_len - 1]]);
        if crc16(&data[..total_len - 2]) != stored_crc {
            return Err(StorageError::DataCorruption);
        }
        Ok(&data[7..7 + payload_len])
    }
}

/// CRC-16 for data integrity
fn crc16(data: &[u8]) -> u16 {
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
