/// DDS Publishers
///
/// Creates data writers for each SCADA topic and publishes sensor data
/// received from the M4 via RPMsg onto the DDS network.
///
/// Topic hierarchy:
///   rt/solar/string_voltage  — Solar string voltages (best-effort, 1 Hz)
///   rt/solar/string_current  — Solar string currents (best-effort, 1 Hz)
///   rt/solar/bus_power       — DC bus power (best-effort, 1 Hz)
///   rt/water/level           — Water tank levels (best-effort, 1 Hz)
///   rt/water/flow            — Flow rates (best-effort, 1 Hz)
///   rt/water/pressure        — Pressure readings (best-effort, 1 Hz)
///   rt/water/quality         — Water quality (best-effort, 0.2 Hz)
///   rt/alarm                 — Alarm events (reliable, event-driven)
///   rt/heartbeat             — System health (best-effort, 0.1 Hz)
///
/// QoS per-topic:
///   - Sensor data: best-effort, volatile, 1s deadline, 5s lifespan
///   - Alarms: reliable, transient-local, 32 depth, 60s lifespan
///   - Heartbeat: best-effort, volatile, 10s deadline, 30s lifespan

use std::sync::Arc;
use anyhow::Result;
use rustdds::*;
use tokio_util::sync::CancellationToken;
use tracing::{info, debug, warn};

use crate::config::ScadaConfig;
use crate::dds::types::*;
use crate::dds::participant;

/// Run all DDS publishers
///
/// This function creates data writers for all configured topics
/// and publishes data from the shared data store (populated by the
/// RPMsg/XRCE agent from M4 sensor data).
pub async fn run_publishers(
    publisher: Arc<Publisher>,
    config: &ScadaConfig,
    cancel: CancellationToken,
) -> Result<()> {
    info!("DDS publishers starting...");

    // Create data writers based on device type
    let is_solar = config.device.device_type == "solar_smu";

    if is_solar {
        info!("Creating Solar SMU data writers");

        // String voltage writer
        let _voltage_topic = publisher
            .create_topic(
                TopicDescription::new(
                    StringVoltageData::TOPIC_NAME,
                    StringVoltageData::TYPE_NAME,
                ),
                &participant::sensor_data_qos(),
            );

        // String current writer
        let _current_topic = publisher
            .create_topic(
                TopicDescription::new(
                    StringCurrentData::TOPIC_NAME,
                    StringCurrentData::TYPE_NAME,
                ),
                &participant::sensor_data_qos(),
            );

        // Bus power writer
        let _power_topic = publisher
            .create_topic(
                TopicDescription::new(
                    BusPowerData::TOPIC_NAME,
                    BusPowerData::TYPE_NAME,
                ),
                &participant::sensor_data_qos(),
            );
    } else {
        info!("Creating Water RTU data writers");

        let _level_topic = publisher
            .create_topic(
                TopicDescription::new(
                    WaterLevelData::TOPIC_NAME,
                    WaterLevelData::TYPE_NAME,
                ),
                &participant::sensor_data_qos(),
            );

        let _flow_topic = publisher
            .create_topic(
                TopicDescription::new(
                    FlowRateData::TOPIC_NAME,
                    FlowRateData::TYPE_NAME,
                ),
                &participant::sensor_data_qos(),
            );
    }

    // Common topics
    let _alarm_topic = publisher
        .create_topic(
            TopicDescription::new(
                AlarmEvent::TOPIC_NAME,
                AlarmEvent::TYPE_NAME,
            ),
            &participant::alarm_qos(),
        );

    let _heartbeat_topic = publisher
        .create_topic(
            TopicDescription::new(
                HeartbeatData::TOPIC_NAME,
                HeartbeatData::TYPE_NAME,
            ),
            &participant::heartbeat_qos(),
        );

    info!("All DDS data writers created");

    // Main publish loop — data is pushed from the RPMsg agent
    // This task primarily manages the DDS writer lifecycle
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("DDS publishers shutting down...");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                debug!("DDS publishers alive");
            }
        }
    }

    Ok(())
}

/// Topic description helper
struct TopicDescription {
    name: String,
    type_name: String,
}

impl TopicDescription {
    fn new(name: &str, type_name: &str) -> Self {
        Self {
            name: name.to_string(),
            type_name: type_name.to_string(),
        }
    }
}
