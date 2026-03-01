/// SP3485 — RS-485 Transceiver Driver
///
/// The SP3485 is a 3.3V RS-485 transceiver with:
///   - Half-duplex differential bus
///   - DE/RE combined direction control pin
///   - Up to 10 Mbps data rate
///
/// On WS-PCB-001:
///   USART2_TX (PA2) → SP3485 DI
///   USART2_RX (PA3) → SP3485 RO
///   PA8              → SP3485 DE/RE (high=transmit, low=receive)
///
/// Default Modbus RTU settings: 9600 baud, 8N1, slave address 1

use embassy_stm32::gpio::Output;
use embassy_stm32::usart::Uart;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::warn;

/// RS-485 half-duplex transceiver
pub struct Rs485<'a> {
    uart: Uart<'a, Async>,
    de_pin: Output<'a>,
}

impl<'a> Rs485<'a> {
    pub fn new(uart: Uart<'a, Async>, de_pin: Output<'a>) -> Self {
        // Start in receive mode
        let mut s = Self { uart, de_pin };
        s.set_receive_mode();
        s
    }

    /// Switch to transmit mode (DE=high)
    fn set_transmit_mode(&mut self) {
        self.de_pin.set_high();
    }

    /// Switch to receive mode (DE=low)
    fn set_receive_mode(&mut self) {
        self.de_pin.set_low();
    }

    /// Send data on RS-485 bus
    /// Automatically handles DE/RE direction switching
    pub async fn send(&mut self, data: &[u8]) -> bool {
        self.set_transmit_mode();

        // Small delay for transceiver to switch (typ 0.1µs, we wait 1µs to be safe)
        Timer::after(Duration::from_micros(1)).await;

        let result = self.uart.write(data).await;

        // Wait for UART TX to complete before switching back to RX
        // At 9600 baud, 1 byte ≈ 1ms. We flush by waiting.
        Timer::after(Duration::from_micros(100)).await;

        self.set_receive_mode();

        result.is_ok()
    }

    /// Receive data from RS-485 bus with timeout
    /// Returns number of bytes read
    pub async fn recv(&mut self, buf: &mut [u8], timeout_ms: u32) -> usize {
        self.set_receive_mode();

        let mut total = 0usize;

        // Read bytes with inter-character timeout
        // Modbus RTU uses 3.5 character times as frame delimiter
        // At 9600 baud: 3.5 chars × (11 bits/char) / 9600 = ~4ms
        let inter_char_timeout = if timeout_ms > 0 {
            Duration::from_millis(4)
        } else {
            Duration::from_millis(timeout_ms as u64)
        };

        // Wait for first byte with full timeout
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

        // Read subsequent bytes with inter-character timeout
        while total < buf.len() {
            match embassy_time::with_timeout(
                inter_char_timeout,
                self.uart.read(&mut single),
            ).await {
                Ok(Ok(())) => {
                    buf[total] = single[0];
                    total += 1;
                }
                _ => break, // Timeout = end of frame
            }
        }

        total
    }
}
