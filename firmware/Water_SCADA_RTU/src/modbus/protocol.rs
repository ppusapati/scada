/// Modbus Protocol Engine — RTU and TCP frame handling
///
/// Handles PDU (Protocol Data Unit) processing independent of transport.
/// RTU framing adds slave address + CRC16.
/// TCP framing adds MBAP header (transaction ID, protocol ID, length, unit ID).

use crate::modbus::registers::RegisterStore;
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
    pub const SLAVE_DEVICE_FAILURE: u8 = 0x04;
}

/// Maximum PDU size (without transport framing)
pub const MAX_PDU_SIZE: usize = 253;

/// Modbus RTU frame processor
pub struct ModbusRtu {
    slave_address: u8,
}

impl ModbusRtu {
    pub const fn new(slave_address: u8) -> Self {
        Self { slave_address }
    }

    pub fn set_slave_address(&mut self, addr: u8) {
        self.slave_address = addr;
    }

    /// Process an incoming RTU frame and generate response
    /// Returns number of bytes in response, or 0 if frame was not for us
    pub fn process_frame(
        &self,
        request: &[u8],
        req_len: usize,
        response: &mut [u8],
        registers: &mut RegisterStore,
    ) -> usize {
        if req_len < 4 {
            return 0; // Too short (addr + fc + crc16)
        }

        // Verify CRC
        let crc_calc = MODBUS_CRC.checksum(&request[..req_len - 2]);
        let crc_recv = (request[req_len - 2] as u16) | ((request[req_len - 1] as u16) << 8);
        if crc_calc != crc_recv {
            return 0; // CRC mismatch, ignore
        }

        // Check slave address (0 = broadcast, accept our address only)
        let addr = request[0];
        if addr != self.slave_address && addr != 0 {
            return 0; // Not for us
        }

        // Extract PDU (skip slave address)
        let pdu = &request[1..req_len - 2];
        let mut resp_pdu = [0u8; MAX_PDU_SIZE];
        let pdu_len = process_pdu(pdu, &mut resp_pdu, registers);

        if pdu_len == 0 || addr == 0 {
            return 0; // No response for broadcast
        }

        // Build RTU response: slave_addr + PDU + CRC
        response[0] = self.slave_address;
        response[1..1 + pdu_len].copy_from_slice(&resp_pdu[..pdu_len]);
        let resp_len = 1 + pdu_len;

        // Append CRC
        let crc = MODBUS_CRC.checksum(&response[..resp_len]);
        response[resp_len] = (crc & 0xFF) as u8;
        response[resp_len + 1] = (crc >> 8) as u8;

        resp_len + 2
    }
}

/// Modbus TCP frame processor
pub struct ModbusTcp {
    unit_id: u8,
}

impl ModbusTcp {
    pub const fn new(unit_id: u8) -> Self {
        Self { unit_id }
    }

    /// Process an incoming TCP frame (MBAP header + PDU)
    /// Returns number of bytes in response, or 0 on error
    pub fn process_frame(
        &self,
        request: &[u8],
        req_len: usize,
        response: &mut [u8],
        registers: &mut RegisterStore,
    ) -> usize {
        if req_len < 7 {
            return 0; // MBAP header is 7 bytes minimum
        }

        // Parse MBAP header
        let transaction_id = ((request[0] as u16) << 8) | request[1] as u16;
        let protocol_id = ((request[2] as u16) << 8) | request[3] as u16;
        let length = ((request[4] as u16) << 8) | request[5] as u16;
        let unit_id = request[6];

        if protocol_id != 0 {
            return 0; // Not Modbus protocol
        }

        if unit_id != self.unit_id && unit_id != 0xFF {
            return 0; // Not for us
        }

        // Extract PDU (after MBAP header)
        let pdu = &request[7..req_len];
        let mut resp_pdu = [0u8; MAX_PDU_SIZE];
        let pdu_len = process_pdu(pdu, &mut resp_pdu, registers);

        if pdu_len == 0 {
            return 0;
        }

        // Build TCP response: MBAP header + PDU
        // Transaction ID (echo back)
        response[0] = (transaction_id >> 8) as u8;
        response[1] = (transaction_id & 0xFF) as u8;
        // Protocol ID (0x0000)
        response[2] = 0;
        response[3] = 0;
        // Length (unit_id + PDU length)
        let resp_length = (1 + pdu_len) as u16;
        response[4] = (resp_length >> 8) as u8;
        response[5] = (resp_length & 0xFF) as u8;
        // Unit ID
        response[6] = self.unit_id;
        // PDU
        response[7..7 + pdu_len].copy_from_slice(&resp_pdu[..pdu_len]);

        7 + pdu_len
    }
}

/// Process a Modbus PDU (function code + data) — transport-independent
/// Returns number of bytes written to response
fn process_pdu(
    pdu: &[u8],
    response: &mut [u8],
    registers: &mut RegisterStore,
) -> usize {
    if pdu.is_empty() {
        return 0;
    }

    let function_code = pdu[0];

    match function_code {
        fc::READ_HOLDING_REGISTERS => read_registers(pdu, response, &registers.holding_regs),
        fc::READ_INPUT_REGISTERS => read_registers(pdu, response, &registers.input_regs),
        fc::WRITE_SINGLE_REGISTER => write_single_register(pdu, response, &mut registers.holding_regs),
        fc::WRITE_MULTIPLE_REGISTERS => write_multiple_registers(pdu, response, &mut registers.holding_regs),
        _ => {
            // Exception: Illegal Function
            response[0] = function_code | 0x80;
            response[1] = exception::ILLEGAL_FUNCTION;
            2
        }
    }
}

/// FC 0x03 / 0x04: Read Holding/Input Registers
fn read_registers(pdu: &[u8], response: &mut [u8], regs: &[u16]) -> usize {
    if pdu.len() < 5 {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    let start_addr = ((pdu[1] as u16) << 8) | pdu[2] as u16;
    let quantity = ((pdu[3] as u16) << 8) | pdu[4] as u16;

    // Validate range
    if quantity == 0 || quantity > 125 {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    let end_addr = start_addr + quantity;
    if end_addr as usize > regs.len() {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_ADDRESS;
        return 2;
    }

    // Build response
    let byte_count = (quantity * 2) as u8;
    response[0] = pdu[0]; // Echo function code
    response[1] = byte_count;

    let mut offset = 2usize;
    for i in start_addr..end_addr {
        let val = regs[i as usize];
        response[offset] = (val >> 8) as u8;
        response[offset + 1] = (val & 0xFF) as u8;
        offset += 2;
    }

    offset
}

/// FC 0x06: Write Single Register
fn write_single_register(pdu: &[u8], response: &mut [u8], regs: &mut [u16]) -> usize {
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

    // Echo request as response
    response[..5].copy_from_slice(&pdu[..5]);
    5
}

/// FC 0x10: Write Multiple Registers
fn write_multiple_registers(pdu: &[u8], response: &mut [u8], regs: &mut [u16]) -> usize {
    if pdu.len() < 6 {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    let start_addr = ((pdu[1] as u16) << 8) | pdu[2] as u16;
    let quantity = ((pdu[3] as u16) << 8) | pdu[4] as u16;
    let byte_count = pdu[5] as usize;

    if quantity == 0 || quantity > 123 || byte_count != (quantity as usize) * 2 {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    let end_addr = start_addr + quantity;
    if end_addr as usize > regs.len() {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_ADDRESS;
        return 2;
    }

    if pdu.len() < 6 + byte_count {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    // Write registers
    for i in 0..quantity as usize {
        let data_offset = 6 + i * 2;
        let value = ((pdu[data_offset] as u16) << 8) | pdu[data_offset + 1] as u16;
        regs[(start_addr as usize) + i] = value;
    }

    // Response: echo FC + start_addr + quantity
    response[0] = pdu[0];
    response[1] = pdu[1];
    response[2] = pdu[2];
    response[3] = pdu[3];
    response[4] = pdu[4];
    5
}

/// Calculate Modbus RTU CRC-16
pub fn crc16(data: &[u8]) -> u16 {
    MODBUS_CRC.checksum(data)
}
