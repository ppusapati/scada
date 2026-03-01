/// Embassy Task Modules for STM32MP157 M4 Real-Time Firmware
///
/// Task priorities (Embassy cooperative scheduling):
///   1. sensor_acquisition — ADC + MUX scan, highest urgency (< 500 us budget)
///   2. dds_publish        — CDR serialize + RPMsg to A7 XRCE agent
///   3. modbus_slave       — Legacy RS-485 Modbus RTU slave
///   4. diagnostics        — LEDs, temperature, health monitoring
///
/// All tasks share state through the SharedState structure protected
/// by Embassy's CriticalSection mutexes and signals.

pub mod shared;
pub mod sensor_acquisition;
pub mod dds_publish;
pub mod diagnostics;

/// Re-export the modbus task under a name that avoids conflict with the modbus module
pub mod mod_task {
    use embassy_executor::task;
    use embassy_stm32::gpio::Output;
    use embassy_stm32::usart::Uart;
    use embassy_stm32::mode::Async;
    use embassy_time::Duration;
    use defmt::{info, warn, debug};

    use crate::modbus::{ModbusRtu, Rs485, holding_reg, device_status};
    use crate::tasks::shared::SharedState;

    const MAX_FRAME: usize = 256;

    #[task]
    pub async fn modbus_slave(
        uart: Uart<'static, Async>,
        de_pin: Output<'static>,
        shared: &'static SharedState,
    ) {
        info!("Modbus RTU task: starting");

        let mut rs485 = Rs485::new(uart, de_pin);

        let slave_addr = {
            let regs = shared.registers.lock().await;
            let addr = regs.holding_regs[holding_reg::SLAVE_ADDR as usize] as u8;
            if addr == 0 { 1 } else { addr }
        };

        let modbus = ModbusRtu::new(slave_addr);

        {
            let mut status = shared.device_status.lock().await;
            *status |= device_status::RS485_OK;
        }

        info!("Modbus RTU: slave addr={}, listening on RS-485", slave_addr);

        let mut rx = [0u8; MAX_FRAME];
        let mut tx = [0u8; MAX_FRAME];
        let mut msg_count: u32 = 0;

        loop {
            let rx_len = rs485.recv(&mut rx, 1000).await;
            if rx_len == 0 {
                continue;
            }

            let tx_len = {
                let mut regs = shared.registers.lock().await;
                modbus.process_frame(&rx, rx_len, &mut tx, &mut regs)
            };

            if tx_len > 0 {
                msg_count += 1;
                // Modbus spec: 3.5 char delay before response
                embassy_time::Timer::after(Duration::from_millis(4)).await;
                rs485.send(&tx[..tx_len]).await;

                if msg_count % 100 == 0 {
                    info!("Modbus RTU: {} messages processed", msg_count);
                }
            }
        }
    }
}
