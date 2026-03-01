/// ADS1263 32-bit Delta-Sigma ADC Driver
///
/// The ADS1263 is a high-precision 32-bit delta-sigma ADC from Texas Instruments,
/// used in the Solar SMU (String Monitoring Unit) for high-accuracy voltage
/// and current measurements of individual PV strings.
///
/// Key specifications:
///   - 32-bit delta-sigma ADC (ADC1) + 24-bit auxiliary ADC (ADC2)
///   - Data rates: 2.5 SPS to 38.4 kSPS
///   - Input multiplexer: 10 differential channels (AINCOM + AIN0..AIN9)
///   - Internal reference: 2.5V (also supports external VREF)
///   - PGA: 1x to 32x
///   - Digital filter: sinc1..sinc5, FIR
///   - SPI interface: Mode 1 (CPOL=0, CPHA=1), up to 8 MHz
///   - Checksum on data output for data integrity
///
/// Register map (from ADS1263 datasheet SBAS661C):
///   Addr  Name         Description
///   0x00  ID           Device ID (0x01 for ADS1263)
///   0x01  POWER        Power/reset control
///   0x02  INTERFACE    SPI interface configuration
///   0x03  MODE0        ADC1 data rate, filter, conversion delay
///   0x04  MODE1        ADC1 digital filter mode
///   0x05  MODE2        ADC1 PGA gain, data rate
///   0x06  INPMUX       ADC1 input multiplexer
///   0x07  OFCAL0       ADC1 offset calibration (byte 0)
///   0x08  OFCAL1       ADC1 offset calibration (byte 1)
///   0x09  OFCAL2       ADC1 offset calibration (byte 2)
///   0x0A  FSCAL0       ADC1 gain calibration (byte 0)
///   0x0B  FSCAL1       ADC1 gain calibration (byte 1)
///   0x0C  FSCAL2       ADC1 gain calibration (byte 2)
///   0x0D  IDACMUX      IDAC multiplexer
///   0x0E  IDACMAG      IDAC magnitude
///   0x0F  REFMUX       Reference multiplexer
///   0x10  TDACP        TDAC positive output
///   0x11  TDACN        TDAC negative output
///   0x12  GPIOCON      GPIO connection register
///   0x13  GPIODIR      GPIO direction register
///   0x14  GPIODAT      GPIO data register
///   0x15  ADC2CFG      ADC2 configuration
///   0x16  ADC2MUX      ADC2 input multiplexer
///   0x17  ADC2OFC0     ADC2 offset calibration (byte 0)
///   0x18  ADC2OFC1     ADC2 offset calibration (byte 1)
///   0x19  ADC2FSC0     ADC2 gain calibration (byte 0)
///   0x1A  ADC2FSC1     ADC2 gain calibration (byte 1)

use crate::hal::spi::Spi1Adc;
use crate::hal::adc::{AdcError, Ads1263Reading};
use defmt::{info, warn, error, Format};
use embassy_time::{Timer, Instant};

// ============================================================================
// Register addresses
// ============================================================================

#[allow(dead_code)]
pub mod regs {
    pub const ID: u8 = 0x00;
    pub const POWER: u8 = 0x01;
    pub const INTERFACE: u8 = 0x02;
    pub const MODE0: u8 = 0x03;
    pub const MODE1: u8 = 0x04;
    pub const MODE2: u8 = 0x05;
    pub const INPMUX: u8 = 0x06;
    pub const OFCAL0: u8 = 0x07;
    pub const OFCAL1: u8 = 0x08;
    pub const OFCAL2: u8 = 0x09;
    pub const FSCAL0: u8 = 0x0A;
    pub const FSCAL1: u8 = 0x0B;
    pub const FSCAL2: u8 = 0x0C;
    pub const IDACMUX: u8 = 0x0D;
    pub const IDACMAG: u8 = 0x0E;
    pub const REFMUX: u8 = 0x0F;
    pub const TDACP: u8 = 0x10;
    pub const TDACN: u8 = 0x11;
    pub const GPIOCON: u8 = 0x12;
    pub const GPIODIR: u8 = 0x13;
    pub const GPIODAT: u8 = 0x14;
    pub const ADC2CFG: u8 = 0x15;
    pub const ADC2MUX: u8 = 0x16;
    pub const ADC2OFC0: u8 = 0x17;
    pub const ADC2OFC1: u8 = 0x18;
    pub const ADC2FSC0: u8 = 0x19;
    pub const ADC2FSC1: u8 = 0x1A;
}

// ============================================================================
// SPI commands
// ============================================================================

#[allow(dead_code)]
pub mod cmd {
    /// No operation
    pub const NOP: u8 = 0x00;
    /// Reset the device
    pub const RESET: u8 = 0x06;
    /// Start ADC1 conversion
    pub const START1: u8 = 0x08;
    /// Stop ADC1 conversion
    pub const STOP1: u8 = 0x0A;
    /// Start ADC2 conversion
    pub const START2: u8 = 0x0C;
    /// Stop ADC2 conversion
    pub const STOP2: u8 = 0x0E;
    /// Read ADC1 data
    pub const RDATA1: u8 = 0x12;
    /// Read ADC2 data
    pub const RDATA2: u8 = 0x14;
    /// System offset calibration for ADC1
    pub const SYOCAL1: u8 = 0x16;
    /// System gain calibration for ADC1
    pub const SYGCAL1: u8 = 0x17;
    /// Self offset calibration for ADC1
    pub const SFOCAL1: u8 = 0x19;
    /// System offset calibration for ADC2
    pub const SYOCAL2: u8 = 0x1B;
    /// System gain calibration for ADC2
    pub const SYGCAL2: u8 = 0x1C;
    /// Self offset calibration for ADC2
    pub const SFOCAL2: u8 = 0x1E;
    /// Read register (OR with register address)
    pub const RREG: u8 = 0x20;
    /// Write register (OR with register address)
    pub const WREG: u8 = 0x40;
}

// ============================================================================
// Register bit definitions
// ============================================================================

/// POWER register (0x01)
#[allow(dead_code)]
pub mod power_bits {
    /// Reset bit (write 1 to reset, reads back 0 when done)
    pub const RESET: u8 = 1 << 4;
    /// Internal voltage reference enable
    pub const VBIAS: u8 = 1 << 1;
    /// Internal reference enable
    pub const INTREF: u8 = 1 << 0;
}

/// INTERFACE register (0x02)
#[allow(dead_code)]
pub mod interface_bits {
    /// Timeout enable (auto SPI reset after ~65536 SCLKs)
    pub const TIMEOUT: u8 = 1 << 3;
    /// Status byte enable (prepend status to data)
    pub const STATUS: u8 = 1 << 2;
    /// Checksum mode: 00=disabled, 01=checksum, 10=CRC
    pub const CRC_CHKSUM: u8 = 0b01 << 0;
    pub const CRC_CRC: u8 = 0b10 << 0;
    pub const CRC_NONE: u8 = 0b00 << 0;
}

/// MODE0 register (0x03)
#[allow(dead_code)]
pub mod mode0_bits {
    /// Conversion delay (bits 6:4)
    pub const DELAY_NONE: u8 = 0b000 << 4;
    pub const DELAY_8_7US: u8 = 0b001 << 4;
    pub const DELAY_17US: u8 = 0b010 << 4;
    pub const DELAY_35US: u8 = 0b011 << 4;
    pub const DELAY_69US: u8 = 0b100 << 4;
    pub const DELAY_139US: u8 = 0b101 << 4;
    pub const DELAY_278US: u8 = 0b110 << 4;
    pub const DELAY_555US: u8 = 0b111 << 4;
    /// Chop mode enable (bit 3)
    pub const CHOP: u8 = 1 << 3;
    /// Input buffer disable (bit 1)
    pub const IDAC_ROT: u8 = 1 << 1;
    /// Run mode: continuous (bit 0 = 0) or pulse (bit 0 = 1)
    pub const RUN_PULSE: u8 = 1 << 0;
    pub const RUN_CONTINUOUS: u8 = 0 << 0;
}

/// MODE1 register (0x04) -- Digital filter
#[allow(dead_code)]
pub mod mode1_bits {
    /// Filter mode (bits 7:5)
    pub const FILTER_SINC1: u8 = 0b000 << 5;
    pub const FILTER_SINC2: u8 = 0b001 << 5;
    pub const FILTER_SINC3: u8 = 0b010 << 5;
    pub const FILTER_SINC4: u8 = 0b011 << 5;
    pub const FILTER_FIR: u8 = 0b100 << 5;
    /// Sensor bias connection (bits 4:0)
    pub const SBPOL_PULLUP: u8 = 1 << 4;
    pub const SBMAG_NONE: u8 = 0b000;
    pub const SBMAG_0_5UA: u8 = 0b001;
    pub const SBMAG_2UA: u8 = 0b010;
    pub const SBMAG_10UA: u8 = 0b011;
    pub const SBMAG_50UA: u8 = 0b100;
    pub const SBMAG_200UA: u8 = 0b101;
}

/// MODE2 register (0x05) -- PGA and data rate
#[allow(dead_code)]
pub mod mode2_bits {
    /// PGA bypass (bit 7): 0=PGA enabled, 1=PGA bypassed
    pub const BYPASS: u8 = 1 << 7;
    /// PGA gain (bits 6:4)
    pub const GAIN_1: u8 = 0b000 << 4;
    pub const GAIN_2: u8 = 0b001 << 4;
    pub const GAIN_4: u8 = 0b010 << 4;
    pub const GAIN_8: u8 = 0b011 << 4;
    pub const GAIN_16: u8 = 0b100 << 4;
    pub const GAIN_32: u8 = 0b101 << 4;
    /// ADC1 data rate (bits 3:0)
    pub const DR_2_5: u8 = 0b0000;
    pub const DR_5: u8 = 0b0001;
    pub const DR_10: u8 = 0b0010;
    pub const DR_16_6: u8 = 0b0011;
    pub const DR_20: u8 = 0b0100;
    pub const DR_50: u8 = 0b0101;
    pub const DR_60: u8 = 0b0110;
    pub const DR_100: u8 = 0b0111;
    pub const DR_400: u8 = 0b1000;
    pub const DR_1200: u8 = 0b1001;
    pub const DR_2400: u8 = 0b1010;
    pub const DR_4800: u8 = 0b1011;
    pub const DR_7200: u8 = 0b1100;
    pub const DR_14400: u8 = 0b1101;
    pub const DR_19200: u8 = 0b1110;
    pub const DR_38400: u8 = 0b1111;
}

/// INPMUX register (0x06) -- input channel selection
#[allow(dead_code)]
pub mod inpmux {
    // Positive input (bits 7:4)
    pub const MUXP_AIN0: u8 = 0x00;
    pub const MUXP_AIN1: u8 = 0x10;
    pub const MUXP_AIN2: u8 = 0x20;
    pub const MUXP_AIN3: u8 = 0x30;
    pub const MUXP_AIN4: u8 = 0x40;
    pub const MUXP_AIN5: u8 = 0x50;
    pub const MUXP_AIN6: u8 = 0x60;
    pub const MUXP_AIN7: u8 = 0x70;
    pub const MUXP_AIN8: u8 = 0x80;
    pub const MUXP_AIN9: u8 = 0x90;
    pub const MUXP_AINCOM: u8 = 0xA0;
    pub const MUXP_TEMP_P: u8 = 0xB0;
    pub const MUXP_AVDD: u8 = 0xC0;
    pub const MUXP_DVDD: u8 = 0xD0;
    pub const MUXP_TDAC_P: u8 = 0xE0;
    pub const MUXP_FLOAT: u8 = 0xF0;

    // Negative input (bits 3:0)
    pub const MUXN_AIN0: u8 = 0x00;
    pub const MUXN_AIN1: u8 = 0x01;
    pub const MUXN_AIN2: u8 = 0x02;
    pub const MUXN_AIN3: u8 = 0x03;
    pub const MUXN_AIN4: u8 = 0x04;
    pub const MUXN_AIN5: u8 = 0x05;
    pub const MUXN_AIN6: u8 = 0x06;
    pub const MUXN_AIN7: u8 = 0x07;
    pub const MUXN_AIN8: u8 = 0x08;
    pub const MUXN_AIN9: u8 = 0x09;
    pub const MUXN_AINCOM: u8 = 0x0A;
    pub const MUXN_TEMP_N: u8 = 0x0B;
    pub const MUXN_AVSS: u8 = 0x0C;
    pub const MUXN_DVSS: u8 = 0x0D;
    pub const MUXN_TDAC_N: u8 = 0x0E;
    pub const MUXN_FLOAT: u8 = 0x0F;
}

/// REFMUX register (0x0F) -- reference source selection
#[allow(dead_code)]
pub mod refmux {
    // Positive reference (bits 5:3)
    pub const RMUXP_INT: u8 = 0b000 << 3;    // Internal 2.5V
    pub const RMUXP_AIN0: u8 = 0b001 << 3;   // AIN0
    pub const RMUXP_AIN2: u8 = 0b010 << 3;   // AIN2
    pub const RMUXP_AIN4: u8 = 0b011 << 3;   // AIN4
    pub const RMUXP_AVDD: u8 = 0b100 << 3;   // AVDD (supply)

    // Negative reference (bits 2:0)
    pub const RMUXN_INT: u8 = 0b000;          // Internal 2.5V
    pub const RMUXN_AIN1: u8 = 0b001;         // AIN1
    pub const RMUXN_AIN3: u8 = 0b010;         // AIN3
    pub const RMUXN_AIN5: u8 = 0b011;         // AIN5
    pub const RMUXN_AVSS: u8 = 0b100;         // AVSS (ground)
}

/// ADC2CFG register (0x15) -- ADC2 data rate and reference
#[allow(dead_code)]
pub mod adc2cfg {
    pub const DR2_10: u8 = 0b0000 << 4;
    pub const DR2_100: u8 = 0b0001 << 4;
    pub const DR2_400: u8 = 0b0010 << 4;
    pub const DR2_800: u8 = 0b0011 << 4;
    pub const REF2_INT: u8 = 0b000 << 1;     // Internal 2.5V
    pub const REF2_AIN01: u8 = 0b001 << 1;
    pub const REF2_AIN23: u8 = 0b010 << 1;
    pub const REF2_AIN45: u8 = 0b011 << 1;
    pub const REF2_AVDD: u8 = 0b100 << 1;
    pub const GAIN2_1: u8 = 0;
    pub const GAIN2_2: u8 = 1;
}

// ============================================================================
// Expected device ID
// ============================================================================

const ADS1263_ID: u8 = 0x01;

// ============================================================================
// Configuration structures
// ============================================================================

/// ADS1263 PGA gain setting
#[derive(Clone, Copy, PartialEq, Format)]
pub enum PgaGain {
    Gain1 = 0,
    Gain2 = 1,
    Gain4 = 2,
    Gain8 = 3,
    Gain16 = 4,
    Gain32 = 5,
}

/// ADS1263 data rate setting
#[derive(Clone, Copy, PartialEq, Format)]
pub enum DataRate {
    Sps2_5 = 0,
    Sps5 = 1,
    Sps10 = 2,
    Sps16_6 = 3,
    Sps20 = 4,
    Sps50 = 5,
    Sps60 = 6,
    Sps100 = 7,
    Sps400 = 8,
    Sps1200 = 9,
    Sps2400 = 10,
    Sps4800 = 11,
    Sps7200 = 12,
    Sps14400 = 13,
    Sps19200 = 14,
    Sps38400 = 15,
}

/// ADS1263 digital filter mode
#[derive(Clone, Copy, PartialEq, Format)]
pub enum FilterMode {
    Sinc1 = 0,
    Sinc2 = 1,
    Sinc3 = 2,
    Sinc4 = 3,
    Fir = 4,
}

/// ADS1263 input channel pair (positive, negative)
#[derive(Clone, Copy, PartialEq, Format)]
pub struct ChannelPair {
    pub positive: u8,
    pub negative: u8,
}

impl ChannelPair {
    pub const fn new(positive: u8, negative: u8) -> Self {
        Self { positive, negative }
    }

    /// Encode as INPMUX register value
    pub fn to_mux_byte(&self) -> u8 {
        ((self.positive & 0x0F) << 4) | (self.negative & 0x0F)
    }
}

/// ADS1263 driver configuration
pub struct Ads1263Config {
    /// PGA gain
    pub gain: PgaGain,
    /// Data rate
    pub data_rate: DataRate,
    /// Digital filter mode
    pub filter: FilterMode,
    /// Enable status byte in data output
    pub status_enabled: bool,
    /// Enable checksum in data output
    pub checksum_enabled: bool,
    /// Enable internal voltage reference
    pub internal_ref: bool,
    /// Chop mode enable (reduces offset drift)
    pub chop_mode: bool,
    /// Continuous or pulse conversion mode
    pub continuous: bool,
}

impl Default for Ads1263Config {
    fn default() -> Self {
        Self {
            gain: PgaGain::Gain1,
            data_rate: DataRate::Sps20,
            filter: FilterMode::Sinc3,
            status_enabled: true,
            checksum_enabled: true,
            internal_ref: true,
            chop_mode: false,
            continuous: true,
        }
    }
}

// ============================================================================
// ADS1263 Status byte parsing
// ============================================================================

/// Parsed ADS1263 status byte
#[derive(Clone, Copy, Debug, Format)]
pub struct Ads1263Status {
    /// ADC1 new data ready
    pub adc1_new: bool,
    /// ADC2 new data ready
    pub adc2_new: bool,
    /// PGA output at positive rail (high)
    pub pgah_alarm: bool,
    /// PGA output at negative rail (low)
    pub pgal_alarm: bool,
    /// Reference voltage low alarm
    pub ref_alarm: bool,
    /// DRDY (data ready)
    pub drdy: bool,
    /// Clock source (0=internal, 1=external)
    pub clock: bool,
    /// Reset detected
    pub reset: bool,
}

impl Ads1263Status {
    /// Parse from the status byte value
    pub fn parse(status: u8) -> Self {
        Self {
            adc1_new: status & 0x40 != 0,
            adc2_new: status & 0x20 != 0,
            pgah_alarm: status & 0x10 != 0,
            pgal_alarm: status & 0x08 != 0,
            ref_alarm: status & 0x04 != 0,
            drdy: status & 0x02 != 0,
            clock: status & 0x01 != 0,
            reset: status & 0x80 != 0,
        }
    }

    /// Check if any fault condition is active
    pub fn has_fault(&self) -> bool {
        self.pgah_alarm || self.pgal_alarm || self.ref_alarm
    }
}

// ============================================================================
// ADS1263 Driver
// ============================================================================

/// ADS1263 32-bit delta-sigma ADC driver
pub struct Ads1263 {
    config: Ads1263Config,
    /// Last status byte received
    last_status: u8,
    /// Offset calibration value (24-bit signed)
    offset_cal: i32,
    /// Gain calibration value (24-bit unsigned)
    gain_cal: u32,
    /// Whether the device has been initialized
    initialized: bool,
}

impl Ads1263 {
    /// Create a new ADS1263 driver with the given configuration
    pub fn new(config: Ads1263Config) -> Self {
        Self {
            config,
            last_status: 0,
            offset_cal: 0,
            gain_cal: 0x400000, // Default gain = 1.0 (0x400000)
            initialized: false,
        }
    }

    /// Create with default configuration
    pub fn new_default() -> Self {
        Self::new(Ads1263Config::default())
    }

    /// Initialize the ADS1263
    ///
    /// Performs reset, verifies device ID, and writes configuration registers.
    pub async fn init(&mut self, spi: &mut Spi1Adc) -> Result<(), AdcError> {
        // Reset the device
        self.send_command(spi, cmd::RESET).await?;
        Timer::after_millis(10).await; // Wait for reset to complete

        // Verify device ID
        let id = self.read_register(spi, regs::ID).await?;
        if id != ADS1263_ID {
            error!("ADS1263: wrong device ID 0x{:02X}, expected 0x{:02X}", id, ADS1263_ID);
            return Err(AdcError::InitFailed);
        }
        info!("ADS1263: device ID verified (0x{:02X})", id);

        // Configure POWER register: enable internal reference
        let power = if self.config.internal_ref {
            power_bits::INTREF
        } else {
            0
        };
        self.write_register(spi, regs::POWER, power).await?;

        // Configure INTERFACE register: status + checksum
        let mut iface = 0u8;
        if self.config.status_enabled {
            iface |= interface_bits::STATUS;
        }
        if self.config.checksum_enabled {
            iface |= interface_bits::CRC_CHKSUM;
        }
        iface |= interface_bits::TIMEOUT;
        self.write_register(spi, regs::INTERFACE, iface).await?;

        // Configure MODE0: conversion delay, chop, run mode
        let mut mode0 = mode0_bits::DELAY_8_7US;
        if self.config.chop_mode {
            mode0 |= mode0_bits::CHOP;
        }
        if !self.config.continuous {
            mode0 |= mode0_bits::RUN_PULSE;
        }
        self.write_register(spi, regs::MODE0, mode0).await?;

        // Configure MODE1: digital filter
        let mode1 = match self.config.filter {
            FilterMode::Sinc1 => mode1_bits::FILTER_SINC1,
            FilterMode::Sinc2 => mode1_bits::FILTER_SINC2,
            FilterMode::Sinc3 => mode1_bits::FILTER_SINC3,
            FilterMode::Sinc4 => mode1_bits::FILTER_SINC4,
            FilterMode::Fir => mode1_bits::FILTER_FIR,
        };
        self.write_register(spi, regs::MODE1, mode1).await?;

        // Configure MODE2: PGA gain + data rate
        let gain_bits = match self.config.gain {
            PgaGain::Gain1 => mode2_bits::GAIN_1,
            PgaGain::Gain2 => mode2_bits::GAIN_2,
            PgaGain::Gain4 => mode2_bits::GAIN_4,
            PgaGain::Gain8 => mode2_bits::GAIN_8,
            PgaGain::Gain16 => mode2_bits::GAIN_16,
            PgaGain::Gain32 => mode2_bits::GAIN_32,
        };
        let dr_bits = self.config.data_rate as u8;
        self.write_register(spi, regs::MODE2, gain_bits | dr_bits).await?;

        // Configure REFMUX: internal 2.5V reference
        self.write_register(
            spi,
            regs::REFMUX,
            refmux::RMUXP_INT | refmux::RMUXN_INT,
        ).await?;

        // Default INPMUX: AIN0 vs AINCOM
        self.write_register(
            spi,
            regs::INPMUX,
            inpmux::MUXP_AIN0 | inpmux::MUXN_AINCOM,
        ).await?;

        // Wait for reference to settle
        Timer::after_millis(50).await;

        self.initialized = true;
        info!("ADS1263: initialized (gain={}, rate={}, filter={})",
            self.config.gain, self.config.data_rate, self.config.filter);

        Ok(())
    }

    /// Set the input channel multiplexer
    pub async fn set_channel(&mut self, spi: &mut Spi1Adc, channel: ChannelPair) -> Result<(), AdcError> {
        self.write_register(spi, regs::INPMUX, channel.to_mux_byte()).await
    }

    /// Start ADC1 conversion
    pub async fn start_conversion(&mut self, spi: &mut Spi1Adc) -> Result<(), AdcError> {
        self.send_command(spi, cmd::START1).await
    }

    /// Stop ADC1 conversion
    pub async fn stop_conversion(&mut self, spi: &mut Spi1Adc) -> Result<(), AdcError> {
        self.send_command(spi, cmd::STOP1).await
    }

    /// Read ADC1 conversion data
    ///
    /// Returns a 32-bit signed reading with status and checksum validation.
    /// The data format depends on INTERFACE register settings:
    ///   [STATUS] + DATA[3:0] + [CHECKSUM]
    pub async fn read_data(&mut self, spi: &mut Spi1Adc) -> Result<Ads1263Reading, AdcError> {
        let has_status = self.config.status_enabled;
        let has_checksum = self.config.checksum_enabled;

        // Calculate total bytes: 1 (cmd) + optional status + 4 data + optional checksum
        let data_len = 4;
        let total = 1 + (if has_status { 1 } else { 0 }) + data_len + (if has_checksum { 1 } else { 0 });

        let mut tx = [0u8; 8];
        let mut rx = [0u8; 8];
        tx[0] = cmd::RDATA1;
        // Remaining bytes are NOP (0x00)

        spi.cs.assert();
        let result = spi.spi.transfer(&mut rx[..total], &tx[..total]).await;
        spi.cs.deassert();

        if result.is_err() {
            return Err(AdcError::SpiError);
        }

        let timestamp_us = Instant::now().as_micros();

        // Parse response
        let mut idx = 1; // Skip the first byte (response to command byte)

        let status = if has_status {
            let s = rx[idx];
            idx += 1;
            self.last_status = s;
            s
        } else {
            0
        };

        // 4 bytes of data, MSB first, signed
        let raw = ((rx[idx] as i32) << 24)
            | ((rx[idx + 1] as i32) << 16)
            | ((rx[idx + 2] as i32) << 8)
            | (rx[idx + 3] as i32);
        idx += 4;

        // Verify checksum if enabled
        let checksum_ok = if has_checksum {
            let received_chk = rx[idx];
            // Checksum = sum of STATUS + DATA bytes, modulo 256, plus 0x9B
            let mut computed: u8 = 0x9B;
            let start = 1; // After command byte
            for i in start..idx {
                computed = computed.wrapping_add(rx[i]);
            }
            computed == received_chk
        } else {
            true
        };

        if !checksum_ok {
            warn!("ADS1263: checksum mismatch");
        }

        Ok(Ads1263Reading {
            raw,
            status,
            checksum_ok,
            timestamp_us,
        })
    }

    /// Poll for data ready, then read
    ///
    /// Polls the status register until new data is available, or until timeout.
    pub async fn read_data_blocking(
        &mut self,
        spi: &mut Spi1Adc,
        timeout_ms: u32,
    ) -> Result<Ads1263Reading, AdcError> {
        let start = Instant::now();
        let timeout_us = timeout_ms as u64 * 1000;

        loop {
            let reading = self.read_data(spi).await?;
            if reading.is_new_data() {
                return Ok(reading);
            }

            if Instant::now().as_micros() - start.as_micros() > timeout_us {
                return Err(AdcError::Timeout);
            }

            // Small delay between polls
            Timer::after_micros(100).await;
        }
    }

    /// Read a specific channel (set mux, start, wait, read)
    pub async fn read_channel(
        &mut self,
        spi: &mut Spi1Adc,
        channel: ChannelPair,
        timeout_ms: u32,
    ) -> Result<Ads1263Reading, AdcError> {
        // Switch channel
        self.set_channel(spi, channel).await?;

        // If in pulse mode, start conversion
        if !self.config.continuous {
            self.start_conversion(spi).await?;
        }

        // Allow settling time based on data rate
        Timer::after_micros(200).await;

        // Read result
        self.read_data_blocking(spi, timeout_ms).await
    }

    // ── Calibration ──

    /// Run self offset calibration (ADC1)
    ///
    /// Inputs are internally shorted and the offset is measured and stored.
    /// Takes approximately 2^24 / data_rate seconds.
    pub async fn self_offset_calibrate(&mut self, spi: &mut Spi1Adc) -> Result<(), AdcError> {
        info!("ADS1263: starting self offset calibration...");
        self.send_command(spi, cmd::SFOCAL1).await?;
        // Wait for calibration to complete (depends on data rate, up to ~10s at 2.5 SPS)
        Timer::after_millis(1000).await;

        // Read back calibration registers
        let ofcal0 = self.read_register(spi, regs::OFCAL0).await?;
        let ofcal1 = self.read_register(spi, regs::OFCAL1).await?;
        let ofcal2 = self.read_register(spi, regs::OFCAL2).await?;
        self.offset_cal = ((ofcal2 as i32) << 16) | ((ofcal1 as i32) << 8) | (ofcal0 as i32);
        // Sign extend from 24-bit
        if self.offset_cal & 0x800000 != 0 {
            self.offset_cal |= 0xFF000000u32 as i32;
        }

        info!("ADS1263: offset cal = {}", self.offset_cal);
        Ok(())
    }

    /// Run system gain calibration (ADC1)
    ///
    /// Apply a known full-scale voltage to the inputs before calling.
    pub async fn system_gain_calibrate(&mut self, spi: &mut Spi1Adc) -> Result<(), AdcError> {
        info!("ADS1263: starting system gain calibration...");
        self.send_command(spi, cmd::SYGCAL1).await?;
        Timer::after_millis(1000).await;

        let fscal0 = self.read_register(spi, regs::FSCAL0).await?;
        let fscal1 = self.read_register(spi, regs::FSCAL1).await?;
        let fscal2 = self.read_register(spi, regs::FSCAL2).await?;
        self.gain_cal = ((fscal2 as u32) << 16) | ((fscal1 as u32) << 8) | (fscal0 as u32);

        info!("ADS1263: gain cal = 0x{:06X}", self.gain_cal);
        Ok(())
    }

    /// Write offset calibration registers directly
    pub async fn set_offset_calibration(
        &mut self,
        spi: &mut Spi1Adc,
        offset: i32,
    ) -> Result<(), AdcError> {
        self.write_register(spi, regs::OFCAL0, (offset & 0xFF) as u8).await?;
        self.write_register(spi, regs::OFCAL1, ((offset >> 8) & 0xFF) as u8).await?;
        self.write_register(spi, regs::OFCAL2, ((offset >> 16) & 0xFF) as u8).await?;
        self.offset_cal = offset;
        Ok(())
    }

    /// Write gain calibration registers directly
    pub async fn set_gain_calibration(
        &mut self,
        spi: &mut Spi1Adc,
        gain: u32,
    ) -> Result<(), AdcError> {
        self.write_register(spi, regs::FSCAL0, (gain & 0xFF) as u8).await?;
        self.write_register(spi, regs::FSCAL1, ((gain >> 8) & 0xFF) as u8).await?;
        self.write_register(spi, regs::FSCAL2, ((gain >> 16) & 0xFF) as u8).await?;
        self.gain_cal = gain;
        Ok(())
    }

    // ── ADC2 (24-bit auxiliary) ──

    /// Configure and start ADC2
    pub async fn start_adc2(
        &mut self,
        spi: &mut Spi1Adc,
        mux_positive: u8,
        mux_negative: u8,
    ) -> Result<(), AdcError> {
        // Set ADC2 mux
        let mux = ((mux_positive & 0x0F) << 4) | (mux_negative & 0x0F);
        self.write_register(spi, regs::ADC2MUX, mux).await?;

        // Configure ADC2: 100 SPS, internal ref, gain 1
        self.write_register(spi, regs::ADC2CFG, adc2cfg::DR2_100 | adc2cfg::REF2_INT).await?;

        // Start ADC2
        self.send_command(spi, cmd::START2).await
    }

    /// Read ADC2 data (24-bit)
    pub async fn read_adc2(&mut self, spi: &mut Spi1Adc) -> Result<i32, AdcError> {
        let mut tx = [0u8; 5];
        let mut rx = [0u8; 5];
        tx[0] = cmd::RDATA2;

        spi.cs.assert();
        let result = spi.spi.transfer(&mut rx[..5], &tx[..5]).await;
        spi.cs.deassert();

        if result.is_err() {
            return Err(AdcError::SpiError);
        }

        // Parse: [cmd_resp, status, data2, data1, data0]
        let raw_unsigned = ((rx[2] as u32) << 16) | ((rx[3] as u32) << 8) | (rx[4] as u32);
        let raw = if raw_unsigned & 0x800000 != 0 {
            (raw_unsigned | 0xFF000000) as i32
        } else {
            raw_unsigned as i32
        };

        Ok(raw)
    }

    // ── Internal temperature sensor ──

    /// Read the internal temperature sensor
    ///
    /// Temporarily switches INPMUX to the internal temp sensor,
    /// takes a reading, then restores the previous mux setting.
    pub async fn read_internal_temp(&mut self, spi: &mut Spi1Adc) -> Result<f32, AdcError> {
        // Save current mux
        let saved_mux = self.read_register(spi, regs::INPMUX).await?;

        // Set mux to internal temp sensor
        self.write_register(
            spi,
            regs::INPMUX,
            inpmux::MUXP_TEMP_P | inpmux::MUXN_TEMP_N,
        ).await?;

        // Wait for conversion
        Timer::after_millis(100).await;

        let reading = self.read_data_blocking(spi, 500).await?;

        // Restore previous mux
        self.write_register(spi, regs::INPMUX, saved_mux).await?;

        // Convert to temperature:
        // Internal temp sensor output = 122.4 uV/degC
        // With 2.5V reference: 1 LSB = 2.5V / 2^31 = 1.164 nV
        // So: degC = raw * 1.164e-9 / 122.4e-6 = raw * 9.51e-6
        // With offset: T = (V / 122.4e-6) + 25.0 (sensor is 0V at ~25C)
        let voltage = reading.to_voltage();
        let temp_c = (voltage / 122.4e-6) + 25.0;

        Ok(temp_c)
    }

    // ── Low-level SPI access ──

    /// Send a single-byte command
    async fn send_command(&mut self, spi: &mut Spi1Adc, command: u8) -> Result<(), AdcError> {
        spi.write(&[command]).await.map_err(|_| AdcError::SpiError)
    }

    /// Read a single register
    pub async fn read_register(&mut self, spi: &mut Spi1Adc, reg: u8) -> Result<u8, AdcError> {
        let tx = [cmd::RREG | reg, 0x00, 0x00]; // RREG | addr, count-1=0, NOP for data
        let mut rx = [0u8; 3];

        spi.cs.assert();
        let result = spi.spi.transfer(&mut rx, &tx).await;
        spi.cs.deassert();

        match result {
            Ok(_) => Ok(rx[2]),
            Err(_) => Err(AdcError::SpiError),
        }
    }

    /// Write a single register
    pub async fn write_register(
        &mut self,
        spi: &mut Spi1Adc,
        reg: u8,
        value: u8,
    ) -> Result<(), AdcError> {
        let tx = [cmd::WREG | reg, 0x00, value]; // WREG | addr, count-1=0, data
        spi.write(&tx).await.map_err(|_| AdcError::SpiError)
    }

    /// Read multiple consecutive registers
    pub async fn read_registers(
        &mut self,
        spi: &mut Spi1Adc,
        start_reg: u8,
        count: usize,
        buf: &mut [u8],
    ) -> Result<(), AdcError> {
        if count == 0 || count > buf.len() || count > 27 {
            return Err(AdcError::InvalidChannel);
        }

        let mut tx = [0u8; 29]; // Max: 2 cmd bytes + 27 register bytes
        tx[0] = cmd::RREG | start_reg;
        tx[1] = (count - 1) as u8;

        let total = 2 + count;
        let mut rx = [0u8; 29];

        spi.cs.assert();
        let result = spi.spi.transfer(&mut rx[..total], &tx[..total]).await;
        spi.cs.deassert();

        match result {
            Ok(_) => {
                buf[..count].copy_from_slice(&rx[2..2 + count]);
                Ok(())
            }
            Err(_) => Err(AdcError::SpiError),
        }
    }

    // ── Status helpers ──

    /// Get the last status byte
    pub fn last_status(&self) -> Ads1263Status {
        Ads1263Status::parse(self.last_status)
    }

    /// Check if the device is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get current offset calibration value
    pub fn offset_calibration(&self) -> i32 {
        self.offset_cal
    }

    /// Get current gain calibration value
    pub fn gain_calibration(&self) -> u32 {
        self.gain_cal
    }
}
