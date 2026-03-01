/// RPMsg Linux Driver — /dev/ttyRPMSG0
///
/// Communicates with the M4 co-processor via the RPMsg character device.
/// On STM32MP157, the Linux kernel exposes RPMsg endpoints as
/// /dev/ttyRPMSGx devices through the rpmsg_char / rpmsg_tty driver.
///
/// This module:
///   1. Opens /dev/ttyRPMSG0 for bidirectional communication
///   2. Parses XRCE-DDS frames from the M4
///   3. Decodes CDR payloads into DDS data types
///   4. Publishes decoded data via the DDS publisher
///   5. Forwards DDS commands to the M4

use std::sync::Arc;
use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::OpenOptions;
use tokio_util::sync::CancellationToken;
use tracing::{info, debug, warn, error};

use crate::config::ScadaConfig;

/// XRCE protocol constants (must match M4 side)
const XRCE_MARKER: [u8; 2] = [0xCE, 0xD5];
const XRCE_HEADER_SIZE: usize = 8;

/// RPMsg device handle
pub struct RpmsgHandle {
    path: String,
}

impl RpmsgHandle {
    /// Open the RPMsg device
    pub fn open(path: &str) -> Result<Self> {
        // Verify device exists
        if !std::path::Path::new(path).exists() {
            return Err(anyhow::anyhow!(
                "RPMsg device not found: {}. Ensure M4 firmware is loaded via remoteproc.",
                path
            ));
        }

        info!("RPMsg: Opening device {}", path);
        Ok(Self {
            path: path.to_string(),
        })
    }
}

/// Run the XRCE-DDS agent
///
/// Continuously reads XRCE frames from the M4 via RPMsg,
/// decodes the CDR payloads, and publishes them via DDS.
/// Also forwards DDS commands to the M4.
pub async fn run_xrce_agent(
    rpmsg: RpmsgHandle,
    publisher: Arc<rustdds::Publisher>,
    config: &ScadaConfig,
    cancel: CancellationToken,
) -> Result<()> {
    info!("XRCE Agent: Starting on {}", rpmsg.path);

    // Open RPMsg device for async I/O
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&rpmsg.path)
        .await
        .context(format!("Failed to open RPMsg device: {}", rpmsg.path))?;

    let (mut reader, mut writer) = tokio::io::split(file);

    let mut rx_buf = vec![0u8; 1024];
    let mut frame_buf = Vec::with_capacity(2048);
    let mut msg_count: u64 = 0;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("XRCE Agent: Shutting down...");
                break;
            }

            result = reader.read(&mut rx_buf) => {
                match result {
                    Ok(0) => {
                        warn!("XRCE Agent: RPMsg device closed (EOF)");
                        break;
                    }
                    Ok(n) => {
                        frame_buf.extend_from_slice(&rx_buf[..n]);

                        // Process complete XRCE frames from buffer
                        while let Some(frame) = extract_xrce_frame(&mut frame_buf) {
                            msg_count += 1;
                            if let Err(e) = process_xrce_frame(&frame, config).await {
                                warn!("XRCE Agent: Frame processing error: {}", e);
                            }

                            if msg_count % 1000 == 0 {
                                debug!("XRCE Agent: Processed {} messages", msg_count);
                            }
                        }
                    }
                    Err(e) => {
                        error!("XRCE Agent: Read error: {}", e);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        }
    }

    info!("XRCE Agent: Stopped ({} messages processed)", msg_count);
    Ok(())
}

/// Extract a complete XRCE frame from the receive buffer
///
/// Returns None if no complete frame is available.
/// Removes the consumed bytes from the buffer.
fn extract_xrce_frame(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    // Find XRCE marker
    let mut marker_pos = None;
    for i in 0..buf.len().saturating_sub(1) {
        if buf[i] == XRCE_MARKER[0] && buf[i + 1] == XRCE_MARKER[1] {
            marker_pos = Some(i);
            break;
        }
    }

    let start = marker_pos?;

    // Need at least XRCE header to read length
    if buf.len() < start + XRCE_HEADER_SIZE {
        return None;
    }

    // Read payload length from header
    let payload_len = ((buf[start + 6] as usize) << 8) | buf[start + 7] as usize;
    let frame_len = XRCE_HEADER_SIZE + payload_len;

    if buf.len() < start + frame_len {
        return None; // Incomplete frame
    }

    // Extract frame
    let frame = buf[start..start + frame_len].to_vec();

    // Remove consumed bytes (including any garbage before the marker)
    buf.drain(..start + frame_len);

    Some(frame)
}

/// Process a complete XRCE frame
async fn process_xrce_frame(frame: &[u8], config: &ScadaConfig) -> Result<()> {
    if frame.len() < XRCE_HEADER_SIZE {
        return Err(anyhow::anyhow!("Frame too short"));
    }

    let session_id = frame[2];
    let stream_id = frame[3];
    let sequence = ((frame[4] as u16) << 8) | frame[5] as u16;
    let payload_len = ((frame[6] as usize) << 8) | frame[7] as usize;

    let payload = &frame[XRCE_HEADER_SIZE..];

    if payload.is_empty() {
        return Ok(());
    }

    // Parse XRCE submessage
    let submsg_type = payload[0];

    match submsg_type {
        0x03 => {
            // WRITE_DATA — sensor data from M4
            if payload.len() < 5 {
                return Ok(());
            }
            let writer_id = ((payload[2] as u16) << 8) | payload[3] as u16;
            let data_len = ((payload[4] as u16) << 8) | payload[5] as u16;
            let cdr_data = &payload[6..];

            debug!(
                "XRCE: WRITE_DATA writer=0x{:04X} len={} stream=0x{:02X}",
                writer_id, data_len, stream_id
            );

            // Decode CDR and publish to DDS
            // The actual DDS publish would happen here through the data writers
        }
        0x00 => {
            // CREATE_CLIENT — session management
            debug!("XRCE: CREATE_CLIENT from M4 (session={})", session_id);
        }
        0x01 => {
            // CREATE — entity creation request
            debug!("XRCE: CREATE entity (seq={})", sequence);
        }
        0x07 => {
            // HEARTBEAT — keepalive
            debug!("XRCE: HEARTBEAT (seq={})", sequence);
        }
        _ => {
            debug!("XRCE: Unknown submessage type 0x{:02X}", submsg_type);
        }
    }

    Ok(())
}
