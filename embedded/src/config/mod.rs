use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub device: DeviceConfig,
    pub mqtt: MqttConfig,
    pub sampling: SamplingConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceConfig {
    pub id: String,
    pub name: String,
    pub device_type: String,     // water_sensor, solar_panel, flow_meter, pump
    pub subsystem: String,       // water, solar
    pub location: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MqttConfig {
    pub broker_host: String,
    pub broker_port: u16,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keep_alive_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SamplingConfig {
    pub interval_ms: u64,        // How often to read sensors
    pub batch_size: usize,       // How many readings to batch before publishing
    pub publish_interval_ms: u64, // Minimum interval between publishes
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default_water() -> Self {
        Config {
            device: DeviceConfig {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Water Sensor Node 1".into(),
                device_type: "water_sensor".into(),
                subsystem: "water".into(),
                location: "Treatment Plant A".into(),
            },
            mqtt: MqttConfig {
                broker_host: "localhost".into(),
                broker_port: 1883,
                client_id: format!("water-sensor-{}", &uuid::Uuid::new_v4().to_string()[..8]),
                username: None,
                password: None,
                keep_alive_secs: 30,
            },
            sampling: SamplingConfig {
                interval_ms: 1000,
                batch_size: 5,
                publish_interval_ms: 5000,
            },
        }
    }

    pub fn default_solar() -> Self {
        Config {
            device: DeviceConfig {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Solar Panel Array 1".into(),
                device_type: "solar_panel".into(),
                subsystem: "solar".into(),
                location: "Solar Field Alpha".into(),
            },
            mqtt: MqttConfig {
                broker_host: "localhost".into(),
                broker_port: 1883,
                client_id: format!("solar-sensor-{}", &uuid::Uuid::new_v4().to_string()[..8]),
                username: None,
                password: None,
                keep_alive_secs: 30,
            },
            sampling: SamplingConfig {
                interval_ms: 2000,
                batch_size: 3,
                publish_interval_ms: 6000,
            },
        }
    }
}
