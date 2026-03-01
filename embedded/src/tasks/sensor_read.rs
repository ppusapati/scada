/// Sensor reading task -- periodic ADC + I2C sensor acquisition
///
/// Reads all configured sensor channels at the configured interval,
/// applies calibration, and stores results in shared state for
/// other tasks (mqtt_publish, dds_publish, modbus_task) to consume.
///
/// When DDS feature is enabled, also publishes sensor data directly
/// to the XRCE-DDS agent with PTP timestamps.

use embassy_time::{Duration, Ticker, Instant};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use defmt::{info, warn};

use crate::hal::adc::{WaterAdcReadings, SolarAdcReadings};
#[cfg(not(feature = "simulator"))]
use crate::hal::adc::read_oversampled;
use crate::hal::i2c_sensors::{bme280_read, ina219_read, Bme280Calibration};

/// Sensor reading configuration
pub struct SensorConfig {
    pub interval_ms: u64,
    pub oversample_count: u32,
    /// Enable DDS publishing directly from the sensor task
    #[cfg(feature = "dds")]
    pub dds_enabled: bool,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            oversample_count: 16,
            #[cfg(feature = "dds")]
            dds_enabled: true,
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
    /// PTP timestamp of last reading (nanoseconds since epoch)
    pub ptp_timestamp_ns: u64,
    /// Environmental data from BME280
    pub ambient_temp_c: f32,
    pub humidity_pct: f32,
    pub pressure_hpa: f32,
    /// High-precision temperature from TMP117
    pub water_temp_c: f32,
    /// Power data from INA219
    pub bus_voltage_v: f32,
    pub panel_current_ma: f32,
    pub panel_power_mw: f32,
    /// Flow pulse data
    pub flow_rate_lpm: f32,
    pub total_volume_l: f32,
    /// Digital inputs
    pub tank_overflow: bool,
    pub pump_running: bool,
    pub valve_open: bool,
}

impl SensorData {
    pub const fn new() -> Self {
        Self {
            water: None,
            solar: None,
            last_read_ms: 0,
            read_count: 0,
            fault: false,
            ptp_timestamp_ns: 0,
            ambient_temp_c: 0.0,
            humidity_pct: 0.0,
            pressure_hpa: 0.0,
            water_temp_c: 0.0,
            bus_voltage_v: 0.0,
            panel_current_ma: 0.0,
            panel_power_mw: 0.0,
            flow_rate_lpm: 0.0,
            total_volume_l: 0.0,
            tank_overflow: false,
            pump_running: false,
            valve_open: false,
        }
    }

    /// Capture PTP timestamp for the current reading
    pub fn capture_timestamp(&mut self) {
        #[cfg(feature = "dds")]
        {
            self.ptp_timestamp_ns = crate::tasks::dds_publish::get_ptp_timestamp_ns();
        }
        #[cfg(not(feature = "dds"))]
        {
            self.ptp_timestamp_ns = Instant::now().as_micros() * 1000;
        }
    }

    /// Check if any sensor data is in a fault/alarm state
    pub fn has_fault(&self) -> bool {
        self.fault || self.tank_overflow
    }

    /// Get a quality indicator (0=good, 1=uncertain, 2=bad)
    pub fn quality(&self) -> u8 {
        if self.fault {
            2
        } else if self.tank_overflow {
            1
        } else {
            0
        }
    }
}

pub type SharedSensorData = Mutex<CriticalSectionRawMutex, SensorData>;

// ============================================================================
// Sensor read task DDS integration
// ============================================================================

/// After reading sensors, publish via DDS if enabled
///
/// This function is called by the sensor read task after updating
/// the shared sensor data. It publishes the data to the DDS-XRCE agent
/// using the appropriate topic and QoS.
#[cfg(feature = "dds")]
pub fn publish_sensor_data_dds(
    data: &SensorData,
    _node_type: NodeType,
) {
    // DDS publishing is handled by the dds_publish task which reads from shared state.
    // This function is a hook point for direct publishing if needed.
    // In the current architecture, the dds_publish task runs independently
    // and reads from SharedSensorData on its own schedule.
}

/// Node type for sensor-specific DDS publishing
#[derive(Clone, Copy, PartialEq)]
pub enum NodeType {
    Water,
    Solar,
}
