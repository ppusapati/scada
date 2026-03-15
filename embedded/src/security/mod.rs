//! Security Module
//!
//! Flash protection, secure boot, and anti-tamper features for
//! locking down the device after initial firmware provisioning.
//!
//! Protection strategy (applied after first firmware install):
//! 1. Flash RDP Level 1 - prevents SWD flash readout
//! 2. Write protection on bootloader sectors
//! 3. BOOT0 pin locked low via option bytes (disables DFU)
//! 4. Firmware signature verification on every boot
//! 5. OTA-only updates via authenticated channels

pub mod flash_protection;
pub mod secure_boot;
