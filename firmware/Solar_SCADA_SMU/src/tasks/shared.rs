/// Shared state between Solar SMU Embassy tasks

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use crate::modbus::registers::RegisterStore;
use crate::drivers::ads1263::SolarReading;
use crate::drivers::cd74hc4067::TOTAL_CHANNELS;

/// Per-string computed data
#[derive(Clone, Copy, Default)]
pub struct StringData {
    pub voltage: f32,
    pub current: f32,
    pub power: f32,
    pub status: u8, // 0=OK, 1=low voltage, 2=overcurrent, 3=disconnected
}

/// Shared state for all tasks
pub struct SharedState {
    pub registers: Mutex<CriticalSectionRawMutex, RegisterStore>,
    pub new_scan: Signal<CriticalSectionRawMutex, ()>,
    pub strings: Mutex<CriticalSectionRawMutex, [StringData; 16]>,
    pub bus_voltage: Mutex<CriticalSectionRawMutex, f32>,
    pub bus_current: Mutex<CriticalSectionRawMutex, f32>,
    pub irradiance: Mutex<CriticalSectionRawMutex, f32>,
    pub module_temp: Mutex<CriticalSectionRawMutex, f32>,
    pub ambient_temp: Mutex<CriticalSectionRawMutex, f32>,
    pub device_status: Mutex<CriticalSectionRawMutex, u16>,
    pub daily_energy_wh: Mutex<CriticalSectionRawMutex, f32>,
}

impl SharedState {
    pub const fn new() -> Self {
        Self {
            registers: Mutex::new(RegisterStore::new()),
            new_scan: Signal::new(),
            strings: Mutex::new([StringData {
                voltage: 0.0,
                current: 0.0,
                power: 0.0,
                status: 3, // disconnected at boot
            }; 16]),
            bus_voltage: Mutex::new(0.0),
            bus_current: Mutex::new(0.0),
            irradiance: Mutex::new(0.0),
            module_temp: Mutex::new(0.0),
            ambient_temp: Mutex::new(0.0),
            device_status: Mutex::new(0),
            daily_energy_wh: Mutex::new(0.0),
        }
    }
}
