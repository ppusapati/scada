/// Modbus RTU Slave Implementation
///
/// Implements a Modbus RTU slave (server) for SCADA register access.
/// The slave responds to master requests over RS-485 (via SP3485 driver).
///
/// Supported function codes:
///   0x03 - Read Holding Registers (read configuration/setpoints)
///   0x04 - Read Input Registers (read sensor data, read-only)
///   0x06 - Write Single Register (write one configuration register)
///   0x10 - Write Multiple Registers (write block of configuration)
///
/// Register maps are defined per node type (Solar SMU and Water RTU).
///
/// Modbus RTU frame format:
///   [Address(1)] [Function(1)] [Data(N)] [CRC-Lo(1)] [CRC-Hi(1)]
///
/// Register addressing:
///   Input Registers (FC04): sensor data, read-only, updated by sensor tasks
///   Holding Registers (FC03): configuration, setpoints, writable

use crate::drivers::sp3485::{ModbusFrame, MODBUS_MAX_FRAME, crc16_modbus};
use defmt::{info, warn, error, Format};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

// ============================================================================
// Modbus function codes
// ============================================================================

#[allow(dead_code)]
pub mod func {
    pub const READ_HOLDING_REGISTERS: u8 = 0x03;
    pub const READ_INPUT_REGISTERS: u8 = 0x04;
    pub const WRITE_SINGLE_REGISTER: u8 = 0x06;
    pub const WRITE_MULTIPLE_REGISTERS: u8 = 0x10;
}

// ============================================================================
// Modbus exception codes
// ============================================================================

#[allow(dead_code)]
pub mod exception {
    pub const ILLEGAL_FUNCTION: u8 = 0x01;
    pub const ILLEGAL_DATA_ADDRESS: u8 = 0x02;
    pub const ILLEGAL_DATA_VALUE: u8 = 0x03;
    pub const SLAVE_DEVICE_FAILURE: u8 = 0x04;
    pub const ACKNOWLEDGE: u8 = 0x05;
    pub const SLAVE_DEVICE_BUSY: u8 = 0x06;
}

// ============================================================================
// Solar SMU Register Map
// ============================================================================

/// Solar SMU Input Registers (FC04, read-only sensor data)
/// Register addresses are 0-based
#[allow(dead_code)]
pub mod solar_input {
    // String 1 (registers 0-9)
    pub const STR1_VOLTAGE_HI: u16 = 0;      // String 1 voltage (V) x10, high word
    pub const STR1_VOLTAGE_LO: u16 = 1;      // String 1 voltage (V) x10, low word
    pub const STR1_CURRENT_HI: u16 = 2;      // String 1 current (A) x100, high word
    pub const STR1_CURRENT_LO: u16 = 3;      // String 1 current (A) x100, low word
    pub const STR1_POWER_HI: u16 = 4;        // String 1 power (W), high word
    pub const STR1_POWER_LO: u16 = 5;        // String 1 power (W), low word
    pub const STR1_ENERGY_HI: u16 = 6;       // String 1 energy (Wh), high word
    pub const STR1_ENERGY_LO: u16 = 7;       // String 1 energy (Wh), low word
    pub const STR1_TEMP: u16 = 8;            // String 1 junction temp (C x10)
    pub const STR1_STATUS: u16 = 9;          // String 1 status flags

    // String 2 (registers 10-19) -- same layout
    pub const STR2_VOLTAGE_HI: u16 = 10;
    pub const STR2_VOLTAGE_LO: u16 = 11;
    pub const STR2_CURRENT_HI: u16 = 12;
    pub const STR2_CURRENT_LO: u16 = 13;
    pub const STR2_POWER_HI: u16 = 14;
    pub const STR2_POWER_LO: u16 = 15;
    pub const STR2_ENERGY_HI: u16 = 16;
    pub const STR2_ENERGY_LO: u16 = 17;
    pub const STR2_TEMP: u16 = 18;
    pub const STR2_STATUS: u16 = 19;

    // Bus totals (registers 40-49)
    pub const BUS_VOLTAGE_HI: u16 = 40;
    pub const BUS_VOLTAGE_LO: u16 = 41;
    pub const BUS_CURRENT_HI: u16 = 42;
    pub const BUS_CURRENT_LO: u16 = 43;
    pub const BUS_POWER_HI: u16 = 44;
    pub const BUS_POWER_LO: u16 = 45;
    pub const BUS_ENERGY_HI: u16 = 46;
    pub const BUS_ENERGY_LO: u16 = 47;
    pub const BUS_FREQ: u16 = 48;           // Grid frequency (Hz x100)
    pub const BUS_GRID_V: u16 = 49;         // Grid voltage (V x10)

    // Environmental (registers 50-55)
    pub const IRRADIANCE: u16 = 50;          // W/m2
    pub const AMBIENT_TEMP: u16 = 51;        // degC x10
    pub const HUMIDITY: u16 = 52;            // % x10
    pub const WIND_SPEED: u16 = 53;          // m/s x10

    // Alarms (registers 60-63)
    pub const ALARM_WORD_0: u16 = 60;        // Alarm bits 0-15
    pub const ALARM_WORD_1: u16 = 61;        // Alarm bits 16-31
    pub const FAULT_COUNT: u16 = 62;         // Active fault count
    pub const LAST_FAULT_CODE: u16 = 63;     // Last fault code

    // Device info (registers 90-99)
    pub const UPTIME_HI: u16 = 90;
    pub const UPTIME_LO: u16 = 91;
    pub const FW_VERSION_MAJOR: u16 = 92;
    pub const FW_VERSION_MINOR: u16 = 93;
    pub const HW_VERSION: u16 = 94;
    pub const SERIAL_HI: u16 = 95;
    pub const SERIAL_LO: u16 = 96;
    pub const NODE_ID: u16 = 97;

    /// Total number of input registers
    pub const COUNT: usize = 100;
}

/// Solar SMU Holding Registers (FC03, read-write configuration)
#[allow(dead_code)]
pub mod solar_holding {
    pub const SLAVE_ADDRESS: u16 = 0;        // Modbus slave address
    pub const BAUD_RATE: u16 = 1;            // Baud rate code (0=9600,1=19200...)
    pub const SCAN_INTERVAL_MS: u16 = 2;     // Sensor scan interval
    pub const ALARM_MASK_0: u16 = 3;         // Alarm enable mask word 0
    pub const ALARM_MASK_1: u16 = 4;         // Alarm enable mask word 1
    pub const OV_THRESHOLD: u16 = 5;         // Overvoltage threshold (V x10)
    pub const UV_THRESHOLD: u16 = 6;         // Undervoltage threshold (V x10)
    pub const OC_THRESHOLD: u16 = 7;         // Overcurrent threshold (A x100)
    pub const OT_THRESHOLD: u16 = 8;         // Overtemp threshold (C x10)
    pub const ENERGY_RESET: u16 = 9;         // Write 1 to reset energy counters
    pub const INVERTER_ENABLE: u16 = 10;     // Inverter enable (0/1)
    pub const MODE: u16 = 11;                // Operating mode (0=auto,1=manual,2=maint)
    pub const CALIBRATION_CMD: u16 = 12;     // Calibration command
    pub const CALIBRATION_VALUE: u16 = 13;   // Calibration value
    pub const REBOOT_CMD: u16 = 14;          // Write 0xAA55 to reboot

    /// Total number of holding registers
    pub const COUNT: usize = 20;
}

// ============================================================================
// Water RTU Register Map
// ============================================================================

/// Water RTU Input Registers (FC04, read-only sensor data)
#[allow(dead_code)]
pub mod water_input {
    // Tank levels (registers 0-5)
    pub const TANK1_LEVEL: u16 = 0;           // Tank 1 level (% x10)
    pub const TANK1_VOLUME: u16 = 1;          // Tank 1 volume (L)
    pub const TANK2_LEVEL: u16 = 2;
    pub const TANK2_VOLUME: u16 = 3;
    pub const TANK1_OVERFLOW: u16 = 4;        // Boolean (0/1)
    pub const TANK2_OVERFLOW: u16 = 5;

    // Flow rates (registers 10-17)
    pub const FLOW_IN_HI: u16 = 10;           // Inlet flow (L/min x10) high
    pub const FLOW_IN_LO: u16 = 11;           // Inlet flow low word
    pub const FLOW_OUT_HI: u16 = 12;          // Outlet flow high
    pub const FLOW_OUT_LO: u16 = 13;          // Outlet flow low
    pub const TOTAL_IN_HI: u16 = 14;          // Total inlet volume (L) high
    pub const TOTAL_IN_LO: u16 = 15;          // Total inlet volume low
    pub const TOTAL_OUT_HI: u16 = 16;
    pub const TOTAL_OUT_LO: u16 = 17;

    // Pressure (registers 20-23)
    pub const PRESSURE_IN: u16 = 20;           // Inlet pressure (bar x100)
    pub const PRESSURE_OUT: u16 = 21;          // Outlet pressure
    pub const PRESSURE_DIFF: u16 = 22;         // Differential pressure
    pub const PRESSURE_FILTER: u16 = 23;       // Filter pressure

    // Water quality (registers 30-37)
    pub const PH: u16 = 30;                    // pH x100
    pub const TURBIDITY: u16 = 31;             // NTU x100
    pub const CHLORINE: u16 = 32;              // mg/L x1000
    pub const CONDUCTIVITY: u16 = 33;          // uS/cm
    pub const DISSOLVED_O2: u16 = 34;          // mg/L x100
    pub const ORP: u16 = 35;                   // mV
    pub const WATER_TEMP: u16 = 36;            // degC x10
    pub const AMBIENT_TEMP: u16 = 37;          // degC x10

    // Pump/valve status (registers 40-45)
    pub const PUMP1_STATUS: u16 = 40;          // 0=off, 1=on, 2=fault
    pub const PUMP2_STATUS: u16 = 41;
    pub const VALVE1_STATUS: u16 = 42;         // 0=closed, 1=open, 2=transit
    pub const VALVE2_STATUS: u16 = 43;
    pub const PUMP1_RUNTIME_HI: u16 = 44;     // Runtime (minutes) high
    pub const PUMP1_RUNTIME_LO: u16 = 45;     // Runtime low

    // Alarms (registers 60-63)
    pub const ALARM_WORD_0: u16 = 60;
    pub const ALARM_WORD_1: u16 = 61;
    pub const FAULT_COUNT: u16 = 62;
    pub const LAST_FAULT_CODE: u16 = 63;

    // Device info (registers 90-99)
    pub const UPTIME_HI: u16 = 90;
    pub const UPTIME_LO: u16 = 91;
    pub const FW_VERSION_MAJOR: u16 = 92;
    pub const FW_VERSION_MINOR: u16 = 93;
    pub const HW_VERSION: u16 = 94;
    pub const SERIAL_HI: u16 = 95;
    pub const SERIAL_LO: u16 = 96;
    pub const NODE_ID: u16 = 97;

    /// Total number of input registers
    pub const COUNT: usize = 100;
}

/// Water RTU Holding Registers (FC03, configuration)
#[allow(dead_code)]
pub mod water_holding {
    pub const SLAVE_ADDRESS: u16 = 0;
    pub const BAUD_RATE: u16 = 1;
    pub const SCAN_INTERVAL_MS: u16 = 2;
    pub const ALARM_MASK_0: u16 = 3;
    pub const ALARM_MASK_1: u16 = 4;
    pub const PH_HIGH_LIMIT: u16 = 5;         // pH x100
    pub const PH_LOW_LIMIT: u16 = 6;
    pub const TURB_HIGH_LIMIT: u16 = 7;       // NTU x100
    pub const CL_LOW_LIMIT: u16 = 8;          // mg/L x1000
    pub const CL_HIGH_LIMIT: u16 = 9;
    pub const LEVEL_HIGH_LIMIT: u16 = 10;     // % x10
    pub const LEVEL_LOW_LIMIT: u16 = 11;
    pub const PUMP1_ENABLE: u16 = 12;
    pub const PUMP2_ENABLE: u16 = 13;
    pub const VALVE1_CMD: u16 = 14;           // 0=close, 1=open
    pub const VALVE2_CMD: u16 = 15;
    pub const MODE: u16 = 16;                 // 0=auto, 1=manual, 2=maint
    pub const VOLUME_RESET: u16 = 17;         // Write 1 to reset totalizers
    pub const CALIBRATION_CMD: u16 = 18;
    pub const REBOOT_CMD: u16 = 19;

    /// Total number of holding registers
    pub const COUNT: usize = 20;
}

// ============================================================================
// Register storage
// ============================================================================

/// Maximum register count per map
const MAX_INPUT_REGS: usize = 100;
const MAX_HOLDING_REGS: usize = 20;

/// Register map for a Modbus slave
///
/// Thread-safe access is provided by embassy_sync Mutex.
/// The sensor read task updates input registers,
/// the Modbus task reads them and handles writes to holding registers.
pub struct RegisterMap {
    /// Input registers (read-only from Modbus perspective, updated by sensor tasks)
    pub input: [u16; MAX_INPUT_REGS],
    /// Holding registers (read-write from Modbus perspective)
    pub holding: [u16; MAX_HOLDING_REGS],
    /// Write event counter (incremented on any holding register write)
    pub write_count: u32,
    /// Flag indicating a holding register was written (for the command handler to check)
    pub write_pending: bool,
    /// Last written register address
    pub last_write_addr: u16,
}

impl RegisterMap {
    pub const fn new() -> Self {
        Self {
            input: [0u16; MAX_INPUT_REGS],
            holding: [0u16; MAX_HOLDING_REGS],
            write_count: 0,
            write_pending: false,
            last_write_addr: 0,
        }
    }

    /// Read input register(s)
    pub fn read_input(&self, addr: u16, count: u16) -> Option<&[u16]> {
        let start = addr as usize;
        let end = start + count as usize;
        if end <= MAX_INPUT_REGS {
            Some(&self.input[start..end])
        } else {
            None
        }
    }

    /// Read holding register(s)
    pub fn read_holding(&self, addr: u16, count: u16) -> Option<&[u16]> {
        let start = addr as usize;
        let end = start + count as usize;
        if end <= MAX_HOLDING_REGS {
            Some(&self.holding[start..end])
        } else {
            None
        }
    }

    /// Write a single holding register
    pub fn write_holding(&mut self, addr: u16, value: u16) -> bool {
        let idx = addr as usize;
        if idx < MAX_HOLDING_REGS {
            self.holding[idx] = value;
            self.write_count += 1;
            self.write_pending = true;
            self.last_write_addr = addr;
            true
        } else {
            false
        }
    }

    /// Write multiple holding registers
    pub fn write_holding_multi(&mut self, start_addr: u16, values: &[u16]) -> bool {
        let start = start_addr as usize;
        let end = start + values.len();
        if end <= MAX_HOLDING_REGS {
            self.holding[start..end].copy_from_slice(values);
            self.write_count += 1;
            self.write_pending = true;
            self.last_write_addr = start_addr;
            true
        } else {
            false
        }
    }

    /// Update an input register (called by sensor tasks)
    pub fn set_input(&mut self, addr: u16, value: u16) {
        let idx = addr as usize;
        if idx < MAX_INPUT_REGS {
            self.input[idx] = value;
        }
    }

    /// Update a 32-bit value across two consecutive input registers (high, low)
    pub fn set_input_u32(&mut self, addr_hi: u16, value: u32) {
        self.set_input(addr_hi, (value >> 16) as u16);
        self.set_input(addr_hi + 1, (value & 0xFFFF) as u16);
    }

    /// Update a float as a scaled integer (e.g., value_f * 10 => u16)
    pub fn set_input_scaled(&mut self, addr: u16, value_f: f32, scale: f32) {
        let scaled = (value_f * scale) as i16;
        self.set_input(addr, scaled as u16);
    }

    /// Clear the write pending flag
    pub fn clear_write_pending(&mut self) {
        self.write_pending = false;
    }
}

/// Thread-safe register map
pub type SharedRegisterMap = Mutex<CriticalSectionRawMutex, RegisterMap>;

// ============================================================================
// Modbus RTU Slave Protocol Handler
// ============================================================================

/// Modbus RTU slave processor
///
/// Processes incoming Modbus requests and generates responses.
/// Uses a shared RegisterMap for data access.
pub struct ModbusSlave {
    slave_address: u8,
}

impl ModbusSlave {
    pub fn new(slave_address: u8) -> Self {
        Self { slave_address }
    }

    /// Process an incoming Modbus RTU frame and generate a response
    ///
    /// Returns true if a response was generated (caller should send it).
    /// Returns false for broadcast frames (address 0) that don't require a response.
    pub fn process_request(
        &self,
        request: &ModbusFrame,
        response: &mut ModbusFrame,
        registers: &mut RegisterMap,
    ) -> bool {
        response.clear();

        // Verify address
        let addr = request.address();
        if addr != self.slave_address && addr != 0 {
            return false;
        }

        // Verify CRC
        if !request.verify_crc() {
            warn!("Modbus: CRC error");
            return false;
        }

        let fc = request.function();
        let is_broadcast = addr == 0;

        let result = match fc {
            func::READ_HOLDING_REGISTERS => {
                self.handle_read_holding(request, response, registers)
            }
            func::READ_INPUT_REGISTERS => {
                self.handle_read_input(request, response, registers)
            }
            func::WRITE_SINGLE_REGISTER => {
                self.handle_write_single(request, response, registers)
            }
            func::WRITE_MULTIPLE_REGISTERS => {
                self.handle_write_multiple(request, response, registers)
            }
            _ => {
                // Unsupported function code
                response.build_exception(self.slave_address, fc, exception::ILLEGAL_FUNCTION);
                true
            }
        };

        // Don't send response for broadcast messages
        if is_broadcast {
            return false;
        }

        result
    }

    /// FC03: Read Holding Registers
    fn handle_read_holding(
        &self,
        request: &ModbusFrame,
        response: &mut ModbusFrame,
        registers: &RegisterMap,
    ) -> bool {
        if request.len < 8 {
            response.build_exception(
                self.slave_address,
                func::READ_HOLDING_REGISTERS,
                exception::ILLEGAL_DATA_VALUE,
            );
            return true;
        }

        let start_addr = u16::from_be_bytes([request.buf[2], request.buf[3]]);
        let quantity = u16::from_be_bytes([request.buf[4], request.buf[5]]);

        if quantity == 0 || quantity > 125 {
            response.build_exception(
                self.slave_address,
                func::READ_HOLDING_REGISTERS,
                exception::ILLEGAL_DATA_VALUE,
            );
            return true;
        }

        match registers.read_holding(start_addr, quantity) {
            Some(regs) => {
                let byte_count = (quantity * 2) as u8;
                let mut data = [0u8; 252]; // Max: 125 registers * 2 bytes + 1 byte count
                data[0] = byte_count;
                for (i, &reg) in regs.iter().enumerate() {
                    let bytes = reg.to_be_bytes();
                    data[1 + i * 2] = bytes[0];
                    data[2 + i * 2] = bytes[1];
                }
                response.build_response(
                    self.slave_address,
                    func::READ_HOLDING_REGISTERS,
                    &data[..1 + byte_count as usize],
                );
                true
            }
            None => {
                response.build_exception(
                    self.slave_address,
                    func::READ_HOLDING_REGISTERS,
                    exception::ILLEGAL_DATA_ADDRESS,
                );
                true
            }
        }
    }

    /// FC04: Read Input Registers
    fn handle_read_input(
        &self,
        request: &ModbusFrame,
        response: &mut ModbusFrame,
        registers: &RegisterMap,
    ) -> bool {
        if request.len < 8 {
            response.build_exception(
                self.slave_address,
                func::READ_INPUT_REGISTERS,
                exception::ILLEGAL_DATA_VALUE,
            );
            return true;
        }

        let start_addr = u16::from_be_bytes([request.buf[2], request.buf[3]]);
        let quantity = u16::from_be_bytes([request.buf[4], request.buf[5]]);

        if quantity == 0 || quantity > 125 {
            response.build_exception(
                self.slave_address,
                func::READ_INPUT_REGISTERS,
                exception::ILLEGAL_DATA_VALUE,
            );
            return true;
        }

        match registers.read_input(start_addr, quantity) {
            Some(regs) => {
                let byte_count = (quantity * 2) as u8;
                let mut data = [0u8; 252];
                data[0] = byte_count;
                for (i, &reg) in regs.iter().enumerate() {
                    let bytes = reg.to_be_bytes();
                    data[1 + i * 2] = bytes[0];
                    data[2 + i * 2] = bytes[1];
                }
                response.build_response(
                    self.slave_address,
                    func::READ_INPUT_REGISTERS,
                    &data[..1 + byte_count as usize],
                );
                true
            }
            None => {
                response.build_exception(
                    self.slave_address,
                    func::READ_INPUT_REGISTERS,
                    exception::ILLEGAL_DATA_ADDRESS,
                );
                true
            }
        }
    }

    /// FC06: Write Single Register
    fn handle_write_single(
        &self,
        request: &ModbusFrame,
        response: &mut ModbusFrame,
        registers: &mut RegisterMap,
    ) -> bool {
        if request.len < 8 {
            response.build_exception(
                self.slave_address,
                func::WRITE_SINGLE_REGISTER,
                exception::ILLEGAL_DATA_VALUE,
            );
            return true;
        }

        let addr = u16::from_be_bytes([request.buf[2], request.buf[3]]);
        let value = u16::from_be_bytes([request.buf[4], request.buf[5]]);

        if registers.write_holding(addr, value) {
            // Echo the request as response (per Modbus spec)
            response.build_response(
                self.slave_address,
                func::WRITE_SINGLE_REGISTER,
                &request.buf[2..6],
            );
            true
        } else {
            response.build_exception(
                self.slave_address,
                func::WRITE_SINGLE_REGISTER,
                exception::ILLEGAL_DATA_ADDRESS,
            );
            true
        }
    }

    /// FC10 (16): Write Multiple Registers
    fn handle_write_multiple(
        &self,
        request: &ModbusFrame,
        response: &mut ModbusFrame,
        registers: &mut RegisterMap,
    ) -> bool {
        if request.len < 9 {
            response.build_exception(
                self.slave_address,
                func::WRITE_MULTIPLE_REGISTERS,
                exception::ILLEGAL_DATA_VALUE,
            );
            return true;
        }

        let start_addr = u16::from_be_bytes([request.buf[2], request.buf[3]]);
        let quantity = u16::from_be_bytes([request.buf[4], request.buf[5]]);
        let byte_count = request.buf[6] as usize;

        // Validate
        if quantity == 0 || quantity > 123 || byte_count != (quantity as usize * 2) {
            response.build_exception(
                self.slave_address,
                func::WRITE_MULTIPLE_REGISTERS,
                exception::ILLEGAL_DATA_VALUE,
            );
            return true;
        }

        if request.len < 7 + byte_count + 2 {
            response.build_exception(
                self.slave_address,
                func::WRITE_MULTIPLE_REGISTERS,
                exception::ILLEGAL_DATA_VALUE,
            );
            return true;
        }

        // Parse register values
        let mut values = [0u16; 123];
        for i in 0..quantity as usize {
            values[i] = u16::from_be_bytes([
                request.buf[7 + i * 2],
                request.buf[8 + i * 2],
            ]);
        }

        if registers.write_holding_multi(start_addr, &values[..quantity as usize]) {
            // Response: address + function + start_addr + quantity
            let mut data = [0u8; 4];
            data[0..2].copy_from_slice(&start_addr.to_be_bytes());
            data[2..4].copy_from_slice(&quantity.to_be_bytes());
            response.build_response(
                self.slave_address,
                func::WRITE_MULTIPLE_REGISTERS,
                &data,
            );
            true
        } else {
            response.build_exception(
                self.slave_address,
                func::WRITE_MULTIPLE_REGISTERS,
                exception::ILLEGAL_DATA_ADDRESS,
            );
            true
        }
    }

    /// Get the configured slave address
    pub fn slave_address(&self) -> u8 {
        self.slave_address
    }

    /// Update slave address
    pub fn set_slave_address(&mut self, addr: u8) {
        self.slave_address = addr;
    }
}
