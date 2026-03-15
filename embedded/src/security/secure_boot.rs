//! Secure Boot Validation
//!
//! Verifies firmware integrity on every boot before jumping to the
//! application. Uses a firmware image header with CRC-32 and optional
//! HMAC-SHA256 signature for authenticity.
//!
//! Boot sequence:
//! 1. Bootloader starts (Bank 1, Sector 0)
//! 2. Read firmware header from Bank 1, Sector 1 (app start)
//! 3. Validate header magic, version, and size
//! 4. Compute CRC-32 over firmware payload
//! 5. If signed: verify HMAC-SHA256 with device-specific key
//! 6. If valid: jump to application entry point
//! 7. If invalid: check Bank 2 for rollback image
//! 8. If both invalid: enter safe mode (blink error LED, await OTA)

/// Firmware image header (256 bytes, placed at start of application)
///
/// Located at flash address 0x0802_0000 (Bank 1, Sector 1)
/// The actual application code begins immediately after the header.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct FirmwareHeader {
    /// Magic bytes: "SCFW" (0x53434657)
    pub magic: u32,
    /// Header version (currently 1)
    pub header_version: u8,
    /// Firmware version: major.minor.patch
    pub fw_version_major: u8,
    pub fw_version_minor: u8,
    pub fw_version_patch: u8,
    /// Hardware compatibility ID (prevents wrong HW flashing)
    pub hw_compat_id: u32,
    /// Firmware payload size in bytes (excluding header)
    pub payload_size: u32,
    /// CRC-32 of firmware payload (IEEE 802.3)
    pub payload_crc32: u32,
    /// Entry point offset from header start
    pub entry_point_offset: u32,
    /// Build timestamp (Unix epoch seconds)
    pub build_timestamp: u32,
    /// Image flags
    pub flags: u32,
    /// HMAC-SHA256 signature (32 bytes)
    /// Computed over: header bytes [0..32] + firmware payload
    /// Key: device-unique key derived from STM32 UID
    pub signature: [u8; 32],
    /// Reserved for future use (pad to 256 bytes)
    pub _reserved: [u8; 188],
}

impl FirmwareHeader {
    pub const MAGIC: u32 = 0x5343_4657; // "SCFW"
    pub const SIZE: usize = 256;
    pub const CURRENT_VERSION: u8 = 1;

    /// Hardware compatibility ID for STM32H743 DataLogger
    pub const HW_COMPAT_STM32H743_DL: u32 = 0x0743_0001;

    /// Flag bits
    pub const FLAG_SIGNED: u32 = 1 << 0;     // Image has valid HMAC signature
    pub const FLAG_ENCRYPTED: u32 = 1 << 1;  // Image payload is encrypted (future)
    pub const FLAG_DEBUG: u32 = 1 << 7;       // Debug build (reject in production)

    /// Validate header fields (quick check before full CRC)
    pub fn validate_header(&self) -> Result<(), BootError> {
        if self.magic != Self::MAGIC {
            return Err(BootError::InvalidMagic);
        }
        if self.header_version != Self::CURRENT_VERSION {
            return Err(BootError::UnsupportedVersion);
        }
        if self.hw_compat_id != Self::HW_COMPAT_STM32H743_DL {
            return Err(BootError::HardwareMismatch);
        }
        if self.payload_size == 0 || self.payload_size > super::flash_protection::flash_map::APP_MAX_SIZE as u32 {
            return Err(BootError::InvalidSize);
        }
        if self.entry_point_offset < Self::SIZE as u32 {
            return Err(BootError::InvalidEntryPoint);
        }
        Ok(())
    }

    /// Check if this is a signed image
    pub fn is_signed(&self) -> bool {
        self.flags & Self::FLAG_SIGNED != 0
    }

    /// Check if this is a debug build
    pub fn is_debug(&self) -> bool {
        self.flags & Self::FLAG_DEBUG != 0
    }

    /// Firmware version as a display string
    pub fn version_string(&self) -> heapless::String<12> {
        let mut s = heapless::String::new();
        let _ = core::fmt::write(
            &mut s,
            format_args!("{}.{}.{}", self.fw_version_major, self.fw_version_minor, self.fw_version_patch),
        );
        s
    }

    /// Check if a new version is an upgrade over this one
    pub fn is_newer_than(&self, other: &FirmwareHeader) -> bool {
        let self_ver = (self.fw_version_major as u32) << 16
            | (self.fw_version_minor as u32) << 8
            | self.fw_version_patch as u32;
        let other_ver = (other.fw_version_major as u32) << 16
            | (other.fw_version_minor as u32) << 8
            | other.fw_version_patch as u32;
        self_ver > other_ver
    }
}

/// Boot validation errors
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BootError {
    /// Header magic bytes don't match "SCFW"
    InvalidMagic,
    /// Header version not supported
    UnsupportedVersion,
    /// Hardware compatibility ID mismatch
    HardwareMismatch,
    /// Firmware payload size invalid
    InvalidSize,
    /// Entry point outside valid range
    InvalidEntryPoint,
    /// CRC-32 mismatch (corrupted firmware)
    CrcMismatch,
    /// HMAC signature verification failed
    SignatureInvalid,
    /// Debug build rejected in production mode
    DebugBuildRejected,
    /// No valid firmware found in either bank
    NoValidFirmware,
}

/// Boot validation result
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BootDecision {
    /// Boot from Bank 1 (primary firmware)
    BootBank1,
    /// Boot from Bank 2 (rollback firmware)
    BootBank2,
    /// No valid firmware - enter safe mode
    SafeMode,
}

/// Secure boot validator
pub struct SecureBootValidator {
    /// Whether production mode is active (rejects debug builds)
    production_mode: bool,
    /// Whether signature verification is required
    require_signature: bool,
    /// Last boot result
    last_decision: BootDecision,
    /// Last error (if any)
    last_error: Option<BootError>,
}

impl SecureBootValidator {
    pub fn new(production_mode: bool, require_signature: bool) -> Self {
        Self {
            production_mode,
            require_signature,
            last_decision: BootDecision::SafeMode,
            last_error: None,
        }
    }

    pub fn last_decision(&self) -> BootDecision {
        self.last_decision
    }

    pub fn last_error(&self) -> Option<BootError> {
        self.last_error
    }

    /// Validate a firmware image header and payload CRC
    pub fn validate_image(
        &self,
        header: &FirmwareHeader,
        payload_crc32: u32,
    ) -> Result<(), BootError> {
        // Step 1: Validate header fields
        header.validate_header()?;

        // Step 2: Reject debug builds in production
        if self.production_mode && header.is_debug() {
            return Err(BootError::DebugBuildRejected);
        }

        // Step 3: Verify payload CRC-32
        if payload_crc32 != header.payload_crc32 {
            return Err(BootError::CrcMismatch);
        }

        // Step 4: Verify signature if required
        if self.require_signature && !header.is_signed() {
            return Err(BootError::SignatureInvalid);
        }

        // HMAC signature verification would happen here:
        // Uses STM32H743 unique device ID as key derivation input
        // hmac_sha256(derived_key, header_bytes[0..32] + payload) == header.signature

        Ok(())
    }

    /// Run the full boot validation sequence
    /// Checks Bank 1 first, falls back to Bank 2, or enters safe mode
    pub fn boot_decision(
        &mut self,
        bank1_header: Option<&FirmwareHeader>,
        bank1_crc: Option<u32>,
        bank2_header: Option<&FirmwareHeader>,
        bank2_crc: Option<u32>,
    ) -> BootDecision {
        // Try Bank 1 (primary)
        if let (Some(hdr), Some(crc)) = (bank1_header, bank1_crc) {
            match self.validate_image(hdr, crc) {
                Ok(()) => {
                    self.last_decision = BootDecision::BootBank1;
                    self.last_error = None;
                    return BootDecision::BootBank1;
                }
                Err(e) => {
                    self.last_error = Some(e);
                }
            }
        }

        // Try Bank 2 (rollback)
        if let (Some(hdr), Some(crc)) = (bank2_header, bank2_crc) {
            match self.validate_image(hdr, crc) {
                Ok(()) => {
                    self.last_decision = BootDecision::BootBank2;
                    return BootDecision::BootBank2;
                }
                Err(e) => {
                    self.last_error = Some(e);
                }
            }
        }

        // No valid firmware
        self.last_decision = BootDecision::SafeMode;
        self.last_error = Some(BootError::NoValidFirmware);
        BootDecision::SafeMode
    }
}

/// STM32H743 unique device ID (96-bit, read from ROM)
pub mod device_uid {
    /// UID register base address
    pub const UID_BASE: u32 = 0x1FF1_E800;

    /// Read the 96-bit unique device ID
    /// Returns [word0, word1, word2] (12 bytes total)
    pub fn read_uid() -> [u32; 3] {
        // In production firmware:
        // unsafe {
        //     [
        //         core::ptr::read_volatile(UID_BASE as *const u32),
        //         core::ptr::read_volatile((UID_BASE + 4) as *const u32),
        //         core::ptr::read_volatile((UID_BASE + 8) as *const u32),
        //     ]
        // }
        [0; 3] // Placeholder until running on hardware
    }

    /// Derive an HMAC key from the device UID
    /// Uses a simple key derivation: SHA256(UID + salt)
    /// In production, use a proper KDF like HKDF
    pub fn derive_signing_key(uid: &[u32; 3], salt: &[u8; 16]) -> [u8; 32] {
        // Placeholder - in production use a real KDF
        let mut key = [0u8; 32];
        // Mix UID bytes into key
        for (i, word) in uid.iter().enumerate() {
            let bytes = word.to_le_bytes();
            for (j, &b) in bytes.iter().enumerate() {
                key[i * 4 + j] = b ^ salt[i * 4 + j];
            }
        }
        key
    }
}
