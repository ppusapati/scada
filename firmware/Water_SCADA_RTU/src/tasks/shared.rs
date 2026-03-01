/// Shared state between Embassy tasks
///
/// Uses Embassy's synchronization primitives for safe inter-task communication.
/// The Mutex protects the register store; the Signal notifies tasks of new data.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use crate::modbus::registers::RegisterStore;
use crate::drivers::ads1258::{Channel, ChannelReading};

/// Shared state accessible by all tasks
pub struct SharedState {
    /// Modbus register store (protected by mutex)
    pub registers: Mutex<CriticalSectionRawMutex, RegisterStore>,
    /// Signal: new ADC readings available
    pub new_data: Signal<CriticalSectionRawMutex, ()>,
    /// Signal: calibration save requested
    pub cal_save_requested: Signal<CriticalSectionRawMutex, ()>,
    /// Latest readings cache (for diagnostics)
    pub latest_readings: Mutex<CriticalSectionRawMutex, [ChannelReading; Channel::COUNT]>,
    /// Device status flags
    pub device_status: Mutex<CriticalSectionRawMutex, u16>,
    /// Boot timestamp (tick count at startup)
    pub boot_tick: u64,
}

impl SharedState {
    pub const fn new() -> Self {
        Self {
            registers: Mutex::new(RegisterStore::new()),
            new_data: Signal::new(),
            cal_save_requested: Signal::new(),
            latest_readings: Mutex::new([ChannelReading {
                raw_code: 0,
                current_ma: 0.0,
                eng_value: 0.0,
                status: crate::drivers::ads1258::ChannelStatus::OpenCircuit,
                alarm: 0,
            }; Channel::COUNT]),
            device_status: Mutex::new(0),
            boot_tick: 0,
        }
    }
}
