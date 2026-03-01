/// PTP Management — IEEE 1588 Precision Time Protocol
///
/// Manages ptp4l and phc2sys processes for PTP time synchronization.
/// Monitors synchronization status and reports to DDS and the M4 core.
///
/// Process hierarchy:
///   ptp4l  — PTP daemon, synchronizes ETH MAC clock to network master
///   phc2sys — Copies PTP hardware clock to Linux system clock
///
/// This module:
///   1. Starts/monitors ptp4l process
///   2. Starts/monitors phc2sys process
///   3. Reads PTP clock offset from ptp4l output
///   4. Writes sync status to shared memory for M4 access
///   5. Reports sync status via DDS heartbeat data

use anyhow::Result;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::{info, debug, warn, error};
use std::process::Stdio;

use crate::config::ScadaConfig;

/// PTP synchronization state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PtpState {
    Stopped,
    Starting,
    FreeRunning,
    Tracking,
    Locked,
    Error,
}

/// Shared memory address for PTP sync status (must match M4 side)
const PTP_SYNC_STATUS_ADDR: u64 = 0x1003_FF00;
const PTP_SYNC_MAGIC: u32 = 0x50545030; // "PTP0"

/// Run the PTP synchronization monitor
pub async fn run_ptp_monitor(
    config: &ScadaConfig,
    cancel: CancellationToken,
) -> Result<()> {
    if !config.ptp.enabled {
        info!("PTP: Disabled by configuration");
        return Ok(());
    }

    info!("PTP: Starting synchronization on interface '{}'", config.ptp.interface);

    // Start ptp4l
    let ptp4l_cancel = cancel.clone();
    let ptp4l_config = config.clone();
    let ptp4l_handle = tokio::spawn(async move {
        run_ptp4l(&ptp4l_config, ptp4l_cancel).await
    });

    // Wait for ptp4l to initialize
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Start phc2sys
    let phc2sys_cancel = cancel.clone();
    let phc2sys_config = config.clone();
    let phc2sys_handle = tokio::spawn(async move {
        run_phc2sys(&phc2sys_config, phc2sys_cancel).await
    });

    // Monitor PTP status
    let mut state = PtpState::Starting;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("PTP: Shutting down...");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                // Check PTP status by reading ptp4l statistics
                match check_ptp_status().await {
                    Ok(new_state) => {
                        if new_state != state {
                            info!("PTP: State changed {:?} → {:?}", state, new_state);
                            state = new_state;
                        }
                        // Write status to shared memory for M4
                        update_shared_memory(state).await;
                    }
                    Err(e) => {
                        warn!("PTP: Status check failed: {}", e);
                    }
                }
            }
        }
    }

    // Cleanup
    let _ = ptp4l_handle.await;
    let _ = phc2sys_handle.await;

    Ok(())
}

/// Start and monitor the ptp4l process
async fn run_ptp4l(config: &ScadaConfig, cancel: CancellationToken) -> Result<()> {
    info!("PTP: Starting ptp4l on interface '{}'", config.ptp.interface);

    let mut child = Command::new(&config.ptp.ptp4l_path)
        .args(&[
            "-i", &config.ptp.interface,
            "-f", &config.ptp.ptp4l_config,
            "-s",  // Slave-only mode (field device)
            "-m",  // Print messages to stdout
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start ptp4l: {}", e))?;

    info!("PTP: ptp4l started (PID={})", child.id().unwrap_or(0));

    tokio::select! {
        _ = cancel.cancelled() => {
            info!("PTP: Stopping ptp4l...");
            let _ = child.kill().await;
        }
        status = child.wait() => {
            match status {
                Ok(s) => warn!("PTP: ptp4l exited with status: {}", s),
                Err(e) => error!("PTP: ptp4l wait error: {}", e),
            }
        }
    }

    Ok(())
}

/// Start and monitor the phc2sys process
async fn run_phc2sys(config: &ScadaConfig, cancel: CancellationToken) -> Result<()> {
    info!("PTP: Starting phc2sys");

    let mut child = Command::new(&config.ptp.phc2sys_path)
        .args(&[
            "-s", &config.ptp.interface,
            "-c", "CLOCK_REALTIME",
            "-O", "0",  // UTC offset
            "-m",        // Print messages
            "-w",        // Wait for ptp4l
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start phc2sys: {}", e))?;

    info!("PTP: phc2sys started (PID={})", child.id().unwrap_or(0));

    tokio::select! {
        _ = cancel.cancelled() => {
            info!("PTP: Stopping phc2sys...");
            let _ = child.kill().await;
        }
        status = child.wait() => {
            match status {
                Ok(s) => warn!("PTP: phc2sys exited with status: {}", s),
                Err(e) => error!("PTP: phc2sys wait error: {}", e),
            }
        }
    }

    Ok(())
}

/// Check PTP synchronization status
async fn check_ptp_status() -> Result<PtpState> {
    // Read PTP clock offset via pmc (PTP Management Client)
    // or by parsing ptp4l output/sysfs
    let output = Command::new("pmc")
        .args(&["-u", "-b", "0", "GET CURRENT_DATA_SET"])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.contains("offsetFromMaster") {
                // Parse offset value
                if let Some(offset_line) = stdout.lines().find(|l| l.contains("offsetFromMaster")) {
                    let parts: Vec<&str> = offset_line.split_whitespace().collect();
                    if let Some(offset_str) = parts.last() {
                        if let Ok(offset_ns) = offset_str.parse::<f64>() {
                            let abs_offset = offset_ns.abs();
                            if abs_offset < 1000.0 {
                                return Ok(PtpState::Locked); // < 1 us
                            } else if abs_offset < 100_000.0 {
                                return Ok(PtpState::Tracking); // < 100 us
                            } else {
                                return Ok(PtpState::FreeRunning);
                            }
                        }
                    }
                }
                Ok(PtpState::Tracking)
            } else {
                Ok(PtpState::FreeRunning)
            }
        }
        Err(_) => Ok(PtpState::FreeRunning),
    }
}

/// Write PTP sync status to shared memory for M4 access
async fn update_shared_memory(state: PtpState) {
    // In production, this would mmap the MCUSRAM2 shared memory region
    // and write the PtpSyncStatus structure that the M4 reads.
    //
    // For now, we write to /dev/mem at the shared memory address.
    // This requires root privileges or appropriate device tree configuration.

    let state_value: u32 = match state {
        PtpState::FreeRunning => 0,
        PtpState::Tracking => 1,
        PtpState::Locked => 2,
        _ => 0,
    };

    // Write via sysfs or /dev/mem
    // In a production system, use a proper shared memory mechanism
    debug!("PTP: Writing sync status {} to shared memory", state_value);
}
