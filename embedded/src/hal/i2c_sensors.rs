/// I2C sensor drivers for STM32F407VGT6
///
/// I2C1 bus: PB6 = SCL, PB7 = SDA
///   - BME280: Temperature + Humidity + Barometric pressure (addr 0x76/0x77)
///   - INA219: DC current/voltage sensor for panel monitoring (addr 0x40)
///   - TMP117: High-accuracy temperature sensor +/-0.1C (addr 0x48-0x4B)

use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;
use defmt::{info, warn, error, Format};

// ============================================================================
// Error types
// ============================================================================

/// I2C sensor error types
#[derive(Clone, Copy, Debug, Format)]
pub enum I2cSensorError {
    /// I2C bus communication failed (NACK, bus error, timeout)
    BusError,
    /// Device did not respond (wrong address or not present)
    DeviceNotFound,
    /// Unexpected chip ID or WHO_AM_I value
    WrongDeviceId,
    /// Data not ready or conversion in progress
    DataNotReady,
    /// Configuration write failed
    ConfigError,
    /// Calibration data invalid or out of range
    CalibrationError,
    /// Temperature reading out of sensor range
    OutOfRange,
}

// ============================================================================
// TMP117 High-Accuracy Temperature Sensor
// ============================================================================

/// TMP117 I2C addresses (determined by ADD0 pin)
///   ADD0 = GND:  0x48
///   ADD0 = V+:   0x49
///   ADD0 = SDA:  0x4A
///   ADD0 = SCL:  0x4B
const TMP117_DEFAULT_ADDR: u8 = 0x48;

/// TMP117 register addresses
mod tmp117_regs {
    /// Temperature result register (16-bit, read-only)
    pub const TEMP_RESULT: u8 = 0x00;
    /// Configuration register (16-bit, R/W)
    pub const CONFIGURATION: u8 = 0x01;
    /// High limit register (16-bit, R/W)
    pub const T_HIGH_LIMIT: u8 = 0x02;
    /// Low limit register (16-bit, R/W)
    pub const T_LOW_LIMIT: u8 = 0x03;
    /// EEPROM unlock register (16-bit, R/W)
    pub const EEPROM_UL: u8 = 0x04;
    /// EEPROM1 register (16-bit, R/W)
    pub const EEPROM1: u8 = 0x05;
    /// EEPROM2 register (16-bit, R/W)
    pub const EEPROM2: u8 = 0x06;
    /// Temperature offset register (16-bit, R/W)
    pub const TEMP_OFFSET: u8 = 0x07;
    /// EEPROM3 register (16-bit, R/W)
    pub const EEPROM3: u8 = 0x08;
    /// Device ID register (16-bit, read-only)
    pub const DEVICE_ID: u8 = 0x0F;
}

/// TMP117 expected device ID
const TMP117_DEVICE_ID: u16 = 0x0117;

/// TMP117 configuration register bits
mod tmp117_config {
    /// High alert flag (bit 15, read-only)
    pub const HIGH_ALERT: u16 = 1 << 15;
    /// Low alert flag (bit 14, read-only)
    pub const LOW_ALERT: u16 = 1 << 14;
    /// Data ready flag (bit 13, read-only)
    pub const DATA_READY: u16 = 1 << 13;
    /// EEPROM busy flag (bit 12, read-only)
    pub const EEPROM_BUSY: u16 = 1 << 12;

    /// Conversion mode bits [11:10]
    pub const MODE_SHIFT: u16 = 10;
    pub const MODE_CONTINUOUS: u16 = 0b00 << 10;
    pub const MODE_SHUTDOWN: u16 = 0b01 << 10;
    pub const MODE_ONE_SHOT: u16 = 0b11 << 10;

    /// Conversion cycle bits [9:7] - sets the conversion averaging
    pub const AVG_SHIFT: u16 = 7;
    pub const AVG_NONE: u16 = 0b000 << 7;   // No averaging, 15.5ms
    pub const AVG_8: u16 = 0b001 << 7;      // 8 averages, 125ms
    pub const AVG_32: u16 = 0b010 << 7;     // 32 averages, 500ms
    pub const AVG_64: u16 = 0b011 << 7;     // 64 averages, 1s

    /// Conversion cycle time bits [6:4]
    pub const CONV_SHIFT: u16 = 4;
    pub const CONV_15_5MS: u16 = 0b000 << 4;
    pub const CONV_125MS: u16 = 0b001 << 4;
    pub const CONV_250MS: u16 = 0b010 << 4;
    pub const CONV_500MS: u16 = 0b011 << 4;
    pub const CONV_1S: u16 = 0b100 << 4;
    pub const CONV_4S: u16 = 0b101 << 4;
    pub const CONV_8S: u16 = 0b110 << 4;
    pub const CONV_16S: u16 = 0b111 << 4;

    /// DR/Alert pin mode (bit 2)
    pub const DR_ALERT: u16 = 1 << 2;
    /// Alert pin polarity (bit 3)
    pub const POL: u16 = 1 << 3;
    /// Soft reset (bit 1)
    pub const SOFT_RESET: u16 = 1 << 1;
}

/// TMP117 averaging mode
#[derive(Clone, Copy, PartialEq, Format)]
pub enum Tmp117Averaging {
    /// No averaging (15.5ms conversion)
    None,
    /// 8-sample averaging (125ms)
    Avg8,
    /// 32-sample averaging (500ms)
    Avg32,
    /// 64-sample averaging (1s) - best accuracy
    Avg64,
}

/// TMP117 conversion mode
#[derive(Clone, Copy, PartialEq, Format)]
pub enum Tmp117Mode {
    /// Continuous conversion
    Continuous,
    /// Shutdown (low power)
    Shutdown,
    /// Single-shot then shutdown
    OneShot,
}

/// TMP117 high-accuracy temperature sensor driver
///
/// The TMP117 provides +/-0.1 degC accuracy from -20 to +50 degC
/// with 16-bit resolution (0.0078125 degC per LSB).
///
/// Features:
///   - Programmable averaging (1/8/32/64 samples)
///   - Alert thresholds with interrupt output
///   - EEPROM for storing calibration offsets
///   - 0.0078125 degC resolution
pub struct Tmp117 {
    addr: u8,
    config: u16,
}

impl Tmp117 {
    /// Create a new TMP117 driver with the given I2C address
    pub fn new(addr: u8) -> Self {
        Self {
            addr,
            config: 0,
        }
    }

    /// Create a TMP117 driver with default address (0x48)
    pub fn new_default() -> Self {
        Self::new(TMP117_DEFAULT_ADDR)
    }

    /// Initialize the TMP117: verify device ID and configure
    pub async fn init(
        &mut self,
        i2c: &mut I2c<'_, Async>,
        averaging: Tmp117Averaging,
        mode: Tmp117Mode,
    ) -> Result<(), I2cSensorError> {
        // Verify device ID
        let device_id = self.read_register(i2c, tmp117_regs::DEVICE_ID).await?;
        if device_id != TMP117_DEVICE_ID {
            warn!("TMP117: unexpected device ID 0x{:04X}, expected 0x{:04X}", device_id, TMP117_DEVICE_ID);
            return Err(I2cSensorError::WrongDeviceId);
        }
        info!("TMP117: device ID verified (0x{:04X})", device_id);

        // Perform soft reset
        self.write_register(i2c, tmp117_regs::CONFIGURATION, tmp117_config::SOFT_RESET).await?;
        // Wait for reset to complete (~2ms)
        embassy_time::Timer::after_millis(5).await;

        // Build configuration
        let avg_bits = match averaging {
            Tmp117Averaging::None => tmp117_config::AVG_NONE,
            Tmp117Averaging::Avg8 => tmp117_config::AVG_8,
            Tmp117Averaging::Avg32 => tmp117_config::AVG_32,
            Tmp117Averaging::Avg64 => tmp117_config::AVG_64,
        };

        let mode_bits = match mode {
            Tmp117Mode::Continuous => tmp117_config::MODE_CONTINUOUS,
            Tmp117Mode::Shutdown => tmp117_config::MODE_SHUTDOWN,
            Tmp117Mode::OneShot => tmp117_config::MODE_ONE_SHOT,
        };

        // Set conversion time to match averaging
        let conv_bits = match averaging {
            Tmp117Averaging::None => tmp117_config::CONV_15_5MS,
            Tmp117Averaging::Avg8 => tmp117_config::CONV_125MS,
            Tmp117Averaging::Avg32 => tmp117_config::CONV_500MS,
            Tmp117Averaging::Avg64 => tmp117_config::CONV_1S,
        };

        self.config = avg_bits | mode_bits | conv_bits;
        self.write_register(i2c, tmp117_regs::CONFIGURATION, self.config).await?;

        info!("TMP117: configured (avg={}, mode=0x{:04X})", averaging, self.config);
        Ok(())
    }

    /// Read the current temperature in degrees Celsius
    ///
    /// Returns +/-0.1 degC accuracy in the -20 to +50 degC range.
    /// Resolution is 0.0078125 degC per LSB (128 LSB per degree).
    pub async fn read_temperature(&mut self, i2c: &mut I2c<'_, Async>) -> Result<f32, I2cSensorError> {
        // Check if data is ready
        let config = self.read_register(i2c, tmp117_regs::CONFIGURATION).await?;
        if config & tmp117_config::DATA_READY == 0 {
            return Err(I2cSensorError::DataNotReady);
        }

        let raw = self.read_register(i2c, tmp117_regs::TEMP_RESULT).await?;
        let temp_c = (raw as i16) as f32 * 0.0078125;

        // Sanity check: TMP117 range is -55 to +150 degC
        if temp_c < -55.0 || temp_c > 150.0 {
            return Err(I2cSensorError::OutOfRange);
        }

        Ok(temp_c)
    }

    /// Read temperature, waiting for data ready if needed
    pub async fn read_temperature_blocking(
        &mut self,
        i2c: &mut I2c<'_, Async>,
    ) -> Result<f32, I2cSensorError> {
        // Poll for data ready with timeout
        for _ in 0..200 {
            match self.read_temperature(i2c).await {
                Ok(temp) => return Ok(temp),
                Err(I2cSensorError::DataNotReady) => {
                    embassy_time::Timer::after_millis(10).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        Err(I2cSensorError::DataNotReady)
    }

    /// Set high temperature alert threshold (degrees C)
    pub async fn set_high_alert(
        &mut self,
        i2c: &mut I2c<'_, Async>,
        threshold_c: f32,
    ) -> Result<(), I2cSensorError> {
        let raw = (threshold_c / 0.0078125) as i16;
        self.write_register(i2c, tmp117_regs::T_HIGH_LIMIT, raw as u16).await
    }

    /// Set low temperature alert threshold (degrees C)
    pub async fn set_low_alert(
        &mut self,
        i2c: &mut I2c<'_, Async>,
        threshold_c: f32,
    ) -> Result<(), I2cSensorError> {
        let raw = (threshold_c / 0.0078125) as i16;
        self.write_register(i2c, tmp117_regs::T_LOW_LIMIT, raw as u16).await
    }

    /// Check if high temperature alert is active
    pub async fn is_high_alert(&mut self, i2c: &mut I2c<'_, Async>) -> Result<bool, I2cSensorError> {
        let config = self.read_register(i2c, tmp117_regs::CONFIGURATION).await?;
        Ok(config & tmp117_config::HIGH_ALERT != 0)
    }

    /// Check if low temperature alert is active
    pub async fn is_low_alert(&mut self, i2c: &mut I2c<'_, Async>) -> Result<bool, I2cSensorError> {
        let config = self.read_register(i2c, tmp117_regs::CONFIGURATION).await?;
        Ok(config & tmp117_config::LOW_ALERT != 0)
    }

    /// Set the temperature offset for calibration (stored in EEPROM)
    pub async fn set_offset(
        &mut self,
        i2c: &mut I2c<'_, Async>,
        offset_c: f32,
    ) -> Result<(), I2cSensorError> {
        let raw = (offset_c / 0.0078125) as i16;
        self.write_register(i2c, tmp117_regs::TEMP_OFFSET, raw as u16).await
    }

    /// Trigger a one-shot conversion (device must be in one-shot or shutdown mode)
    pub async fn trigger_one_shot(
        &mut self,
        i2c: &mut I2c<'_, Async>,
    ) -> Result<(), I2cSensorError> {
        let config = self.config | tmp117_config::MODE_ONE_SHOT;
        self.write_register(i2c, tmp117_regs::CONFIGURATION, config).await
    }

    /// Set the conversion mode
    pub async fn set_mode(
        &mut self,
        i2c: &mut I2c<'_, Async>,
        mode: Tmp117Mode,
    ) -> Result<(), I2cSensorError> {
        // Clear mode bits and set new ones
        let mode_bits = match mode {
            Tmp117Mode::Continuous => tmp117_config::MODE_CONTINUOUS,
            Tmp117Mode::Shutdown => tmp117_config::MODE_SHUTDOWN,
            Tmp117Mode::OneShot => tmp117_config::MODE_ONE_SHOT,
        };
        self.config = (self.config & !(0b11 << 10)) | mode_bits;
        self.write_register(i2c, tmp117_regs::CONFIGURATION, self.config).await
    }

    /// Set the averaging mode
    pub async fn set_averaging(
        &mut self,
        i2c: &mut I2c<'_, Async>,
        avg: Tmp117Averaging,
    ) -> Result<(), I2cSensorError> {
        let avg_bits = match avg {
            Tmp117Averaging::None => tmp117_config::AVG_NONE,
            Tmp117Averaging::Avg8 => tmp117_config::AVG_8,
            Tmp117Averaging::Avg32 => tmp117_config::AVG_32,
            Tmp117Averaging::Avg64 => tmp117_config::AVG_64,
        };
        self.config = (self.config & !(0b111 << 7)) | avg_bits;
        self.write_register(i2c, tmp117_regs::CONFIGURATION, self.config).await
    }

    // ---- Low-level I2C register access ----

    async fn read_register(
        &mut self,
        i2c: &mut I2c<'_, Async>,
        reg: u8,
    ) -> Result<u16, I2cSensorError> {
        let mut buf = [0u8; 2];
        i2c.write_read(self.addr, &[reg], &mut buf)
            .await
            .map_err(|_| I2cSensorError::BusError)?;
        Ok(u16::from_be_bytes(buf))
    }

    async fn write_register(
        &mut self,
        i2c: &mut I2c<'_, Async>,
        reg: u8,
        value: u16,
    ) -> Result<(), I2cSensorError> {
        let bytes = value.to_be_bytes();
        i2c.write(self.addr, &[reg, bytes[0], bytes[1]])
            .await
            .map_err(|_| I2cSensorError::BusError)
    }
}

// ============================================================================
// BME280 Temperature/Humidity/Pressure Sensor
// ============================================================================

const BME280_ADDR: u8 = 0x76;
const BME280_REG_ID: u8 = 0xD0;
const BME280_REG_CTRL_HUM: u8 = 0xF2;
const BME280_REG_CTRL_MEAS: u8 = 0xF4;
const BME280_REG_CONFIG: u8 = 0xF5;
const BME280_REG_DATA: u8 = 0xF7;
const BME280_REG_CALIB_00: u8 = 0x88;
const BME280_REG_CALIB_26: u8 = 0xE1;

const BME280_CHIP_ID: u8 = 0x60;

/// BME280 calibration data (from registers 0x88..0xA1 and 0xE1..0xE7)
#[derive(Default)]
pub struct Bme280Calibration {
    pub dig_t1: u16,
    pub dig_t2: i16,
    pub dig_t3: i16,
    pub dig_p1: u16,
    pub dig_p2: i16,
    pub dig_p3: i16,
    pub dig_p4: i16,
    pub dig_p5: i16,
    pub dig_p6: i16,
    pub dig_p7: i16,
    pub dig_p8: i16,
    pub dig_p9: i16,
    pub dig_h1: u8,
    pub dig_h2: i16,
    pub dig_h3: u8,
    pub dig_h4: i16,
    pub dig_h5: i16,
    pub dig_h6: i8,
}

/// Compensated BME280 readings
pub struct EnvReading {
    pub temperature_c: f32,
    pub humidity_pct: f32,
    pub pressure_hpa: f32,
}

/// Initialize BME280 and read calibration data
pub async fn bme280_init(i2c: &mut I2c<'_, Async>) -> Option<Bme280Calibration> {
    // Verify chip ID
    let mut id = [0u8; 1];
    if i2c.write_read(BME280_ADDR, &[BME280_REG_ID], &mut id).await.is_err() {
        warn!("BME280: I2C read failed");
        return None;
    }
    if id[0] != BME280_CHIP_ID {
        warn!("BME280: unexpected chip ID 0x{:02X}", id[0]);
        return None;
    }

    // Read calibration registers 0x88..0xA1 (26 bytes)
    let mut cal1 = [0u8; 26];
    if i2c.write_read(BME280_ADDR, &[BME280_REG_CALIB_00], &mut cal1).await.is_err() {
        return None;
    }

    // Read calibration registers 0xE1..0xE7 (7 bytes)
    let mut cal2 = [0u8; 7];
    if i2c.write_read(BME280_ADDR, &[BME280_REG_CALIB_26], &mut cal2).await.is_err() {
        return None;
    }

    let cal = Bme280Calibration {
        dig_t1: u16::from_le_bytes([cal1[0], cal1[1]]),
        dig_t2: i16::from_le_bytes([cal1[2], cal1[3]]),
        dig_t3: i16::from_le_bytes([cal1[4], cal1[5]]),
        dig_p1: u16::from_le_bytes([cal1[6], cal1[7]]),
        dig_p2: i16::from_le_bytes([cal1[8], cal1[9]]),
        dig_p3: i16::from_le_bytes([cal1[10], cal1[11]]),
        dig_p4: i16::from_le_bytes([cal1[12], cal1[13]]),
        dig_p5: i16::from_le_bytes([cal1[14], cal1[15]]),
        dig_p6: i16::from_le_bytes([cal1[16], cal1[17]]),
        dig_p7: i16::from_le_bytes([cal1[18], cal1[19]]),
        dig_p8: i16::from_le_bytes([cal1[20], cal1[21]]),
        dig_p9: i16::from_le_bytes([cal1[22], cal1[23]]),
        dig_h1: cal1[25],
        dig_h2: i16::from_le_bytes([cal2[0], cal2[1]]),
        dig_h3: cal2[2],
        dig_h4: ((cal2[3] as i16) << 4) | ((cal2[4] as i16) & 0x0F),
        dig_h5: ((cal2[5] as i16) << 4) | ((cal2[4] as i16) >> 4),
        dig_h6: cal2[6] as i8,
    };

    // Configure: humidity oversampling x1
    let _ = i2c.write(BME280_ADDR, &[BME280_REG_CTRL_HUM, 0x01]).await;
    // Configure: temp oversampling x2, pressure oversampling x16, normal mode
    let _ = i2c.write(BME280_ADDR, &[BME280_REG_CTRL_MEAS, 0x57]).await;
    // Configure: standby 500ms, filter coeff 4
    let _ = i2c.write(BME280_ADDR, &[BME280_REG_CONFIG, 0x90]).await;

    Some(cal)
}

/// Read compensated temperature, humidity, and pressure from BME280
pub async fn bme280_read(
    i2c: &mut I2c<'_, Async>,
    cal: &Bme280Calibration,
) -> Option<EnvReading> {
    // Read 8 bytes: pressure[2:0], temperature[2:0], humidity[1:0]
    let mut data = [0u8; 8];
    if i2c.write_read(BME280_ADDR, &[BME280_REG_DATA], &mut data).await.is_err() {
        return None;
    }

    let adc_p = ((data[0] as i32) << 12) | ((data[1] as i32) << 4) | ((data[2] as i32) >> 4);
    let adc_t = ((data[3] as i32) << 12) | ((data[4] as i32) << 4) | ((data[5] as i32) >> 4);
    let adc_h = ((data[6] as i32) << 8) | (data[7] as i32);

    // Temperature compensation (Bosch datasheet algorithm)
    let var1 = (((adc_t >> 3) - ((cal.dig_t1 as i32) << 1)) * (cal.dig_t2 as i32)) >> 11;
    let var2 = (((((adc_t >> 4) - (cal.dig_t1 as i32))
        * ((adc_t >> 4) - (cal.dig_t1 as i32))) >> 12)
        * (cal.dig_t3 as i32)) >> 14;
    let t_fine = var1 + var2;
    let temperature_c = ((t_fine * 5 + 128) >> 8) as f32 / 100.0;

    // Pressure compensation
    let mut var1_p = (t_fine as i64) - 128000;
    let mut var2_p = var1_p * var1_p * (cal.dig_p6 as i64);
    var2_p += (var1_p * (cal.dig_p5 as i64)) << 17;
    var2_p += (cal.dig_p4 as i64) << 35;
    var1_p = ((var1_p * var1_p * (cal.dig_p3 as i64)) >> 8)
        + ((var1_p * (cal.dig_p2 as i64)) << 12);
    var1_p = (((1i64 << 47) + var1_p) * (cal.dig_p1 as i64)) >> 33;

    let pressure_hpa = if var1_p == 0 {
        0.0
    } else {
        let mut p: i64 = 1048576 - adc_p as i64;
        p = (((p << 31) - var2_p) * 3125) / var1_p;
        let v1 = ((cal.dig_p9 as i64) * (p >> 13) * (p >> 13)) >> 25;
        let v2 = ((cal.dig_p8 as i64) * p) >> 19;
        p = ((p + v1 + v2) >> 8) + ((cal.dig_p7 as i64) << 4);
        (p as f32) / 25600.0
    };

    // Humidity compensation
    let mut h = t_fine - 76800i32;
    if h == 0 {
        return Some(EnvReading { temperature_c, humidity_pct: 0.0, pressure_hpa });
    }
    h = ((((adc_h << 14) - ((cal.dig_h4 as i32) << 20) - ((cal.dig_h5 as i32) * h))
        + 16384) >> 15)
        * (((((((h * (cal.dig_h6 as i32)) >> 10)
            * (((h * (cal.dig_h3 as i32)) >> 11) + 32768)) >> 10)
            + 2097152) * (cal.dig_h2 as i32) + 8192) >> 14);
    h -= ((((h >> 15) * (h >> 15)) >> 7) * (cal.dig_h1 as i32)) >> 4;
    h = if h < 0 { 0 } else { h };
    h = if h > 419430400 { 419430400 } else { h };
    let humidity_pct = (h >> 12) as f32 / 1024.0;

    Some(EnvReading { temperature_c, humidity_pct, pressure_hpa })
}

// ============================================================================
// INA219 Current/Voltage Sensor (Solar Panel Monitoring)
// ============================================================================

const INA219_ADDR: u8 = 0x40;
const INA219_REG_CONFIG: u8 = 0x00;
const INA219_REG_SHUNT_VOLTAGE: u8 = 0x01;
const INA219_REG_BUS_VOLTAGE: u8 = 0x02;
const INA219_REG_CURRENT: u8 = 0x04;
const INA219_REG_CALIBRATION: u8 = 0x05;

/// INA219 readings
pub struct PowerReading {
    pub bus_voltage_v: f32,
    pub shunt_voltage_mv: f32,
    pub current_ma: f32,
    pub power_mw: f32,
}

/// Initialize INA219 for solar panel monitoring
/// Assumes 0.1 Ohm shunt resistor, 32V bus, max 3.2A
pub async fn ina219_init(i2c: &mut I2c<'_, Async>) -> bool {
    // Config: 32V range, +/-320mV shunt, 12-bit, continuous both
    let config: u16 = 0x399F;
    let config_bytes = config.to_be_bytes();
    if i2c.write(INA219_ADDR, &[INA219_REG_CONFIG, config_bytes[0], config_bytes[1]]).await.is_err() {
        warn!("INA219: config write failed");
        return false;
    }

    // Calibration value for 0.1 Ohm shunt:
    // Cal = trunc(0.04096 / (current_lsb * r_shunt))
    // current_lsb = max_expected_current / 2^15 = 3.2 / 32768 ~= 0.0001
    // Cal = trunc(0.04096 / (0.0001 * 0.1)) = 4096
    let cal: u16 = 4096;
    let cal_bytes = cal.to_be_bytes();
    if i2c.write(INA219_ADDR, &[INA219_REG_CALIBRATION, cal_bytes[0], cal_bytes[1]]).await.is_err() {
        warn!("INA219: calibration write failed");
        return false;
    }

    true
}

/// Read current, voltage, and power from INA219
pub async fn ina219_read(i2c: &mut I2c<'_, Async>) -> Option<PowerReading> {
    let mut buf = [0u8; 2];

    // Bus voltage (register 0x02)
    if i2c.write_read(INA219_ADDR, &[INA219_REG_BUS_VOLTAGE], &mut buf).await.is_err() {
        return None;
    }
    let raw_bus = i16::from_be_bytes(buf);
    let bus_voltage_v = ((raw_bus >> 3) as f32) * 0.004; // 4mV per LSB

    // Shunt voltage (register 0x01)
    if i2c.write_read(INA219_ADDR, &[INA219_REG_SHUNT_VOLTAGE], &mut buf).await.is_err() {
        return None;
    }
    let raw_shunt = i16::from_be_bytes(buf);
    let shunt_voltage_mv = (raw_shunt as f32) * 0.01; // 10uV per LSB

    // Current (register 0x04)
    if i2c.write_read(INA219_ADDR, &[INA219_REG_CURRENT], &mut buf).await.is_err() {
        return None;
    }
    let raw_current = i16::from_be_bytes(buf);
    let current_ma = (raw_current as f32) * 0.1; // current_lsb = 0.1mA

    let power_mw = bus_voltage_v * current_ma;

    Some(PowerReading {
        bus_voltage_v,
        shunt_voltage_mv,
        current_ma,
        power_mw,
    })
}
