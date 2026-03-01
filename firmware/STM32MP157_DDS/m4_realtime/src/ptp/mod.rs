/// IEEE 1588 PTP Timestamp Support for STM32MP157 M4
///
/// The STM32MP157 Ethernet MAC includes IEEE 1588v2 PTP hardware
/// timestamping support. The A7 Linux core runs ptp4l and phc2sys
/// to synchronize the PTP hardware clock with a grandmaster.
///
/// The M4 can read the PTP timestamp counter via memory-mapped
/// registers to attach precise timestamps to sensor readings.
///
/// This achieves sub-microsecond time synchronization across all
/// SCADA devices on the network without the M4 needing to run
/// the full PTP stack.
///
/// PTP Register Access:
///   The ETH MAC PTP registers are at base 0x5800_A700 (ETH MAC subsystem).
///   On STM32MP157, both A7 and M4 can read these registers.
///   IMPORTANT: Only the A7 should WRITE to PTP registers (clock correction).
///   The M4 only READS the current timestamp.
///
/// Synchronization Flow:
///   1. A7 runs ptp4l → synchronizes ETH MAC PTP clock to network grandmaster
///   2. A7 runs phc2sys → copies PTP time to system clock
///   3. M4 reads PTP timestamp registers directly for sensor timestamping
///   4. M4 optionally reads a shared register for PTP sync status

use core::sync::atomic::{AtomicBool, Ordering};
use defmt::{info, debug, warn};

/// ETH MAC base address (STM32MP157)
const ETH_MAC_BASE: u32 = 0x5800_A000;

/// PTP timestamp registers within ETH MAC
mod ptp_reg {
    /// MAC Timestamp Control Register
    pub const MACTSCR: u32 = 0x0B00;
    /// MAC Sub-second Increment Register
    pub const MACSSIR: u32 = 0x0B04;
    /// MAC System Time Seconds Register (read-only for M4)
    pub const MACSTSR: u32 = 0x0B08;
    /// MAC System Time Nanoseconds Register (read-only for M4)
    pub const MACSTNR: u32 = 0x0B0C;
    /// MAC System Time Seconds Update Register
    pub const MACSTSUR: u32 = 0x0B10;
    /// MAC System Time Nanoseconds Update Register
    pub const MACSTNUR: u32 = 0x0B14;
    /// MAC Timestamp Addend Register
    pub const MACTSAR: u32 = 0x0B18;
    /// MAC Timestamp Status Register
    pub const MACTSSR: u32 = 0x0B20;
}

/// Shared memory location for PTP sync status from A7
/// The A7 application writes the current sync state here
const PTP_SYNC_STATUS_ADDR: u32 = 0x1003_FF00; // Last 256 bytes of shared memory

/// PTP sync status structure (written by A7, read by M4)
#[repr(C)]
struct PtpSyncStatus {
    /// 0xPTP0 magic to validate struct
    magic: u32,
    /// PTP clock state: 0=free-running, 1=tracking, 2=locked
    state: u32,
    /// Offset from master in nanoseconds (signed)
    offset_ns: i64,
    /// Mean path delay in nanoseconds
    path_delay_ns: u64,
    /// Last update timestamp (PTP seconds)
    last_update_sec: u32,
}

const PTP_SYNC_MAGIC: u32 = 0x50545030; // "PTP0"

/// PTP timestamp (seconds + nanoseconds)
#[derive(Clone, Copy, Default)]
pub struct PtpTimestamp {
    pub seconds: u32,
    pub nanoseconds: u32,
}

impl PtpTimestamp {
    /// Check if this timestamp is valid (non-zero and nanoseconds < 1e9)
    pub fn is_valid(&self) -> bool {
        self.seconds > 0 && self.nanoseconds < 1_000_000_000
    }

    /// Convert to total nanoseconds (for short durations)
    pub fn to_nanos(&self) -> u64 {
        (self.seconds as u64) * 1_000_000_000 + self.nanoseconds as u64
    }
}

/// PTP clock synchronization state
#[derive(Clone, Copy, PartialEq, defmt::Format)]
pub enum PtpState {
    /// PTP hardware not available or not initialized
    Unavailable,
    /// Clock is free-running (no master)
    FreeRunning,
    /// Clock is tracking master but not yet locked
    Tracking,
    /// Clock is locked to master (sub-us accuracy)
    Locked,
}

/// PTP timestamp manager
pub struct PtpClock {
    /// Whether PTP hardware is accessible
    available: AtomicBool,
    /// Current sync state
    state: PtpState,
}

impl PtpClock {
    pub const fn new() -> Self {
        Self {
            available: AtomicBool::new(false),
            state: PtpState::Unavailable,
        }
    }

    /// Initialize PTP timestamp reading
    ///
    /// Checks if the ETH MAC PTP hardware is accessible from M4.
    /// The A7 must have already initialized the ETH MAC and PTP subsystem.
    pub fn init(&mut self) -> bool {
        // Check if ETH MAC is accessible by reading the timestamp control register
        let tscr = unsafe {
            core::ptr::read_volatile((ETH_MAC_BASE + ptp_reg::MACTSCR) as *const u32)
        };

        // Bit 0 (TSENA) should be set if PTP is enabled by A7
        if tscr & 0x01 != 0 {
            self.available.store(true, Ordering::Release);
            self.state = PtpState::FreeRunning;
            info!("PTP: Hardware timestamp available (TSCR=0x{:08X})", tscr);
            true
        } else {
            warn!("PTP: Hardware timestamp not enabled (A7 must initialize ETH MAC first)");
            self.state = PtpState::Unavailable;
            false
        }
    }

    /// Read the current PTP hardware timestamp
    ///
    /// Returns the current seconds and nanoseconds from the ETH MAC
    /// PTP timestamp counter. This counter is synchronized to the
    /// network grandmaster clock by ptp4l running on the A7.
    ///
    /// IMPORTANT: Read seconds first, then nanoseconds, then verify
    /// seconds didn't roll over. If it did, re-read both.
    pub fn read_timestamp(&self) -> PtpTimestamp {
        if !self.available.load(Ordering::Acquire) {
            return PtpTimestamp::default();
        }

        // Double-read to handle seconds rollover
        loop {
            let sec1 = unsafe {
                core::ptr::read_volatile((ETH_MAC_BASE + ptp_reg::MACSTSR) as *const u32)
            };
            let nsec = unsafe {
                core::ptr::read_volatile((ETH_MAC_BASE + ptp_reg::MACSTNR) as *const u32)
            };
            let sec2 = unsafe {
                core::ptr::read_volatile((ETH_MAC_BASE + ptp_reg::MACSTSR) as *const u32)
            };

            if sec1 == sec2 {
                return PtpTimestamp {
                    seconds: sec1,
                    nanoseconds: nsec & 0x7FFF_FFFF, // Bit 31 is sign in update mode
                };
            }
            // Seconds rolled over between reads — retry
        }
    }

    /// Check PTP synchronization status from the A7 shared memory
    ///
    /// The A7 application periodically writes PTP status to a known
    /// shared memory address. The M4 reads this to determine if
    /// timestamps are reliable.
    pub fn update_sync_status(&mut self) {
        let status_ptr = PTP_SYNC_STATUS_ADDR as *const PtpSyncStatus;

        let magic = unsafe { core::ptr::read_volatile(&(*status_ptr).magic) };
        if magic != PTP_SYNC_MAGIC {
            // A7 hasn't written status yet
            return;
        }

        let state = unsafe { core::ptr::read_volatile(&(*status_ptr).state) };
        self.state = match state {
            0 => PtpState::FreeRunning,
            1 => PtpState::Tracking,
            2 => PtpState::Locked,
            _ => PtpState::Unavailable,
        };
    }

    /// Get current PTP sync state
    pub fn sync_state(&self) -> PtpState {
        self.state
    }

    /// Check if PTP clock is synchronized (tracking or locked)
    pub fn is_synchronized(&self) -> bool {
        matches!(self.state, PtpState::Tracking | PtpState::Locked)
    }

    /// Get the PTP offset from master (nanoseconds, from A7 shared memory)
    pub fn offset_from_master_ns(&self) -> i64 {
        let status_ptr = PTP_SYNC_STATUS_ADDR as *const PtpSyncStatus;
        let magic = unsafe { core::ptr::read_volatile(&(*status_ptr).magic) };
        if magic != PTP_SYNC_MAGIC {
            return 0;
        }
        unsafe { core::ptr::read_volatile(&(*status_ptr).offset_ns) }
    }
}
