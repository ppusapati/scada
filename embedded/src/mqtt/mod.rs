use log::{error, info, warn};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::MqttConfig;

#[derive(Debug, Serialize)]
pub struct TelemetryPayload {
    pub device_id: String,
    pub timestamp: String,
    pub metrics: HashMap<String, f64>,
    pub quality: u8,
}

#[derive(Debug, Serialize)]
pub struct StatusPayload {
    pub device_id: String,
    pub status: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct CommandPayload {
    pub command_id: String,
    pub action: String,
    pub parameters: HashMap<String, String>,
    pub issued_by: String,
}

pub struct MqttTransport {
    client: AsyncClient,
    device_id: String,
    subsystem: String,
    command_rx: mpsc::Receiver<CommandPayload>,
}

impl MqttTransport {
    pub async fn new(
        config: &MqttConfig,
        device_id: &str,
        subsystem: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut mqttoptions = MqttOptions::new(
            &config.client_id,
            &config.broker_host,
            config.broker_port,
        );
        mqttoptions.set_keep_alive(Duration::from_secs(config.keep_alive_secs));
        mqttoptions.set_clean_session(true);

        if let (Some(user), Some(pass)) = (&config.username, &config.password) {
            mqttoptions.set_credentials(user, pass);
        }

        // Set Last Will and Testament
        let will_topic = format!("scada/{}/{}/status", subsystem, device_id);
        let will_payload = serde_json::to_string(&StatusPayload {
            device_id: device_id.to_string(),
            status: "offline".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        })?;
        mqttoptions.set_last_will(rumqttc::LastWill::new(
            &will_topic,
            will_payload,
            QoS::AtLeastOnce,
            true,
        ));

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 50);
        let (command_tx, command_rx) = mpsc::channel::<CommandPayload>(32);

        let dev_id = device_id.to_string();
        let sub = subsystem.to_string();

        // Spawn event loop handler
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        let topic = publish.topic.clone();
                        if topic.ends_with("/command") {
                            match serde_json::from_slice::<CommandPayload>(&publish.payload) {
                                Ok(cmd) => {
                                    info!("[MQTT] Received command: {:?}", cmd.action);
                                    if let Err(e) = command_tx.send(cmd).await {
                                        error!("[MQTT] Failed to forward command: {}", e);
                                    }
                                }
                                Err(e) => warn!("[MQTT] Invalid command payload: {}", e),
                            }
                        }
                    }
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        info!("[MQTT] Connected to broker");
                        // Subscribe to command topic
                        let cmd_topic = format!("scada/{}/{}/command", sub, dev_id);
                        // We can't easily subscribe from here, so we do it after creation
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("[MQTT] Event loop error: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        // Subscribe to command topic
        let cmd_topic = format!("scada/{}/{}/command", subsystem, device_id);
        client.subscribe(&cmd_topic, QoS::AtLeastOnce).await?;
        info!("[MQTT] Subscribed to {}", cmd_topic);

        Ok(MqttTransport {
            client,
            device_id: device_id.to_string(),
            subsystem: subsystem.to_string(),
            command_rx,
        })
    }

    pub async fn publish_telemetry(
        &self,
        metrics: HashMap<String, f64>,
        quality: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = TelemetryPayload {
            device_id: self.device_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metrics,
            quality,
        };

        let topic = format!("scada/{}/{}/telemetry", self.subsystem, self.device_id);
        let json = serde_json::to_string(&payload)?;

        self.client
            .publish(&topic, QoS::AtLeastOnce, false, json.as_bytes())
            .await?;

        Ok(())
    }

    pub async fn publish_status(&self, status: &str) -> Result<(), Box<dyn std::error::Error>> {
        let payload = StatusPayload {
            device_id: self.device_id.clone(),
            status: status.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let topic = format!("scada/{}/{}/status", self.subsystem, self.device_id);
        let json = serde_json::to_string(&payload)?;

        self.client
            .publish(&topic, QoS::AtLeastOnce, true, json.as_bytes())
            .await?;

        Ok(())
    }

    pub async fn receive_command(&mut self) -> Option<CommandPayload> {
        self.command_rx.recv().await
    }
}
