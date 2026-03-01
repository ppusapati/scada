/// ADS1258 24-bit Multi-Channel ADC Driver
///
/// The ADS1258 is a 24-bit delta-sigma ADC from Texas Instruments with
/// 16 single-ended or 8 differential inputs. Used in the Water RTU
/// for multi-channel monitoring (level, flow, pressure, pH, etc.).
///
/// Key specifications:
///   - 24-bit resolution, up to 23.7 kSPS per channel
///   - 16 single-ended or 8 differential inputs (auto-scan)
///   - Fixed-channel mode or auto-scan mode
///   - Internal oscillator or external clock
///   - SPI interface: Mode 1 (CPOL=0, CPHA=1), up to 16 MHz
///   - Data ready output (DRDY) for interrupt-driven reads
///   - Burnout current sources for open-wire detection
///
/// Register map (from ADS1258 datasheet SBAS498B):
///   Addr  Name      Description
///   0x00  CONFIG0   Mux control, scan mode, burnout, CHOP
///   0x01  CONFIG1   IDAC config, DLY
///   0x02  MUXSCH    Channel scan selection (start:end)
///   0x03  MUXDIF    Differential channel enable
///   0x04  MUXSG0    Single-ended channels 0-7 enable
///   0x05  MUXSG1    Single-ended channels 8-15 enable
///   0x06  SYSRED    System reading enable (offset, gain, temp, ref, VCC)
///   0x07  GPIOC     GPIO configuration
///   0x08  GPIOD     GPIO data
///   0x09  ID        Device ID

use crate::hal::spi::Spi1Adc;
use crate::hal::adc::{AdcError, Ads1258Reading, WaterMultiChannelReading};
use defmt::{info, warn, error, Format};
use embassy_time::{Timer, Instant};

// ============================================================================
// Register addresses
// ============================================================================

#[allow(dead_code)]
pub mod regs {
    pub const CONFIG0: u8 = 0x00;
    pub const CONFIG1: u8 = 0x01;
    pub const MUXSCH: u8 = 0x02;
    pub const MUXDIF: u8 = 0x03;
    pub const MUXSG0: u8 = 0x04;
    pub const MUXSG1: u8 = 0x05;
    pub const SYSRED: u8 = 0x06;
    pub const GPIOC: u8 = 0x07;
    pub const GPIOD: u8 = 0x08;
    pub const ID: u8 = 0x09;
}

// ============================================================================
// SPI command bytes
// ============================================================================

#[allow(dead_code)]
pub mod cmd {
    /// Read direct: returns status + data without register access
    /// Byte pattern: 0b011r_rrrr where rrrrr = channel read-back bits
    pub const READ_DIRECT: u8 = 0x60;
    /// Read register: 0b01rr_rrrr (r = reg address)
    pub const RREG_BASE: u8 = 0x40;
    /// Write register: 0b011r_rrrr (but not same as read_direct)
    pub const WREG_BASE: u8 = 0x60;
    /// Channel read command
    pub const CHANNEL_READ: u8 = 0x00;
    /// Pulse convert (in fixed channel mode)
    pub const PULSE_CONVERT: u8 = 0x80;
    /// Reset
    pub const RESET: u8 = 0xC0;
}

// ============================================================================
// Register bit definitions
// ============================================================================

/// CONFIG0 register (0x00)
#[allow(dead_code)]
pub mod config0 {
    /// Scan mode (bits 7:6)
    pub const SCAN_AUTO: u8 = 0b00 << 6;      // Auto-scan enabled channels
    pub const SCAN_FIXED: u8 = 0b01 << 6;     // Fixed channel from MUXSCH
    pub const SCAN_MANUAL: u8 = 0b10 << 6;    // Manual (register per read)

    /// MUX mode (bit 5): 0 = auto, 1 = fixed
    pub const MUXMOD_AUTO: u8 = 0 << 5;
    pub const MUXMOD_FIXED: u8 = 1 << 5;

    /// CHOP enable (bit 4)
    pub const CHOP: u8 = 1 << 4;

    /// Burnout current source (bits 3:2)
    pub const BYPASS_NONE: u8 = 0b00 << 2;
    pub const BYPASS_0_5UA: u8 = 0b01 << 2;
    pub const BYPASS_2UA: u8 = 0b10 << 2;
    pub const BYPASS_10UA: u8 = 0b11 << 2;

    /// Status enable (bit 1): prepend status byte to data
    pub const STAT: u8 = 1 << 1;

    /// Clock select (bit 0): 0 = internal, 1 = external
    pub const CLKENB: u8 = 1 << 0;
}

/// CONFIG1 register (0x01)
#[allow(dead_code)]
pub mod config1 {
    /// IDAC enable bits (7:6)
    pub const IDAC_OFF: u8 = 0b00 << 6;
    pub const IDAC_AIN0: u8 = 0b01 << 6;
    pub const IDAC_AIN1: u8 = 0b10 << 6;

    /// Conversion delay (bits 4:2)
    pub const DLY_NONE: u8 = 0b000 << 2;
    pub const DLY_8US: u8 = 0b001 << 2;
    pub const DLY_16US: u8 = 0b010 << 2;
    pub const DLY_32US: u8 = 0b011 << 2;
    pub const DLY_64US: u8 = 0b100 << 2;
    pub const DLY_128US: u8 = 0b101 << 2;
    pub const DLY_256US: u8 = 0b110 << 2;
    pub const DLY_384US: u8 = 0b111 << 2;

    /// Data rate prescaler (bits 1:0)
    pub const DRATE_DIV1: u8 = 0b00;   // Max speed
    pub const DRATE_DIV2: u8 = 0b01;
    pub const DRATE_DIV4: u8 = 0b10;
    pub const DRATE_DIV8: u8 = 0b11;
}

/// SYSRED register (0x06) -- system reading enable
#[allow(dead_code)]
pub mod sysred {
    /// Offset calibration reading enable
    pub const OFFSET: u8 = 1 << 7;
    /// VCC measurement enable
    pub const VCC: u8 = 1 << 6;
    /// Temperature measurement enable
    pub const TEMP: u8 = 1 << 5;
    /// Gain calibration reading enable
    pub const GAIN: u8 = 1 << 4;
    /// Reference voltage measurement enable
    pub const REF: u8 = 1 << 3;
}

// ============================================================================
// Expected device ID
// ============================================================================

const ADS1258_ID_MASK: u8 = 0xF0;
const ADS1258_ID_VALUE: u8 = 0xA0; // Upper nibble = 0xA for ADS1258

// ============================================================================
// Configuration structures
// ============================================================================

/// ADS1258 scan mode
#[derive(Clone, Copy, PartialEq, Format)]
pub enum ScanMode {
    /// Auto-scan through all enabled channels
    Auto,
    /// Fixed single channel from MUXSCH register
    Fixed,
}

/// ADS1258 data rate prescaler
#[derive(Clone, Copy, PartialEq, Format)]
pub enum DataRatePrescaler {
    Div1 = 0, // ~23.7 kSPS
    Div2 = 1, // ~11.9 kSPS
    Div4 = 2, // ~5.9 kSPS
    Div8 = 3, // ~3.0 kSPS
}

/// ADS1258 driver configuration
pub struct Ads1258Config {
    /// Scan mode
    pub scan_mode: ScanMode,
    /// Enable status byte in output
    pub status_enabled: bool,
    /// CHOP mode for offset correction
    pub chop: bool,
    /// Data rate prescaler
    pub data_rate: DataRatePrescaler,
    /// Conversion delay
    pub delay_us: u16,
    /// Single-ended channels to enable (bitmask, channels 0-15)
    pub se_channels: u16,
    /// Differential channels to enable (bitmask, pairs 0-7)
    pub diff_channels: u8,
    /// Fixed channel for SCAN_FIXED mode (positive, negative)
    pub fixed_channel: (u8, u8),
    /// Burnout current enable
    pub burnout: bool,
}

impl Default for Ads1258Config {
    fn default() -> Self {
        Self {
            scan_mode: ScanMode::Auto,
            status_enabled: true,
            chop: false,
            data_rate: DataRatePrescaler::Div4,
            delay_us: 32,
            se_channels: 0x00FF, // Enable channels 0-7
            diff_channels: 0x00,
            fixed_channel: (0, 0xFF), // AINCOM for negative
            burnout: false,
        }
    }
}

// ============================================================================
// Status byte parsing
// ============================================================================

/// Parsed ADS1258 status byte
#[derive(Clone, Copy, Debug, Format)]
pub struct Ads1258Status {
    /// New data available
    pub new_data: bool,
    /// Overrange indicator
    pub ovr: bool,
    /// Channel ID (0-15 for SE, 0-7 for diff, or system channel)
    pub channel_id: u8,
    /// Supply out of range
    pub supply: bool,
}

impl Ads1258Status {
    /// Parse from raw status byte
    pub fn parse(status: u8) -> Self {
        Self {
            new_data: status & 0x80 != 0,
            ovr: status & 0x40 != 0,
            channel_id: (status >> 3) & 0x0F,
            supply: status & 0x04 != 0,
        }
    }

    /// Check if any fault is present
    pub fn has_fault(&self) -> bool {
        self.ovr || self.supply
    }
}

// ============================================================================
// ADS1258 Driver
// ============================================================================

/// ADS1258 24-bit multi-channel ADC driver
pub struct Ads1258 {
    config: Ads1258Config,
    initialized: bool,
    scan_sequence: u32,
}

impl Ads1258 {
    /// Create a new ADS1258 driver with the given configuration
    pub fn new(config: Ads1258Config) -> Self {
        Self {
            config,
            initialized: false,
            scan_sequence: 0,
        }
    }

    /// Create with default configuration (auto-scan, channels 0-7)
    pub fn new_default() -> Self {
        Self::new(Ads1258Config::default())
    }

    /// Initialize the ADS1258
    pub async fn init(&mut self, spi: &mut Spi1Adc) -> Result<(), AdcError> {
        // Send reset command
        self.send_command(spi, cmd::RESET).await?;
        Timer::after_millis(10).await;

        // Read and verify device ID
        let id = self.read_register(spi, regs::ID).await?;
        if (id & ADS1258_ID_MASK) != ADS1258_ID_VALUE {
            error!("ADS1258: wrong device ID 0x{:02X}, expected 0xAx", id);
            return Err(AdcError::InitFailed);
        }
        info!("ADS1258: device ID verified (0x{:02X})", id);

        // Configure CONFIG0: scan mode, chop, status, burnout
        let mut cfg0: u8 = 0;
        cfg0 |= match self.config.scan_mode {
            ScanMode::Auto => config0::SCAN_AUTO,
            ScanMode::Fixed => config0::SCAN_FIXED,
        };
        if self.config.chop {
            cfg0 |= config0::CHOP;
        }
        if self.config.status_enabled {
            cfg0 |= config0::STAT;
        }
        if self.config.burnout {
            cfg0 |= config0::BYPASS_0_5UA;
        }
        self.write_register(spi, regs::CONFIG0, cfg0).await?;

        // Configure CONFIG1: data rate and conversion delay
        let mut cfg1: u8 = self.config.data_rate as u8;
        cfg1 |= match self.config.delay_us {
            0 => config1::DLY_NONE,
            1..=8 => config1::DLY_8US,
            9..=16 => config1::DLY_16US,
            17..=32 => config1::DLY_32US,
            33..=64 => config1::DLY_64US,
            65..=128 => config1::DLY_128US,
            129..=256 => config1::DLY_256US,
            _ => config1::DLY_384US,
        };
        self.write_register(spi, regs::CONFIG1, cfg1).await?;

        // Configure channel enables
        if self.config.scan_mode == ScanMode::Auto {
            // Single-ended channel enable (lower 8 bits)
            let muxsg0 = (self.config.se_channels & 0xFF) as u8;
            self.write_register(spi, regs::MUXSG0, muxsg0).await?;

            // Single-ended channel enable (upper 8 bits)
            let muxsg1 = ((self.config.se_channels >> 8) & 0xFF) as u8;
            self.write_register(spi, regs::MUXSG1, muxsg1).await?;

            // Differential channel enable
            self.write_register(spi, regs::MUXDIF, self.config.diff_channels).await?;
        } else {
            // Fixed channel mode: set MUXSCH register
            let muxsch = ((self.config.fixed_channel.0 & 0x0F) << 4)
                | (self.config.fixed_channel.1 & 0x0F);
            self.write_register(spi, regs::MUXSCH, muxsch).await?;
        }

        // Enable system readings (temperature, VCC monitoring)
        self.write_register(spi, regs::SYSRED, sysred::TEMP | sysred::VCC).await?;

        self.initialized = true;
        info!("ADS1258: initialized (mode={}, channels=0x{:04X})",
            self.config.scan_mode, self.config.se_channels);

        Ok(())
    }

    /// Read a single channel data frame
    ///
    /// In auto-scan mode, returns the next available channel.
    /// In fixed-channel mode, returns the configured channel.
    ///
    /// Data format: [STATUS, DATA2, DATA1, DATA0] (4 bytes with status enabled)
    /// or [DATA2, DATA1, DATA0] (3 bytes without status)
    pub async fn read_data(&mut self, spi: &mut Spi1Adc) -> Result<Ads1258Reading, AdcError> {
        let has_status = self.config.status_enabled;
        let total_bytes = if has_status { 4 } else { 3 };

        let mut tx = [0u8; 4];
        let mut rx = [0u8; 4];

        spi.cs.assert();
        let result = spi.spi.transfer(&mut rx[..total_bytes], &tx[..total_bytes]).await;
        spi.cs.deassert();

        if result.is_err() {
            return Err(AdcError::SpiError);
        }

        let timestamp_us = Instant::now().as_micros();

        if has_status {
            let status = rx[0];
            let parsed = Ads1258Status::parse(status);

            // 24-bit signed, MSB first
            let raw_unsigned = ((rx[1] as u32) << 16) | ((rx[2] as u32) << 8) | (rx[3] as u32);
            let raw = if raw_unsigned & 0x800000 != 0 {
                (raw_unsigned | 0xFF000000) as i32
            } else {
                raw_unsigned as i32
            };

            Ok(Ads1258Reading {
                raw,
                channel: parsed.channel_id,
                status,
                new_data: parsed.new_data,
                timestamp_us,
            })
        } else {
            let raw_unsigned = ((rx[0] as u32) << 16) | ((rx[1] as u32) << 8) | (rx[2] as u32);
            let raw = if raw_unsigned & 0x800000 != 0 {
                (raw_unsigned | 0xFF000000) as i32
            } else {
                raw_unsigned as i32
            };

            Ok(Ads1258Reading {
                raw,
                channel: 0, // Unknown without status byte
                status: 0,
                new_data: true,
                timestamp_us,
            })
        }
    }

    /// Read data, waiting for DRDY (data ready)
    pub async fn read_data_blocking(
        &mut self,
        spi: &mut Spi1Adc,
        timeout_ms: u32,
    ) -> Result<Ads1258Reading, AdcError> {
        let start = Instant::now();
        let timeout_us = timeout_ms as u64 * 1000;

        loop {
            let reading = self.read_data(spi).await?;
            if reading.new_data {
                return Ok(reading);
            }

            if Instant::now().as_micros() - start.as_micros() > timeout_us {
                return Err(AdcError::Timeout);
            }

            Timer::after_micros(50).await;
        }
    }

    /// Perform a complete auto-scan of all enabled channels
    ///
    /// Reads each enabled channel once and returns the collected results.
    /// In auto-scan mode, the ADC cycles through enabled channels automatically.
    pub async fn scan_all_channels(
        &mut self,
        spi: &mut Spi1Adc,
        timeout_ms: u32,
    ) -> Result<WaterMultiChannelReading, AdcError> {
        let mut result = WaterMultiChannelReading::new();
        result.scan_start_us = Instant::now().as_micros();
        result.sequence = self.scan_sequence;
        self.scan_sequence = self.scan_sequence.wrapping_add(1);

        // Count enabled channels
        let enabled_count = self.config.se_channels.count_ones()
            + self.config.diff_channels.count_ones();

        if enabled_count == 0 {
            return Err(AdcError::InvalidChannel);
        }

        let start = Instant::now();
        let timeout_us = timeout_ms as u64 * 1000;
        let mut channels_read: u16 = 0;

        // Read until all enabled channels have been sampled or timeout
        while channels_read.count_ones() < enabled_count {
            let reading = self.read_data(spi).await?;

            if reading.new_data && (reading.channel as usize) < 16 {
                let ch = reading.channel as usize;
                if result.channels[ch].is_none() {
                    result.channels[ch] = Some(reading);
                    result.valid_count += 1;
                    channels_read |= 1 << ch;
                }
            }

            if Instant::now().as_micros() - start.as_micros() > timeout_us {
                warn!("ADS1258: scan timeout, got {}/{} channels",
                    result.valid_count, enabled_count);
                break;
            }

            Timer::after_micros(20).await;
        }

        result.scan_end_us = Instant::now().as_micros();
        Ok(result)
    }

    /// Set fixed channel for single-channel reading
    pub async fn set_fixed_channel(
        &mut self,
        spi: &mut Spi1Adc,
        positive: u8,
        negative: u8,
    ) -> Result<(), AdcError> {
        let muxsch = ((positive & 0x0F) << 4) | (negative & 0x0F);
        self.write_register(spi, regs::MUXSCH, muxsch).await?;
        self.config.fixed_channel = (positive, negative);
        Ok(())
    }

    /// Enable or disable specific single-ended channels
    pub async fn set_se_channels(
        &mut self,
        spi: &mut Spi1Adc,
        channels: u16,
    ) -> Result<(), AdcError> {
        self.config.se_channels = channels;
        self.write_register(spi, regs::MUXSG0, (channels & 0xFF) as u8).await?;
        self.write_register(spi, regs::MUXSG1, ((channels >> 8) & 0xFF) as u8).await
    }

    /// Enable or disable specific differential channels
    pub async fn set_diff_channels(
        &mut self,
        spi: &mut Spi1Adc,
        channels: u8,
    ) -> Result<(), AdcError> {
        self.config.diff_channels = channels;
        self.write_register(spi, regs::MUXDIF, channels).await
    }

    /// Set the data rate prescaler
    pub async fn set_data_rate(
        &mut self,
        spi: &mut Spi1Adc,
        prescaler: DataRatePrescaler,
    ) -> Result<(), AdcError> {
        self.config.data_rate = prescaler;
        let cfg1 = self.read_register(spi, regs::CONFIG1).await?;
        let new_cfg1 = (cfg1 & 0xFC) | (prescaler as u8);
        self.write_register(spi, regs::CONFIG1, new_cfg1).await
    }

    /// Switch between auto-scan and fixed-channel mode
    pub async fn set_scan_mode(
        &mut self,
        spi: &mut Spi1Adc,
        mode: ScanMode,
    ) -> Result<(), AdcError> {
        self.config.scan_mode = mode;
        let cfg0 = self.read_register(spi, regs::CONFIG0).await?;
        let mode_bits = match mode {
            ScanMode::Auto => config0::SCAN_AUTO,
            ScanMode::Fixed => config0::SCAN_FIXED,
        };
        let new_cfg0 = (cfg0 & 0x3F) | mode_bits;
        self.write_register(spi, regs::CONFIG0, new_cfg0).await
    }

    /// Enable burnout current source for open-wire detection
    pub async fn set_burnout(
        &mut self,
        spi: &mut Spi1Adc,
        enable: bool,
    ) -> Result<(), AdcError> {
        self.config.burnout = enable;
        let cfg0 = self.read_register(spi, regs::CONFIG0).await?;
        let new_cfg0 = if enable {
            (cfg0 & 0xF3) | config0::BYPASS_0_5UA
        } else {
            cfg0 & 0xF3
        };
        self.write_register(spi, regs::CONFIG0, new_cfg0).await
    }

    /// Read the internal temperature sensor
    ///
    /// Temperature is available when SYSRED.TEMP is enabled.
    /// The temperature reading appears as a system channel in auto-scan mode.
    pub async fn read_system_temp(&mut self, spi: &mut Spi1Adc) -> Result<f32, AdcError> {
        // Ensure temp reading is enabled
        let sysred_val = self.read_register(spi, regs::SYSRED).await?;
        if sysred_val & sysred::TEMP == 0 {
            self.write_register(spi, regs::SYSRED, sysred_val | sysred::TEMP).await?;
        }

        // In auto-scan mode, wait for the temperature channel to appear
        // System channel IDs: offset=0x10, vcc=0x11, temp=0x12, gain=0x13, ref=0x14
        let start = Instant::now();
        loop {
            let reading = self.read_data(spi).await?;
            // Check for temperature system channel (0x12 in some implementations)
            // The exact channel ID depends on which system readings are enabled
            if reading.new_data && reading.channel >= 0x10 {
                // Temperature conversion: LSB = 0.3125 mV, T = (V / 0.3976e-3) - 273.15
                let voltage = reading.to_voltage();
                let temp_c = (voltage / 0.3976e-3) - 273.15;
                return Ok(temp_c);
            }

            if Instant::now().as_micros() - start.as_micros() > 500_000 {
                return Err(AdcError::Timeout);
            }
            Timer::after_micros(50).await;
        }
    }

    /// Read VCC supply voltage
    pub async fn read_vcc(&mut self, spi: &mut Spi1Adc) -> Result<f32, AdcError> {
        let sysred_val = self.read_register(spi, regs::SYSRED).await?;
        if sysred_val & sysred::VCC == 0 {
            self.write_register(spi, regs::SYSRED, sysred_val | sysred::VCC).await?;
        }

        let start = Instant::now();
        loop {
            let reading = self.read_data(spi).await?;
            if reading.new_data && reading.channel >= 0x10 {
                // VCC reading: direct voltage in ADC counts
                let voltage = reading.to_voltage();
                // The VCC monitor has a built-in divider; scale factor is ~4x
                let vcc = voltage * 4.0;
                return Ok(vcc);
            }

            if Instant::now().as_micros() - start.as_micros() > 500_000 {
                return Err(AdcError::Timeout);
            }
            Timer::after_micros(50).await;
        }
    }

    // ── GPIO control (ADS1258 has 4 GPIOs) ──

    /// Configure GPIO pins (4 available: GPIO0-GPIO3)
    /// direction: bit set = output, bit clear = input
    pub async fn configure_gpio(
        &mut self,
        spi: &mut Spi1Adc,
        direction: u8,
    ) -> Result<(), AdcError> {
        self.write_register(spi, regs::GPIOC, direction & 0x0F).await
    }

    /// Write to GPIO output pins
    pub async fn write_gpio(
        &mut self,
        spi: &mut Spi1Adc,
        data: u8,
    ) -> Result<(), AdcError> {
        self.write_register(spi, regs::GPIOD, data & 0x0F).await
    }

    /// Read GPIO pin states
    pub async fn read_gpio(&mut self, spi: &mut Spi1Adc) -> Result<u8, AdcError> {
        self.read_register(spi, regs::GPIOD).await.map(|v| v & 0x0F)
    }

    // ── Low-level SPI access ──

    async fn send_command(&mut self, spi: &mut Spi1Adc, command: u8) -> Result<(), AdcError> {
        spi.write(&[command]).await.map_err(|_| AdcError::SpiError)
    }

    /// Read a single register
    pub async fn read_register(&mut self, spi: &mut Spi1Adc, reg: u8) -> Result<u8, AdcError> {
        // ADS1258 register read: first byte = 0b01aa_aaaa (a=addr), second byte = NOP
        let tx = [0x40 | (reg & 0x3F), 0x00];
        let mut rx = [0u8; 2];

        spi.cs.assert();
        let result = spi.spi.transfer(&mut rx, &tx).await;
        spi.cs.deassert();

        match result {
            Ok(_) => Ok(rx[1]),
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
        // ADS1258 register write: first byte = 0b011a_aaaa (a=addr), second byte = data
        let tx = [0x60 | (reg & 0x1F), value];
        spi.write(&tx).await.map_err(|_| AdcError::SpiError)
    }

    /// Read multiple consecutive registers
    pub async fn read_registers(
        &mut self,
        spi: &mut Spi1Adc,
        start_reg: u8,
        buf: &mut [u8],
    ) -> Result<(), AdcError> {
        if buf.is_empty() || buf.len() > 10 {
            return Err(AdcError::InvalidChannel);
        }

        // For multi-register read, send address then clock out data
        let mut tx = [0u8; 11];
        let mut rx = [0u8; 11];
        tx[0] = 0x40 | (start_reg & 0x3F);
        let total = 1 + buf.len();

        spi.cs.assert();
        let result = spi.spi.transfer(&mut rx[..total], &tx[..total]).await;
        spi.cs.deassert();

        match result {
            Ok(_) => {
                buf.copy_from_slice(&rx[1..total]);
                Ok(())
            }
            Err(_) => Err(AdcError::SpiError),
        }
    }

    // ── Status helpers ──

    /// Check if the device is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the current scan sequence number
    pub fn scan_sequence(&self) -> u32 {
        self.scan_sequence
    }

    /// Get the number of enabled channels
    pub fn enabled_channel_count(&self) -> u32 {
        self.config.se_channels.count_ones() + self.config.diff_channels.count_ones() as u32
    }
}
