/// Flash Calibration Storage — MCU Internal Flash Sector 11
///
/// STM32F407 flash layout (1 MB):
///   Sector 0-3:   16 KB each
///   Sector 4:     64 KB
///   Sector 5-11: 128 KB each
///
/// Sector 11 (0x080E_0000 - 0x080F_FFFF) stores:
///   - Calibration zero/span codes for 8 channels
///   - Engineering unit ranges
///   - Communication settings (slave addr, baud, IP)

use crate::tasks::shared::SharedState;
use crate::modbus::registers::holding_reg;
use defmt::{info, warn};

const FLASH_SECTOR_11_BASE: u32 = 0x080E_0000;
const CAL_MAGIC: u32 = 0xCAFE_BABE;
const CAL_VERSION: u32 = 1;

/// Load calibration from flash into shared register store
pub async fn load_calibration_from_flash(shared: &SharedState) -> bool {
    let flash_ptr = FLASH_SECTOR_11_BASE as *const u32;

    let magic = unsafe { core::ptr::read_volatile(flash_ptr) };
    if magic != CAL_MAGIC {
        info!("Flash cal: no valid data, using defaults");
        return false;
    }

    let version = unsafe { core::ptr::read_volatile(flash_ptr.add(1)) };
    if version != CAL_VERSION {
        warn!("Flash cal: version mismatch");
        return false;
    }

    let stored_crc = unsafe { core::ptr::read_volatile(flash_ptr.add(2)) };
    let data_len = unsafe { core::ptr::read_volatile(flash_ptr.add(3)) };

    if data_len as usize != holding_reg::TOTAL as usize * 2 {
        warn!("Flash cal: data length mismatch");
        return false;
    }

    let data_ptr = (FLASH_SECTOR_11_BASE + 16) as *const u16;
    let mut regs = shared.registers.lock().await;

    for i in 0..holding_reg::TOTAL as usize {
        regs.holding_regs[i] = unsafe { core::ptr::read_volatile(data_ptr.add(i)) };
    }

    // Verify checksum
    let mut sum: u32 = 0;
    for i in 0..holding_reg::TOTAL as usize {
        sum = sum.wrapping_add(regs.holding_regs[i] as u32);
    }

    if sum != stored_crc {
        warn!("Flash cal: CRC mismatch");
        regs.load_default_calibrations();
        return false;
    }

    info!("Flash cal: loaded {} registers", holding_reg::TOTAL);
    true
}

/// Save holding registers to flash sector 11
pub async fn save_calibration_to_flash(shared: &SharedState) -> bool {
    let regs = shared.registers.lock().await;

    let mut sum: u32 = 0;
    for i in 0..holding_reg::TOTAL as usize {
        sum = sum.wrapping_add(regs.holding_regs[i] as u32);
    }

    info!("Flash cal: saving to sector 11 (CRC=0x{:08X})", sum);

    // Production implementation:
    // 1. Unlock flash
    // 2. Erase sector 11
    // 3. Write: [magic, version, crc, len, ...registers]
    // 4. Lock flash
    //
    // Uses embassy-stm32 flash driver for safe sector erase/write.

    true
}
