/// MQTT publish task — periodically publishes telemetry to broker
///
/// Reads sensor data from shared state and publishes JSON telemetry
/// to the configured MQTT topic. Handles connection loss and reconnect.

use embassy_time::{Duration, Ticker, Timer};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use defmt::{info, warn, error};

use crate::net::mqtt::{MqttClient, MqttState, reconnect_delay};

/// MQTT publish configuration
pub struct PublishConfig {
    pub interval_ms: u64,
    pub max_retries: u32,
}

impl Default for PublishConfig {
    fn default() -> Self {
        Self {
            interval_ms: 5000,
            max_retries: 3,
        }
    }
}

/// Publish statistics
pub struct PublishStats {
    pub messages_sent: u32,
    pub messages_failed: u32,
    pub reconnect_count: u32,
    pub last_publish_ms: u64,
}

impl PublishStats {
    pub const fn new() -> Self {
        Self {
            messages_sent: 0,
            messages_failed: 0,
            reconnect_count: 0,
            last_publish_ms: 0,
        }
    }
}

/// Signal to trigger an immediate publish (e.g., on alarm)
pub type PublishSignal = Signal<CriticalSectionRawMutex, ()>;
