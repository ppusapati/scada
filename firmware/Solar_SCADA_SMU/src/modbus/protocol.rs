/// Modbus Protocol Engine — Solar SCADA SMU
///
/// Identical protocol logic to Water RTU (shared Modbus spec),
/// but operating on a different register map.
///
/// Supports:
///   FC 0x03: Read Holding Registers
///   FC 0x04: Read Input Registers
///   FC 0x06: Write Single Register
///   FC 0x10: Write Multiple Registers

use crate::modbus::registers::RegisterStore;
use crc::{Crc, CRC_16_MODBUS};

const MODBUS_CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

pub mod fc {
    pub const READ_HOLDING_REGISTERS: u8 = 0x03;
    pub const READ_INPUT_REGISTERS: u8 = 0x04;
    pub const WRITE_SINGLE_REGISTER: u8 = 0x06;
    pub const WRITE_MULTIPLE_REGISTERS: u8 = 0x10;
}

pub mod exception {
    pub const ILLEGAL_FUNCTION: u8 = 0x01;
    pub const ILLEGAL_DATA_ADDRESS: u8 = 0x02;
    pub const ILLEGAL_DATA_VALUE: u8 = 0x03;
}

pub const MAX_PDU_SIZE: usize = 253;

pub struct ModbusRtu {
    slave_address: u8,
}

impl ModbusRtu {
    pub const fn new(slave_address: u8) -> Self {
        Self { slave_address }
    }

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

fn process_pdu(
    pdu: &[u8],
    response: &mut [u8],
    registers: &mut RegisterStore,
) -> usize {
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

    if qty == 0 || qty > 123 || byte_count != (qty as usize) * 2 || pdu.len() < 6 + byte_count {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_VALUE;
        return 2;
    }

    if (start + qty) as usize > regs.len() {
        response[0] = pdu[0] | 0x80;
        response[1] = exception::ILLEGAL_DATA_ADDRESS;
        return 2;
    }

    for i in 0..qty as usize {
        let off = 6 + i * 2;
        regs[(start as usize) + i] = ((pdu[off] as u16) << 8) | pdu[off + 1] as u16;
    }

    response[..5].copy_from_slice(&pdu[..5]);
    5
}
