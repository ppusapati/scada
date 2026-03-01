/// ADC driver for SCADA sensor nodes
///
/// Supports two external high-resolution ADCs over SPI:
///   - ADS1263: 32-bit delta-sigma for Solar SMU (high precision voltage/current)
///   - ADS1258: 24-bit SAR for Water RTU (multi-channel scanning)
///
/// Also supports the internal STM32F407 12-bit ADC for auxiliary channels.
///
/// Signal conditioning:
///   4-20mA sensors -> 250 Ohm resistor -> 1.0V-5.0V -> voltage divider -> 0-3.3V ADC range
///   0-10V sensors  -> voltage divider (3:1) -> 0-3.3V ADC range

#[cfg(not(feature = "simulator"))]
use embassy_stm32::adc::{Adc, SampleTime};
#[cfg(not(feature = "simulator"))]
use embassy_stm32::peripherals;
use defmt::{info, warn, error, Format};

// ============================================================================
// Error types
// ============================================================================

/// ADC subsystem errors
#[derive(Clone, Copy, Debug, Format)]
pub enum AdcError {
    /// SPI communication failure
    SpiError,
    /// ADC did not respond to initialization
    InitFailed,
    /// Data not ready within timeout
    Timeout,
    /// Calibration data out of range
    CalibrationError,
    /// Channel index out of bounds
    InvalidChannel,
    /// Checksum mismatch on ADC data frame
    ChecksumError,
    /// ADC reported internal fault via status byte
    DeviceFault,
    /// DMA transfer error
    DmaError,
    /// Reading outside valid engineering range (broken wire / saturated)
    OutOfRange,
}

// ============================================================================
// Calibration
// ============================================================================

/// Raw ADC reading to engineering unit conversion factors
#[derive(Clone, Copy)]
pub struct AdcCalibration {
    pub offset: f32,     // Zero offset in ADC counts
    pub scale: f32,      // Counts-to-engineering-unit scale factor
    pub min_valid: f32,  // Minimum valid engineering value (fault detection)
    pub max_valid: f32,  // Maximum valid engineering value
}

impl AdcCalibration {
    /// Convert a raw ADC count to an engineering unit value
    pub fn convert(&self, raw: u16) -> f32 {
        (raw as f32 - self.offset) * self.scale
    }

    /// Convert a high-resolution (32-bit) raw count to engineering units
    pub fn convert_i32(&self, raw: i32) -> f32 {
        (raw as f32 - self.offset) * self.scale
    }

    /// Check if a converted engineering value is within the valid range
    pub fn is_valid(&self, value: f32) -> bool {
        value >= self.min_valid && value <= self.max_valid
    }

    /// Apply a two-point calibration adjustment
    /// low_raw/low_eng: reading at known low reference
    /// high_raw/high_eng: reading at known high reference
    pub fn two_point_calibrate(
        &mut self,
        low_raw: f32,
        low_eng: f32,
        high_raw: f32,
        high_eng: f32,
    ) -> Result<(), AdcError> {
        let denom = high_raw - low_raw;
        if denom.abs() < 1.0 {
            return Err(AdcError::CalibrationError);
        }
        self.scale = (high_eng - low_eng) / denom;
        self.offset = low_raw - (low_eng / self.scale);
        Ok(())
    }
}

// ============================================================================
// Calibration constants: Water sensors (4-20mA into 250 Ohm = 1V-5V range)
// STM32 ADC 12-bit: 0-4095 maps to 0-3.3V
// With voltage divider 5V->3.3V: 4mA=~800 counts, 20mA=~4000 counts
// ============================================================================

pub static PRESSURE_CAL: AdcCalibration = AdcCalibration {
    offset: 800.0,
    scale: 0.00313,   // (10.0 bar) / (4000-800) counts
    min_valid: -0.5,
    max_valid: 12.0,   // bar
};

pub static PH_CAL: AdcCalibration = AdcCalibration {
    offset: 800.0,
    scale: 0.00438,   // (14.0 pH) / 3200 counts
    min_valid: 0.0,
    max_valid: 14.0,
};

pub static TURBIDITY_CAL: AdcCalibration = AdcCalibration {
    offset: 800.0,
    scale: 0.00313,   // (10.0 NTU) / 3200
    min_valid: 0.0,
    max_valid: 15.0,   // NTU
};

pub static CHLORINE_CAL: AdcCalibration = AdcCalibration {
    offset: 800.0,
    scale: 0.00156,   // (5.0 mg/L) / 3200
    min_valid: 0.0,
    max_valid: 6.0,    // mg/L
};

pub static TANK_LEVEL_CAL: AdcCalibration = AdcCalibration {
    offset: 800.0,
    scale: 0.03125,   // (100%) / 3200
    min_valid: -1.0,
    max_valid: 105.0,  // %
};

pub static IRRADIANCE_CAL: AdcCalibration = AdcCalibration {
    offset: 0.0,
    scale: 0.366,      // (1500 W/m2) / 4095
    min_valid: 0.0,
    max_valid: 1600.0,  // W/m2
};

pub static PANEL_VOLTAGE_CAL: AdcCalibration = AdcCalibration {
    offset: 0.0,
    scale: 0.122,      // (500V) / 4095 (with voltage divider)
    min_valid: 0.0,
    max_valid: 600.0,   // V
};

// ============================================================================
// Calibration constants: ADS1263 Solar SMU (32-bit delta-sigma)
// Full-scale = 2.5V (VREF), 32-bit signed: +/- 2^31
// LSB = 2.5V / 2^31 = ~1.16 nV
// ============================================================================

/// String voltage measurement calibration (per-string, via voltage divider)
/// External divider ratio 200:1 => 0-1000V maps to 0-5V, then into ADS1263 +-2.5V
pub static STRING_VOLTAGE_CAL: AdcCalibration = AdcCalibration {
    offset: 0.0,
    scale: 4.657e-7,   // (1000V) / 2^31 (scaled by divider)
    min_valid: -10.0,
    max_valid: 1100.0,  // V
};

/// String current measurement calibration (via CT or hall sensor)
/// 0-50A maps to 0-2.5V on ADC input
pub static STRING_CURRENT_CAL: AdcCalibration = AdcCalibration {
    offset: 0.0,
    scale: 2.328e-8,   // (50A) / 2^31
    min_valid: -2.0,
    max_valid: 60.0,    // A
};

// ============================================================================
// Calibration constants: ADS1258 Water RTU (24-bit multi-channel)
// Full-scale = 2.5V (VREF), 24-bit signed: +/- 2^23
// LSB = 2.5V / 2^23 = ~298 nV
// ============================================================================

pub static WATER_LEVEL_24BIT_CAL: AdcCalibration = AdcCalibration {
    offset: 0.0,
    scale: 1.192e-5,   // (100%) / 2^23
    min_valid: -1.0,
    max_valid: 105.0,
};

pub static FLOW_RATE_24BIT_CAL: AdcCalibration = AdcCalibration {
    offset: 0.0,
    scale: 5.96e-6,    // (50 L/min) / 2^23
    min_valid: -0.5,
    max_valid: 60.0,
};

pub static PRESSURE_24BIT_CAL: AdcCalibration = AdcCalibration {
    offset: 0.0,
    scale: 1.192e-6,   // (10 bar) / 2^23
    min_valid: -0.5,
    max_valid: 12.0,
};

// ============================================================================
// Internal STM32 ADC: Read with oversampling
// ============================================================================

/// Read a single ADC channel with oversampling for noise reduction
#[cfg(not(feature = "simulator"))]
pub async fn read_oversampled(
    adc: &mut Adc<'_, peripherals::ADC1>,
    pin: &mut impl embassy_stm32::adc::AdcChannel<peripherals::ADC1>,
    samples: u32,
) -> u16 {
    let mut sum: u32 = 0;
    for _ in 0..samples {
        let raw = adc.blocking_read(pin);
        sum += raw as u32;
    }
    (sum / samples) as u16
}

/// Read a single ADC channel with oversampling and return Result
#[cfg(not(feature = "simulator"))]
pub async fn read_oversampled_checked(
    adc: &mut Adc<'_, peripherals::ADC1>,
    pin: &mut impl embassy_stm32::adc::AdcChannel<peripherals::ADC1>,
    samples: u32,
    cal: &AdcCalibration,
) -> Result<f32, AdcError> {
    if samples == 0 {
        return Err(AdcError::InvalidChannel);
    }
    let raw = read_oversampled(adc, pin, samples).await;
    let value = cal.convert(raw);
    if cal.is_valid(value) {
        Ok(value)
    } else {
        Err(AdcError::OutOfRange)
    }
}

// ============================================================================
// ADS1263 interface (Solar SMU - 32-bit delta-sigma)
// Full driver in drivers/ads1263.rs; this provides the HAL-level read API
// ============================================================================

/// ADS1263 reading result with status and timestamp
#[derive(Clone, Copy)]
pub struct Ads1263Reading {
    /// 32-bit signed conversion result
    pub raw: i32,
    /// Status byte from the ADC
    pub status: u8,
    /// Checksum valid flag
    pub checksum_ok: bool,
    /// Timestamp in microseconds (from embassy_time)
    pub timestamp_us: u64,
}

impl Ads1263Reading {
    /// Check if the new data bit is set in status
    pub fn is_new_data(&self) -> bool {
        self.status & 0x40 != 0
    }

    /// Check ADC fault bits in status
    pub fn has_fault(&self) -> bool {
        self.status & 0x3F != 0
    }

    /// Convert raw reading to voltage (assuming 2.5V reference)
    pub fn to_voltage(&self) -> f32 {
        // 32-bit signed, full-scale = +/- VREF = +/- 2.5V
        (self.raw as f32) * (2.5 / 2147483648.0)
    }

    /// Convert using a calibration structure
    pub fn to_engineering(&self, cal: &AdcCalibration) -> f32 {
        cal.convert_i32(self.raw)
    }
}

/// Solar SMU measurement set from ADS1263 (per string combiner)
#[derive(Clone, Copy, Default)]
pub struct SolarStringReading {
    /// String DC voltage (V)
    pub voltage_v: f32,
    /// String DC current (A)
    pub current_a: f32,
    /// Power = V * I (W)
    pub power_w: f32,
    /// Temperature at junction box (deg C)
    pub temperature_c: f32,
    /// String index (0-based)
    pub string_index: u8,
    /// Data quality (true = valid)
    pub valid: bool,
    /// PTP-synchronized timestamp (nanoseconds since epoch)
    pub ptp_timestamp_ns: u64,
}

impl SolarStringReading {
    /// Compute power from voltage and current
    pub fn compute_power(&mut self) {
        self.power_w = self.voltage_v * self.current_a;
    }
}

// ============================================================================
// ADS1258 interface (Water RTU - 24-bit multi-channel)
// Full driver in drivers/ads1258.rs; this provides the HAL-level read API
// ============================================================================

/// ADS1258 reading result with channel ID
#[derive(Clone, Copy)]
pub struct Ads1258Reading {
    /// 24-bit signed conversion result (sign-extended to i32)
    pub raw: i32,
    /// Channel ID (0-15 for single-ended, 0-7 for differential)
    pub channel: u8,
    /// Status byte
    pub status: u8,
    /// New data flag
    pub new_data: bool,
    /// Timestamp in microseconds
    pub timestamp_us: u64,
}

impl Ads1258Reading {
    /// Convert raw reading to voltage (assuming 2.5V reference)
    pub fn to_voltage(&self) -> f32 {
        // 24-bit signed, full-scale = +/- VREF = +/- 2.5V
        (self.raw as f32) * (2.5 / 8388608.0)
    }

    /// Convert using a calibration structure
    pub fn to_engineering(&self, cal: &AdcCalibration) -> f32 {
        cal.convert_i32(self.raw)
    }
}

/// Water RTU multi-channel scan result from ADS1258
#[derive(Clone)]
pub struct WaterMultiChannelReading {
    /// Per-channel raw readings (up to 16 channels)
    pub channels: [Option<Ads1258Reading>; 16],
    /// Number of channels that were successfully read
    pub valid_count: u8,
    /// Scan sequence number
    pub sequence: u32,
    /// Timestamp of scan start
    pub scan_start_us: u64,
    /// Timestamp of scan end
    pub scan_end_us: u64,
}

impl WaterMultiChannelReading {
    pub const fn new() -> Self {
        Self {
            channels: [None; 16],
            valid_count: 0,
            sequence: 0,
            scan_start_us: 0,
            scan_end_us: 0,
        }
    }

    /// Get the reading for a specific channel
    pub fn get_channel(&self, ch: u8) -> Option<&Ads1258Reading> {
        if (ch as usize) < 16 {
            self.channels[ch as usize].as_ref()
        } else {
            None
        }
    }

    /// Get the voltage for a specific channel
    pub fn get_voltage(&self, ch: u8) -> Option<f32> {
        self.get_channel(ch).map(|r| r.to_voltage())
    }

    /// Get the engineering value for a specific channel with calibration
    pub fn get_engineering(&self, ch: u8, cal: &AdcCalibration) -> Option<f32> {
        self.get_channel(ch).map(|r| r.to_engineering(cal))
    }
}

// ============================================================================
// Water sensor readings (from internal STM32 ADC)
// ============================================================================

/// Water sensor readings from ADC channels
pub struct WaterAdcReadings {
    pub pressure_raw: u16,
    pub ph_raw: u16,
    pub turbidity_raw: u16,
    pub chlorine_raw: u16,
    pub tank_level_raw: u16,
}

impl WaterAdcReadings {
    pub fn pressure_bar(&self) -> f32 {
        PRESSURE_CAL.convert(self.pressure_raw)
    }

    pub fn ph(&self) -> f32 {
        PH_CAL.convert(self.ph_raw)
    }

    pub fn turbidity_ntu(&self) -> f32 {
        TURBIDITY_CAL.convert(self.turbidity_raw)
    }

    pub fn chlorine_mgl(&self) -> f32 {
        CHLORINE_CAL.convert(self.chlorine_raw)
    }

    pub fn tank_level_pct(&self) -> f32 {
        TANK_LEVEL_CAL.convert(self.tank_level_raw)
    }

    /// Check all readings for sensor faults (broken wire = <4mA, saturated = >20mA)
    pub fn all_valid(&self) -> bool {
        PRESSURE_CAL.is_valid(self.pressure_bar())
            && PH_CAL.is_valid(self.ph())
            && TURBIDITY_CAL.is_valid(self.turbidity_ntu())
            && CHLORINE_CAL.is_valid(self.chlorine_mgl())
            && TANK_LEVEL_CAL.is_valid(self.tank_level_pct())
    }

    pub fn log(&self) {
        info!(
            "ADC: P={} pH={} T={} Cl={} Level={}",
            self.pressure_raw, self.ph_raw, self.turbidity_raw,
            self.chlorine_raw, self.tank_level_raw
        );
    }
}

// ============================================================================
// Solar sensor readings (from internal STM32 ADC)
// ============================================================================

/// Solar sensor readings from ADC channels
pub struct SolarAdcReadings {
    pub irradiance_raw: u16,
    pub panel_voltage_raw: u16,
}

impl SolarAdcReadings {
    pub fn irradiance_wm2(&self) -> f32 {
        IRRADIANCE_CAL.convert(self.irradiance_raw)
    }

    pub fn panel_voltage(&self) -> f32 {
        PANEL_VOLTAGE_CAL.convert(self.panel_voltage_raw)
    }

    pub fn all_valid(&self) -> bool {
        IRRADIANCE_CAL.is_valid(self.irradiance_wm2())
            && PANEL_VOLTAGE_CAL.is_valid(self.panel_voltage())
    }
}

// ============================================================================
// DMA-based ADC reading support
// ============================================================================

/// DMA transfer descriptor for SPI-based ADC reads
pub struct DmaAdcTransfer {
    /// TX buffer for SPI command (RREG, NOP, etc.)
    pub tx_buf: [u8; 8],
    /// RX buffer for SPI response (status + data + checksum)
    pub rx_buf: [u8; 8],
    /// Number of bytes to transfer
    pub len: usize,
}

impl DmaAdcTransfer {
    pub const fn new() -> Self {
        Self {
            tx_buf: [0u8; 8],
            rx_buf: [0u8; 8],
            len: 0,
        }
    }

    /// Set up a read-data command for ADS1263 (RDATA1 = 0x12)
    pub fn setup_ads1263_read(&mut self) {
        self.tx_buf[0] = 0x12; // RDATA1
        self.tx_buf[1] = 0x00; // NOP (status)
        self.tx_buf[2] = 0x00; // NOP (data byte 3)
        self.tx_buf[3] = 0x00; // NOP (data byte 2)
        self.tx_buf[4] = 0x00; // NOP (data byte 1)
        self.tx_buf[5] = 0x00; // NOP (data byte 0)
        self.tx_buf[6] = 0x00; // NOP (checksum)
        self.len = 7;
    }

    /// Set up a read-data command for ADS1258
    pub fn setup_ads1258_read(&mut self) {
        // ADS1258: just clock out NOP bytes to receive status + 3 data bytes
        self.tx_buf[0] = 0x00; // NOP
        self.tx_buf[1] = 0x00; // NOP
        self.tx_buf[2] = 0x00; // NOP
        self.tx_buf[3] = 0x00; // NOP
        self.len = 4;
    }

    /// Parse ADS1263 response from rx_buf: [status, d3, d2, d1, d0, checksum]
    pub fn parse_ads1263_response(&self) -> Ads1263Reading {
        let status = self.rx_buf[1];
        let raw = ((self.rx_buf[2] as i32) << 24)
            | ((self.rx_buf[3] as i32) << 16)
            | ((self.rx_buf[4] as i32) << 8)
            | (self.rx_buf[5] as i32);

        // Verify checksum: sum of status + 4 data bytes + checksum should be 0x9B
        let mut sum: u8 = 0x9B;
        for i in 1..7 {
            sum = sum.wrapping_add(self.rx_buf[i]);
        }
        let checksum_ok = sum == 0;

        Ads1263Reading {
            raw,
            status,
            checksum_ok,
            timestamp_us: 0, // Caller should fill in
        }
    }

    /// Parse ADS1258 response from rx_buf: [status, d2, d1, d0]
    pub fn parse_ads1258_response(&self) -> Ads1258Reading {
        let status = self.rx_buf[0];
        let channel = (status >> 3) & 0x0F;
        let new_data = status & 0x80 != 0;

        // 24-bit signed, MSB first
        let raw_unsigned =
            ((self.rx_buf[1] as u32) << 16)
            | ((self.rx_buf[2] as u32) << 8)
            | (self.rx_buf[3] as u32);

        // Sign extend from 24-bit to 32-bit
        let raw = if raw_unsigned & 0x800000 != 0 {
            (raw_unsigned | 0xFF000000) as i32
        } else {
            raw_unsigned as i32
        };

        Ads1258Reading {
            raw,
            channel,
            status,
            new_data,
            timestamp_us: 0,
        }
    }
}

// ============================================================================
// Oversampling/averaging helper
// ============================================================================

/// Accumulator for multi-sample averaging of 32-bit ADC readings
pub struct OversampleAccumulator {
    sum: i64,
    count: u32,
    target: u32,
}

impl OversampleAccumulator {
    pub const fn new(target_samples: u32) -> Self {
        Self {
            sum: 0,
            count: 0,
            target: target_samples,
        }
    }

    /// Add a sample. Returns Some(average) when target count is reached.
    pub fn add_sample(&mut self, sample: i32) -> Option<i32> {
        self.sum += sample as i64;
        self.count += 1;
        if self.count >= self.target {
            let avg = (self.sum / self.count as i64) as i32;
            self.reset();
            Some(avg)
        } else {
            None
        }
    }

    /// Reset the accumulator
    pub fn reset(&mut self) {
        self.sum = 0;
        self.count = 0;
    }

    /// Check if accumulation is complete
    pub fn is_complete(&self) -> bool {
        self.count >= self.target
    }

    /// Current count
    pub fn current_count(&self) -> u32 {
        self.count
    }
}
