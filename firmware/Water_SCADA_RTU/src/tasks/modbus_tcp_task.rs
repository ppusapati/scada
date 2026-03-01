/// Modbus TCP Server Task — Normal Priority
///
/// Listens on TCP port 502 via W5500 Ethernet controller.
/// Handles Modbus TCP (MBAP) framing with transaction ID tracking.
/// Uses W5500 Socket 0.

use embassy_executor::task;
use embassy_stm32::gpio::Output;
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::{info, warn, debug};

use crate::drivers::w5500::{W5500, NetConfig};
use crate::modbus::protocol::ModbusTcp;
use crate::modbus::registers::device_status;
use crate::tasks::shared::SharedState;

const MODBUS_TCP_PORT: u16 = 502;
const SOCKET_ID: u8 = 0;
const MAX_FRAME_SIZE: usize = 260; // MBAP(7) + PDU(253)

#[task]
pub async fn modbus_tcp_server(
    spi: Spi<'static, Async>,
    cs: Output<'static>,
    rst: Output<'static>,
    shared: &'static SharedState,
) {
    info!("Modbus TCP task: starting");

    let mut w5500 = W5500::new(spi, cs, rst);

    // Get IP config from holding registers (or use defaults)
    let net_config = {
        let regs = shared.registers.lock().await;
        let ip_base = crate::modbus::registers::holding_reg::IP_ADDR_BASE as usize;
        NetConfig {
            ip: [
                regs.holding_regs[ip_base] as u8,
                regs.holding_regs[ip_base + 1] as u8,
                regs.holding_regs[ip_base + 2] as u8,
                regs.holding_regs[ip_base + 3] as u8,
            ],
            ..NetConfig::default()
        }
    };

    // Initialize W5500
    if !w5500.init(&net_config).await {
        error!("Modbus TCP: W5500 init FAILED");
        // Retry loop
        loop {
            Timer::after(Duration::from_secs(5)).await;
            if w5500.init(&net_config).await {
                info!("Modbus TCP: W5500 recovered");
                break;
            }
        }
    }

    // Wait for Ethernet link
    let mut link_wait = 0u32;
    while !w5500.link_up().await {
        Timer::after(Duration::from_millis(500)).await;
        link_wait += 1;
        if link_wait % 10 == 0 {
            warn!("Modbus TCP: waiting for Ethernet link...");
        }
    }

    // Set Ethernet link status
    {
        let mut status = shared.device_status.lock().await;
        *status |= device_status::ETH_LINK;
    }

    info!("Modbus TCP: Ethernet link UP");

    let modbus = ModbusTcp::new(1); // Unit ID = 1
    let mut rx_buf = [0u8; MAX_FRAME_SIZE];
    let mut tx_buf = [0u8; MAX_FRAME_SIZE];
    let mut msg_count: u32 = 0;

    loop {
        // Open TCP server socket on port 502
        if !w5500.socket_listen(SOCKET_ID, MODBUS_TCP_PORT).await {
            warn!("Modbus TCP: failed to open socket, retrying...");
            Timer::after(Duration::from_secs(1)).await;
            continue;
        }

        // Connection handling loop
        loop {
            let status = w5500.socket_status(SOCKET_ID).await;

            match status {
                0x17 => {
                    // ESTABLISHED — check for incoming data
                    let available = w5500.socket_rx_available(SOCKET_ID).await;
                    if available > 0 {
                        let rx_len = w5500.socket_recv(SOCKET_ID, &mut rx_buf).await;
                        if rx_len > 0 {
                            debug!("Modbus TCP: received {} bytes", rx_len);

                            let tx_len = {
                                let mut regs = shared.registers.lock().await;
                                modbus.process_frame(&rx_buf, rx_len, &mut tx_buf, &mut regs)
                            };

                            if tx_len > 0 {
                                msg_count += 1;
                                if !w5500.socket_send(SOCKET_ID, &tx_buf[..tx_len]).await {
                                    warn!("Modbus TCP: send failed");
                                }
                            }

                            if msg_count % 100 == 0 {
                                info!("Modbus TCP: {} messages processed", msg_count);
                            }
                        }
                    } else {
                        // No data, yield to other tasks
                        Timer::after(Duration::from_millis(1)).await;
                    }
                }
                0x1C => {
                    // CLOSE_WAIT — client disconnected
                    info!("Modbus TCP: client disconnected");
                    w5500.socket_disconnect(SOCKET_ID).await;
                    Timer::after(Duration::from_millis(10)).await;
                    break; // Re-listen
                }
                0x00 => {
                    // CLOSED — re-listen
                    break;
                }
                _ => {
                    // LISTEN or transitional state
                    Timer::after(Duration::from_millis(10)).await;
                }
            }

            // Periodically check Ethernet link
            if !w5500.link_up().await {
                warn!("Modbus TCP: Ethernet link DOWN");
                let mut status = shared.device_status.lock().await;
                *status &= !device_status::ETH_LINK;
                // Wait for link recovery
                while !w5500.link_up().await {
                    Timer::after(Duration::from_secs(1)).await;
                }
                let mut status = shared.device_status.lock().await;
                *status |= device_status::ETH_LINK;
                info!("Modbus TCP: Ethernet link restored");
                break; // Re-listen
            }
        }
    }
}
