//! RS485 Modbus RTU Driver
//!
//! ISO3082 isolated RS485 transceiver with Modbus RTU protocol.
//! Supports master and slave modes with CRC-16/Modbus.
//!
//! Hardware: USART2 (PD5=TX, PD6=RX, PD4=DE/RE)
//! Isolation: ISO3082DWR (5kVrms)

use embassy_time::Duration;
use heapless::Vec;

/// Modbus function codes
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum FunctionCode {
    ReadCoils = 0x01,
    ReadDiscreteInputs = 0x02,
    ReadHoldingRegisters = 0x03,
    ReadInputRegisters = 0x04,
    WriteSingleCoil = 0x05,
    WriteSingleRegister = 0x06,
    WriteMultipleCoils = 0x0F,
    WriteMultipleRegisters = 0x10,
    /// Exception response (function code + 0x80)
    Exception = 0x80,
}

impl FunctionCode {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(Self::ReadCoils),
            0x02 => Some(Self::ReadDiscreteInputs),
            0x03 => Some(Self::ReadHoldingRegisters),
            0x04 => Some(Self::ReadInputRegisters),
            0x05 => Some(Self::WriteSingleCoil),
            0x06 => Some(Self::WriteSingleRegister),
            0x0F => Some(Self::WriteMultipleCoils),
            0x10 => Some(Self::WriteMultipleRegisters),
            _ => None,
        }
    }
}

/// Modbus exception codes
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum ExceptionCode {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    SlaveDeviceFailure = 0x04,
    Acknowledge = 0x05,
    SlaveDeviceBusy = 0x06,
}

/// Modbus operating mode
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModbusMode {
    Master,
    Slave { address: u8 },
}

/// RS485 driver configuration
#[derive(Clone)]
pub struct Rs485Config {
    pub baud_rate: u32,
    pub mode: ModbusMode,
    /// Inter-frame silence (3.5 character times)
    pub t35_us: u32,
    /// Response timeout for master mode
    pub response_timeout: Duration,
}

impl Default for Rs485Config {
    fn default() -> Self {
        // 3.5 chars at 9600 baud = ~4.06ms
        Self {
            baud_rate: 9600,
            mode: ModbusMode::Master,
            t35_us: 4063,
            response_timeout: Duration::from_millis(1000),
        }
    }
}

/// Modbus RTU frame (max 256 bytes)
pub struct ModbusFrame {
    pub address: u8,
    pub function: u8,
    pub data: Vec<u8, 252>,
}

/// RS485 Modbus driver
pub struct Rs485ModbusDriver {
    config: Rs485Config,
    tx_buffer: Vec<u8, 256>,
    rx_buffer: Vec<u8, 256>,
    frame_count: u32,
    error_count: u32,
}

impl Rs485ModbusDriver {
    pub fn new(config: Rs485Config) -> Self {
        Self {
            config,
            tx_buffer: Vec::new(),
            rx_buffer: Vec::new(),
            frame_count: 0,
            error_count: 0,
        }
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    /// Build a Modbus RTU request frame (master mode)
    pub fn build_read_holding_registers(
        slave_addr: u8,
        start_reg: u16,
        count: u16,
    ) -> Vec<u8, 256> {
        let mut frame = Vec::new();
        let _ = frame.push(slave_addr);
        let _ = frame.push(FunctionCode::ReadHoldingRegisters as u8);
        let _ = frame.extend_from_slice(&start_reg.to_be_bytes());
        let _ = frame.extend_from_slice(&count.to_be_bytes());
        let crc = crc16_modbus(&frame);
        let _ = frame.extend_from_slice(&crc.to_le_bytes());
        frame
    }

    /// Build a write single register frame
    pub fn build_write_single_register(
        slave_addr: u8,
        register: u16,
        value: u16,
    ) -> Vec<u8, 256> {
        let mut frame = Vec::new();
        let _ = frame.push(slave_addr);
        let _ = frame.push(FunctionCode::WriteSingleRegister as u8);
        let _ = frame.extend_from_slice(&register.to_be_bytes());
        let _ = frame.extend_from_slice(&value.to_be_bytes());
        let crc = crc16_modbus(&frame);
        let _ = frame.extend_from_slice(&crc.to_le_bytes());
        frame
    }

    /// Validate a received Modbus RTU frame CRC
    pub fn validate_frame(data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }
        let payload = &data[..data.len() - 2];
        let received_crc =
            u16::from_le_bytes([data[data.len() - 2], data[data.len() - 1]]);
        let computed_crc = crc16_modbus(payload);
        received_crc == computed_crc
    }

    /// Parse holding register values from a response frame
    pub fn parse_register_response(data: &[u8]) -> Option<Vec<u16, 125>> {
        if data.len() < 5 || !Self::validate_frame(data) {
            return None;
        }
        if data[1] != FunctionCode::ReadHoldingRegisters as u8 {
            return None;
        }
        let byte_count = data[2] as usize;
        if data.len() < 3 + byte_count + 2 {
            return None;
        }
        let mut registers = Vec::new();
        for i in (0..byte_count).step_by(2) {
            let val = u16::from_be_bytes([data[3 + i], data[3 + i + 1]]);
            let _ = registers.push(val);
        }
        Some(registers)
    }
}

/// CRC-16/Modbus (polynomial 0xA001, init 0xFFFF)
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

/// Calculate T3.5 silence time in microseconds for a given baud rate
pub fn calculate_t35(baud_rate: u32) -> u32 {
    if baud_rate <= 19200 {
        // 3.5 character times (11 bits per char)
        (3_500_000 * 11) / baud_rate
    } else {
        // Fixed 1750us for higher baud rates (per Modbus spec)
        1750
    }
}
