//! microSD Card Driver via Native SDMMC1
//!
//! Uses STM32H743 native SDMMC1 peripheral in 4-bit mode for high-speed
//! data logging with FAT32 filesystem via `embedded-sdmmc` crate.
//!
//! Hardware: SDMMC1 4-bit mode
//!   PC12 = SDMMC1_CLK
//!   PD2  = SDMMC1_CMD
//!   PC8  = SDMMC1_D0
//!   PC9  = SDMMC1_D1
//!   PC10 = SDMMC1_D2
//!   PC11 = SDMMC1_D3
//!
//! Connector: Molex 104031-0811 push-push with card detect
//! Pull-ups: 47kΩ on all data lines (RN1)
//! Decoupling: 100nF + 10µF on VDD_SD

use heapless::{String, Vec};
use super::StorageError;

/// SD card type detected during initialization
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SdCardType {
    Unknown,
    /// SD v1.x (byte-addressed, up to 2GB)
    SdV1,
    /// SD v2.0 standard capacity (byte-addressed, up to 2GB)
    SdV2,
    /// SDHC/SDXC (block-addressed, 4GB-2TB)
    SdHC,
}

/// SD card operational state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SdState {
    /// No card inserted (card detect pin low)
    NotPresent,
    /// Card detected, running initialization sequence
    Initializing,
    /// Card initialized, filesystem mounted, ready for I/O
    Ready,
    /// Write operation in progress
    Writing,
    /// Card full or filesystem error
    Error,
}

/// SD card hardware information
#[derive(Clone, Debug)]
pub struct SdCardInfo {
    pub card_type: SdCardType,
    /// Total capacity in megabytes
    pub capacity_mb: u32,
    /// Block size (always 512 for SDHC)
    pub block_size: u16,
    /// Number of total blocks
    pub total_blocks: u32,
    /// Bus width (1 or 4)
    pub bus_width: u8,
    /// Clock speed in kHz
    pub clock_khz: u32,
}

impl Default for SdCardInfo {
    fn default() -> Self {
        Self {
            card_type: SdCardType::Unknown,
            capacity_mb: 0,
            block_size: 512,
            total_blocks: 0,
            bus_width: 4,
            clock_khz: 25_000,
        }
    }
}

/// SD card logging configuration
#[derive(Clone, Debug)]
pub struct SdLogConfig {
    /// Maximum file size before rotation (bytes, default 10MB)
    pub max_file_size: u32,
    /// Maximum number of log files to keep (ring of files)
    pub max_log_files: u16,
    /// Write buffer size (flush after N records)
    pub buffer_flush_count: u16,
    /// Directory name for log files
    pub log_directory: String<12>,
    /// File name prefix
    pub file_prefix: String<8>,
    /// Include CSV header in new files
    pub csv_header: bool,
}

impl Default for SdLogConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB per file
            max_log_files: 1000,
            buffer_flush_count: 10,
            log_directory: String::try_from("SCADA_LOG").unwrap_or_default(),
            file_prefix: String::try_from("LOG_").unwrap_or_default(),
            csv_header: true,
        }
    }
}

/// Log file naming utilities
pub struct LogFileNaming;

impl LogFileNaming {
    /// Generate sequential log filename: LOG_NNNN.CSV
    pub fn sequential_name(prefix: &str, index: u16) -> String<16> {
        let mut name = String::new();
        let _ = core::fmt::write(&mut name, format_args!("{}{:04}.CSV", prefix, index));
        name
    }

    /// Generate date-based filename: YYYYMMDD.CSV
    pub fn dated_name(year: u16, month: u8, day: u8) -> String<16> {
        let mut name = String::new();
        let _ = core::fmt::write(
            &mut name,
            format_args!("{:04}{:02}{:02}.CSV", year, month, day),
        );
        name
    }

    /// Generate date+sequence filename: MMDD_NNN.CSV (8.3 safe)
    pub fn dated_seq_name(month: u8, day: u8, seq: u8) -> String<16> {
        let mut name = String::new();
        let _ = core::fmt::write(
            &mut name,
            format_args!("{:02}{:02}_{:03}.CSV", month, day, seq),
        );
        name
    }
}

/// Write buffer for batching SD card writes
/// Accumulates CSV lines and flushes when full or on demand
pub struct SdWriteBuffer {
    buffer: Vec<u8, 4096>,
    records_buffered: u16,
    flush_threshold: u16,
    bytes_written_total: u64,
    writes_total: u32,
}

impl SdWriteBuffer {
    pub fn new(flush_threshold: u16) -> Self {
        Self {
            buffer: Vec::new(),
            records_buffered: 0,
            flush_threshold,
            bytes_written_total: 0,
            writes_total: 0,
        }
    }

    /// Append a CSV line to the write buffer
    /// Returns true if buffer should be flushed
    pub fn append(&mut self, csv_line: &str) -> bool {
        let _ = self.buffer.extend_from_slice(csv_line.as_bytes());
        self.records_buffered += 1;
        self.records_buffered >= self.flush_threshold
    }

    /// Get buffer contents for writing to SD card
    pub fn data(&self) -> &[u8] {
        &self.buffer
    }

    /// Number of bytes currently buffered
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear buffer after successful write
    pub fn clear(&mut self) {
        let written = self.buffer.len() as u64;
        self.buffer.clear();
        self.records_buffered = 0;
        self.bytes_written_total += written;
        self.writes_total += 1;
    }

    /// Total bytes written to SD card across all flushes
    pub fn total_bytes_written(&self) -> u64 {
        self.bytes_written_total
    }

    /// Total number of flush operations
    pub fn total_writes(&self) -> u32 {
        self.writes_total
    }

    /// Check if buffer is near capacity (>75% full)
    pub fn is_near_full(&self) -> bool {
        self.buffer.len() > 3072
    }
}

/// SD card statistics for monitoring
#[derive(Clone, Copy, Debug, Default)]
pub struct SdStats {
    /// Current log file index
    pub current_file_index: u16,
    /// Bytes written to current file
    pub current_file_bytes: u32,
    /// Total records written across all files
    pub total_records: u64,
    /// Total bytes written to card
    pub total_bytes: u64,
    /// Number of write errors
    pub write_errors: u32,
    /// Number of file rotations
    pub file_rotations: u16,
    /// Estimated free space in MB
    pub free_space_mb: u32,
}

/// microSD card driver with FAT32 filesystem
pub struct MicroSdDriver {
    state: SdState,
    card_info: SdCardInfo,
    config: SdLogConfig,
    write_buffer: SdWriteBuffer,
    stats: SdStats,
}

impl MicroSdDriver {
    pub fn new(config: SdLogConfig) -> Self {
        let flush_threshold = config.buffer_flush_count;
        Self {
            state: SdState::NotPresent,
            card_info: SdCardInfo::default(),
            config,
            write_buffer: SdWriteBuffer::new(flush_threshold),
            stats: SdStats::default(),
        }
    }

    pub fn state(&self) -> SdState {
        self.state
    }

    pub fn card_info(&self) -> &SdCardInfo {
        &self.card_info
    }

    pub fn stats(&self) -> &SdStats {
        &self.stats
    }

    pub fn is_ready(&self) -> bool {
        self.state == SdState::Ready
    }

    pub fn set_state(&mut self, state: SdState) {
        self.state = state;
    }

    pub fn set_card_info(&mut self, info: SdCardInfo) {
        self.card_info = info;
    }

    /// Add a data record as CSV to the write buffer
    /// Returns true if the buffer is ready to flush
    pub fn buffer_record(&mut self, csv_line: &str) -> bool {
        self.write_buffer.append(csv_line)
    }

    /// Get the buffered data for writing
    pub fn flush_buffer(&mut self) -> &[u8] {
        let data = self.write_buffer.data();
        data
    }

    /// Mark buffer as successfully written
    pub fn confirm_write(&mut self, bytes: u32) {
        self.write_buffer.clear();
        self.stats.current_file_bytes += bytes;
        self.stats.total_bytes += bytes as u64;
    }

    /// Record a write error
    pub fn record_error(&mut self) {
        self.stats.write_errors += 1;
    }

    /// Check if current log file needs rotation
    pub fn needs_rotation(&self) -> bool {
        self.stats.current_file_bytes >= self.config.max_file_size
    }

    /// Advance to next log file
    pub fn rotate_file(&mut self) {
        self.stats.current_file_index =
            (self.stats.current_file_index + 1) % self.config.max_log_files;
        self.stats.current_file_bytes = 0;
        self.stats.file_rotations += 1;
    }

    /// Get the current log filename
    pub fn current_filename(&self) -> String<16> {
        LogFileNaming::sequential_name(
            self.config.file_prefix.as_str(),
            self.stats.current_file_index,
        )
    }

    /// Force flush any remaining buffered data
    pub fn has_pending_data(&self) -> bool {
        !self.write_buffer.is_empty()
    }

    /// CSV header line for SCADA data files
    pub fn csv_header() -> &'static str {
        "timestamp_ms,seq,ch0,ch1,ch2,ch3,di_mask,do_mask,mcu_temp_C,supply_V\r\n"
    }

    /// SDMMC1 clock frequency for initialization (400 kHz)
    pub const INIT_CLOCK_KHZ: u32 = 400;

    /// SDMMC1 clock frequency for data transfer (25 MHz)
    pub const DATA_CLOCK_KHZ: u32 = 25_000;

    /// SDMMC1 clock frequency for high-speed mode (50 MHz)
    pub const HS_CLOCK_KHZ: u32 = 50_000;
}

/// SDMMC1 register block helpers for low-level initialization
pub mod sdmmc_hal {
    /// SDMMC command indices
    pub const CMD0_GO_IDLE: u8 = 0;
    pub const CMD2_ALL_SEND_CID: u8 = 2;
    pub const CMD3_SEND_RCA: u8 = 3;
    pub const CMD7_SELECT_CARD: u8 = 7;
    pub const CMD8_SEND_IF_COND: u8 = 8;
    pub const CMD12_STOP_TRANSMISSION: u8 = 12;
    pub const CMD13_SEND_STATUS: u8 = 13;
    pub const CMD16_SET_BLOCKLEN: u8 = 16;
    pub const CMD17_READ_SINGLE: u8 = 17;
    pub const CMD18_READ_MULTIPLE: u8 = 18;
    pub const CMD24_WRITE_SINGLE: u8 = 24;
    pub const CMD25_WRITE_MULTIPLE: u8 = 25;
    pub const CMD55_APP_CMD: u8 = 55;
    pub const CMD58_READ_OCR: u8 = 58;
    pub const ACMD6_SET_BUS_WIDTH: u8 = 6;
    pub const ACMD41_SD_SEND_OP_COND: u8 = 41;

    /// Card status bits
    pub const CARD_STATUS_READY: u32 = 1 << 8;
    pub const CARD_STATUS_ERROR: u32 = 0xFDF9_0008;

    /// OCR register bits
    pub const OCR_HCS: u32 = 1 << 30; // Host Capacity Support (SDHC)
    pub const OCR_BUSY: u32 = 1 << 31; // Card power up status

    /// Block size for all operations
    pub const BLOCK_SIZE: u32 = 512;
}
