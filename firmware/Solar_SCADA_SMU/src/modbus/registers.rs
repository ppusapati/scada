/// Modbus Register Map — Solar SCADA SMU

/// Input register offsets (from 30001)
pub mod input_reg {
    /// String voltages: float32, 2 regs × 16 strings
    pub const STRING_VOLTAGE_BASE: u16 = 0;       // 30001
    pub const STRING_VOLTAGE_COUNT: u16 = 32;

    /// String currents: float32, 2 regs × 16 strings
    pub const STRING_CURRENT_BASE: u16 = 32;      // 30033
    pub const STRING_CURRENT_COUNT: u16 = 32;

    /// String powers: float32, 2 regs × 16 strings
    pub const STRING_POWER_BASE: u16 = 64;        // 30065
    pub const STRING_POWER_COUNT: u16 = 32;

    /// Bus voltage (float32)
    pub const BUS_VOLTAGE: u16 = 96;              // 30097-30098

    /// Bus current (float32)
    pub const BUS_CURRENT: u16 = 98;              // 30099-30100

    /// Total power (float32)
    pub const TOTAL_POWER: u16 = 100;             // 30101-30102

    /// Irradiance (float32, W/m²)
    pub const IRRADIANCE: u16 = 102;              // 30103-30104

    /// Module temperature (float32, °C, from ADC)
    pub const MODULE_TEMP: u16 = 104;             // 30105-30106

    /// Ambient temperature (float32, °C, from TMP117)
    pub const AMBIENT_TEMP: u16 = 106;            // 30107-30108

    /// String status bitmap (1 bit per string, 1=fault)
    pub const STRING_STATUS: u16 = 108;           // 30109

    /// Device status word
    pub const DEVICE_STATUS: u16 = 109;           // 30110

    /// Uptime seconds (32-bit)
    pub const UPTIME: u16 = 110;                  // 30111-30112

    /// Daily energy kWh (float32)
    pub const DAILY_ENERGY: u16 = 112;            // 30113-30114

    pub const TOTAL: u16 = 114;
}

/// Holding register offsets (from 40001)
pub mod holding_reg {
    pub const SLAVE_ADDR: u16 = 0;           // 40001
    pub const SCAN_INTERVAL_MS: u16 = 1;     // 40002
    pub const LOG_INTERVAL_S: u16 = 2;       // 40003
    pub const ALARM_ENABLE: u16 = 3;         // 40004

    /// Voltage calibration: offset(float32) per string, 2 regs × 16
    pub const VCAL_BASE: u16 = 4;            // 40005-40036
    pub const VCAL_COUNT: u16 = 32;

    /// Current calibration: offset(float32) per string, 2 regs × 16
    pub const ICAL_BASE: u16 = 36;           // 40037-40068
    pub const ICAL_COUNT: u16 = 32;

    /// Command register
    pub const COMMAND: u16 = 99;             // 40100

    pub const TOTAL: u16 = 100;
}

/// Device status bits
pub mod device_status {
    pub const ADC_OK: u16 = 0x0001;
    pub const TMP_OK: u16 = 0x0002;
    pub const SD_OK: u16 = 0x0004;
    pub const RS485_OK: u16 = 0x0008;
    pub const CAL_VALID: u16 = 0x0010;
    pub const ANY_ALARM: u16 = 0x8000;
}

/// Register store
pub struct RegisterStore {
    pub input_regs: [u16; input_reg::TOTAL as usize],
    pub holding_regs: [u16; holding_reg::TOTAL as usize],
}

impl RegisterStore {
    pub const fn new() -> Self {
        Self {
            input_regs: [0u16; input_reg::TOTAL as usize],
            holding_regs: [0u16; holding_reg::TOTAL as usize],
        }
    }

    /// Store a float32 value in 2 consecutive registers
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

    /// Update string voltage (float32)
    pub fn set_string_voltage(&mut self, string_id: u8, voltage: f32) {
        let base = input_reg::STRING_VOLTAGE_BASE + (string_id as u16) * 2;
        let bits = voltage.to_bits();
        self.input_regs[base as usize] = (bits >> 16) as u16;
        self.input_regs[(base + 1) as usize] = (bits & 0xFFFF) as u16;
    }

    /// Update string current (float32)
    pub fn set_string_current(&mut self, string_id: u8, current: f32) {
        let base = input_reg::STRING_CURRENT_BASE + (string_id as u16) * 2;
        let bits = current.to_bits();
        self.input_regs[base as usize] = (bits >> 16) as u16;
        self.input_regs[(base + 1) as usize] = (bits & 0xFFFF) as u16;
    }

    /// Update string power (auto-calculated from V × I)
    pub fn set_string_power(&mut self, string_id: u8, power: f32) {
        let base = input_reg::STRING_POWER_BASE + (string_id as u16) * 2;
        let bits = power.to_bits();
        self.input_regs[base as usize] = (bits >> 16) as u16;
        self.input_regs[(base + 1) as usize] = (bits & 0xFFFF) as u16;
    }

    /// Update bus measurements
    pub fn set_bus_voltage(&mut self, v: f32) {
        let bits = v.to_bits();
        self.input_regs[input_reg::BUS_VOLTAGE as usize] = (bits >> 16) as u16;
        self.input_regs[(input_reg::BUS_VOLTAGE + 1) as usize] = (bits & 0xFFFF) as u16;
    }

    pub fn set_bus_current(&mut self, i: f32) {
        let bits = i.to_bits();
        self.input_regs[input_reg::BUS_CURRENT as usize] = (bits >> 16) as u16;
        self.input_regs[(input_reg::BUS_CURRENT + 1) as usize] = (bits & 0xFFFF) as u16;
    }

    pub fn set_total_power(&mut self, p: f32) {
        let bits = p.to_bits();
        self.input_regs[input_reg::TOTAL_POWER as usize] = (bits >> 16) as u16;
        self.input_regs[(input_reg::TOTAL_POWER + 1) as usize] = (bits & 0xFFFF) as u16;
    }

    /// Load defaults
    pub fn load_defaults(&mut self) {
        self.holding_regs[holding_reg::SLAVE_ADDR as usize] = 2; // Different from water RTU
        self.holding_regs[holding_reg::SCAN_INTERVAL_MS as usize] = 1000; // 1s scan
        self.holding_regs[holding_reg::LOG_INTERVAL_S as usize] = 60; // Log every 60s
        self.holding_regs[holding_reg::ALARM_ENABLE as usize] = 0xFFFF;
    }
}
