/// Solar energy monitoring sensor driver
///
/// Combines ADC readings (irradiance, panel voltage) with
/// INA219 current/voltage measurement for complete solar telemetry.

use heapless::String;
use serde::Serialize;

use crate::hal::adc::{SolarAdcReadings, IRRADIANCE_CAL, PANEL_VOLTAGE_CAL};
use crate::hal::i2c_sensors::{EnvReading, PowerReading};

/// Complete solar subsystem telemetry snapshot
#[derive(Serialize)]
pub struct SolarTelemetry {
    pub irradiance_wm2: f32,
    pub panel_voltage_v: f32,
    pub panel_current_ma: f32,
    pub panel_power_w: f32,
    pub ambient_temp_c: f32,
    pub humidity_pct: f32,
    pub inverter_on: bool,
    pub sensor_fault: bool,
}

impl SolarTelemetry {
    /// Build telemetry from raw sensor readings
    pub fn from_readings(
        adc: &SolarAdcReadings,
        power: Option<&PowerReading>,
        env: Option<&EnvReading>,
        inverter_on: bool,
    ) -> Self {
        let sensor_fault = !adc.all_valid();

        Self {
            irradiance_wm2: adc.irradiance_wm2(),
            panel_voltage_v: adc.panel_voltage(),
            panel_current_ma: power.map(|p| p.current_ma).unwrap_or(0.0),
            panel_power_w: power.map(|p| p.power_mw / 1000.0).unwrap_or(0.0),
            ambient_temp_c: env.map(|e| e.temperature_c).unwrap_or(0.0),
            humidity_pct: env.map(|e| e.humidity_pct).unwrap_or(0.0),
            inverter_on,
            sensor_fault,
        }
    }

    /// Check if readings indicate an alarm condition
    pub fn has_alarm(&self) -> bool {
        self.sensor_fault
            || !IRRADIANCE_CAL.is_valid(self.irradiance_wm2)
            || !PANEL_VOLTAGE_CAL.is_valid(self.panel_voltage_v)
    }

    /// Calculate panel efficiency (output power / theoretical solar power)
    pub fn efficiency(&self, panel_area_m2: f32) -> f32 {
        if self.irradiance_wm2 < 10.0 || panel_area_m2 <= 0.0 {
            return 0.0;
        }
        let theoretical_w = self.irradiance_wm2 * panel_area_m2;
        (self.panel_power_w / theoretical_w) * 100.0
    }
}

/// Solar subsystem operating state
#[derive(Clone, Copy, PartialEq)]
pub enum SolarMode {
    Auto,
    Manual,
    Maintenance,
}

/// Command from SCADA backend to solar node
pub enum SolarCommand {
    SetInverter(bool),
    SetMode(SolarMode),
    EmergencyStop,
}

impl SolarCommand {
    /// Parse a command from MQTT JSON payload
    pub fn from_payload(payload: &[u8]) -> Option<Self> {
        if let Ok(s) = core::str::from_utf8(payload) {
            if s.contains("inverter_on") {
                return Some(SolarCommand::SetInverter(true));
            }
            if s.contains("inverter_off") {
                return Some(SolarCommand::SetInverter(false));
            }
            if s.contains("mode_auto") {
                return Some(SolarCommand::SetMode(SolarMode::Auto));
            }
            if s.contains("mode_manual") {
                return Some(SolarCommand::SetMode(SolarMode::Manual));
            }
            if s.contains("mode_maintenance") {
                return Some(SolarCommand::SetMode(SolarMode::Maintenance));
            }
            if s.contains("emergency_stop") {
                return Some(SolarCommand::EmergencyStop);
            }
        }
        None
    }
}

/// MQTT topic builder for solar subsystem
pub struct SolarTopics;

impl SolarTopics {
    pub fn telemetry(device_id: &str) -> String<128> {
        let mut s = String::new();
        let _ = s.push_str("scada/solar/");
        let _ = s.push_str(device_id);
        let _ = s.push_str("/telemetry");
        s
    }

    pub fn status(device_id: &str) -> String<128> {
        let mut s = String::new();
        let _ = s.push_str("scada/solar/");
        let _ = s.push_str(device_id);
        let _ = s.push_str("/status");
        s
    }

    pub fn command(device_id: &str) -> String<128> {
        let mut s = String::new();
        let _ = s.push_str("scada/solar/");
        let _ = s.push_str(device_id);
        let _ = s.push_str("/command");
        s
    }
}
