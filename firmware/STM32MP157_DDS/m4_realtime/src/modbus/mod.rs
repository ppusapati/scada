/// Legacy Modbus RTU Slave — RS-485 Interface
///
/// Provides backward compatibility with existing SCADA masters that
/// communicate via Modbus RTU over RS-485. The register map is shared
/// with the DDS data model: sensor readings published via DDS are also
/// accessible via Modbus holding/input registers.
///
/// RS-485 Configuration:
///   USART2: PA2 (TX), PA3 (RX)
///   DE/RE control: PA8 (high = transmit, low = receive)
///   Default: 9600 baud, 8N1
///   Slave address: configurable via DDS or register 40001
///
/// Supported Function Codes:
///   0x03: Read Holding Registers (calibration, config)
///   0x04: Read Input Registers (sensor data, status)
///   0x06: Write Single Register (set config parameter)
///   0x10: Write Multiple Registers (batch calibration update)
///
/// Register Map (unified Solar/Water):
///   ┌──────────────┬──────────────────────────────────────────────────┐
///   │ Address      │ Description                                      │
///   ├──────────────┼──────────────────────────────────────────────────┤
///   │ Input Registers (FC 0x04, read-only)                            │
///   │ 30001-30032  │ Channel values (float32, 2 regs × 16 ch max)    │
///   │ 30033-30048  │ Channel status codes                             │
///   │ 30049-30050  │ Alarm bitmaps                                    │
///   │ 30051        │ Device status word                               │
///   │ 30052-30053  │ Uptime (seconds, 32-bit)                        │
///   │ 30054-30055  │ Total power / flow (float32)                    │
///   │ 30056-30057  │ Daily energy / volume (float32)                 │
///   │ 30058-30059  │ Temperature (float32, from TMP117)              │
///   ├──────────────┼──────────────────────────────────────────────────┤
///   │ Holding Registers (FC 0x03/0x06/0x10, read-write)              │
///   │ 40001        │ Modbus slave address (1-247)                     │
///   │ 40002        │ Scan interval (ms)                               │
///   │ 40003        │ Alarm enable bitmap                              │
///   │ 40004-40067  │ Calibration data (variant-specific)             │
///   │ 40100        │ Command register (0xCAFE=save, 0xDEAD=reset)    │
///   └──────────────┴──────────────────────────────────────────────────┘

use embassy_stm32::gpio::Output;
use embassy_stm32::usart::Uart;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::{info, warn, debug};
use crc::{Crc, CRC_16_MODBUS};

const MODBUS_CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

/// Modbus function codes
pub mod fc {
    pub const READ_HOLDING_REGISTERS: u8 = 0x03;
    pub const READ_INPUT_REGISTERS: u8 = 0x04;
    pub const WRITE_SINGLE_REGISTER: u8 = 0x06;
    pub const WRITE_MULTIPLE_REGISTERS: u8 = 0x10;
}

/// Modbus exception codes
pub mod exception {
    pub const ILLEGAL_FUNCTION: u8 = 0x01;
    pub const ILLEGAL_DATA_ADDRESS: u8 = 0x02;
    pub const ILLEGAL_DATA_VALUE: u8 = 0x03;
}

pub const MAX_PDU_SIZE: usize = 253;
pub const MAX_FRAME_SIZE: usize = 256;

/// Register counts
pub const INPUT_REG_COUNT: usize = 60;
pub const HOLDING_REG_COUNT: usize = 101;

/// Input register offsets
pub mod input_reg {
    pub const CHANNEL_VALUES_BASE: u16 = 0;   // 30001: float32 × 16 channels
    pub const CHANNEL_STATUS_BASE: u16 = 32;  // 30033: u16 × 16 channels
    pub const ALARM_BITMAP_LO: u16 = 48;      // 30049
    pub const ALARM_BITMAP_HI: u16 = 49;      // 30050
    pub const DEVICE_STATUS: u16 = 50;         // 30051
    pub const UPTIME: u16 = 51;                // 30052-30053
    pub const TOTAL_POWER: u16 = 53;           // 30054-30055
    pub const DAILY_ENERGY: u16 = 55;          // 30056-30057
    pub const TEMPERATURE: u16 = 57;           // 30058-30059
    pub const TOTAL: usize = 60;
}

/// Holding register offsets
pub mod holding_reg {
    pub const SLAVE_ADDR: u16 = 0;         // 40001
    pub const SCAN_INTERVAL_MS: u16 = 1;   // 40002
    pub const ALARM_ENABLE: u16 = 2;       // 40003
    pub const CAL_DATA_BASE: u16 = 3;      // 40004-40067
    pub const COMMAND: u16 = 99;           // 40100
    pub const TOTAL: usize = 101;
}

/// Device status bits
pub mod device_status {
    pub const ADC_OK: u16 = 0x0001;
    pub const TMP_OK: u16 = 0x0002;
    pub const RS485_OK: u16 = 0x0004;
    pub const ETH_LINK: u16 = 0x0008;
    pub const DDS_OK: u16 = 0x0010;
    pub const RPMSG_OK: u16 = 0x0020;
    pub const CAL_VALID: u16 = 0x0040;
    pub const PTP_SYNC: u16 = 0x0080;
    pub const ANY_ALARM: u16 = 0x8000;
}

/// Register store for Modbus data
pub struct RegisterStore {
    pub input_regs: [u16; INPUT_REG_COUNT],
    pub holding_regs: [u16; HOLDING_REG_COUNT],
}

impl RegisterStore {
    pub const fn new() -> Self {
        Self {
            input_regs: [0u16; INPUT_REG_COUNT],
            holding_regs: [0u16; HOLDING_REG_COUNT],
        }
    }

    /// Store a float32 value in 2 consecutive registers (big-endian word order)
    pub fn set_float(&mut self, regs: &mut [u16], base: u16, value: f32) {
        let bits = value.to_bits();
        regs[base as usize] = (bits >> 16) as u16;
        regs[(base + 1) as usize] = (bits & 0xFFFF) as u16;
    }

    /// Read a float32 from 2 consecutive registers
    pub fn get_float(&self, regs: &[u16], base: u16) -> f32 {
        let bits = ((regs[base as usize] as u32) << 16) | regs[(base + 1) as usize] as u32;
        f32::from_bits(bits)
    }

    /// Load default configuration
    pub fn load_defaults(&mut self) {
        self.holding_regs[holding_reg::SLAVE_ADDR as usize] = 1;
        self.holding_regs[holding_reg::SCAN_INTERVAL_MS as usize] = 1000;
        self.holding_regs[holding_reg::ALARM_ENABLE as usize] = 0xFFFF;
    }
}

/// RS-485 half-duplex transceiver driver
pub struct Rs485<'a> {
    uart: Uart<'a, Async>,
    de: Output<'a>,
}

impl<'a> Rs485<'a> {
    pub fn new(uart: Uart<'a, Async>, de: Output<'a>) -> Self {
        let mut s = Self { uart, de };
        s.de.set_low(); // Start in receive mode
        s
    }

    pub async fn send(&mut self, data: &[u8]) -> bool {
        self.de.set_high(); // Enable transmitter
        Timer::after(Duration::from_micros(1)).await;
        let result = self.uart.write(data).await;
        Timer::after(Duration::from_micros(100)).await;
        self.de.set_low(); // Back to receive mode
        result.is_ok()
    }

    pub async fn recv(&mut self, buf: &mut [u8], timeout_ms: u32) -> usize {
        self.de.set_low();
        let mut total = 0usize;
        let mut single = [0u8; 1];

        // Wait for first byte with full timeout
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
        // Modbus RTU: 3.5 character times at 9600 baud = ~4ms
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

/// Modbus RTU protocol engine
pub struct ModbusRtu {
    slave_address: u8,
}

impl ModbusRtu {
    pub const fn new(slave_address: u8) -> Self {
        Self { slave_address }
    }

    /// Process an incoming RTU frame and generate response
    pub fn process_frame(
        &self,
        request: &[u8],
        req_len: usize,
        response: &mut [u8],
        registers: &mut RegisterStore,
    ) -> usize {
        if req_len < 4 {
            return 0;
        }

        // Verify CRC
        let crc_calc = MODBUS_CRC.checksum(&request[..req_len - 2]);
        let crc_recv = (request[req_len - 2] as u16) | ((request[req_len - 1] as u16) << 8);
        if crc_calc != crc_recv {
            return 0;
        }

        let addr = request[0];
        if addr != self.slave_address && addr != 0 {
            return 0;
        }

        let pdu = &request[1..req_len - 2];
        let mut resp_pdu = [0u8; MAX_PDU_SIZE];
        let pdu_len = process_pdu(pdu, &mut resp_pdu, registers);

        if pdu_len == 0 || addr == 0 {
            return 0;
        }

        response[0] = self.slave_address;
        response[1..1 + pdu_len].copy_from_slice(&resp_pdu[..pdu_len]);
        let resp_len = 1 + pdu_len;

        let crc = MODBUS_CRC.checksum(&response[..resp_len]);
        response[resp_len] = (crc & 0xFF) as u8;
        response[resp_len + 1] = (crc >> 8) as u8;

        resp_len + 2
    }
}

fn process_pdu(pdu: &[u8], response: &mut [u8], registers: &mut RegisterStore) -> usize {
    if pdu.is_empty() {
        return 0;
    }

    match pdu[0] {
        fc::READ_HOLDING_REGISTERS => read_registers(pdu, response, &registers.holding_regs),
        fc::READ_INPUT_REGISTERS => read_registers(pdu, response, &registers.input_regs),
        fc::WRITE_SINGLE_REGISTER => write_single(pdu, response, &mut registers.holding_regs),
        fc::WRITE_MULTIPLE_REGISTERS => write_multiple(pdu, response, &mut registers.holding_regs),
        _ => {
            response[0] = pdu[0] | 0x80;
            response[1] = exception::ILLEGAL_FUNCTION;
            2
        }
    }
}

fn read_registers(pdu: &[u8], response: &mut [u8], regs: &[u16]) -> usize {
    if pdu.len() < 5 {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    let start = ((pdu[1] as u16) << 8) | pdu[2] as u16;
    let qty = ((pdu[3] as u16) << 8) | pdu[4] as u16;

    if qty == 0 || qty > 125 || (start + qty) as usize > regs.len() {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_ADDRESS;
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

fn write_single(pdu: &[u8], response: &mut [u8], regs: &mut [u16]) -> usize {
    if pdu.len() < 5 {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    let addr = ((pdu[1] as u16) << 8) | pdu[2] as u16;
    let value = ((pdu[3] as u16) << 8) | pdu[4] as u16;

    if addr as usize >= regs.len() {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_ADDRESS;
        return 2;
    }

    regs[addr as usize] = value;
    response[..5].copy_from_slice(&pdu[..5]);
    5
}

fn write_multiple(pdu: &[u8], response: &mut [u8], regs: &mut [u16]) -> usize {
    if pdu.len() < 6 {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    let start = ((pdu[1] as u16) << 8) | pdu[2] as u16;
    let qty = ((pdu[3] as u16) << 8) | pdu[4] as u16;
    let byte_count = pdu[5] as usize;

    if qty == 0 || qty > 123 || byte_count != (qty as usize) * 2
        || pdu.len() < 6 + byte_count
        || (start + qty) as usize > regs.len()
    {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    for i in 0..qty as usize {
        let off = 6 + i * 2;
        regs[(start as usize) + i] = ((pdu[off] as u16) << 8) | pdu[off + 1] as u16;
    }

    response[..5].copy_from_slice(&pdu[..5]);
    5
}

/// Compute Modbus CRC-16
pub fn crc16(data: &[u8]) -> u16 {
    MODBUS_CRC.checksum(data)
}
