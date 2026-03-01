/// TMP117 — High-Accuracy Digital Temperature Sensor Driver
///
/// Datasheet: Texas Instruments SNAS749
///
/// The TMP117 provides:
///   - ±0.1°C accuracy from -20°C to +50°C
///   - 16-bit resolution (0.0078125°C per LSB)
///   - I2C interface (up to 400 kHz)
///   - Programmable alert thresholds
///
/// On SS-PCB-001:
///   I2C1_SCL = PB6
///   I2C1_SDA = PB7
///   TMP117 address = 0x48 (ADD0 pin to GND)
///
/// Used for ambient/module temperature measurement for solar performance ratio.

use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::{info, warn, error};

/// TMP117 I2C address (ADD0 = GND)
const TMP117_ADDR: u8 = 0x48;

/// TMP117 register addresses
mod reg {
    pub const TEMP_RESULT: u8 = 0x00;   // Temperature result (16-bit, read-only)
    pub const CONFIGURATION: u8 = 0x01; // Configuration register
    pub const T_HIGH_LIMIT: u8 = 0x02;  // High temperature alert threshold
    pub const T_LOW_LIMIT: u8 = 0x03;   // Low temperature alert threshold
    pub const EEPROM_UL: u8 = 0x04;     // EEPROM unlock register
    pub const EEPROM1: u8 = 0x05;       // EEPROM1 data
    pub const EEPROM2: u8 = 0x06;       // EEPROM2 data
    pub const TEMP_OFFSET: u8 = 0x07;   // Temperature offset register
    pub const EEPROM3: u8 = 0x08;       // EEPROM3 data
    pub const DEVICE_ID: u8 = 0x0F;     // Device ID (should read 0x0117)
}

/// Configuration bits
mod config {
    pub const CONV_CYCLE_1S: u16 = 0x0600;  // 1 second conversion cycle
    pub const MOD_CC: u16 = 0x0000;          // Continuous conversion mode
    pub const AVG_8: u16 = 0x0020;           // 8x averaging
    pub const DR_ALERT: u16 = 0x0004;        // Data ready on ALERT pin
}

/// Temperature reading
#[derive(Clone, Copy, Default)]
pub struct TemperatureReading {
    pub raw: i16,
    pub celsius: f32,
    pub valid: bool,
}

/// TMP117 driver
pub struct Tmp117<'a> {
    i2c: I2c<'a, Async>,
    offset: f32,
}

impl<'a> Tmp117<'a> {
    pub fn new(i2c: I2c<'a, Async>) -> Self {
        Self { i2c, offset: 0.0 }
    }

    /// Set temperature offset for calibration
    pub fn set_offset(&mut self, offset_celsius: f32) {
        self.offset = offset_celsius;
    }

    /// Initialize the TMP117
    pub async fn init(&mut self) -> bool {
        info!("TMP117: Initializing...");

        // Read device ID
        let id = self.read_register(reg::DEVICE_ID).await;
        if id != 0x0117 {
            error!("TMP117: ID mismatch (got 0x{:04X}, expected 0x0117)", id);
            return false;
        }

        // Configure: continuous conversion, 1s cycle, 8x averaging
        let config_val = config::MOD_CC | config::CONV_CYCLE_1S | config::AVG_8;
        self.write_register(reg::CONFIGURATION, config_val).await;

        // Set alert thresholds for solar panel monitoring
        // High alert: 85°C (module overtemp)
        let high_raw = (85.0f32 / 0.0078125) as i16;
        self.write_register(reg::T_HIGH_LIMIT, high_raw as u16).await;

        // Low alert: -40°C (freeze protection)
        let low_raw = (-40.0f32 / 0.0078125) as i16;
        self.write_register(reg::T_LOW_LIMIT, low_raw as u16).await;

        info!("TMP117: Initialized, continuous 1s cycle, 8x avg");
        true
    }

    /// Read current temperature
    pub async fn read_temperature(&mut self) -> TemperatureReading {
        let raw = self.read_register(reg::TEMP_RESULT).await as i16;

        // Convert: 1 LSB = 0.0078125°C
        let celsius = raw as f32 * 0.0078125 + self.offset;

        TemperatureReading {
            raw,
            celsius,
            valid: true,
        }
    }

    /// Check if data is ready (new conversion complete)
    pub async fn data_ready(&mut self) -> bool {
        let config = self.read_register(reg::CONFIGURATION).await;
        (config & 0x2000) != 0 // Bit 13 = Data Ready
    }

    async fn read_register(&mut self, reg: u8) -> u16 {
        let mut buf = [0u8; 2];
        match self.i2c.write_read(TMP117_ADDR, &[reg], &mut buf).await {
            Ok(()) => u16::from_be_bytes(buf),
            Err(_) => {
                warn!("TMP117: I2C read error at reg 0x{:02X}", reg);
                0
            }
        }
    }

    async fn write_register(&mut self, reg: u8, value: u16) {
        let bytes = value.to_be_bytes();
        let buf = [reg, bytes[0], bytes[1]];
        if let Err(_) = self.i2c.write(TMP117_ADDR, &buf).await {
            warn!("TMP117: I2C write error at reg 0x{:02X}", reg);
        }
    }
}
