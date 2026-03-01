/// Modbus RTU Slave Task — Solar SCADA SMU
///
/// Listens on RS-485 bus (SP3485 transceiver).
/// Default slave address: 2 (different from Water RTU which is 1).
/// Baud: 9600, 8N1.

use embassy_executor::task;
use embassy_stm32::gpio::Output;
use embassy_stm32::usart::Uart;
use embassy_stm32::mode::Async;
use embassy_time::Duration;
use defmt::{info, warn, debug};

use crate::modbus::protocol::ModbusRtu;
use crate::modbus::registers::{device_status, holding_reg};
use crate::tasks::shared::SharedState;

const MAX_FRAME: usize = 256;

/// RS-485 half-duplex (same as Water RTU driver, inlined here)
struct Rs485<'a> {
    uart: Uart<'a, Async>,
    de: Output<'a>,
}

impl<'a> Rs485<'a> {
    fn new(uart: Uart<'a, Async>, de: Output<'a>) -> Self {
        let mut s = Self { uart, de };
        s.de.set_low(); // Start in receive mode
        s
    }

    async fn send(&mut self, data: &[u8]) -> bool {
        self.de.set_high();
        embassy_time::Timer::after(Duration::from_micros(1)).await;
        let result = self.uart.write(data).await;
        embassy_time::Timer::after(Duration::from_micros(100)).await;
        self.de.set_low();
        result.is_ok()
    }

    async fn recv(&mut self, buf: &mut [u8], timeout_ms: u32) -> usize {
        self.de.set_low();
        let mut total = 0usize;
        let mut single = [0u8; 1];

        match embassy_time::with_timeout(
            Duration::from_millis(timeout_ms as u64),
            self.uart.read(&mut single),
        ).await {
            Ok(Ok(())) => {
                buf[0] = single[0];
                total = 1;
            }
            _ => return 0,
        }

        let inter_char = Duration::from_millis(4);
        while total < buf.len() {
            match embassy_time::with_timeout(inter_char, self.uart.read(&mut single)).await {
                Ok(Ok(())) => {
                    buf[total] = single[0];
                    total += 1;
                }
                _ => break,
            }
        }

        total
    }
}

#[task]
pub async fn modbus_slave(
    uart: Uart<'static, Async>,
    de_pin: Output<'static>,
    shared: &'static SharedState,
) {
    info!("Modbus task (Solar): starting");

    let mut rs485 = Rs485::new(uart, de_pin);

    let slave_addr = {
        let regs = shared.registers.lock().await;
        let addr = regs.holding_regs[holding_reg::SLAVE_ADDR as usize] as u8;
        if addr == 0 { 2 } else { addr }
    };

    let modbus = ModbusRtu::new(slave_addr);

    {
        let mut status = shared.device_status.lock().await;
        *status |= device_status::RS485_OK;
    }

    info!("Modbus RTU (Solar): slave addr={}", slave_addr);

    let mut rx = [0u8; MAX_FRAME];
    let mut tx = [0u8; MAX_FRAME];

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
            embassy_time::Timer::after(Duration::from_millis(4)).await;
            rs485.send(&tx[..tx_len]).await;
        }
    }
}
