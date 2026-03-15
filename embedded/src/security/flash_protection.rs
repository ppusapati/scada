//! STM32H743 Flash Protection & Debug Lockdown
//!
//! After first firmware provisioning, this module configures the option bytes
//! to prevent unauthorized access via SWD, JTAG, UART bootloader, and DFU:
//!
//! Protection levels (applied incrementally):
//!
//! 1. **RDP Level 1** (Reversible)
//!    - SWD/JTAG cannot read flash or SRAM contents
//!    - Debug connection still possible but flash is inaccessible
//!    - Reverting to Level 0 triggers full flash mass erase
//!    - OTA updates still work (firmware writes to own flash)
//!
//! 2. **RDP Level 2** (PERMANENT - use with extreme caution)
//!    - SWD/JTAG completely disabled at hardware level
//!    - BOOT0 pin ignored (system bootloader inaccessible)
//!    - IRREVERSIBLE - chip can NEVER be debugged again
//!    - Only use for final production units, never during development
//!
//! 3. **Write Protection (WRP)**
//!    - Protects bootloader and security-critical flash sectors
//!    - Prevents accidental or malicious overwrite of boot code
//!
//! 4. **BOOT0 Lock**
//!    - Forces boot from internal flash (option byte nSWBOOT0)
//!    - Disables UART/USB DFU bootloader entry via BOOT0 pin
//!    - Device can only be updated via application OTA path
//!
//! STM32H743 Flash Layout (2MB dual-bank):
//!   Bank 1 (0x0800_0000 - 0x080F_FFFF): 8 sectors x 128KB
//!   Bank 2 (0x0810_0000 - 0x081F_FFFF): 8 sectors x 128KB

use heapless::String;

/// STM32H743 flash memory constants
pub mod flash_map {
    /// Flash bank 1 start address
    pub const BANK1_BASE: u32 = 0x0800_0000;
    /// Flash bank 2 start address
    pub const BANK2_BASE: u32 = 0x0810_0000;
    /// Sector size (128KB for STM32H743)
    pub const SECTOR_SIZE: u32 = 128 * 1024;
    /// Number of sectors per bank
    pub const SECTORS_PER_BANK: u8 = 8;
    /// Total flash size
    pub const TOTAL_SIZE: u32 = 2 * 1024 * 1024;

    /// Flash layout for dual-bank OTA:
    ///   Bank 1, Sector 0:   Bootloader (128KB, write-protected)
    ///   Bank 1, Sectors 1-7: Active firmware (896KB)
    ///   Bank 2, Sectors 0-7: OTA staging area (1MB)
    pub const BOOTLOADER_SECTOR: u8 = 0;
    pub const BOOTLOADER_ADDR: u32 = BANK1_BASE;
    pub const BOOTLOADER_SIZE: u32 = SECTOR_SIZE;

    pub const APP_START_SECTOR: u8 = 1;
    pub const APP_START_ADDR: u32 = BANK1_BASE + SECTOR_SIZE;
    pub const APP_MAX_SIZE: u32 = 7 * SECTOR_SIZE; // 896KB

    pub const OTA_STAGING_ADDR: u32 = BANK2_BASE;
    pub const OTA_STAGING_SIZE: u32 = 8 * SECTOR_SIZE; // 1MB
}

/// Readout protection levels
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RdpLevel {
    /// Level 0: No protection (factory default)
    /// SWD full access, flash readable, option bytes modifiable
    Level0,
    /// Level 1: Read protection (RECOMMENDED for deployed devices)
    /// SWD cannot read flash/SRAM, debug still connectable
    /// Reverting to Level 0 triggers full mass erase
    Level1,
    /// Level 2: PERMANENT chip lockdown (IRREVERSIBLE!)
    /// SWD/JTAG completely disabled at hardware level
    /// BOOT0 pin ignored, system bootloader inaccessible
    /// WARNING: Cannot be undone - chip is permanently locked
    Level2,
}

impl RdpLevel {
    /// Option byte value for each RDP level (STM32H743)
    pub fn option_byte_value(&self) -> u8 {
        match self {
            Self::Level0 => 0xAA,
            Self::Level1 => 0xBB, // Any value except 0xAA and 0xCC
            Self::Level2 => 0xCC,
        }
    }

    /// Parse RDP level from option byte value
    pub fn from_option_byte(val: u8) -> Self {
        match val {
            0xAA => Self::Level0,
            0xCC => Self::Level2,
            _ => Self::Level1,
        }
    }
}

/// BOOT0 source configuration
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Boot0Config {
    /// BOOT0 value comes from physical pin (default)
    /// Allows entering DFU bootloader via J13 jumper
    FromPin,
    /// BOOT0 value forced from option byte nBOOT0 bit
    /// Pin is ignored - cannot enter DFU via hardware
    FromOptionByte,
}

/// Write protection configuration for flash sectors
#[derive(Clone, Copy, Debug)]
pub struct WriteProtection {
    /// Bitmask of protected sectors in Bank 1 (bit N = sector N)
    pub bank1_sectors: u8,
    /// Bitmask of protected sectors in Bank 2
    pub bank2_sectors: u8,
}

impl WriteProtection {
    /// Protect only the bootloader sector (Bank 1, Sector 0)
    pub fn bootloader_only() -> Self {
        Self {
            bank1_sectors: 0x01, // Sector 0 only
            bank2_sectors: 0x00,
        }
    }

    /// Protect bootloader + active firmware (Bank 1, all sectors)
    pub fn full_bank1() -> Self {
        Self {
            bank1_sectors: 0xFF, // All 8 sectors
            bank2_sectors: 0x00, // Bank 2 writable for OTA staging
        }
    }
}

/// Current protection status read from option bytes
#[derive(Clone, Debug)]
pub struct ProtectionStatus {
    pub rdp_level: RdpLevel,
    pub boot0_config: Boot0Config,
    pub boot0_value: bool,
    pub write_protection: WriteProtection,
    pub swd_accessible: bool,
    pub dfu_accessible: bool,
}

impl ProtectionStatus {
    /// Check if device is in factory-default (unprotected) state
    pub fn is_unprotected(&self) -> bool {
        self.rdp_level == RdpLevel::Level0
            && self.boot0_config == Boot0Config::FromPin
            && self.swd_accessible
    }

    /// Check if device is fully locked for production
    pub fn is_production_locked(&self) -> bool {
        self.rdp_level == RdpLevel::Level1
            && self.boot0_config == Boot0Config::FromOptionByte
            && !self.dfu_accessible
    }

    /// Human-readable status for debug console
    pub fn summary(&self) -> String<128> {
        let mut s = String::new();
        let rdp = match self.rdp_level {
            RdpLevel::Level0 => "L0(OPEN)",
            RdpLevel::Level1 => "L1(READ-PROT)",
            RdpLevel::Level2 => "L2(PERMANENT)",
        };
        let boot = match self.boot0_config {
            Boot0Config::FromPin => "PIN(J13)",
            Boot0Config::FromOptionByte => "LOCKED",
        };
        let _ = core::fmt::write(
            &mut s,
            format_args!("RDP={} BOOT0={} SWD={} DFU={}", rdp, boot,
                if self.swd_accessible { "YES" } else { "NO" },
                if self.dfu_accessible { "YES" } else { "NO" }),
        );
        s
    }
}

/// STM32H743 option byte register addresses
pub mod option_bytes {
    /// FLASH_OPTSR_CUR - current option status register
    pub const FLASH_OPTSR_CUR: u32 = 0x5200_201C;
    /// FLASH_OPTSR_PRG - option programming register
    pub const FLASH_OPTSR_PRG: u32 = 0x5200_2020;
    /// FLASH_OPTCR - option control register
    pub const FLASH_OPTCR: u32 = 0x5200_2018;
    /// FLASH_WPSN_CUR - write protection current (bank 1)
    pub const FLASH_WPSN_CUR1: u32 = 0x5200_2038;
    /// FLASH_WPSN_PRG - write protection programming (bank 1)
    pub const FLASH_WPSN_PRG1: u32 = 0x5200_203C;

    /// Bit positions in OPTSR
    pub const OPTSR_RDP_SHIFT: u32 = 8;
    pub const OPTSR_RDP_MASK: u32 = 0xFF << 8;
    pub const OPTSR_NSWBOOT0: u32 = 1 << 26; // 0=BOOT0 from option byte, 1=BOOT0 from pin
    pub const OPTSR_NBOOT0: u32 = 1 << 27;   // BOOT0 value when nSWBOOT0=0

    /// OPTCR bits
    pub const OPTCR_OPTLOCK: u32 = 1 << 0;
    pub const OPTCR_OPTSTRT: u32 = 1 << 1;
}

/// Flash protection manager
///
/// Handles reading current protection status and applying new protection
/// settings via STM32H743 option bytes.
pub struct FlashProtection {
    status: ProtectionStatus,
    provisioned: bool,
}

impl FlashProtection {
    pub fn new() -> Self {
        Self {
            status: ProtectionStatus {
                rdp_level: RdpLevel::Level0,
                boot0_config: Boot0Config::FromPin,
                boot0_value: false,
                write_protection: WriteProtection::bootloader_only(),
                swd_accessible: true,
                dfu_accessible: true,
            },
            provisioned: false,
        }
    }

    pub fn status(&self) -> &ProtectionStatus {
        &self.status
    }

    pub fn is_provisioned(&self) -> bool {
        self.provisioned
    }

    /// Read current option byte values from hardware registers
    /// Call this at boot to determine current protection state
    pub fn read_current_status(&mut self) {
        // In production firmware, this reads the actual FLASH_OPTSR_CUR register:
        //   let optsr = unsafe { core::ptr::read_volatile(FLASH_OPTSR_CUR as *const u32) };
        //   let rdp_byte = ((optsr & OPTSR_RDP_MASK) >> OPTSR_RDP_SHIFT) as u8;
        //   self.status.rdp_level = RdpLevel::from_option_byte(rdp_byte);
        //   self.status.boot0_config = if optsr & OPTSR_NSWBOOT0 != 0 {
        //       Boot0Config::FromPin
        //   } else {
        //       Boot0Config::FromOptionByte
        //   };
        //   self.status.swd_accessible = self.status.rdp_level != RdpLevel::Level2;
        //   self.status.dfu_accessible = self.status.boot0_config == Boot0Config::FromPin;
    }

    /// Apply production lockdown (recommended after first OTA-capable firmware install)
    ///
    /// This performs the following steps:
    /// 1. Sets RDP Level 1 (SWD read protection)
    /// 2. Locks BOOT0 to 0 via option byte (disables DFU entry)
    /// 3. Write-protects the bootloader sector
    ///
    /// After this, the device can ONLY be updated via OTA.
    /// SWD can still connect but cannot read flash contents.
    /// Reverting RDP to Level 0 would mass-erase the chip.
    ///
    /// Returns the option byte values to program.
    pub fn production_lockdown_config(&self) -> ProductionLockdownConfig {
        ProductionLockdownConfig {
            rdp_level: RdpLevel::Level1,
            boot0_config: Boot0Config::FromOptionByte,
            boot0_value: false, // Force boot from flash
            wrp: WriteProtection::bootloader_only(),
        }
    }

    /// Generate the OPTSR register value for production lockdown
    pub fn build_optsr_value(config: &ProductionLockdownConfig) -> u32 {
        let mut optsr: u32 = 0;

        // RDP level
        optsr |= (config.rdp_level.option_byte_value() as u32) << option_bytes::OPTSR_RDP_SHIFT;

        // BOOT0 source: clear nSWBOOT0 bit to use option byte value
        if config.boot0_config == Boot0Config::FromOptionByte {
            // nSWBOOT0 = 0 means BOOT0 comes from option byte
            // (bit is already 0, nothing to set)
            if !config.boot0_value {
                // nBOOT0 = 0 means BOOT0 = 0 (boot from flash)
                // (bit is already 0)
            } else {
                optsr |= option_bytes::OPTSR_NBOOT0;
            }
        } else {
            // nSWBOOT0 = 1 means BOOT0 comes from physical pin
            optsr |= option_bytes::OPTSR_NSWBOOT0;
        }

        optsr
    }

    /// Sequence to program option bytes (must be called from privileged mode)
    ///
    /// WARNING: Incorrect option byte programming can brick the device.
    /// The sequence is:
    /// 1. Unlock flash (write KEY1, KEY2 to FLASH_KEYR)
    /// 2. Unlock option bytes (write OPTKEY1, OPTKEY2 to FLASH_OPTKEYR)
    /// 3. Write new values to FLASH_OPTSR_PRG
    /// 4. Set OPTSTRT bit in FLASH_OPTCR
    /// 5. Wait for BSY flag to clear
    /// 6. System reset to apply new option bytes
    ///
    /// Returns the step-by-step register write sequence.
    pub fn option_byte_program_sequence() -> OptionByteSequence {
        OptionByteSequence {
            // Flash unlock keys
            flash_key1: 0x4567_0123,
            flash_key2: 0xCDEF_89AB,
            // Option byte unlock keys
            opt_key1: 0x0819_2A3B,
            opt_key2: 0x4C5D_6E7F,
        }
    }

    /// Mark device as provisioned (protection applied successfully)
    pub fn mark_provisioned(&mut self) {
        self.provisioned = true;
        self.status.rdp_level = RdpLevel::Level1;
        self.status.boot0_config = Boot0Config::FromOptionByte;
        self.status.swd_accessible = true; // L1 allows connection but not read
        self.status.dfu_accessible = false;
    }

    /// Check if it's safe to apply lockdown
    /// Returns an error message if conditions are not met
    pub fn validate_lockdown_preconditions(&self) -> Result<(), &'static str> {
        if self.status.rdp_level == RdpLevel::Level2 {
            return Err("Already at RDP Level 2 (permanent)");
        }
        // Ensure OTA subsystem is functional before locking out other update paths
        // This check should be done by the caller with actual OTA status
        Ok(())
    }
}

/// Configuration for production lockdown
#[derive(Clone, Debug)]
pub struct ProductionLockdownConfig {
    pub rdp_level: RdpLevel,
    pub boot0_config: Boot0Config,
    pub boot0_value: bool,
    pub wrp: WriteProtection,
}

/// Flash unlock key sequence
#[derive(Clone, Debug)]
pub struct OptionByteSequence {
    pub flash_key1: u32,
    pub flash_key2: u32,
    pub opt_key1: u32,
    pub opt_key2: u32,
}

/// Provisioning state machine
/// Tracks the device through the first-boot lockdown process
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProvisioningState {
    /// Factory fresh, no protection applied
    Unprovisioned,
    /// Firmware installed, waiting for lockdown command
    FirmwareInstalled,
    /// OTA subsystem verified working
    OtaVerified,
    /// Lockdown applied, pending reboot
    LockdownPending,
    /// Device is locked and production-ready
    Locked,
}

/// Provisioning manager
/// Guides the first-boot process from factory to locked state
pub struct ProvisioningManager {
    state: ProvisioningState,
    protection: FlashProtection,
}

impl ProvisioningManager {
    pub fn new() -> Self {
        Self {
            state: ProvisioningState::Unprovisioned,
            protection: FlashProtection::new(),
        }
    }

    pub fn state(&self) -> ProvisioningState {
        self.state
    }

    pub fn protection(&self) -> &FlashProtection {
        &self.protection
    }

    /// Initialize: read current protection and determine provisioning state
    pub fn init(&mut self) {
        self.protection.read_current_status();
        if self.protection.status().is_production_locked() {
            self.state = ProvisioningState::Locked;
        } else if self.protection.is_provisioned() {
            self.state = ProvisioningState::LockdownPending;
        } else {
            self.state = ProvisioningState::Unprovisioned;
        }
    }

    /// Mark firmware as installed (call after first successful boot)
    pub fn confirm_firmware_installed(&mut self) {
        if self.state == ProvisioningState::Unprovisioned {
            self.state = ProvisioningState::FirmwareInstalled;
        }
    }

    /// Mark OTA subsystem as verified (call after first successful OTA test)
    pub fn confirm_ota_working(&mut self) {
        if self.state == ProvisioningState::FirmwareInstalled {
            self.state = ProvisioningState::OtaVerified;
        }
    }

    /// Apply production lockdown
    /// Only proceeds if OTA has been verified working
    pub fn apply_lockdown(&mut self) -> Result<ProductionLockdownConfig, &'static str> {
        if self.state != ProvisioningState::OtaVerified {
            return Err("OTA must be verified before lockdown");
        }
        self.protection.validate_lockdown_preconditions()?;
        let config = self.protection.production_lockdown_config();
        self.state = ProvisioningState::LockdownPending;
        Ok(config)
    }

    /// Confirm lockdown was applied (call after successful option byte program + reboot)
    pub fn confirm_lockdown(&mut self) {
        self.protection.mark_provisioned();
        self.state = ProvisioningState::Locked;
    }
}
