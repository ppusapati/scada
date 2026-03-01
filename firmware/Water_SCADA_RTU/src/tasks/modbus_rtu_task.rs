/// Modbus RTU Slave Task — High Priority
///
/// Listens on RS-485 bus via SP3485 transceiver.
/// Responds to Modbus RTU requests from SCADA master.
/// Default: slave address 1, 9600 baud, 8N1.

use embassy_executor::task;
use embassy_stm32::gpio::Output;
use embassy_stm32::usart::Uart;
use embassy_stm32::mode::Async;
use embassy_time::Duration;
use defmt::{info, warn, debug};

use crate::drivers::sp3485::Rs485;
use crate::modbus::protocol::ModbusRtu;
use crate::modbus::registers::device_status;
use crate::tasks::shared::SharedState;

/// Max Modbus RTU frame size (256 bytes per spec)
const MAX_FRAME_SIZE: usize = 256;

#[task]
pub async fn modbus_rtu_slave(
    uart: Uart<'static, Async>,
    de_pin: Output<'static>,
    shared: &'static SharedState,
) {
    info!("Modbus RTU task: starting");

    let mut rs485 = Rs485::new(uart, de_pin);

    // Get slave address from holding registers
    let slave_addr = {
        let regs = shared.registers.lock().await;
        regs.holding_regs[crate::modbus::registers::holding_reg::SLAVE_ADDR as usize] as u8
    };
    let slave_addr = if slave_addr == 0 { 1 } else { slave_addr };

    let modbus = ModbusRtu::new(slave_addr);

    // Set RS-485 OK in device status
    {
        let mut status = shared.device_status.lock().await;
        *status |= device_status::RS485_OK;
    }

    info!("Modbus RTU: slave addr={}, listening on RS-485", slave_addr);

    let mut rx_buf = [0u8; MAX_FRAME_SIZE];
    let mut tx_buf = [0u8; MAX_FRAME_SIZE];
    let mut msg_count: u32 = 0;

    loop {
        // Wait for incoming frame (with timeout for bus idle detection)
        let rx_len = rs485.recv(&mut rx_buf, 1000).await;

        if rx_len == 0 {
            continue; // Timeout, no data
        }

        debug!("Modbus RTU: received {} bytes", rx_len);

        // Process frame through Modbus protocol engine
        let tx_len = {
            let mut regs = shared.registers.lock().await;
            modbus.process_frame(&rx_buf, rx_len, &mut tx_buf, &mut regs)
        };

        if tx_len > 0 {
            msg_count += 1;

            // Modbus spec: 3.5 character delay before response
            // At 9600 baud: 3.5 × 11 bits / 9600 ≈ 4ms
            embassy_time::Timer::after(Duration::from_millis(4)).await;

            if !rs485.send(&tx_buf[..tx_len]).await {
                warn!("Modbus RTU: TX failed");
            }

            if msg_count % 100 == 0 {
                info!("Modbus RTU: {} messages processed", msg_count);
            }
        }
    }
}
