/// SP3485 RS-485 Transceiver Driver + Modbus RTU Frame Handling
///
/// The SP3485 is a 3.3V RS-485 transceiver from MaxLinear/Sipex.
/// It connects to USART2 (PA2=TX, PA3=RX) with direction control
/// via GPIO (PB0=DE, PB1=/RE).
///
/// RS-485 is half-duplex: the driver must switch between TX and RX mode.
/// Modbus RTU uses inter-character timeouts (1.5 char times) for frame
/// delimiting and inter-frame silence (3.5 char times) for message boundaries.
///
/// Baud rates supported: 9600, 19200, 38400, 57600, 115200
///
/// At 9600 baud, 8N1:
///   1 character = 11 bits (start + 8 data + parity/none + stop)
///   Character time = 11/9600 = 1.146 ms
///   1.5 char timeout = 1.719 ms
///   3.5 char timeout = 4.010 ms
///
/// At 115200 baud:
///   Character time = 11/115200 = 95.5 us
///   1.5 char timeout = 143 us (but minimum of 750 us per Modbus spec)
///   3.5 char timeout = 334 us (but minimum of 1750 us per Modbus spec)

use embassy_stm32::usart::{self, Uart, Config as UartConfig};
use embassy_stm32::peripherals;
use embassy_stm32::mode::Async;
use embassy_time::{Timer, Duration, Instant};
use crate::hal::gpio::Rs485Control;
use defmt::{info, warn, error, Format};

// ============================================================================
// Error types
// ============================================================================

#[derive(Clone, Copy, Debug, Format)]
pub enum Rs485Error {
    /// USART transmit error
    TxError,
    /// USART receive error (framing, overrun, noise)
    RxError,
    /// Frame timeout (no complete frame received)
    Timeout,
    /// Frame too short (less than 4 bytes: addr + func + 2 CRC)
    FrameTooShort,
    /// Frame too long (exceeds Modbus RTU max of 256 bytes)
    FrameTooLong,
    /// CRC mismatch
    CrcError,
    /// Buffer overflow
    BufferOverflow,
    /// Invalid baud rate
    InvalidBaud,
}

// ============================================================================
// Configuration
// ============================================================================

/// RS-485 baud rate
#[derive(Clone, Copy, PartialEq, Format)]
pub enum BaudRate {
    Baud9600,
    Baud19200,
    Baud38400,
    Baud57600,
    Baud115200,
}

impl BaudRate {
    /// Get the numeric baud rate value
    pub fn value(self) -> u32 {
        match self {
            BaudRate::Baud9600 => 9600,
            BaudRate::Baud19200 => 19200,
            BaudRate::Baud38400 => 38400,
            BaudRate::Baud57600 => 57600,
            BaudRate::Baud115200 => 115200,
        }
    }

    /// Calculate 1.5 character timeout in microseconds
    /// Per Modbus spec: at 19200+ baud, fixed at 750 us minimum
    pub fn char_timeout_1_5_us(self) -> u64 {
        let char_time_us = 11_000_000u64 / self.value() as u64; // 11 bits per char
        let t = char_time_us * 3 / 2; // 1.5 character times
        if self.value() > 19200 {
            t.max(750)
        } else {
            t
        }
    }

    /// Calculate 3.5 character timeout in microseconds
    /// Per Modbus spec: at 19200+ baud, fixed at 1750 us minimum
    pub fn char_timeout_3_5_us(self) -> u64 {
        let char_time_us = 11_000_000u64 / self.value() as u64;
        let t = char_time_us * 7 / 2; // 3.5 character times
        if self.value() > 19200 {
            t.max(1750)
        } else {
            t
        }
    }
}

/// Modbus RTU parity setting
#[derive(Clone, Copy, PartialEq, Format)]
pub enum Parity {
    None,
    Even,
    Odd,
}

/// SP3485 driver configuration
pub struct Sp3485Config {
    pub baud_rate: BaudRate,
    pub parity: Parity,
    /// Modbus slave address (1-247)
    pub slave_address: u8,
    /// Turnaround delay in microseconds (DE assert to first TX byte)
    pub turnaround_us: u32,
}

impl Default for Sp3485Config {
    fn default() -> Self {
        Self {
            baud_rate: BaudRate::Baud9600,
            parity: Parity::None,
            slave_address: 1,
            turnaround_us: 100,
        }
    }
}

// ============================================================================
// Modbus RTU Frame
// ============================================================================

/// Maximum Modbus RTU frame size (per specification: 256 bytes)
pub const MODBUS_MAX_FRAME: usize = 256;

/// Minimum Modbus RTU frame size (address + function + 2 CRC bytes)
pub const MODBUS_MIN_FRAME: usize = 4;

/// Modbus RTU frame buffer
pub struct ModbusFrame {
    pub buf: [u8; MODBUS_MAX_FRAME],
    pub len: usize,
}

impl ModbusFrame {
    pub const fn new() -> Self {
        Self {
            buf: [0u8; MODBUS_MAX_FRAME],
            len: 0,
        }
    }

    /// Get the slave address (first byte)
    pub fn address(&self) -> u8 {
        if self.len > 0 { self.buf[0] } else { 0 }
    }

    /// Get the function code (second byte)
    pub fn function(&self) -> u8 {
        if self.len > 1 { self.buf[1] } else { 0 }
    }

    /// Get the data portion (between header and CRC)
    pub fn data(&self) -> &[u8] {
        if self.len > 4 {
            &self.buf[2..self.len - 2]
        } else {
            &[]
        }
    }

    /// Verify CRC-16
    pub fn verify_crc(&self) -> bool {
        if self.len < MODBUS_MIN_FRAME {
            return false;
        }
        let computed = crc16_modbus(&self.buf[..self.len - 2]);
        let received = u16::from_le_bytes([
            self.buf[self.len - 2],
            self.buf[self.len - 1],
        ]);
        computed == received
    }

    /// Build a response frame: address + function + data + CRC
    pub fn build_response(&mut self, address: u8, function: u8, data: &[u8]) {
        self.buf[0] = address;
        self.buf[1] = function;
        let data_len = data.len().min(MODBUS_MAX_FRAME - 4);
        self.buf[2..2 + data_len].copy_from_slice(&data[..data_len]);
        self.len = 2 + data_len;

        // Append CRC
        let crc = crc16_modbus(&self.buf[..self.len]);
        self.buf[self.len] = (crc & 0xFF) as u8;
        self.buf[self.len + 1] = (crc >> 8) as u8;
        self.len += 2;
    }

    /// Build an exception response
    pub fn build_exception(&mut self, address: u8, function: u8, exception_code: u8) {
        self.buf[0] = address;
        self.buf[1] = function | 0x80; // Set error bit
        self.buf[2] = exception_code;
        self.len = 3;

        let crc = crc16_modbus(&self.buf[..self.len]);
        self.buf[self.len] = (crc & 0xFF) as u8;
        self.buf[self.len + 1] = (crc >> 8) as u8;
        self.len += 2;
    }

    /// Reset the frame buffer
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

// ============================================================================
// CRC-16 (Modbus RTU)
// ============================================================================

/// Compute CRC-16/MODBUS
///
/// Polynomial: 0xA001 (bit-reversed 0x8005)
/// Initial value: 0xFFFF
/// No final XOR
pub fn crc16_modbus(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

// ============================================================================
// SP3485 Driver
// ============================================================================

/// SP3485 RS-485 transceiver driver
///
/// Manages USART2 and the DE/RE direction pins for half-duplex RS-485
/// communication. Implements Modbus RTU frame reception with proper
/// inter-character timeout detection.
pub struct Sp3485 {
    config: Sp3485Config,
    /// Character timeout tracking
    char_timeout_1_5_us: u64,
    char_timeout_3_5_us: u64,
}

impl Sp3485 {
    /// Create a new SP3485 driver
    pub fn new(config: Sp3485Config) -> Self {
        let char_timeout_1_5_us = config.baud_rate.char_timeout_1_5_us();
        let char_timeout_3_5_us = config.baud_rate.char_timeout_3_5_us();

        Self {
            config,
            char_timeout_1_5_us,
            char_timeout_3_5_us,
        }
    }

    /// Create USART configuration matching the SP3485 settings
    pub fn usart_config(&self) -> UartConfig {
        let mut config = UartConfig::default();
        config.baudrate = self.config.baud_rate.value();
        config.parity = match self.config.parity {
            Parity::None => usart::Parity::ParityNone,
            Parity::Even => usart::Parity::ParityEven,
            Parity::Odd => usart::Parity::ParityOdd,
        };
        config.stop_bits = usart::StopBits::STOP1;
        config.data_bits = usart::DataBits::DataBits8;
        config
    }

    /// Receive a Modbus RTU frame
    ///
    /// Reads bytes from USART until an inter-character silence of 3.5
    /// character times is detected, indicating end of frame.
    ///
    /// The RS-485 direction control must already be in Receive mode.
    pub async fn receive_frame(
        &self,
        uart: &mut Uart<'_, Async>,
        frame: &mut ModbusFrame,
        timeout_ms: u32,
    ) -> Result<(), Rs485Error> {
        frame.clear();
        let overall_start = Instant::now();
        let overall_timeout_us = timeout_ms as u64 * 1000;

        // Wait for first byte
        loop {
            let mut byte = [0u8; 1];
            match embassy_futures::select::select(
                uart.read(&mut byte),
                Timer::after(Duration::from_millis(timeout_ms as u64)),
            ).await {
                embassy_futures::select::Either::First(result) => {
                    match result {
                        Ok(_) => {
                            frame.buf[0] = byte[0];
                            frame.len = 1;
                            break;
                        }
                        Err(_) => return Err(Rs485Error::RxError),
                    }
                }
                embassy_futures::select::Either::Second(_) => {
                    return Err(Rs485Error::Timeout);
                }
            }
        }

        // Continue reading bytes until 3.5-character timeout between bytes
        loop {
            let mut byte = [0u8; 1];
            let timeout = Duration::from_micros(self.char_timeout_3_5_us);

            match embassy_futures::select::select(
                uart.read(&mut byte),
                Timer::after(timeout),
            ).await {
                embassy_futures::select::Either::First(result) => {
                    match result {
                        Ok(_) => {
                            if frame.len >= MODBUS_MAX_FRAME {
                                return Err(Rs485Error::FrameTooLong);
                            }
                            frame.buf[frame.len] = byte[0];
                            frame.len += 1;
                        }
                        Err(_) => return Err(Rs485Error::RxError),
                    }
                }
                embassy_futures::select::Either::Second(_) => {
                    // Timeout between characters = end of frame
                    break;
                }
            }

            // Check overall timeout
            if Instant::now().as_micros() - overall_start.as_micros() > overall_timeout_us {
                break;
            }
        }

        // Validate minimum frame length
        if frame.len < MODBUS_MIN_FRAME {
            return Err(Rs485Error::FrameTooShort);
        }

        Ok(())
    }

    /// Send a Modbus RTU frame
    ///
    /// Switches to TX mode, sends the frame, then switches back to RX mode.
    /// Includes proper turnaround delay before and after transmission.
    pub async fn send_frame(
        &self,
        uart: &mut Uart<'_, Async>,
        rs485: &mut Rs485Control,
        frame: &ModbusFrame,
    ) -> Result<(), Rs485Error> {
        if frame.len < MODBUS_MIN_FRAME {
            return Err(Rs485Error::FrameTooShort);
        }

        // Switch to TX mode
        rs485.enable_tx();

        // Turnaround delay (SP3485 DE->TX propagation)
        Timer::after_micros(self.config.turnaround_us as u64).await;

        // Send the frame
        uart.write(&frame.buf[..frame.len])
            .await
            .map_err(|_| Rs485Error::TxError)?;

        // Wait for transmission to complete (flush USART FIFO)
        uart.blocking_flush().map_err(|_| Rs485Error::TxError)?;

        // Wait for the last character to be fully transmitted
        // Character time = 11 bits / baud_rate
        let char_time_us = 11_000_000u64 / self.config.baud_rate.value() as u64;
        Timer::after_micros(char_time_us + self.config.turnaround_us as u64).await;

        // Switch back to RX mode
        rs485.enable_rx();

        // Inter-frame silence (3.5 character times)
        Timer::after_micros(self.char_timeout_3_5_us).await;

        Ok(())
    }

    /// Process a received frame as a Modbus slave
    ///
    /// Checks if the frame is addressed to us, verifies CRC,
    /// and returns true if it should be processed.
    pub fn is_for_us(&self, frame: &ModbusFrame) -> bool {
        if frame.len < MODBUS_MIN_FRAME {
            return false;
        }

        let addr = frame.address();
        // Accept frames addressed to us or broadcast (address 0)
        if addr != self.config.slave_address && addr != 0 {
            return false;
        }

        // Verify CRC
        if !frame.verify_crc() {
            warn!("Modbus: CRC error on frame for addr {}", addr);
            return false;
        }

        true
    }

    /// Get the configured slave address
    pub fn slave_address(&self) -> u8 {
        self.config.slave_address
    }

    /// Set a new slave address
    pub fn set_slave_address(&mut self, addr: u8) {
        self.config.slave_address = addr;
    }

    /// Set a new baud rate
    pub fn set_baud_rate(&mut self, baud: BaudRate) {
        self.config.baud_rate = baud;
        self.char_timeout_1_5_us = baud.char_timeout_1_5_us();
        self.char_timeout_3_5_us = baud.char_timeout_3_5_us();
    }

    /// Get the inter-character timeout (1.5 char times) in microseconds
    pub fn char_timeout_1_5(&self) -> u64 {
        self.char_timeout_1_5_us
    }

    /// Get the inter-frame timeout (3.5 char times) in microseconds
    pub fn char_timeout_3_5(&self) -> u64 {
        self.char_timeout_3_5_us
    }
}
