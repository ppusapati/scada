/// Sensor reading task — periodic ADC + I2C sensor acquisition
///
/// Reads all configured sensor channels at the configured interval,
/// applies calibration, and stores results in shared state for
/// other tasks (mqtt_publish, command_handler) to consume.

use embassy_time::{Duration, Ticker};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use defmt::{info, warn};

use crate::hal::adc::{read_oversampled, WaterAdcReadings, SolarAdcReadings};
use crate::hal::i2c_sensors::{bme280_read, ina219_read, Bme280Calibration};

/// Sensor reading configuration
pub struct SensorConfig {
    pub interval_ms: u64,
    pub oversample_count: u32,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            oversample_count: 16,
        }
    }
}

/// Shared sensor data updated by the read task
pub struct SensorData {
    pub water: Option<WaterAdcReadings>,
    pub solar: Option<SolarAdcReadings>,
    pub last_read_ms: u64,
    pub read_count: u32,
    pub fault: bool,
}

impl SensorData {
    pub const fn new() -> Self {
        Self {
            water: None,
            solar: None,
            last_read_ms: 0,
            read_count: 0,
            fault: false,
        }
    }
}

pub type SharedSensorData = Mutex<CriticalSectionRawMutex, SensorData>;
