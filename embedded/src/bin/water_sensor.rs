use log::{error, info, warn};
use std::time::Duration;
use tokio::time;

use scada_embedded::actuators::{Actuator, WaterActuator};
use scada_embedded::config::Config;
use scada_embedded::mqtt::MqttTransport;
use scada_embedded::sensors::water::WaterSensorArray;
use scada_embedded::sensors::SensorArray;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("╔══════════════════════════════════════════╗");
    info!("║  SCADA Water Sensor Node                 ║");
    info!("║  Embedded Firmware v0.1.0                ║");
    info!("╚══════════════════════════════════════════╝");

    // Load config or use defaults
    let config = Config::load("config.toml").unwrap_or_else(|_| {
        info!("No config.toml found, using defaults");
        Config::default_water()
    });

    info!("Device ID: {}", config.device.id);
    info!("Device Name: {}", config.device.name);
    info!("Subsystem: {}", config.device.subsystem);
    info!("MQTT Broker: {}:{}", config.mqtt.broker_host, config.mqtt.broker_port);

    // Initialize MQTT
    let mut mqtt = MqttTransport::new(&config.mqtt, &config.device.id, &config.device.subsystem).await?;

    // Publish online status
    mqtt.publish_status("online").await?;

    // Initialize sensors and actuators
    let mut sensors = WaterSensorArray::new();
    let mut actuator = WaterActuator::new();

    // Perform initial self-check
    if sensors.self_check() {
        info!("Sensor self-check: PASSED");
    } else {
        warn!("Sensor self-check: FAILED - running in degraded mode");
    }

    // Main control loop
    let mut sampling_interval = time::interval(Duration::from_millis(config.sampling.publish_interval_ms));
    let mut health_check_interval = time::interval(Duration::from_secs(60));

    info!("Entering main control loop (publish every {}ms)", config.sampling.publish_interval_ms);

    loop {
        tokio::select! {
            // Sensor reading & publishing
            _ = sampling_interval.tick() => {
                // Sync pump state from actuator to sensor simulation
                sensors.set_pump_running(actuator.is_pump_running());

                let metrics = sensors.read_all();
                let quality = sensors.quality();

                info!(
                    "Tank: {:.1}% | Flow In: {:.1} L/min | pH: {:.2} | Pressure: {:.2} bar",
                    metrics.get("tank_level").unwrap_or(&0.0),
                    metrics.get("flow_rate_in").unwrap_or(&0.0),
                    metrics.get("ph_level").unwrap_or(&0.0),
                    metrics.get("pressure").unwrap_or(&0.0),
                );

                if let Err(e) = mqtt.publish_telemetry(metrics, quality).await {
                    error!("Failed to publish telemetry: {}", e);
                }
            }

            // Command reception
            cmd = mqtt.receive_command() => {
                if let Some(command) = cmd {
                    info!("Received command: {} (ID: {})", command.action, command.command_id);
                    match actuator.execute_command(&command.action, &command.parameters) {
                        Ok(result) => info!("Command executed: {}", result),
                        Err(e) => error!("Command failed: {}", e),
                    }
                }
            }

            // Periodic health check
            _ = health_check_interval.tick() => {
                let ok = sensors.self_check();
                if !ok {
                    warn!("Health check FAILED - sensor quality degraded");
                    mqtt.publish_status("fault").await.ok();
                } else {
                    mqtt.publish_status("online").await.ok();
                }
            }
        }
    }
}
