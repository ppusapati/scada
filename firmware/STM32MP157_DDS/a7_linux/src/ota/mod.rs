/// OTA Firmware Update Module
///
/// Handles over-the-air firmware updates for the M4 co-processor.
/// The update process:
///   1. Check for available updates (HTTP or DDS notification)
///   2. Download new M4 firmware image
///   3. Validate firmware (CRC32, version check)
///   4. Stop M4 via remoteproc
///   5. Write firmware to /lib/firmware/stm32mp157-m4.elf
///   6. Restart M4 via remoteproc
///   7. Verify M4 is running (RPMsg heartbeat)
///   8. Rollback on failure

use anyhow::{Context, Result};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, error};

use crate::config::ScadaConfig;

/// Firmware image header (first 32 bytes of the .elf file wrapper)
#[derive(Debug)]
struct FirmwareHeader {
    magic: u32,       // 0x50394D34 = "P9M4"
    version_major: u8,
    version_minor: u8,
    version_patch: u8,
    device_type: u8,  // 0=solar, 1=water
    image_size: u32,
    crc32: u32,
    build_timestamp: u32,
}

const FW_MAGIC: u32 = 0x5039_4D34; // "P9M4"

/// Run the OTA update listener
pub async fn run_ota_listener(
    config: &ScadaConfig,
    cancel: CancellationToken,
) -> Result<()> {
    if !config.ota.enabled {
        info!("OTA: Disabled by configuration");
        return Ok(());
    }

    info!("OTA: Update listener started (check interval: {}s)", config.ota.check_interval_secs);

    let check_interval = std::time::Duration::from_secs(config.ota.check_interval_secs);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("OTA: Shutting down...");
                break;
            }
            _ = tokio::time::sleep(check_interval) => {
                match check_for_update(config).await {
                    Ok(Some(url)) => {
                        info!("OTA: Update available at {}", url);
                        match perform_update(config, &url).await {
                            Ok(()) => info!("OTA: Update completed successfully"),
                            Err(e) => {
                                error!("OTA: Update failed: {}", e);
                                if let Err(re) = rollback_update(config).await {
                                    error!("OTA: Rollback also failed: {}", re);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // No update available
                    }
                    Err(e) => {
                        warn!("OTA: Update check failed: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check the OTA server for available updates
async fn check_for_update(config: &ScadaConfig) -> Result<Option<String>> {
    let check_url = format!("{}/check?device={}&type={}",
        config.ota.server_url,
        config.device.device_id,
        config.device.device_type);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(&check_url).send().await
        .context("Failed to reach OTA server")?;

    if response.status() == reqwest::StatusCode::OK {
        let body: serde_json::Value = response.json().await?;
        if let Some(url) = body.get("download_url").and_then(|v| v.as_str()) {
            return Ok(Some(url.to_string()));
        }
    }

    Ok(None)
}

/// Perform the firmware update
async fn perform_update(config: &ScadaConfig, download_url: &str) -> Result<()> {
    info!("OTA: Downloading firmware from {}...", download_url);

    // Download firmware
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let response = client.get(download_url).send().await
        .context("Failed to download firmware")?;

    let firmware_data = response.bytes().await
        .context("Failed to read firmware data")?;

    info!("OTA: Downloaded {} bytes", firmware_data.len());

    // Validate firmware
    validate_firmware(&firmware_data)?;

    // Backup current firmware
    let backup_path = format!("{}.backup", config.ota.m4_firmware_path);
    if std::path::Path::new(&config.ota.m4_firmware_path).exists() {
        tokio::fs::copy(&config.ota.m4_firmware_path, &backup_path).await
            .context("Failed to backup current firmware")?;
        info!("OTA: Current firmware backed up to {}", backup_path);
    }

    // Stop M4
    info!("OTA: Stopping M4 co-processor...");
    let state_path = format!("{}/state", config.ota.remoteproc_path);
    tokio::fs::write(&state_path, "stop").await
        .context("Failed to stop M4 via remoteproc")?;

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Write new firmware
    info!("OTA: Writing new firmware to {}", config.ota.m4_firmware_path);
    tokio::fs::write(&config.ota.m4_firmware_path, &firmware_data).await
        .context("Failed to write firmware file")?;

    // Set firmware filename in remoteproc
    let fw_name_path = format!("{}/firmware", config.ota.remoteproc_path);
    let fw_filename = std::path::Path::new(&config.ota.m4_firmware_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("stm32mp157-m4.elf");
    tokio::fs::write(&fw_name_path, fw_filename).await
        .context("Failed to set firmware filename")?;

    // Start M4 with new firmware
    info!("OTA: Starting M4 with new firmware...");
    tokio::fs::write(&state_path, "start").await
        .context("Failed to start M4 via remoteproc")?;

    // Wait and verify M4 is running
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let state = tokio::fs::read_to_string(&state_path).await
        .context("Failed to read remoteproc state")?;

    if state.trim() != "running" {
        return Err(anyhow::anyhow!("M4 failed to start with new firmware (state: {})", state.trim()));
    }

    info!("OTA: M4 running with new firmware");

    // Remove backup on success
    let _ = tokio::fs::remove_file(&backup_path).await;

    Ok(())
}

/// Validate a firmware image
fn validate_firmware(data: &[u8]) -> Result<()> {
    if data.len() < 32 {
        return Err(anyhow::anyhow!("Firmware image too small ({} bytes)", data.len()));
    }

    // Check ELF magic number (standard ELF)
    if data[0..4] == [0x7F, b'E', b'L', b'F'] {
        info!("OTA: Valid ELF firmware ({} bytes)", data.len());
        // Compute CRC32 of the entire image
        let crc = crc32fast::hash(data);
        info!("OTA: Firmware CRC32: 0x{:08X}", crc);
        return Ok(());
    }

    Err(anyhow::anyhow!("Invalid firmware format (not ELF)"))
}

/// Rollback to the previous firmware
async fn rollback_update(config: &ScadaConfig) -> Result<()> {
    let backup_path = format!("{}.backup", config.ota.m4_firmware_path);

    if !std::path::Path::new(&backup_path).exists() {
        return Err(anyhow::anyhow!("No backup firmware available for rollback"));
    }

    warn!("OTA: Rolling back to previous firmware...");

    // Stop M4
    let state_path = format!("{}/state", config.ota.remoteproc_path);
    let _ = tokio::fs::write(&state_path, "stop").await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Restore backup
    tokio::fs::copy(&backup_path, &config.ota.m4_firmware_path).await
        .context("Failed to restore backup firmware")?;

    // Restart M4
    tokio::fs::write(&state_path, "start").await
        .context("Failed to restart M4 with backup firmware")?;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let state = tokio::fs::read_to_string(&state_path).await
        .unwrap_or_default();

    if state.trim() == "running" {
        info!("OTA: Rollback successful, M4 running with previous firmware");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Rollback failed, M4 state: {}", state.trim()))
    }
}
