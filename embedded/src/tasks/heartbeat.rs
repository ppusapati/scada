/// Heartbeat task — LED blink and status reporting
///
/// Blinks the heartbeat LED at 1Hz to indicate the device is alive.
/// Also periodically publishes device health status over MQTT.

use embassy_time::{Duration, Ticker};
use embassy_stm32::gpio::Output;
use defmt::info;

/// Heartbeat configuration
pub struct HeartbeatConfig {
    pub blink_interval_ms: u64,
    pub status_report_interval_s: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            blink_interval_ms: 500,
            status_report_interval_s: 30,
        }
    }
}

/// Device health snapshot
pub struct HealthStatus {
    pub uptime_secs: u32,
    pub sensor_ok: bool,
    pub mqtt_connected: bool,
    pub sd_card_ok: bool,
    pub fault_count: u32,
    pub free_stack_bytes: u32,
}

impl HealthStatus {
    pub const fn new() -> Self {
        Self {
            uptime_secs: 0,
            sensor_ok: false,
            mqtt_connected: false,
            sd_card_ok: false,
            fault_count: 0,
            free_stack_bytes: 0,
        }
    }
}

/// Toggle a GPIO output for LED heartbeat
pub fn toggle_led(led: &mut Output<'static>, state: &mut bool) {
    *state = !*state;
    if *state {
        led.set_high();
    } else {
        led.set_low();
    }
}
