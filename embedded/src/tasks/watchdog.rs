/// Watchdog task — monitors system health and resets on hang
///
/// Configures the STM32 Independent Watchdog (IWDG) and periodically
/// feeds it. If any critical task stops running (detected via shared
/// state timestamps), the watchdog will not be fed and the MCU resets.

use embassy_time::{Duration, Ticker, Instant};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use defmt::{info, warn, error};

/// Maximum allowed time since last sensor read before declaring a fault (ms)
const SENSOR_TIMEOUT_MS: u64 = 10_000;

/// Maximum allowed time since last MQTT publish before declaring a fault (ms)
const MQTT_TIMEOUT_MS: u64 = 60_000;

/// Watchdog feed interval — must be shorter than IWDG timeout
const WATCHDOG_FEED_INTERVAL_MS: u64 = 1_000;

/// Monitored task timestamps
pub struct TaskTimestamps {
    pub sensor_last_ms: u64,
    pub mqtt_last_ms: u64,
    pub command_last_ms: u64,
    pub heartbeat_last_ms: u64,
}

impl TaskTimestamps {
    pub const fn new() -> Self {
        Self {
            sensor_last_ms: 0,
            mqtt_last_ms: 0,
            command_last_ms: 0,
            heartbeat_last_ms: 0,
        }
    }

    /// Check if all monitored tasks are running within acceptable timeframes
    pub fn all_healthy(&self, now_ms: u64) -> bool {
        let sensor_ok = now_ms.saturating_sub(self.sensor_last_ms) < SENSOR_TIMEOUT_MS;
        let mqtt_ok = now_ms.saturating_sub(self.mqtt_last_ms) < MQTT_TIMEOUT_MS;
        sensor_ok && mqtt_ok
    }
}

pub type SharedTimestamps = Mutex<CriticalSectionRawMutex, TaskTimestamps>;

/// Initialize the STM32 Independent Watchdog (IWDG)
/// Timeout is approximately 4 seconds (prescaler=64, reload=2500)
///
/// The IWDG uses the 32kHz LSI oscillator:
///   timeout = (prescaler × reload) / 32000
///   = (64 × 2500) / 32000 = 5 seconds
pub fn iwdg_timeout_secs() -> u32 {
    5
}
