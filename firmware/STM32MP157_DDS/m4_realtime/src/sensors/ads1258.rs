/// ADS1258 — 16-Channel, 24-Bit Delta-Sigma ADC Driver (STM32MP157 M4 Variant)
///
/// Datasheet: Texas Instruments SBAS506
///
/// Adapted from the Water_SCADA_RTU STM32F407 driver for the STM32MP157 M4
/// co-processor. Uses Embassy STM32MP1 HAL SPI peripheral.
///
/// The ADS1258 is a high-precision ADC with:
///   - 16 single-ended or 8 differential inputs
///   - 24-bit resolution, up to 23,739 SPS per channel
///   - Built-in multiplexer with auto-scan capability
///   - SPI interface (CPOL=0, CPHA=1 -> SPI Mode 1)
///
/// On the Water SCADA PCB, we use 8 single-ended channels for 4-20mA inputs.
/// Each channel has a 250 ohm precision shunt resistor:
///   4mA  x 250 ohm = 1.000V
///   20mA x 250 ohm = 5.000V
///
/// External Vref = 5.0V is used for the full 1V-5V measurement range.

use embassy_stm32::gpio::{Input, Output};
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::{info, warn, error, debug, Format};

/// ADS1258 SPI register addresses
mod reg {
    // Read/Write command prefix (command byte format: 0b0RW_AAAAA)
    pub const CMD_READ: u8 = 0x40;   // 0b01_AAAAA — read register
    pub const CMD_WRITE: u8 = 0x60;  // 0b011_AAAAA — write register

    // Configuration registers
    pub const CONFIG0: u8 = 0x00;    // System configuration
    pub const CONFIG1: u8 = 0x01;    // Data rate and delay
    pub const MUXSCH: u8 = 0x02;     // MUX scan channel enable (fixed channel mode)
    pub const MUXDIF: u8 = 0x03;     // MUX differential channel enable
    pub const MUXSG0: u8 = 0x04;     // MUX single-ended enable CH0-CH7
    pub const MUXSG1: u8 = 0x05;     // MUX single-ended enable CH8-CH15
    pub const SYSRED: u8 = 0x06;     // System readings enable (offset, gain, temp, etc.)
    pub const GPIOC: u8 = 0x07;      // GPIO config
    pub const GPIOD: u8 = 0x08;      // GPIO data

    // Status byte bits
    pub const STATUS_NEW: u8 = 0x80;  // New data available
    pub const STATUS_OVF: u8 = 0x40;  // Overflow
    pub const STATUS_SUPPLY: u8 = 0x20; // Supply out of range
}

/// Channel identifiers for 8x 4-20mA water quality inputs
///
/// These correspond to physical channels on the Water SCADA PCB,
/// each with a 250 ohm precision shunt resistor for 4-20mA measurement.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq)]
#[repr(u8)]
pub enum Channel {
    Pressure     = 0, // CH0: 0-10 bar — pipeline/tank pressure
    Ph           = 1, // CH1: 0-14 pH — water acidity/alkalinity
    Turbidity    = 2, // CH2: 0-1000 NTU — water clarity
    Chlorine     = 3, // CH3: 0-5 mg/L — residual chlorine
    Flow         = 4, // CH4: 0-100 m3/h — flow rate
    TankLevel    = 5, // CH5: 0-100% — tank level (hydrostatic)
    Conductivity = 6, // CH6: 0-2000 uS/cm — electrical conductivity
    DissolvedO2  = 7, // CH7: 0-20 mg/L — dissolved oxygen
}

impl Channel {
    pub const COUNT: usize = 8;

    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Self::Pressure),
            1 => Some(Self::Ph),
            2 => Some(Self::Turbidity),
            3 => Some(Self::Chlorine),
            4 => Some(Self::Flow),
            5 => Some(Self::TankLevel),
            6 => Some(Self::Conductivity),
            7 => Some(Self::DissolvedO2),
            _ => None,
        }
    }

    /// Get the engineering unit string for this channel
    pub fn unit(&self) -> &'static str {
        match self {
            Self::Pressure => "bar",
            Self::Ph => "pH",
            Self::Turbidity => "NTU",
            Self::Chlorine => "mg/L",
            Self::Flow => "m3/h",
            Self::TankLevel => "%",
            Self::Conductivity => "uS/cm",
            Self::DissolvedO2 => "mg/L",
        }
    }
}

/// Calibration data per channel
///
/// Stored in flash (via CalibrationStore) and loaded at startup.
/// Factory default values assume ideal 250 ohm shunt with Vref=5.0V.
#[derive(Clone, Copy)]
pub struct ChannelCalibration {
    /// ADC code at 4.00mA reference (zero point)
    pub zero_code: i32,
    /// ADC code at 20.00mA reference (span point)
    pub span_code: i32,
    /// Engineering unit value at 4mA (range minimum)
    pub eng_min: f32,
    /// Engineering unit value at 20mA (range maximum)
    pub eng_max: f32,
}

impl ChannelCalibration {
    /// Convert raw 24-bit ADC code to engineering units
    ///
    /// Uses linear interpolation between the zero and span calibration points.
    pub fn convert(&self, raw: i32) -> f32 {
        if self.span_code == self.zero_code {
            return 0.0;
        }
        let fraction = (raw - self.zero_code) as f32 / (self.span_code - self.zero_code) as f32;
        self.eng_min + fraction * (self.eng_max - self.eng_min)
    }

    /// Get the loop current in mA from raw ADC code
    ///
    /// Returns the actual loop current, useful for diagnostics and
    /// wire-fault detection (open circuit, short circuit, etc.)
    pub fn current_ma(&self, raw: i32) -> f32 {
        if self.span_code == self.zero_code {
            return 0.0;
        }
        let fraction = (raw - self.zero_code) as f32 / (self.span_code - self.zero_code) as f32;
        4.0 + fraction * 16.0 // 4-20mA range = 16mA span
    }
}

/// Default calibration constants (factory defaults, before field calibration)
///
/// Calculation for 250 ohm shunt, Vref=5.0V, 24-bit resolution:
///   4mA  x 250 ohm = 1.000V -> code = 1V/5V x 2^23 = 1,677,722
///   20mA x 250 ohm = 5.000V -> code = 5V/5V x 2^23 = 8,388,608 (full scale)
pub const DEFAULT_CALIBRATIONS: [ChannelCalibration; Channel::COUNT] = [
    // CH0: Pressure 0-10 bar
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 10.0 },
    // CH1: pH 0-14
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 14.0 },
    // CH2: Turbidity 0-1000 NTU
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 1000.0 },
    // CH3: Chlorine 0-5 mg/L
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 5.0 },
    // CH4: Flow 0-100 m3/h
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 100.0 },
    // CH5: Tank level 0-100%
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 100.0 },
    // CH6: Conductivity 0-2000 uS/cm
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 2000.0 },
    // CH7: Dissolved O2 0-20 mg/L
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 20.0 },
];

/// Channel status codes for diagnostics and alarm generation
///
/// Determined by the measured loop current. A healthy 4-20mA transmitter
/// will always output >= 3.8mA (even at zero) and <= 20.5mA (at full scale).
#[derive(Clone, Copy, PartialEq, defmt::Format)]
#[repr(u8)]
pub enum ChannelStatus {
    /// Operating normally (3.8-20.5 mA)
    Normal       = 0,
    /// Below live zero (< 3.8mA, possible broken wire)
    UnderRange   = 1,
    /// Above full scale (> 20.5mA, sensor fault)
    OverRange    = 2,
    /// Wire disconnected (< 1.0mA)
    OpenCircuit  = 3,
    /// Wiring short (> 24mA)
    ShortCircuit = 4,
    /// ADC timeout or communication error
    CommError    = 5,
}

impl ChannelStatus {
    /// Determine channel status from measured loop current in mA
    ///
    /// Thresholds based on NAMUR NE43 recommendations:
    ///   < 1.0 mA  = open circuit (wire break)
    ///   < 3.8 mA  = under-range (transmitter fault, not quite open)
    ///   3.8-20.5  = normal operating range
    ///   > 20.5 mA = over-range (sensor saturated or fault)
    ///   > 24.0 mA = short circuit (wiring fault)
    pub fn from_current(ma: f32) -> Self {
        if ma < 1.0 {
            Self::OpenCircuit
        } else if ma < 3.8 {
            Self::UnderRange
        } else if ma > 24.0 {
            Self::ShortCircuit
        } else if ma > 20.5 {
            Self::OverRange
        } else {
            Self::Normal
        }
    }

    /// Whether this status indicates a fault condition
    pub fn is_fault(&self) -> bool {
        !matches!(self, Self::Normal)
    }
}

/// Complete reading for one channel, including raw data, engineering value,
/// loop current, and diagnostic status.
#[derive(Clone, Copy)]
pub struct ChannelReading {
    /// Raw 24-bit ADC code (sign-extended to i32)
    pub raw_code: i32,
    /// Measured loop current in mA
    pub current_ma: f32,
    /// Calibrated engineering value
    pub eng_value: f32,
    /// Channel diagnostic status
    pub status: ChannelStatus,
    /// Alarm code (0 = no alarm)
    pub alarm: u8,
}

impl Default for ChannelReading {
    fn default() -> Self {
        Self {
            raw_code: 0,
            current_ma: 0.0,
            eng_value: 0.0,
            status: ChannelStatus::CommError,
            alarm: 0,
        }
    }
}

/// ADS1258 driver for STM32MP157 M4 core
pub struct Ads1258<'a> {
    spi: Spi<'a, Async>,
    cs: Output<'a>,
    drdy: Input<'a>,
    start: Output<'a>,
    reset: Output<'a>,
    /// Per-channel calibration data
    calibrations: [ChannelCalibration; Channel::COUNT],
    /// Conversion count for statistics
    total_conversions: u32,
    /// Error count for diagnostics
    total_errors: u32,
}

impl<'a> Ads1258<'a> {
    pub fn new(
        spi: Spi<'a, Async>,
        cs: Output<'a>,
        drdy: Input<'a>,
        start: Output<'a>,
        reset: Output<'a>,
    ) -> Self {
        Self {
            spi,
            cs,
            drdy,
            start,
            reset,
            calibrations: DEFAULT_CALIBRATIONS,
            total_conversions: 0,
            total_errors: 0,
        }
    }

    /// Load calibration data (from flash storage or remote command)
    pub fn set_calibrations(&mut self, cals: [ChannelCalibration; Channel::COUNT]) {
        self.calibrations = cals;
        info!("ADS1258: Calibrations updated for all {} channels", Channel::COUNT);
    }

    /// Get current calibration for a specific channel
    pub fn get_calibration(&self, ch: Channel) -> &ChannelCalibration {
        &self.calibrations[ch as usize]
    }

    /// Hardware reset and initialization
    ///
    /// Configures the ADS1258 for auto-scan mode over 8 single-ended channels.
    /// The scan rate per channel depends on CONFIG1 settings; default is ~125 SPS
    /// per channel, giving a complete 8-channel scan in ~64ms.
    pub async fn init(&mut self) -> bool {
        info!("ADS1258: Initializing on STM32MP157 M4...");

        // Hardware reset pulse (active low, min 4 CLKIN cycles at 7.68 MHz)
        self.start.set_low(); // Stop any ongoing conversions
        self.reset.set_low();
        Timer::after(Duration::from_millis(10)).await;
        self.reset.set_high();
        Timer::after(Duration::from_millis(50)).await;

        // CONFIG0: auto-scan mode, CHOP enabled, STATUS byte enabled
        // Bit 7:   SPIX    = 0 (SPI with DOUT/DRDY on same pin)
        // Bit 6:   MUXMOD  = 0 (auto-scan mode)
        // Bit 5:   BYPAS   = 0 (use internal buffer)
        // Bit 4:   CLKENB  = 0 (clock output disabled)
        // Bit 3:   reserved= 0
        // Bit 2:   CHOP    = 0 (chop disabled for faster scan)
        // Bit 1:   STAT    = 1 (status byte in data output)
        // Bit 0:   reserved= 1
        let config0: u8 = 0x03; // STATUS + CHOP for offset drift reduction
        if !self.write_register(reg::CONFIG0, config0).await {
            error!("ADS1258: Failed to write CONFIG0");
            return false;
        }

        // CONFIG1: conversion delay and data rate
        // Bit 7:   IDLMOD  = 0 (normal idle mode)
        // Bits 6:5: DLY    = 01 (default delay between conversions)
        // Bit 4:   SCBCS   = 0 (scan burnout current source off)
        // Bits 3:2: DRATE  = 00 (default data rate)
        // Bits 1:0: reserved
        let config1: u8 = 0x20;
        if !self.write_register(reg::CONFIG1, config1).await {
            error!("ADS1258: Failed to write CONFIG1");
            return false;
        }

        // Enable single-ended channels 0-7 for auto-scan
        // MUXSG0: bits 7:0 correspond to channels 7:0
        let muxsg0: u8 = 0xFF; // All 8 channels enabled
        if !self.write_register(reg::MUXSG0, muxsg0).await {
            error!("ADS1258: Failed to write MUXSG0");
            return false;
        }

        // Disable differential channels and channels 8-15
        if !self.write_register(reg::MUXDIF, 0x00).await {
            return false;
        }
        if !self.write_register(reg::MUXSG1, 0x00).await {
            return false;
        }

        // Enable system readings for internal diagnostics
        // Bit 3: VCC measurement, Bit 2: Temperature, Bit 1: GAIN, Bit 0: REF
        if !self.write_register(reg::SYSRED, 0x0C).await {
            return false;
        }

        // Verify configuration by reading back CONFIG0
        let readback = self.read_register(reg::CONFIG0).await;
        if readback != config0 {
            error!("ADS1258: Config verify failed (got 0x{:02X}, expected 0x{:02X})",
                readback, config0);
            return false;
        }

        // Start auto-scan conversions
        self.start.set_high();
        Timer::after(Duration::from_millis(10)).await;

        info!("ADS1258: Initialized, {} channels auto-scanning", Channel::COUNT);
        true
    }

    /// Read all 8 channels (one complete auto-scan cycle)
    ///
    /// Waits for each channel's conversion to complete in the auto-scan sequence.
    /// Returns readings in channel order [CH0..CH7].
    ///
    /// Typical timing: ~64ms for a complete 8-channel scan at default rate.
    pub async fn read_all_channels(&mut self) -> [ChannelReading; Channel::COUNT] {
        let mut readings = [ChannelReading::default(); Channel::COUNT];
        let mut channels_read: u8 = 0;
        let mut attempts = 0u32;

        // Read conversions until we have all 8 channels or timeout
        while channels_read != 0xFF && attempts < 24 {
            attempts += 1;

            // Wait for DRDY (active low = data ready)
            let mut timeout = 0u32;
            while self.drdy.is_high() {
                Timer::after(Duration::from_micros(50)).await;
                timeout += 1;
                if timeout > 2000 { // 100ms timeout
                    warn!("ADS1258: DRDY timeout (attempt {})", attempts);
                    self.total_errors += 1;
                    break;
                }
            }

            if timeout > 2000 {
                continue;
            }

            // Read conversion result: STATUS(1) + DATA(3) = 4 bytes
            let (channel_id, raw, status_byte) = self.read_conversion().await;

            let ch_idx = (channel_id & 0x0F) as usize;
            if ch_idx < Channel::COUNT && (channels_read & (1 << ch_idx)) == 0 {
                let cal = &self.calibrations[ch_idx];
                let current = cal.current_ma(raw);
                let eng = cal.convert(raw);
                let status = ChannelStatus::from_current(current);

                readings[ch_idx] = ChannelReading {
                    raw_code: raw,
                    current_ma: current,
                    eng_value: eng,
                    status,
                    alarm: if status.is_fault() { status as u8 } else { 0 },
                };

                channels_read |= 1 << ch_idx;
                self.total_conversions += 1;
            }
        }

        // Mark any channels we failed to read
        for i in 0..Channel::COUNT {
            if (channels_read & (1 << i)) == 0 {
                readings[i].status = ChannelStatus::CommError;
                readings[i].alarm = ChannelStatus::CommError as u8;
                warn!("ADS1258: Channel {} not read in scan cycle", i);
            }
        }

        readings
    }

    /// Read a single specific channel (fixed-channel mode)
    ///
    /// Temporarily switches to fixed-channel mode, reads one conversion,
    /// then returns to auto-scan mode. Slower than read_all_channels but
    /// useful for on-demand single-channel reads.
    pub async fn read_single_channel(&mut self, ch: Channel) -> ChannelReading {
        // Switch to fixed-channel mode by writing MUXSCH
        let muxsch = ((ch as u8) << 3) | 0x00; // Positive = CHn, Negative = AINCOM
        self.write_register(reg::MUXSCH, muxsch).await;

        // Wait for conversion
        let mut timeout = 0u32;
        while self.drdy.is_high() {
            Timer::after(Duration::from_micros(100)).await;
            timeout += 1;
            if timeout > 2000 {
                warn!("ADS1258: Single-channel DRDY timeout");
                return ChannelReading::default();
            }
        }

        let (_, raw, _) = self.read_conversion().await;

        let cal = &self.calibrations[ch as usize];
        let current = cal.current_ma(raw);
        let eng = cal.convert(raw);
        let status = ChannelStatus::from_current(current);

        // Return to auto-scan by re-enabling MUXSG0
        self.write_register(reg::MUXSG0, 0xFF).await;

        ChannelReading {
            raw_code: raw,
            current_ma: current,
            eng_value: eng,
            status,
            alarm: if status.is_fault() { status as u8 } else { 0 },
        }
    }

    /// Read a single conversion result from the SPI bus
    ///
    /// Returns (channel_id, raw_24bit_signed, status_byte)
    async fn read_conversion(&mut self) -> (u8, i32, u8) {
        // With STATUS enabled: 4 bytes = status(1) + data(3)
        let mut buf = [0u8; 4];
        self.cs.set_low();
        let _ = self.spi.read(&mut buf).await;
        self.cs.set_high();

        let status_byte = buf[0];
        let channel_id = status_byte & 0x1F; // Channel ID from status byte bits [4:0]

        // 24-bit signed result (MSB first)
        let raw = ((buf[1] as i32) << 16) | ((buf[2] as i32) << 8) | (buf[3] as i32);

        // Sign-extend from 24-bit to 32-bit
        let raw = if raw & 0x80_0000 != 0 {
            raw | (0xFF << 24) as i32
        } else {
            raw
        };

        (channel_id, raw, status_byte)
    }

    /// Read the internal temperature sensor
    ///
    /// The ADS1258 has a built-in temperature sensor that can be enabled
    /// via the SYSRED register. Returns temperature in degrees Celsius.
    pub async fn read_internal_temperature(&mut self) -> Option<f32> {
        // Temperature reading appears in the auto-scan if SYSRED.TEMP is set
        // Channel ID for temperature = 0x12
        let mut timeout = 0u32;
        while self.drdy.is_high() {
            Timer::after(Duration::from_micros(100)).await;
            timeout += 1;
            if timeout > 2000 {
                return None;
            }
        }

        let (ch_id, raw, _) = self.read_conversion().await;
        if ch_id == 0x12 {
            // Temperature = raw * conversion_factor
            // ADS1258 temp sensor: ~420 uV/degC, relative to 25C
            let temp_c = 25.0 + (raw as f32 * 0.000420 / 5.0 * 8_388_608.0);
            Some(temp_c)
        } else {
            None
        }
    }

    /// Get diagnostic statistics
    pub fn stats(&self) -> (u32, u32) {
        (self.total_conversions, self.total_errors)
    }

    /// Reset diagnostic counters
    pub fn reset_stats(&mut self) {
        self.total_conversions = 0;
        self.total_errors = 0;
    }

    /// Write a single register
    async fn write_register(&mut self, addr: u8, value: u8) -> bool {
        let cmd = [reg::CMD_WRITE | (addr & 0x1F), value];
        self.cs.set_low();
        let result = self.spi.write(&cmd).await;
        self.cs.set_high();
        result.is_ok()
    }

    /// Read a single register
    async fn read_register(&mut self, addr: u8) -> u8 {
        let cmd = [reg::CMD_READ | (addr & 0x1F), 0x00];
        let mut buf = [0u8; 2];
        self.cs.set_low();
        let _ = self.spi.transfer(&mut buf, &cmd).await;
        self.cs.set_high();
        buf[1]
    }
}
