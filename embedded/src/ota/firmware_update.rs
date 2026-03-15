//! OTA Firmware Update Manager
//!
//! Manages the complete firmware update lifecycle using STM32H743
//! dual-bank flash architecture for safe, atomic updates.
//!
//! Architecture:
//!   Bank 1 (1MB): Active firmware (bootloader + application)
//!   Bank 2 (1MB): OTA staging area (receives new firmware)
//!
//! The update is atomic: on success, option bytes are modified to
//! swap banks, and a system reset activates the new firmware.
//! If the new firmware fails to boot (watchdog timeout), the
//! bootloader automatically rolls back to the previous bank.
//!
//! Supported download channels:
//! - Ethernet (W5500) via MQTT or HTTP
//! - WiFi (ATWINC1500) via MQTT or HTTP
//! - GSM (SIM7600) via MQTT or HTTP
//! - LoRa (SX1276) via custom protocol (small chunks)
//! - BLE (RN4870) via custom GATT service (local update)
//! - SD Card: firmware file in /SCADA_FW/ directory

use heapless::{String, Vec};
use crate::security::secure_boot::FirmwareHeader;
use crate::security::flash_protection::flash_map;

/// OTA update state machine
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OtaState {
    /// Idle, no update in progress
    Idle,
    /// Metadata received, awaiting approval
    PendingApproval,
    /// Erasing Bank 2 staging area
    Erasing,
    /// Downloading firmware chunks
    Downloading,
    /// Download complete, verifying integrity
    Verifying,
    /// Verified, ready to apply (bank swap)
    ReadyToApply,
    /// Bank swap programmed, pending reboot
    PendingReboot,
    /// Error state (see OtaError for details)
    Error,
}

/// OTA update errors
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OtaError {
    /// Not a newer firmware version
    NotNewer,
    /// Hardware compatibility mismatch
    HardwareMismatch,
    /// Image too large for staging area
    ImageTooLarge,
    /// Flash erase failed
    EraseFailed,
    /// Flash write failed
    WriteFailed,
    /// CRC-32 verification failed
    CrcMismatch,
    /// Signature verification failed
    SignatureInvalid,
    /// Download timeout (no chunks received)
    DownloadTimeout,
    /// Chunk sequence error (gap or duplicate)
    ChunkSequenceError,
    /// Bank swap failed
    BankSwapFailed,
    /// Already in progress
    AlreadyInProgress,
    /// Debug build rejected in production
    DebugBuildRejected,
}

/// Source of the firmware update
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OtaSource {
    /// MQTT via Ethernet/WiFi/GSM
    Mqtt,
    /// HTTP download via Ethernet/WiFi/GSM
    Http,
    /// LoRa custom protocol
    LoRa,
    /// BLE GATT service
    Ble,
    /// SD card file
    SdCard,
}

/// Firmware update metadata (sent by server before download)
#[derive(Clone, Debug)]
pub struct UpdateMetadata {
    /// New firmware version
    pub version_major: u8,
    pub version_minor: u8,
    pub version_patch: u8,
    /// Hardware compatibility ID
    pub hw_compat_id: u32,
    /// Total firmware image size (header + payload)
    pub image_size: u32,
    /// CRC-32 of complete image
    pub image_crc32: u32,
    /// Number of chunks the image will be sent in
    pub total_chunks: u16,
    /// Chunk size in bytes (typically 1024 or 4096)
    pub chunk_size: u16,
    /// Whether the image is signed
    pub is_signed: bool,
}

impl UpdateMetadata {
    /// Version as display string
    pub fn version_string(&self) -> String<12> {
        let mut s = String::new();
        let _ = core::fmt::write(
            &mut s,
            format_args!("{}.{}.{}", self.version_major, self.version_minor, self.version_patch),
        );
        s
    }

    /// Validate metadata before starting download
    pub fn validate(&self, current_header: &FirmwareHeader) -> Result<(), OtaError> {
        // Check hardware compatibility
        if self.hw_compat_id != FirmwareHeader::HW_COMPAT_STM32H743_DL {
            return Err(OtaError::HardwareMismatch);
        }

        // Check image fits in staging area
        if self.image_size > flash_map::OTA_STAGING_SIZE {
            return Err(OtaError::ImageTooLarge);
        }

        // Check this is a newer version
        let new_ver = (self.version_major as u32) << 16
            | (self.version_minor as u32) << 8
            | self.version_patch as u32;
        let cur_ver = (current_header.fw_version_major as u32) << 16
            | (current_header.fw_version_minor as u32) << 8
            | current_header.fw_version_patch as u32;
        if new_ver <= cur_ver {
            return Err(OtaError::NotNewer);
        }

        Ok(())
    }
}

/// OTA download progress tracking
#[derive(Clone, Debug)]
pub struct DownloadProgress {
    /// Total bytes expected
    pub total_bytes: u32,
    /// Bytes received so far
    pub received_bytes: u32,
    /// Total chunks expected
    pub total_chunks: u16,
    /// Chunks received
    pub received_chunks: u16,
    /// Next expected chunk index
    pub next_chunk: u16,
    /// Running CRC-32 computed over received data
    pub running_crc: u32,
    /// Download start timestamp (ms since boot)
    pub start_time_ms: u64,
    /// Last chunk received timestamp
    pub last_chunk_time_ms: u64,
    /// Number of retried chunks
    pub retried_chunks: u16,
}

impl DownloadProgress {
    pub fn new(total_bytes: u32, total_chunks: u16) -> Self {
        Self {
            total_bytes,
            received_bytes: 0,
            total_chunks,
            received_chunks: 0,
            next_chunk: 0,
            running_crc: 0xFFFF_FFFF,
            start_time_ms: 0,
            last_chunk_time_ms: 0,
            retried_chunks: 0,
        }
    }

    /// Progress as percentage (0-100)
    pub fn percent(&self) -> u8 {
        if self.total_bytes == 0 {
            return 0;
        }
        ((self.received_bytes as u64 * 100) / self.total_bytes as u64) as u8
    }

    /// Check if download is complete
    pub fn is_complete(&self) -> bool {
        self.received_chunks >= self.total_chunks
    }

    /// Update running CRC with new chunk data
    pub fn update_crc(&mut self, data: &[u8]) {
        for &byte in data {
            self.running_crc ^= byte as u32;
            for _ in 0..8 {
                if self.running_crc & 1 != 0 {
                    self.running_crc = (self.running_crc >> 1) ^ 0xEDB8_8320;
                } else {
                    self.running_crc >>= 1;
                }
            }
        }
    }

    /// Finalize CRC (invert)
    pub fn finalize_crc(&self) -> u32 {
        !self.running_crc
    }

    /// Elapsed time in seconds
    pub fn elapsed_secs(&self, now_ms: u64) -> u32 {
        ((now_ms - self.start_time_ms) / 1000) as u32
    }
}

/// OTA firmware update manager
pub struct FirmwareUpdater {
    state: OtaState,
    source: Option<OtaSource>,
    metadata: Option<UpdateMetadata>,
    progress: Option<DownloadProgress>,
    error: Option<OtaError>,
    /// Flash write address cursor (within Bank 2)
    write_cursor: u32,
    /// Whether production mode is active (reject debug builds)
    production_mode: bool,
    /// Number of successful updates performed
    update_count: u16,
    /// Boot confirmation flag - set by app after successful boot
    boot_confirmed: bool,
}

impl FirmwareUpdater {
    pub fn new(production_mode: bool) -> Self {
        Self {
            state: OtaState::Idle,
            source: None,
            metadata: None,
            progress: None,
            error: None,
            write_cursor: flash_map::OTA_STAGING_ADDR,
            production_mode,
            update_count: 0,
            boot_confirmed: false,
        }
    }

    pub fn state(&self) -> OtaState {
        self.state
    }

    pub fn error(&self) -> Option<OtaError> {
        self.error
    }

    pub fn progress(&self) -> Option<&DownloadProgress> {
        self.progress.as_ref()
    }

    pub fn is_idle(&self) -> bool {
        self.state == OtaState::Idle
    }

    pub fn update_count(&self) -> u16 {
        self.update_count
    }

    /// Begin an OTA update with the given metadata
    /// Validates the update is acceptable before proceeding
    pub fn begin_update(
        &mut self,
        metadata: UpdateMetadata,
        current_header: &FirmwareHeader,
        source: OtaSource,
        now_ms: u64,
    ) -> Result<(), OtaError> {
        if self.state != OtaState::Idle {
            return Err(OtaError::AlreadyInProgress);
        }

        // Validate the update metadata
        metadata.validate(current_header)?;

        // Initialize progress tracking
        let mut progress = DownloadProgress::new(metadata.image_size, metadata.total_chunks);
        progress.start_time_ms = now_ms;

        self.metadata = Some(metadata);
        self.progress = Some(progress);
        self.source = Some(source);
        self.write_cursor = flash_map::OTA_STAGING_ADDR;
        self.state = OtaState::Erasing;
        self.error = None;

        Ok(())
    }

    /// Called after Bank 2 erase is complete
    pub fn erase_complete(&mut self) {
        if self.state == OtaState::Erasing {
            self.state = OtaState::Downloading;
        }
    }

    /// Called when Bank 2 erase fails
    pub fn erase_failed(&mut self) {
        self.state = OtaState::Error;
        self.error = Some(OtaError::EraseFailed);
    }

    /// Process a received firmware chunk
    ///
    /// Chunks must arrive in order. Returns the flash address and data
    /// to write, or an error if the chunk is invalid.
    pub fn receive_chunk(
        &mut self,
        chunk_index: u16,
        data: &[u8],
        now_ms: u64,
    ) -> Result<(u32, &[u8]), OtaError> {
        if self.state != OtaState::Downloading {
            return Err(OtaError::AlreadyInProgress);
        }

        let progress = self.progress.as_mut().ok_or(OtaError::ChunkSequenceError)?;

        // Verify chunk sequence
        if chunk_index != progress.next_chunk {
            return Err(OtaError::ChunkSequenceError);
        }

        // Calculate write address
        let write_addr = self.write_cursor;

        // Update progress
        progress.update_crc(data);
        progress.received_bytes += data.len() as u32;
        progress.received_chunks += 1;
        progress.next_chunk += 1;
        progress.last_chunk_time_ms = now_ms;

        // Advance write cursor
        self.write_cursor += data.len() as u32;

        // Check if download is complete
        if progress.is_complete() {
            self.state = OtaState::Verifying;
        }

        Ok((write_addr, data))
    }

    /// Verify the downloaded image after all chunks received
    /// Returns Ok(()) if CRC matches
    pub fn verify_download(&mut self) -> Result<(), OtaError> {
        if self.state != OtaState::Verifying {
            return Err(OtaError::AlreadyInProgress);
        }

        let metadata = self.metadata.as_ref().ok_or(OtaError::CrcMismatch)?;
        let progress = self.progress.as_ref().ok_or(OtaError::CrcMismatch)?;

        // Verify CRC-32
        let computed_crc = progress.finalize_crc();
        if computed_crc != metadata.image_crc32 {
            self.state = OtaState::Error;
            self.error = Some(OtaError::CrcMismatch);
            return Err(OtaError::CrcMismatch);
        }

        // In production, also verify HMAC-SHA256 signature here
        // if metadata.is_signed { verify_signature(...) }

        self.state = OtaState::ReadyToApply;
        Ok(())
    }

    /// Apply the update (program option bytes for bank swap)
    ///
    /// After calling this, the system must be reset. On next boot,
    /// the new firmware in Bank 2 will execute as Bank 1.
    ///
    /// The caller must:
    /// 1. Program FLASH_OPTSR_PRG to swap bank mapping
    /// 2. Set OPTSTRT in FLASH_OPTCR
    /// 3. Wait for completion
    /// 4. Trigger system reset
    pub fn apply_update(&mut self) -> Result<BankSwapConfig, OtaError> {
        if self.state != OtaState::ReadyToApply {
            return Err(OtaError::BankSwapFailed);
        }

        let metadata = self.metadata.as_ref().ok_or(OtaError::BankSwapFailed)?;

        self.state = OtaState::PendingReboot;
        self.update_count += 1;

        Ok(BankSwapConfig {
            new_version: [metadata.version_major, metadata.version_minor, metadata.version_patch],
            swap_banks: true,
        })
    }

    /// Confirm the new firmware booted successfully
    /// Call this from the application after verifying all subsystems are OK.
    /// If not called within the watchdog timeout, bootloader rolls back.
    pub fn confirm_boot(&mut self) {
        self.boot_confirmed = true;
        self.state = OtaState::Idle;
        self.metadata = None;
        self.progress = None;
        self.source = None;
    }

    /// Abort an in-progress update
    pub fn abort(&mut self) {
        self.state = OtaState::Idle;
        self.metadata = None;
        self.progress = None;
        self.source = None;
        self.error = None;
        self.write_cursor = flash_map::OTA_STAGING_ADDR;
    }

    /// Check for download timeout (no chunks in 60 seconds)
    pub fn check_timeout(&mut self, now_ms: u64) -> bool {
        if self.state != OtaState::Downloading {
            return false;
        }
        if let Some(progress) = &self.progress {
            if progress.last_chunk_time_ms > 0
                && now_ms - progress.last_chunk_time_ms > 60_000
            {
                self.state = OtaState::Error;
                self.error = Some(OtaError::DownloadTimeout);
                return true;
            }
        }
        false
    }

    /// Get a diagnostic summary
    pub fn diagnostics(&self) -> OtaDiagnostics {
        OtaDiagnostics {
            state: self.state,
            error: self.error,
            progress_pct: self.progress.as_ref().map(|p| p.percent()).unwrap_or(0),
            received_bytes: self.progress.as_ref().map(|p| p.received_bytes).unwrap_or(0),
            total_bytes: self.progress.as_ref().map(|p| p.total_bytes).unwrap_or(0),
            source: self.source,
            update_count: self.update_count,
            target_version: self.metadata.as_ref().map(|m| m.version_string()),
        }
    }
}

/// Bank swap configuration for applying update
#[derive(Clone, Debug)]
pub struct BankSwapConfig {
    pub new_version: [u8; 3],
    pub swap_banks: bool,
}

/// OTA diagnostics for telemetry/debug reporting
#[derive(Clone, Debug)]
pub struct OtaDiagnostics {
    pub state: OtaState,
    pub error: Option<OtaError>,
    pub progress_pct: u8,
    pub received_bytes: u32,
    pub total_bytes: u32,
    pub source: Option<OtaSource>,
    pub update_count: u16,
    pub target_version: Option<String<12>>,
}

/// OTA protocol message types (for comm layer integration)
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum OtaMessageType {
    /// Server -> Device: firmware metadata
    UpdateAvailable = 0xF0,
    /// Device -> Server: accept/reject update
    UpdateResponse = 0xF1,
    /// Server -> Device: firmware chunk
    FirmwareChunk = 0xF2,
    /// Device -> Server: chunk acknowledgment
    ChunkAck = 0xF3,
    /// Device -> Server: update complete/failed
    UpdateStatus = 0xF4,
    /// Device -> Server: request missing chunk (retry)
    ChunkRetry = 0xF5,
}

/// Firmware chunk header (prepended to each chunk in transit)
#[derive(Clone, Debug)]
pub struct ChunkHeader {
    /// Chunk index (0-based)
    pub index: u16,
    /// Chunk data length
    pub length: u16,
    /// CRC-16 of chunk data (for transport integrity)
    pub chunk_crc16: u16,
}

impl ChunkHeader {
    pub const SIZE: usize = 6;

    pub fn serialize(&self) -> [u8; 6] {
        let mut buf = [0u8; 6];
        buf[0..2].copy_from_slice(&self.index.to_le_bytes());
        buf[2..4].copy_from_slice(&self.length.to_le_bytes());
        buf[4..6].copy_from_slice(&self.chunk_crc16.to_le_bytes());
        buf
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }
        Some(Self {
            index: u16::from_le_bytes([data[0], data[1]]),
            length: u16::from_le_bytes([data[2], data[3]]),
            chunk_crc16: u16::from_le_bytes([data[4], data[5]]),
        })
    }
}

/// SD card OTA: check for firmware file on SD card
/// Looks for /SCADA_FW/UPDATE.BIN
pub struct SdCardOta;

impl SdCardOta {
    /// Expected firmware file path on SD card
    pub const FIRMWARE_DIR: &'static str = "SCADA_FW";
    pub const FIRMWARE_FILE: &'static str = "UPDATE.BIN";
    pub const FIRMWARE_BACKUP: &'static str = "PREV.BIN";

    /// After successful update, rename UPDATE.BIN to PREV.BIN
    /// to prevent re-applying on next boot
    pub const APPLIED_MARKER: &'static str = "APPLIED.FLG";
}
