/// DDS Subscribers
///
/// Subscribes to configuration and command topics from the DDS network.
/// Commands received are forwarded to the M4 via RPMsg.
///
/// Subscribed topics:
///   rt/config/command — Configuration commands (reliable delivery)
///   rt/ota/notify     — OTA update notifications

use std::sync::Arc;
use anyhow::Result;
use rustdds::*;
use tokio_util::sync::CancellationToken;
use tracing::{info, debug, warn, error};

use crate::config::ScadaConfig;
use crate::dds::types::*;
use crate::dds::participant;

/// Run all DDS subscribers
pub async fn run_subscribers(
    subscriber: Arc<Subscriber>,
    config: &ScadaConfig,
    cancel: CancellationToken,
) -> Result<()> {
    info!("DDS subscribers starting...");

    // Subscribe to configuration commands
    info!("Subscribing to '{}' topic", ConfigCommand::TOPIC_NAME);

    // Main subscription loop
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("DDS subscribers shutting down...");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // Poll for incoming messages
                // In a full implementation, this would use DDS waitsets
                // or async readers to receive data
            }
        }
    }

    Ok(())
}

/// Handle a received configuration command
async fn handle_command(cmd: &ConfigCommand) -> Result<()> {
    info!(
        "Received DDS command: type=0x{:02x}, target={}",
        cmd.command_type as u8,
        cmd.device_id,
    );

    match cmd.command_type {
        CommandType::SetScanInterval => {
            let interval = u16::from_le_bytes([cmd.parameters[0], cmd.parameters[1]]);
            info!("Setting scan interval to {}ms", interval);
            // Forward to M4 via RPMsg
        }
        CommandType::SetModbusAddress => {
            let addr = cmd.parameters[0];
            info!("Setting Modbus address to {}", addr);
        }
        CommandType::SaveCalibration => {
            info!("Requesting calibration save to flash");
        }
        CommandType::ResetCalibration => {
            info!("Requesting calibration factory reset");
        }
        CommandType::Restart => {
            warn!("M4 restart requested via DDS");
            // Trigger remoteproc restart
            restart_m4().await?;
        }
        _ => {
            warn!("Unknown command type: 0x{:02x}", cmd.command_type as u8);
        }
    }

    Ok(())
}

/// Restart the M4 co-processor via Linux remoteproc
async fn restart_m4() -> Result<()> {
    use tokio::fs;

    let remoteproc_path = "/sys/class/remoteproc/remoteproc0";

    // Stop M4
    info!("Stopping M4 via remoteproc...");
    fs::write(format!("{}/state", remoteproc_path), "stop").await
        .map_err(|e| anyhow::anyhow!("Failed to stop M4: {}", e))?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Start M4
    info!("Starting M4 via remoteproc...");
    fs::write(format!("{}/state", remoteproc_path), "start").await
        .map_err(|e| anyhow::anyhow!("Failed to start M4: {}", e))?;

    info!("M4 restart complete");
    Ok(())
}
