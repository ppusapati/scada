use log::{error, info, warn};
use std::time::Duration;
use tokio::time;

use scada_embedded::actuators::{Actuator, SolarActuator};
use scada_embedded::config::Config;
use scada_embedded::mqtt::MqttTransport;
use scada_embedded::sensors::solar::SolarSensorArray;
use scada_embedded::sensors::SensorArray;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("╔══════════════════════════════════════════╗");
    info!("║  SCADA Solar Sensor Node                 ║");
    info!("║  Embedded Firmware v0.1.0                ║");
    info!("╚══════════════════════════════════════════╝");

    let config = Config::load("config.toml").unwrap_or_else(|_| {
        info!("No config.toml found, using defaults");
        Config::default_solar()
    });

    info!("Device ID: {}", config.device.id);
    info!("Device Name: {}", config.device.name);
    info!("MQTT Broker: {}:{}", config.mqtt.broker_host, config.mqtt.broker_port);

    // Initialize MQTT
    let mut mqtt = MqttTransport::new(&config.mqtt, &config.device.id, &config.device.subsystem).await?;
    mqtt.publish_status("online").await?;

    // Initialize sensors and actuators
    let mut sensors = SolarSensorArray::new();
    let mut actuator = SolarActuator::new();

    if sensors.self_check() {
        info!("Sensor self-check: PASSED");
    } else {
        warn!("Sensor self-check: FAILED");
    }

    let mut sampling_interval = time::interval(Duration::from_millis(config.sampling.publish_interval_ms));
    let mut health_check_interval = time::interval(Duration::from_secs(60));

    info!("Entering main control loop");

    loop {
        tokio::select! {
            _ = sampling_interval.tick() => {
                // Sync inverter state
                sensors.set_inverter_online(actuator.is_inverter_enabled());

                let metrics = sensors.read_all();
                let quality = sensors.quality();

                info!(
                    "Irradiance: {:.0} W/m² | Output: {:.2} kW | Temp: {:.1}°C | Grid: {:.1}V/{:.2}Hz",
                    metrics.get("irradiance").unwrap_or(&0.0),
                    metrics.get("current_output_kw").unwrap_or(&0.0),
                    metrics.get("panel_temperature").unwrap_or(&0.0),
                    metrics.get("grid_voltage").unwrap_or(&0.0),
                    metrics.get("grid_frequency").unwrap_or(&0.0),
                );

                if let Err(e) = mqtt.publish_telemetry(metrics, quality).await {
                    error!("Failed to publish telemetry: {}", e);
                }
            }

            cmd = mqtt.receive_command() => {
                if let Some(command) = cmd {
                    info!("Received command: {} (ID: {})", command.action, command.command_id);
                    match actuator.execute_command(&command.action, &command.parameters) {
                        Ok(result) => info!("Command executed: {}", result),
                        Err(e) => error!("Command failed: {}", e),
                    }
                }
            }

            _ = health_check_interval.tick() => {
                let ok = sensors.self_check();
                let status = if ok { "online" } else { "fault" };
                mqtt.publish_status(status).await.ok();
                if !ok {
                    warn!("Health check FAILED");
                }
            }
        }
    }
}
