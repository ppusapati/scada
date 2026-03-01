/// Water treatment plant sensor/actuator driver
///
/// Combines ADC readings (pressure, pH, turbidity, chlorine, tank level),
/// I2C temperature sensor, GPIO relays (pump, valve), and digital inputs
/// (float switch) into a single water subsystem interface.

use heapless::String;
use serde::Serialize;

use crate::hal::adc::{
    WaterAdcReadings, CHLORINE_CAL, PH_CAL, PRESSURE_CAL, TANK_LEVEL_CAL, TURBIDITY_CAL,
};
use crate::hal::i2c_sensors::EnvReading;

/// Complete water subsystem telemetry snapshot
#[derive(Serialize)]
pub struct WaterTelemetry {
    pub pressure_bar: f32,
    pub ph: f32,
    pub turbidity_ntu: f32,
    pub chlorine_mgl: f32,
    pub tank_level_pct: f32,
    pub water_temp_c: f32,
    pub ambient_temp_c: f32,
    pub humidity_pct: f32,
    pub pump_on: bool,
    pub valve_on: bool,
    pub tank_overflow: bool,
    pub sensor_fault: bool,
}

impl WaterTelemetry {
    /// Build telemetry from raw sensor readings
    ///
    /// `water_temp_c` comes from a dedicated water temperature probe (e.g. DS18B20)
    /// `env` provides ambient temperature and humidity from BME280
    pub fn from_readings(
        adc: &WaterAdcReadings,
        water_temp_c: Option<f32>,
        env: Option<&EnvReading>,
        pump_on: bool,
        valve_on: bool,
        tank_overflow: bool,
    ) -> Self {
        let sensor_fault = !adc.all_valid();

        Self {
            pressure_bar: adc.pressure_bar(),
            ph: adc.ph(),
            turbidity_ntu: adc.turbidity_ntu(),
            chlorine_mgl: adc.chlorine_mgl(),
            tank_level_pct: adc.tank_level_pct(),
            water_temp_c: water_temp_c.unwrap_or(0.0),
            ambient_temp_c: env.map(|e| e.temperature_c).unwrap_or(0.0),
            humidity_pct: env.map(|e| e.humidity_pct).unwrap_or(0.0),
            pump_on,
            valve_on,
            tank_overflow,
            sensor_fault,
        }
    }

    /// Check if any sensor reading is in alarm condition
    pub fn has_alarm(&self) -> bool {
        self.sensor_fault
            || self.tank_overflow
            || !PRESSURE_CAL.is_valid(self.pressure_bar)
            || !PH_CAL.is_valid(self.ph)
            || !TURBIDITY_CAL.is_valid(self.turbidity_ntu)
            || !CHLORINE_CAL.is_valid(self.chlorine_mgl)
            || !TANK_LEVEL_CAL.is_valid(self.tank_level_pct)
    }
}

/// Water subsystem operating state
#[derive(Clone, Copy, PartialEq)]
pub enum WaterMode {
    Auto,
    Manual,
    Maintenance,
}

/// Command from SCADA backend to water node
pub enum WaterCommand {
    SetPump(bool),
    SetValve(bool),
    SetMode(WaterMode),
    EmergencyStop,
}

impl WaterCommand {
    /// Parse a command from MQTT JSON payload
    /// Expected format: {"cmd":"pump_on"} or {"cmd":"pump_off"} etc.
    pub fn from_payload(payload: &[u8]) -> Option<Self> {
        // Simple manual parse for no_std (avoid full JSON parser for commands)
        if let Ok(s) = core::str::from_utf8(payload) {
            if s.contains("pump_on") {
                return Some(WaterCommand::SetPump(true));
            }
            if s.contains("pump_off") {
                return Some(WaterCommand::SetPump(false));
            }
            if s.contains("valve_on") {
                return Some(WaterCommand::SetValve(true));
            }
            if s.contains("valve_off") {
                return Some(WaterCommand::SetValve(false));
            }
            if s.contains("mode_auto") {
                return Some(WaterCommand::SetMode(WaterMode::Auto));
            }
            if s.contains("mode_manual") {
                return Some(WaterCommand::SetMode(WaterMode::Manual));
            }
            if s.contains("mode_maintenance") {
                return Some(WaterCommand::SetMode(WaterMode::Maintenance));
            }
            if s.contains("emergency_stop") {
                return Some(WaterCommand::EmergencyStop);
            }
        }
        None
    }
}

/// MQTT topic builder for water subsystem
pub struct WaterTopics;

impl WaterTopics {
    pub fn telemetry(device_id: &str) -> String<128> {
        let mut s = String::new();
        let _ = s.push_str("scada/water/");
        let _ = s.push_str(device_id);
        let _ = s.push_str("/telemetry");
        s
    }

    pub fn status(device_id: &str) -> String<128> {
        let mut s = String::new();
        let _ = s.push_str("scada/water/");
        let _ = s.push_str(device_id);
        let _ = s.push_str("/status");
        s
    }

    pub fn command(device_id: &str) -> String<128> {
        let mut s = String::new();
        let _ = s.push_str("scada/water/");
        let _ = s.push_str(device_id);
        let _ = s.push_str("/command");
        s
    }
}
