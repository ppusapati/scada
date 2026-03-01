/// Flash Calibration Storage — MCU Internal Flash Sector 11
///
/// STM32F407 flash layout:
///   Sector 0-3:   16 KB each (0x0800_0000 - 0x0800_FFFF)
///   Sector 4:     64 KB      (0x0801_0000 - 0x0801_FFFF)
///   Sector 5-11: 128 KB each (0x0802_0000 - 0x080F_FFFF)
///
/// We use Sector 11 (last sector, 128 KB) for calibration storage.
/// This keeps calibration data far from firmware code.
///
/// Format:
///   Offset 0x00: Magic number (0xCAFE_BABE)
///   Offset 0x04: Version (u32)
///   Offset 0x08: CRC32 of data payload
///   Offset 0x0C: Data length (u32)
///   Offset 0x10: Holding register snapshot (200 bytes = 100 regs × 2)
///
/// Total: 16 + 200 = 216 bytes (well within 128 KB sector)

use crate::tasks::shared::SharedState;
use crate::modbus::registers::holding_reg;
use defmt::{info, warn};

const FLASH_SECTOR_11_BASE: u32 = 0x080E_0000;
const CAL_MAGIC: u32 = 0xCAFE_BABE;
const CAL_VERSION: u32 = 1;

/// Header for calibration data in flash
#[repr(C)]
struct CalHeader {
    magic: u32,
    version: u32,
    crc32: u32,
    data_len: u32,
}

/// Attempt to load calibration data from flash sector 11
/// Returns true if valid calibration was loaded
pub async fn load_calibration_from_flash(shared: &SharedState) -> bool {
    // Read flash memory directly (memory-mapped on STM32)
    let flash_ptr = FLASH_SECTOR_11_BASE as *const u32;

    // Safety: flash memory is always readable on STM32
    let magic = unsafe { core::ptr::read_volatile(flash_ptr) };

    if magic != CAL_MAGIC {
        info!("Flash cal: no valid data (magic=0x{:08X})", magic);
        return false;
    }

    let version = unsafe { core::ptr::read_volatile(flash_ptr.add(1)) };
    if version != CAL_VERSION {
        warn!("Flash cal: version mismatch (got {}, expected {})", version, CAL_VERSION);
        return false;
    }

    let stored_crc = unsafe { core::ptr::read_volatile(flash_ptr.add(2)) };
    let data_len = unsafe { core::ptr::read_volatile(flash_ptr.add(3)) };

    if data_len as usize != holding_reg::TOTAL as usize * 2 {
        warn!("Flash cal: data length mismatch");
        return false;
    }

    // Read holding register data
    let data_ptr = (FLASH_SECTOR_11_BASE + 16) as *const u16;
    let mut regs = shared.registers.lock().await;

    for i in 0..holding_reg::TOTAL as usize {
        let val = unsafe { core::ptr::read_volatile(data_ptr.add(i)) };
        regs.holding_regs[i] = val;
    }

    // Verify CRC (simple sum check for no_std)
    let mut sum: u32 = 0;
    for i in 0..holding_reg::TOTAL as usize {
        sum = sum.wrapping_add(regs.holding_regs[i] as u32);
    }

    if sum != stored_crc {
        warn!("Flash cal: CRC mismatch (calc=0x{:08X}, stored=0x{:08X})", sum, stored_crc);
        // Reload defaults
        regs.load_defaults();
        return false;
    }

    info!("Flash cal: loaded {} registers from flash", holding_reg::TOTAL);
    true
}

/// Flash register addresses for STM32F4 flash controller
const FLASH_KEYR: u32 = 0x4002_3C04;
const FLASH_SR: u32 = 0x4002_3C0C;
const FLASH_CR: u32 = 0x4002_3C10;
const FLASH_KEY1: u32 = 0x4567_0123;
const FLASH_KEY2: u32 = 0xCDEF_89AB;

/// Save current holding registers to flash sector 11
/// WARNING: This erases the entire 128KB sector. Call sparingly (on command only).
///
/// STM32F4 flash write procedure:
/// 1. Unlock flash (FLASH->KEYR = KEY1, KEY2)
/// 2. Erase sector 11 (FLASH->CR: SER=1, SNB=11, STRT=1)
/// 3. Wait for BSY=0
/// 4. Write header + data as 32-bit words
/// 5. Lock flash (FLASH->CR: LOCK=1)
pub async fn save_calibration_to_flash(shared: &SharedState) -> bool {
    let regs = shared.registers.lock().await;

    // Calculate checksum
    let mut sum: u32 = 0;
    for i in 0..holding_reg::TOTAL as usize {
        sum = sum.wrapping_add(regs.holding_regs[i] as u32);
    }

    let data_len = holding_reg::TOTAL as u32 * 2;

    info!(
        "Flash cal: saving {} bytes (CRC=0x{:08X}) to sector 11",
        data_len + 16,
        sum,
    );

    // Step 1: Unlock flash
    unsafe {
        core::ptr::write_volatile(FLASH_KEYR as *mut u32, FLASH_KEY1);
        core::ptr::write_volatile(FLASH_KEYR as *mut u32, FLASH_KEY2);
    }

    // Step 2: Erase sector 11
    // CR: PSIZE=10 (32-bit), SNB=1011 (sector 11), SER=1 (sector erase)
    unsafe {
        let cr_val: u32 = (0b10 << 8) | (11 << 3) | (1 << 1);
        core::ptr::write_volatile(FLASH_CR as *mut u32, cr_val);
        // Set STRT bit
        core::ptr::write_volatile(FLASH_CR as *mut u32, cr_val | (1 << 16));
    }

    // Wait for BSY to clear
    if !wait_flash_ready() {
        warn!("Flash cal: erase timeout");
        lock_flash();
        return false;
    }

    // Step 3: Configure for 32-bit programming
    // CR: PSIZE=10 (32-bit), PG=1 (programming)
    unsafe {
        core::ptr::write_volatile(FLASH_CR as *mut u32, (0b10 << 8) | (1 << 0));
    }

    // Step 4: Write header
    let header = [CAL_MAGIC, CAL_VERSION, sum, data_len];
    let base = FLASH_SECTOR_11_BASE as *mut u32;
    for (i, &word) in header.iter().enumerate() {
        unsafe { core::ptr::write_volatile(base.add(i), word); }
        if !wait_flash_ready() {
            warn!("Flash cal: header write timeout at word {}", i);
            lock_flash();
            return false;
        }
    }

    // Step 5: Write register data (pack two u16 into each u32 write)
    let data_base = (FLASH_SECTOR_11_BASE + 16) as *mut u32;
    let num_words = (holding_reg::TOTAL as usize + 1) / 2;
    for i in 0..num_words {
        let lo = regs.holding_regs[i * 2] as u32;
        let hi = if i * 2 + 1 < holding_reg::TOTAL as usize {
            (regs.holding_regs[i * 2 + 1] as u32) << 16
        } else {
            0
        };
        unsafe { core::ptr::write_volatile(data_base.add(i), lo | hi); }
        if !wait_flash_ready() {
            warn!("Flash cal: data write timeout at word {}", i);
            lock_flash();
            return false;
        }
    }

    // Step 6: Lock flash
    lock_flash();

    info!("Flash cal: saved successfully");
    true
}

/// Wait for flash BSY bit to clear (with timeout)
fn wait_flash_ready() -> bool {
    for _ in 0..100_000 {
        let sr = unsafe { core::ptr::read_volatile(FLASH_SR as *const u32) };
        if sr & (1 << 16) == 0 {
            // BSY bit clear
            return true;
        }
    }
    false
}

/// Lock the flash controller
fn lock_flash() {
    unsafe {
        let cr = core::ptr::read_volatile(FLASH_CR as *const u32);
        core::ptr::write_volatile(FLASH_CR as *mut u32, cr | (1 << 31)); // LOCK bit
    }
}
