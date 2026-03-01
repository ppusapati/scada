/// Shared State Between STM32MP157 M4 Embassy Tasks
///
/// Uses Embassy's synchronization primitives for safe inter-task communication.
/// All mutable state is protected by CriticalSection mutexes.
/// Signals are used for event-driven task coordination.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;

use crate::modbus::RegisterStore;
use crate::sensors::calibration::CalibrationStore;
use crate::ptp::PtpClock;

/// Per-channel computed sensor data (shared between acquisition and publish tasks)
#[derive(Clone, Copy, Default)]
pub struct ChannelData {
    /// Calibrated engineering value
    pub value: f32,
    /// Raw ADC code
    pub raw_code: i32,
    /// Channel status: 0=OK, 1=warning, 2=fault, 3=disconnected
    pub status: u8,
    /// Alarm active on this channel
    pub alarm: bool,
}

/// Maximum number of sensor channels
pub const MAX_CHANNELS: usize = 16;

/// Task execution timing statistics
#[derive(Clone, Copy, Default)]
pub struct TaskStats {
    /// Last execution time in microseconds
    pub last_exec_us: u32,
    /// Maximum execution time in microseconds
    pub max_exec_us: u32,
    /// Total execution count
    pub exec_count: u32,
}

/// Shared state for all M4 tasks
pub struct SharedState {
    /// Modbus register store (holds all sensor data in register format)
    pub registers: Mutex<CriticalSectionRawMutex, RegisterStore>,

    /// Calibration store
    pub calibration: Mutex<CriticalSectionRawMutex, CalibrationStore>,

    /// PTP clock manager
    pub ptp_clock: Mutex<CriticalSectionRawMutex, PtpClock>,

    /// Signal: new sensor scan data available
    pub new_scan: Signal<CriticalSectionRawMutex, ()>,

    /// Signal: calibration save requested
    pub cal_save_requested: Signal<CriticalSectionRawMutex, ()>,

    /// Signal: configuration changed (from DDS command)
    pub config_changed: Signal<CriticalSectionRawMutex, ()>,

    /// Latest channel readings (up to 16 channels)
    pub channels: Mutex<CriticalSectionRawMutex, [ChannelData; MAX_CHANNELS]>,

    /// Number of active channels (16 for solar, 8 for water)
    pub num_channels: Mutex<CriticalSectionRawMutex, u8>,

    // ── Solar-specific shared data ──

    /// Bus voltage (Solar SMU)
    pub bus_voltage: Mutex<CriticalSectionRawMutex, f32>,

    /// Bus current (Solar SMU)
    pub bus_current: Mutex<CriticalSectionRawMutex, f32>,

    /// Total power (kW)
    pub total_power: Mutex<CriticalSectionRawMutex, f32>,

    /// Daily accumulated energy (kWh)
    pub daily_energy_kwh: Mutex<CriticalSectionRawMutex, f32>,

    /// Irradiance (W/m^2, Solar SMU)
    pub irradiance: Mutex<CriticalSectionRawMutex, f32>,

    // ── Common shared data ──

    /// Module/ambient temperature (from TMP117)
    pub temperature: Mutex<CriticalSectionRawMutex, f32>,

    /// Device status flags
    pub device_status: Mutex<CriticalSectionRawMutex, u16>,

    /// Task execution statistics [sensor, dds, modbus, diag]
    pub task_stats: Mutex<CriticalSectionRawMutex, [TaskStats; 4]>,

    /// Scan count (total number of ADC scans since boot)
    pub scan_count: Mutex<CriticalSectionRawMutex, u32>,
}

impl SharedState {
    pub const fn new() -> Self {
        Self {
            registers: Mutex::new(RegisterStore::new()),
            calibration: Mutex::new(CalibrationStore {
                #[cfg(feature = "solar_smu")]
                voltage_cals: [crate::sensors::calibration::default_solar_voltage_cal(); 16],
                #[cfg(feature = "solar_smu")]
                current_cals: [crate::sensors::calibration::default_solar_current_cal(); 16],
                #[cfg(feature = "water_rtu")]
                water_cals: [
                    crate::sensors::calibration::ChannelCal::linear(1_677_722, 8_388_608, 0.0, 10.0, 0x06),
                    crate::sensors::calibration::ChannelCal::linear(1_677_722, 8_388_608, 0.0, 14.0, 0x07),
                    crate::sensors::calibration::ChannelCal::linear(1_677_722, 8_388_608, 0.0, 1000.0, 0x08),
                    crate::sensors::calibration::ChannelCal::linear(1_677_722, 8_388_608, 0.0, 5.0, 0x09),
                    crate::sensors::calibration::ChannelCal::linear(1_677_722, 8_388_608, 0.0, 100.0, 0x0C),
                    crate::sensors::calibration::ChannelCal::linear(1_677_722, 8_388_608, 0.0, 100.0, 0x0B),
                    crate::sensors::calibration::ChannelCal::linear(1_677_722, 8_388_608, 0.0, 2000.0, 0x0A),
                    crate::sensors::calibration::ChannelCal::linear(1_677_722, 8_388_608, 0.0, 20.0, 0x09),
                ],
                valid: false,
            }),
            ptp_clock: Mutex::new(PtpClock::new()),
            new_scan: Signal::new(),
            cal_save_requested: Signal::new(),
            config_changed: Signal::new(),
            channels: Mutex::new([ChannelData {
                value: 0.0,
                raw_code: 0,
                status: 3, // disconnected at boot
                alarm: false,
            }; MAX_CHANNELS]),
            num_channels: Mutex::new(16),
            bus_voltage: Mutex::new(0.0),
            bus_current: Mutex::new(0.0),
            total_power: Mutex::new(0.0),
            daily_energy_kwh: Mutex::new(0.0),
            irradiance: Mutex::new(0.0),
            temperature: Mutex::new(0.0),
            device_status: Mutex::new(0),
            task_stats: Mutex::new([TaskStats {
                last_exec_us: 0,
                max_exec_us: 0,
                exec_count: 0,
            }; 4]),
            scan_count: Mutex::new(0),
        }
    }
}
