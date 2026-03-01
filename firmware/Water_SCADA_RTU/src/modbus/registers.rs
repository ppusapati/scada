/// Modbus Register Map — Water SCADA RTU
///
/// Input Registers (read-only, FC 0x04) — base address 30001
/// Holding Registers (read-write, FC 0x03/0x06/0x10) — base address 40001

use crate::drivers::ads1258::{Channel, ChannelReading, ChannelCalibration, ChannelStatus};
use heapless::Vec;

/// Input register addresses (0-based offset from 30001)
pub mod input_reg {
    /// Raw ADC codes: 2 registers per channel (32-bit signed, big-endian)
    /// CH0 = offset 0-1, CH1 = offset 2-3, ... CH7 = offset 14-15
    pub const RAW_ADC_BASE: u16 = 0;        // 30001
    pub const RAW_ADC_COUNT: u16 = 16;

    /// Loop current in mA × 100 (e.g., 4.00mA = 400)
    /// CH0 = offset 16, CH1 = offset 17, ... CH7 = offset 23
    pub const CURRENT_MA_BASE: u16 = 16;    // 30017
    pub const CURRENT_MA_COUNT: u16 = 8;

    /// Engineering values: 2 registers per channel (IEEE 754 float32)
    /// CH0 = offset 24-25, CH1 = offset 26-27, ... CH7 = offset 38-39
    pub const ENG_VALUE_BASE: u16 = 24;     // 30025
    pub const ENG_VALUE_COUNT: u16 = 16;

    /// Channel status codes (1 register per channel)
    /// 0=Normal, 1=UnderRange, 2=OverRange, 3=OpenCircuit, 4=ShortCircuit
    pub const STATUS_BASE: u16 = 40;        // 30041
    pub const STATUS_COUNT: u16 = 8;

    /// Alarm bitmaps
    pub const ALARM_BITMAP_LO: u16 = 48;   // 30049 (channels 0-7 alarms)
    pub const ALARM_BITMAP_HI: u16 = 49;   // 30050 (reserved)

    /// Device status word
    pub const DEVICE_STATUS: u16 = 50;      // 30051

    /// Uptime in seconds (32-bit, 2 registers)
    pub const UPTIME_BASE: u16 = 51;        // 30052-30053

    /// Total input registers
    pub const TOTAL: u16 = 53;
}

/// Holding register addresses (0-based offset from 40001)
pub mod holding_reg {
    /// Calibration zero codes (32-bit, 2 regs × 8 channels)
    pub const CAL_ZERO_BASE: u16 = 0;       // 40001
    pub const CAL_ZERO_COUNT: u16 = 16;

    /// Calibration span codes (32-bit, 2 regs × 8 channels)
    pub const CAL_SPAN_BASE: u16 = 16;      // 40017
    pub const CAL_SPAN_COUNT: u16 = 16;

    /// Engineering min values (float32, 2 regs × 8 channels)
    pub const ENG_MIN_BASE: u16 = 32;       // 40033
    pub const ENG_MIN_COUNT: u16 = 16;

    /// Engineering max values (float32, 2 regs × 8 channels)
    pub const ENG_MAX_BASE: u16 = 48;       // 40049
    pub const ENG_MAX_COUNT: u16 = 16;

    /// Communication settings
    pub const SLAVE_ADDR: u16 = 64;         // 40065
    pub const BAUD_RATE: u16 = 65;          // 40066
    pub const IP_ADDR_BASE: u16 = 66;       // 40067-40070
    pub const ALARM_ENABLE: u16 = 70;       // 40071

    /// Commands
    pub const CAL_SAVE_CMD: u16 = 99;       // 40100 (write 0xCAFE to save)

    /// Total holding registers
    pub const TOTAL: u16 = 100;
}

/// Device status bits
pub mod device_status {
    pub const ADC_OK: u16 = 0x0001;
    pub const ETH_LINK: u16 = 0x0002;
    pub const RS485_OK: u16 = 0x0004;
    pub const CAL_VALID: u16 = 0x0008;
    pub const ANY_ALARM: u16 = 0x8000;
}

/// Register store for runtime values
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

    /// Update input registers from ADC readings
    pub fn update_from_readings(&mut self, readings: &[ChannelReading; Channel::COUNT]) {
        let mut alarm_bits: u16 = 0;

        for (i, r) in readings.iter().enumerate() {
            let i16_offset = i as u16;

            // Raw ADC (32-bit → 2 registers, big-endian)
            let raw_u32 = r.raw_code as u32;
            self.input_regs[(input_reg::RAW_ADC_BASE + i16_offset * 2) as usize] =
                (raw_u32 >> 16) as u16;
            self.input_regs[(input_reg::RAW_ADC_BASE + i16_offset * 2 + 1) as usize] =
                (raw_u32 & 0xFFFF) as u16;

            // Current mA × 100
            self.input_regs[(input_reg::CURRENT_MA_BASE + i16_offset) as usize] =
                (r.current_ma * 100.0) as u16;

            // Engineering value (float32 → 2 registers)
            let eng_bits = r.eng_value.to_bits();
            self.input_regs[(input_reg::ENG_VALUE_BASE + i16_offset * 2) as usize] =
                (eng_bits >> 16) as u16;
            self.input_regs[(input_reg::ENG_VALUE_BASE + i16_offset * 2 + 1) as usize] =
                (eng_bits & 0xFFFF) as u16;

            // Channel status
            self.input_regs[(input_reg::STATUS_BASE + i16_offset) as usize] =
                r.status as u16;

            // Alarm bitmap
            if r.status != ChannelStatus::Normal {
                alarm_bits |= 1 << i;
            }
        }

        self.input_regs[input_reg::ALARM_BITMAP_LO as usize] = alarm_bits;
    }

    /// Update uptime register
    pub fn update_uptime(&mut self, seconds: u32) {
        self.input_regs[input_reg::UPTIME_BASE as usize] = (seconds >> 16) as u16;
        self.input_regs[(input_reg::UPTIME_BASE + 1) as usize] = (seconds & 0xFFFF) as u16;
    }

    /// Update device status flags
    pub fn set_device_status(&mut self, flags: u16) {
        self.input_regs[input_reg::DEVICE_STATUS as usize] = flags;
    }

    /// Load calibrations from holding registers into ChannelCalibration array
    pub fn get_calibrations(&self) -> [ChannelCalibration; Channel::COUNT] {
        let mut cals = crate::drivers::ads1258::DEFAULT_CALIBRATIONS;

        for i in 0..Channel::COUNT {
            let i16_offset = i as u16;

            let zero_hi = self.holding_regs[(holding_reg::CAL_ZERO_BASE + i16_offset * 2) as usize];
            let zero_lo = self.holding_regs[(holding_reg::CAL_ZERO_BASE + i16_offset * 2 + 1) as usize];
            cals[i].zero_code = ((zero_hi as u32) << 16 | zero_lo as u32) as i32;

            let span_hi = self.holding_regs[(holding_reg::CAL_SPAN_BASE + i16_offset * 2) as usize];
            let span_lo = self.holding_regs[(holding_reg::CAL_SPAN_BASE + i16_offset * 2 + 1) as usize];
            cals[i].span_code = ((span_hi as u32) << 16 | span_lo as u32) as i32;

            let min_hi = self.holding_regs[(holding_reg::ENG_MIN_BASE + i16_offset * 2) as usize];
            let min_lo = self.holding_regs[(holding_reg::ENG_MIN_BASE + i16_offset * 2 + 1) as usize];
            cals[i].eng_min = f32::from_bits((min_hi as u32) << 16 | min_lo as u32);

            let max_hi = self.holding_regs[(holding_reg::ENG_MAX_BASE + i16_offset * 2) as usize];
            let max_lo = self.holding_regs[(holding_reg::ENG_MAX_BASE + i16_offset * 2 + 1) as usize];
            cals[i].eng_max = f32::from_bits((max_hi as u32) << 16 | max_lo as u32);
        }

        cals
    }

    /// Write default calibrations into holding registers
    pub fn load_default_calibrations(&mut self) {
        let defaults = crate::drivers::ads1258::DEFAULT_CALIBRATIONS;

        for i in 0..Channel::COUNT {
            let off = i as u16;
            let zero = defaults[i].zero_code as u32;
            self.holding_regs[(holding_reg::CAL_ZERO_BASE + off * 2) as usize] = (zero >> 16) as u16;
            self.holding_regs[(holding_reg::CAL_ZERO_BASE + off * 2 + 1) as usize] = (zero & 0xFFFF) as u16;

            let span = defaults[i].span_code as u32;
            self.holding_regs[(holding_reg::CAL_SPAN_BASE + off * 2) as usize] = (span >> 16) as u16;
            self.holding_regs[(holding_reg::CAL_SPAN_BASE + off * 2 + 1) as usize] = (span & 0xFFFF) as u16;

            let min_bits = defaults[i].eng_min.to_bits();
            self.holding_regs[(holding_reg::ENG_MIN_BASE + off * 2) as usize] = (min_bits >> 16) as u16;
            self.holding_regs[(holding_reg::ENG_MIN_BASE + off * 2 + 1) as usize] = (min_bits & 0xFFFF) as u16;

            let max_bits = defaults[i].eng_max.to_bits();
            self.holding_regs[(holding_reg::ENG_MAX_BASE + off * 2) as usize] = (max_bits >> 16) as u16;
            self.holding_regs[(holding_reg::ENG_MAX_BASE + off * 2 + 1) as usize] = (max_bits & 0xFFFF) as u16;
        }

        // Default comm settings
        self.holding_regs[holding_reg::SLAVE_ADDR as usize] = 1;    // Slave address 1
        self.holding_regs[holding_reg::BAUD_RATE as usize] = 9600;  // 9600 baud
        self.holding_regs[holding_reg::IP_ADDR_BASE as usize] = 192;
        self.holding_regs[(holding_reg::IP_ADDR_BASE + 1) as usize] = 168;
        self.holding_regs[(holding_reg::IP_ADDR_BASE + 2) as usize] = 1;
        self.holding_regs[(holding_reg::IP_ADDR_BASE + 3) as usize] = 100;
        self.holding_regs[holding_reg::ALARM_ENABLE as usize] = 0xFF; // All channels alarm enabled
    }
}
