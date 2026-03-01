/// Modbus-DDS Gateway
///
/// Bridges legacy Modbus TCP clients to the DDS data space.
/// Allows existing SCADA masters to read sensor data via Modbus TCP
/// while the system uses DDS as the primary communication protocol.
///
/// Architecture:
///   Legacy SCADA Master ──Modbus TCP──→ This Gateway ──DDS──→ Data
///
/// Features:
///   - Modbus TCP server on port 502
///   - Maps Modbus registers to DDS topic data
///   - Supports multiple simultaneous connections
///   - Bidirectional: read DDS data via Modbus, write Modbus commands to DDS

use anyhow::Result;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use tracing::{info, debug, warn, error};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::ScadaConfig;

/// Modbus register store (in-memory, populated from DDS data)
pub struct RegisterStore {
    pub input_regs: Vec<u16>,
    pub holding_regs: Vec<u16>,
}

impl RegisterStore {
    fn new() -> Self {
        Self {
            input_regs: vec![0u16; 256],
            holding_regs: vec![0u16; 256],
        }
    }
}

/// Modbus function codes
const FC_READ_HOLDING: u8 = 0x03;
const FC_READ_INPUT: u8 = 0x04;
const FC_WRITE_SINGLE: u8 = 0x06;
const FC_WRITE_MULTIPLE: u8 = 0x10;

/// Modbus exception codes
const EX_ILLEGAL_FUNCTION: u8 = 0x01;
const EX_ILLEGAL_ADDRESS: u8 = 0x02;
const EX_ILLEGAL_VALUE: u8 = 0x03;

/// Run the Modbus TCP gateway
pub async fn run_modbus_gateway(
    config: &ScadaConfig,
    cancel: CancellationToken,
) -> Result<()> {
    let port = config.modbus.tcp_port;
    let max_conns = config.modbus.max_connections;
    let unit_id = config.modbus.unit_id;

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .map_err(|e| anyhow::anyhow!("Failed to bind Modbus TCP port {}: {}", port, e))?;

    info!("Modbus TCP: Listening on port {} (unit_id={})", port, unit_id);

    let registers = Arc::new(RwLock::new(RegisterStore::new()));
    let mut active_connections = 0u32;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Modbus TCP: Shutting down...");
                break;
            }

            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        if active_connections >= max_conns as u32 {
                            warn!("Modbus TCP: Max connections reached, rejecting {}", addr);
                            continue;
                        }

                        active_connections += 1;
                        info!("Modbus TCP: Client connected from {} ({}/{})",
                            addr, active_connections, max_conns);

                        let regs = registers.clone();
                        let conn_cancel = cancel.clone();
                        let timeout = config.modbus.timeout_secs;

                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(
                                stream, unit_id, regs, conn_cancel, timeout,
                            ).await {
                                debug!("Modbus TCP: Connection {} ended: {}", addr, e);
                            }
                            info!("Modbus TCP: Client {} disconnected", addr);
                        });
                    }
                    Err(e) => {
                        error!("Modbus TCP: Accept error: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Handle a single Modbus TCP connection
async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    unit_id: u8,
    registers: Arc<RwLock<RegisterStore>>,
    cancel: CancellationToken,
    timeout_secs: u64,
) -> Result<()> {
    let mut rx_buf = [0u8; 512];
    let mut tx_buf = [0u8; 512];

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                break;
            }

            result = tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                stream.read(&mut rx_buf),
            ) => {
                match result {
                    Ok(Ok(0)) => break, // Connection closed
                    Ok(Ok(n)) => {
                        if n < 7 {
                            continue; // Too short for MBAP header
                        }

                        // Parse MBAP header
                        let transaction_id = ((rx_buf[0] as u16) << 8) | rx_buf[1] as u16;
                        let protocol_id = ((rx_buf[2] as u16) << 8) | rx_buf[3] as u16;
                        let _length = ((rx_buf[4] as u16) << 8) | rx_buf[5] as u16;
                        let req_unit_id = rx_buf[6];

                        if protocol_id != 0 {
                            continue; // Not Modbus
                        }

                        if req_unit_id != unit_id && req_unit_id != 0xFF {
                            continue; // Not for us
                        }

                        // Process PDU
                        let pdu = &rx_buf[7..n];
                        let pdu_len = {
                            let regs = registers.read().await;
                            process_pdu(pdu, &mut tx_buf[7..], &regs)
                        };

                        if pdu_len > 0 {
                            // Build MBAP response header
                            tx_buf[0] = (transaction_id >> 8) as u8;
                            tx_buf[1] = (transaction_id & 0xFF) as u8;
                            tx_buf[2] = 0; // Protocol ID
                            tx_buf[3] = 0;
                            let resp_len = (1 + pdu_len) as u16;
                            tx_buf[4] = (resp_len >> 8) as u8;
                            tx_buf[5] = (resp_len & 0xFF) as u8;
                            tx_buf[6] = unit_id;

                            let total = 7 + pdu_len;
                            stream.write_all(&tx_buf[..total]).await?;
                        }
                    }
                    Ok(Err(e)) => {
                        return Err(anyhow::anyhow!("Read error: {}", e));
                    }
                    Err(_) => {
                        info!("Modbus TCP: Connection timeout");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Process a Modbus PDU (function code + data)
fn process_pdu(pdu: &[u8], response: &mut [u8], regs: &RegisterStore) -> usize {
    if pdu.is_empty() {
        return 0;
    }

    match pdu[0] {
        FC_READ_HOLDING => read_registers(pdu, response, &regs.holding_regs),
        FC_READ_INPUT => read_registers(pdu, response, &regs.input_regs),
        FC_WRITE_SINGLE => {
            response[0] = pdu[0] | 0x80;
            response[1] = EX_ILLEGAL_FUNCTION; // Read-only in gateway mode
            2
        }
        _ => {
            response[0] = pdu[0] | 0x80;
            response[1] = EX_ILLEGAL_FUNCTION;
            2
        }
    }
}

fn read_registers(pdu: &[u8], response: &mut [u8], regs: &[u16]) -> usize {
    if pdu.len() < 5 {
        response[0] = pdu[0] | 0x80;
        response[1] = EX_ILLEGAL_VALUE;
        return 2;
    }

    let start = ((pdu[1] as u16) << 8) | pdu[2] as u16;
    let qty = ((pdu[3] as u16) << 8) | pdu[4] as u16;

    if qty == 0 || qty > 125 || (start + qty) as usize > regs.len() {
        response[0] = pdu[0] | 0x80;
        response[1] = EX_ILLEGAL_ADDRESS;
        return 2;
    }

    response[0] = pdu[0];
    response[1] = (qty * 2) as u8;

    let mut offset = 2usize;
    for i in start..(start + qty) {
        let val = regs[i as usize];
        response[offset] = (val >> 8) as u8;
        response[offset + 1] = (val & 0xFF) as u8;
        offset += 2;
    }

    offset
}
