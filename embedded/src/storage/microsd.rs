//! microSD Card Driver (SPI Mode)
//!
//! FAT32 filesystem access via SPI mode for long-term data archival.
//! Uses Molex 104031-0811 push-push connector with card detect.
//!
//! Hardware: SDMMC1 or SPI mode via GPIO
//! Note: STM32H743 has native SDMMC but SPI mode used for simplicity

use heapless::String;
use super::StorageError;

/// SD card type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SdCardType {
    Unknown,
    SdV1,
    SdV2,
    SdHC, // SDHC/SDXC (block-addressed)
}

/// SD card state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SdState {
    NotPresent,
    Initializing,
    Ready,
    Error,
}

/// SD card info
#[derive(Clone, Debug, Default)]
pub struct SdCardInfo {
    pub card_type: Option<SdCardType>,
    pub capacity_mb: u32,
    pub block_size: u16,
}

/// Log file naming
pub struct LogFileNaming;

impl LogFileNaming {
    /// Generate a log filename: LOG_NNNN.CSV (sequential)
    pub fn sequential_name(index: u16) -> String<16> {
        let mut name = String::new();
        let _ = core::fmt::write(&mut name, format_args!("LOG_{:04}.CSV", index));
        name
    }

    /// Generate a dated filename: YYYYMMDD.CSV
    pub fn dated_name(year: u16, month: u8, day: u8) -> String<16> {
        let mut name = String::new();
        let _ = core::fmt::write(
            &mut name,
            format_args!("{:04}{:02}{:02}.CSV", year, month, day),
        );
        name
    }
}

/// SD card driver
pub struct MicroSdDriver {
    state: SdState,
    card_info: SdCardInfo,
    current_log_index: u16,
}

impl MicroSdDriver {
    pub fn new() -> Self {
        Self {
            state: SdState::NotPresent,
            card_info: SdCardInfo::default(),
            current_log_index: 0,
        }
    }

    pub fn state(&self) -> SdState {
        self.state
    }

    pub fn card_info(&self) -> &SdCardInfo {
        &self.card_info
    }

    pub fn is_ready(&self) -> bool {
        self.state == SdState::Ready
    }

    /// SPI mode initialization sequence commands
    /// Returns the CMD0 (GO_IDLE_STATE) bytes
    pub fn cmd0_go_idle() -> [u8; 6] {
        [0x40, 0x00, 0x00, 0x00, 0x00, 0x95] // CMD0 + valid CRC
    }

    /// CMD8 (SEND_IF_COND) - voltage check for SD v2
    pub fn cmd8_send_if_cond() -> [u8; 6] {
        [0x48, 0x00, 0x00, 0x01, 0xAA, 0x87] // CMD8 + check pattern
    }

    /// CMD58 (READ_OCR)
    pub fn cmd58_read_ocr() -> [u8; 6] {
        [0x7A, 0x00, 0x00, 0x00, 0x00, 0x01]
    }

    /// ACMD41 (SD_SEND_OP_COND) - initiate initialization
    pub fn acmd41_sd_send_op_cond() -> [u8; 6] {
        [0x69, 0x40, 0x00, 0x00, 0x00, 0x01] // HCS bit set
    }

    /// CMD55 (APP_CMD) - prefix for ACMD commands
    pub fn cmd55_app_cmd() -> [u8; 6] {
        [0x77, 0x00, 0x00, 0x00, 0x00, 0x01]
    }

    /// Format a CSV header line for SCADA data logging
    pub fn csv_header() -> &'static str {
        "timestamp,ch0_mA,ch1_mA,ch2_V,ch3_V,di_mask,do_mask,temp_C,vbat_V\r\n"
    }
}
