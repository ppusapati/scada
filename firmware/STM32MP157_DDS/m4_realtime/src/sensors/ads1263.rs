/// ADS1263 — 32-Bit, 38.4 kSPS Delta-Sigma ADC Driver (STM32MP157 M4 Variant)
///
/// Datasheet: Texas Instruments SBAS661
///
/// Adapted from the Solar_SCADA_SMU STM32F407 driver for the STM32MP157 M4
/// co-processor. The primary differences are:
///   - Uses Embassy STM32MP1 HAL (embassy-stm32 with stm32mp157c feature)
///   - SPI peripheral is SPI1 on M4 (shared with A7 via ETZPC)
///   - DMA channels differ from STM32F407
///   - Results are published via DDS (RPMsg) instead of Modbus registers
///
/// The ADS1263 is a high-precision ADC with:
///   - 32-bit primary ADC (ADC1) + 24-bit auxiliary ADC (ADC2)
///   - 10 differential / 20 single-ended inputs on ADC1
///   - Programmable data rates: 2.5 SPS to 38,400 SPS
///   - PGA with gains 1-32
///   - SPI interface (CPOL=0, CPHA=1 -> SPI Mode 1)
///
/// On the SCADA PCB, the ADS1263 reads through 2x CD74HC4067 16:1 MUX.
/// Combined, this provides 40 analog channels (20 voltage + 20 current).
/// The MUX outputs feed ADS1263 AIN0 (MUX A) and AIN1 (MUX B).

use embassy_stm32::gpio::Output;
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::{info, warn, error, debug, Format};

/// ADS1263 register addresses and SPI commands
mod reg {
    pub const ID: u8 = 0x00;
    pub const POWER: u8 = 0x01;
    pub const INTERFACE: u8 = 0x02;
    pub const MODE0: u8 = 0x03;
    pub const MODE1: u8 = 0x04;
    pub const MODE2: u8 = 0x05;
    pub const INPMUX: u8 = 0x06;   // Input multiplexer (positive + negative)
    pub const OFCAL0: u8 = 0x07;   // Offset calibration
    pub const OFCAL1: u8 = 0x08;
    pub const OFCAL2: u8 = 0x09;
    pub const FSCAL0: u8 = 0x0A;   // Full-scale calibration
    pub const FSCAL1: u8 = 0x0B;
    pub const FSCAL2: u8 = 0x0C;
    pub const IDACMUX: u8 = 0x0D;  // IDAC mux
    pub const IDACMAG: u8 = 0x0E;  // IDAC magnitude
    pub const REFMUX: u8 = 0x0F;   // Reference mux
    pub const TDACP: u8 = 0x10;    // TDAC positive
    pub const TDACN: u8 = 0x11;    // TDAC negative
    pub const GPIOCON: u8 = 0x12;  // GPIO config
    pub const GPIODIR: u8 = 0x13;  // GPIO direction
    pub const GPIODAT: u8 = 0x14;  // GPIO data
    pub const ADC2CFG: u8 = 0x15;  // ADC2 configuration
    pub const ADC2MUX: u8 = 0x16;  // ADC2 input mux
    pub const ADC2OFC0: u8 = 0x17; // ADC2 offset cal
    pub const ADC2OFC1: u8 = 0x18;
    pub const ADC2FSC0: u8 = 0x19; // ADC2 full-scale cal
    pub const ADC2FSC1: u8 = 0x1A;

    // SPI commands
    pub const CMD_NOP: u8 = 0x00;
    pub const CMD_RESET: u8 = 0x06;
    pub const CMD_START1: u8 = 0x08;  // Start ADC1
    pub const CMD_STOP1: u8 = 0x0A;   // Stop ADC1
    pub const CMD_START2: u8 = 0x0C;  // Start ADC2
    pub const CMD_STOP2: u8 = 0x0E;   // Stop ADC2
    pub const CMD_RDATA1: u8 = 0x12;  // Read ADC1 data
    pub const CMD_RDATA2: u8 = 0x14;  // Read ADC2 data
    pub const CMD_SYOCAL1: u8 = 0x16; // System offset calibration ADC1
    pub const CMD_SYGCAL1: u8 = 0x17; // System gain calibration ADC1
    pub const CMD_SFOCAL1: u8 = 0x19; // Self offset calibration ADC1
    pub const CMD_RREG: u8 = 0x20;    // Read register (0x20 + addr)
    pub const CMD_WREG: u8 = 0x40;    // Write register (0x40 + addr)

    // Expected device ID
    pub const DEVICE_ID: u8 = 0x01;   // ADS1263 ID (upper 3 bits of ID register)
}

/// Data rate selections for MODE2 register bits [3:0]
#[derive(Clone, Copy, Format)]
#[repr(u8)]
pub enum DataRate {
    Sps2_5   = 0x00,
    Sps5     = 0x01,
    Sps10    = 0x02,
    Sps16_6  = 0x03,
    Sps20    = 0x04,  // Default for SCADA (50ms per sample)
    Sps50    = 0x05,
    Sps60    = 0x06,
    Sps100   = 0x07,  // Used for fast scanning
    Sps400   = 0x08,
    Sps1200  = 0x09,
    Sps2400  = 0x0A,
    Sps4800  = 0x0B,
    Sps7200  = 0x0C,
    Sps14400 = 0x0D,
    Sps19200 = 0x0E,
    Sps38400 = 0x0F,
}

/// PGA gain selections for MODE2 register bits [7:4]
#[derive(Clone, Copy, Format)]
#[repr(u8)]
pub enum PgaGain {
    Gain1  = 0x00,  // Default — full ±2.5V range
    Gain2  = 0x01,
    Gain4  = 0x02,
    Gain8  = 0x03,
    Gain16 = 0x04,
    Gain32 = 0x05,
}

/// Digital filter mode for MODE1 register bits [7:5]
#[derive(Clone, Copy, Format)]
#[repr(u8)]
pub enum FilterMode {
    Sinc1    = 0x00,
    Sinc2    = 0x20,
    Sinc3    = 0x40,
    Sinc4    = 0x60,
    Fir      = 0x80,
}

/// Reference source for REFMUX register
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum RefSource {
    Internal   = 0x00,  // Internal 2.5V reference
    Ref0Pins   = 0x09,  // AIN0REFP, AIN0REFN
    Ref1Pins   = 0x12,  // AIN1REFP, AIN1REFN
    AvddAvss   = 0x1B,  // AVDD - AVSS
}

/// Single ADC reading from ADS1263
#[derive(Clone, Copy, Default)]
pub struct AdcReading {
    pub raw_code: i32,
    pub voltage: f32,
    pub valid: bool,
    pub status_byte: u8,
}

/// ADS1263 driver for STM32MP157 M4 core
pub struct Ads1263<'a> {
    spi: Spi<'a, Async>,
    cs: Output<'a>,
    drdy: embassy_stm32::gpio::Input<'a>,
    reset: Output<'a>,
    /// Current PGA gain setting (for voltage conversion)
    pga_gain: u8,
    /// Reference voltage in volts
    vref: f32,
    /// Current data rate
    data_rate: DataRate,
    /// Whether CRC is enabled on data reads
    crc_enabled: bool,
    /// Whether STATUS byte is enabled on data reads
    status_enabled: bool,
}

impl<'a> Ads1263<'a> {
    pub fn new(
        spi: Spi<'a, Async>,
        cs: Output<'a>,
        drdy: embassy_stm32::gpio::Input<'a>,
        reset: Output<'a>,
    ) -> Self {
        Self {
            spi,
            cs,
            drdy,
            reset,
            pga_gain: 1,
            vref: 2.5,
            data_rate: DataRate::Sps100,
            crc_enabled: false,
            status_enabled: true,
        }
    }

    /// Hardware reset and initialization
    ///
    /// Configures the ADS1263 for SCADA solar string monitoring:
    ///   - Internal 2.5V reference
    ///   - PGA gain = 1 (full-scale ±2.5V)
    ///   - Data rate = 100 SPS (fast enough for 16-string scan at ~6 Hz)
    ///   - Sinc4 filter for 50/60 Hz rejection
    ///   - STATUS byte enabled for per-read diagnostics
    pub async fn init(&mut self) -> bool {
        info!("ADS1263: Initializing on STM32MP157 M4...");

        // Hardware reset (active low, min 4 CLKIN cycles = ~0.6 us at 7.68 MHz)
        self.reset.set_low();
        Timer::after(Duration::from_millis(10)).await;
        self.reset.set_high();
        Timer::after(Duration::from_millis(100)).await;

        // Software reset for good measure
        self.send_command(reg::CMD_RESET).await;
        Timer::after(Duration::from_millis(50)).await;

        // Verify device ID — upper 3 bits of ID register should be 0x01
        let id = self.read_register(reg::ID).await;
        if (id >> 5) != reg::DEVICE_ID {
            error!("ADS1263: ID mismatch (got 0x{:02X}, expected 0x01 in upper bits)", id);
            return false;
        }
        info!("ADS1263: Device ID verified (0x{:02X})", id);

        // POWER: enable internal reference, disable VBIAS
        // Bit 0 = INTREF (1=enabled), Bit 1 = VBIAS (0=disabled)
        self.write_register(reg::POWER, 0x01).await;

        // INTERFACE: STATUS byte enabled, CRC disabled for speed
        // Bit 2 = STATUS (1=enabled), Bit 1:0 = CRC (00=disabled)
        self.write_register(reg::INTERFACE, 0x04).await;
        self.status_enabled = true;
        self.crc_enabled = false;

        // MODE0: continuous conversion, no delay, no chop
        // Bits [7:6]=REFREV, [5:4]=RUNMODE, [3:0]=DELAY
        // 0x00 = normal ref, continuous, no delay
        self.write_register(reg::MODE0, 0x00).await;

        // MODE1: Sinc4 filter for best 50/60 Hz rejection
        // Bits [7:5]=FILTER, [4:3]=SBADC, [2:0]=SBPOL
        self.write_register(reg::MODE1, FilterMode::Sinc4 as u8).await;

        // MODE2: PGA gain=1, data rate=100 SPS
        // Bits [7:4]=GAIN, [3:0]=DR
        let mode2 = (PgaGain::Gain1 as u8) << 4 | self.data_rate as u8;
        self.write_register(reg::MODE2, mode2).await;
        self.pga_gain = 1;

        // REFMUX: use internal 2.5V reference
        self.write_register(reg::REFMUX, RefSource::Internal as u8).await;
        self.vref = 2.5;

        // INPMUX: default to AIN0+ vs AINCOM- (MUX A voltage output)
        self.write_register(reg::INPMUX, 0x0A).await;

        // IDAC: disabled (not using excitation currents)
        self.write_register(reg::IDACMUX, 0xBB).await; // Disconnected
        self.write_register(reg::IDACMAG, 0x00).await;  // Off

        // Verify configuration by reading back MODE2
        let readback = self.read_register(reg::MODE2).await;
        if readback != mode2 {
            error!("ADS1263: MODE2 verify failed (got 0x{:02X}, expected 0x{:02X})", readback, mode2);
            return false;
        }

        // Run self-offset calibration for best accuracy
        info!("ADS1263: Running self-offset calibration...");
        self.send_command(reg::CMD_SFOCAL1).await;
        Timer::after(Duration::from_millis(200)).await;

        info!("ADS1263: Initialized (rate={} SPS, gain={}, Vref={}V)",
            100, self.pga_gain, 2.5f32);
        true
    }

    /// Set PGA gain and data rate (reconfigures MODE2)
    pub async fn configure(&mut self, gain: PgaGain, rate: DataRate) {
        let mode2 = (gain as u8) << 4 | rate as u8;
        self.write_register(reg::MODE2, mode2).await;
        self.pga_gain = match gain {
            PgaGain::Gain1 => 1,
            PgaGain::Gain2 => 2,
            PgaGain::Gain4 => 4,
            PgaGain::Gain8 => 8,
            PgaGain::Gain16 => 16,
            PgaGain::Gain32 => 32,
        };
        self.data_rate = rate;
    }

    /// Set the reference source
    pub async fn set_reference(&mut self, source: RefSource, vref: f32) {
        self.write_register(reg::REFMUX, source as u8).await;
        self.vref = vref;
    }

    /// Set input multiplexer to specific positive/negative inputs
    ///
    /// positive: AIN0-AIN9 (0x00-0x09), or AINCOM (0x0A)
    /// negative: AIN0-AIN9 (0x00-0x09), or AINCOM (0x0A)
    ///
    /// Common configurations for SCADA PCB:
    ///   (0, 0x0A) = MUX A voltage output vs AINCOM
    ///   (1, 0x0A) = MUX B current output vs AINCOM
    ///   (2, 0x0A) = Bus voltage direct input
    ///   (3, 0x0A) = Bus current direct input
    ///   (4, 0x0A) = Irradiance sensor
    ///   (5, 0x0A) = Module temperature (backup)
    pub async fn set_input_mux(&mut self, positive: u8, negative: u8) {
        let mux_val = ((positive & 0x0F) << 4) | (negative & 0x0F);
        self.write_register(reg::INPMUX, mux_val).await;
    }

    /// Read a single conversion from ADC1 (32-bit)
    ///
    /// Starts a conversion, waits for DRDY, and reads the result.
    /// Returns the raw signed 32-bit code.
    pub async fn read_adc1(&mut self) -> AdcReading {
        // Start conversion
        self.send_command(reg::CMD_START1).await;

        // Wait for DRDY (active low)
        let mut timeout = 0u32;
        while self.drdy.is_high() {
            Timer::after(Duration::from_micros(100)).await;
            timeout += 1;
            if timeout > 5000 { // 500ms timeout
                warn!("ADS1263: DRDY timeout");
                self.send_command(reg::CMD_STOP1).await;
                return AdcReading::default();
            }
        }

        // Read data: RDATA1 command
        // With STATUS enabled: cmd(1) + status(1) + data(4) = 6 bytes
        // Without STATUS:      cmd(1) + data(4) = 5 bytes
        if self.status_enabled {
            let tx = [reg::CMD_RDATA1, 0, 0, 0, 0, 0];
            let mut rx = [0u8; 6];
            self.cs.set_low();
            let _ = self.spi.transfer(&mut rx, &tx).await;
            self.cs.set_high();

            let status = rx[1];
            let raw = ((rx[2] as i32) << 24)
                | ((rx[3] as i32) << 16)
                | ((rx[4] as i32) << 8)
                | (rx[5] as i32);

            let voltage = self.raw_to_voltage(raw);

            AdcReading {
                raw_code: raw,
                voltage,
                valid: (status & 0x40) == 0, // Bit 6 = ADC1 not locked
                status_byte: status,
            }
        } else {
            let tx = [reg::CMD_RDATA1, 0, 0, 0, 0];
            let mut rx = [0u8; 5];
            self.cs.set_low();
            let _ = self.spi.transfer(&mut rx, &tx).await;
            self.cs.set_high();

            let raw = ((rx[1] as i32) << 24)
                | ((rx[2] as i32) << 16)
                | ((rx[3] as i32) << 8)
                | (rx[4] as i32);

            let voltage = self.raw_to_voltage(raw);

            AdcReading {
                raw_code: raw,
                voltage,
                valid: true,
                status_byte: 0,
            }
        }
    }

    /// Read a single conversion from ADC2 (24-bit auxiliary)
    ///
    /// ADC2 can run simultaneously with ADC1 for monitoring
    /// a secondary channel (e.g., module temperature while scanning strings).
    pub async fn read_adc2(&mut self) -> AdcReading {
        self.send_command(reg::CMD_START2).await;

        let mut timeout = 0u32;
        while self.drdy.is_high() {
            Timer::after(Duration::from_micros(100)).await;
            timeout += 1;
            if timeout > 5000 {
                self.send_command(reg::CMD_STOP2).await;
                return AdcReading::default();
            }
        }

        let tx = [reg::CMD_RDATA2, 0, 0, 0];
        let mut rx = [0u8; 4];
        self.cs.set_low();
        let _ = self.spi.transfer(&mut rx, &tx).await;
        self.cs.set_high();

        // 24-bit result, sign-extend to 32-bit
        let raw = ((rx[1] as i32) << 16) | ((rx[2] as i32) << 8) | (rx[3] as i32);
        let raw = if raw & 0x80_0000 != 0 {
            raw | (0xFF << 24) as i32
        } else {
            raw
        };

        // ADC2 has its own reference and gain settings
        // For simplicity, use same Vref but 24-bit full scale
        let voltage = (raw as f64 * self.vref as f64 / 8_388_608.0) as f32;

        AdcReading {
            raw_code: raw,
            voltage,
            valid: true,
            status_byte: 0,
        }
    }

    /// Read one multiplexed channel (MUX A or B output on AIN0/AIN1)
    ///
    /// This is the primary method used by the sensor acquisition task.
    /// The MUX channel must already be selected via the CD74HC4067 driver.
    ///
    /// ain_pin: 0 = MUX A output (AIN0, voltage), 1 = MUX B output (AIN1, current)
    pub async fn read_muxed_channel(&mut self, ain_pin: u8) -> AdcReading {
        // Set input to AINx vs AINCOM
        self.set_input_mux(ain_pin, 0x0A).await;

        // Settling delay: MUX switch + anti-aliasing RC filter + PGA settling
        // CD74HC4067: ~70ns, RC filter: ~50us, PGA: depends on gain
        Timer::after(Duration::from_micros(500)).await;

        self.read_adc1().await
    }

    /// Read a direct input channel (not through MUX)
    ///
    /// For bus voltage (AIN2), bus current (AIN3), irradiance (AIN4),
    /// and module temperature (AIN5).
    pub async fn read_direct_channel(&mut self, ain_pin: u8) -> AdcReading {
        if ain_pin > 9 {
            return AdcReading::default();
        }
        self.set_input_mux(ain_pin, 0x0A).await;
        Timer::after(Duration::from_micros(200)).await;
        self.read_adc1().await
    }

    /// Convert raw 32-bit ADC code to voltage
    ///
    /// V = raw * Vref / (2^31 * PGA_gain)
    /// For PGA=1, Vref=2.5V: 1 LSB = 2.5V / 2^31 = ~1.16 nV
    fn raw_to_voltage(&self, raw: i32) -> f32 {
        let full_scale = 2_147_483_648.0_f64; // 2^31
        let voltage = raw as f64 * self.vref as f64 / (full_scale * self.pga_gain as f64);
        voltage as f32
    }

    /// Read the offset calibration registers (3 bytes, 24-bit)
    pub async fn read_offset_cal(&mut self) -> i32 {
        let b0 = self.read_register(reg::OFCAL0).await;
        let b1 = self.read_register(reg::OFCAL1).await;
        let b2 = self.read_register(reg::OFCAL2).await;
        let val = ((b2 as i32) << 16) | ((b1 as i32) << 8) | (b0 as i32);
        if val & 0x80_0000 != 0 { val | (0xFF << 24) as i32 } else { val }
    }

    /// Read the full-scale calibration registers (3 bytes, 24-bit)
    pub async fn read_fullscale_cal(&mut self) -> i32 {
        let b0 = self.read_register(reg::FSCAL0).await;
        let b1 = self.read_register(reg::FSCAL1).await;
        let b2 = self.read_register(reg::FSCAL2).await;
        ((b2 as i32) << 16) | ((b1 as i32) << 8) | (b0 as i32)
    }

    /// Send a single-byte SPI command
    async fn send_command(&mut self, cmd: u8) {
        self.cs.set_low();
        let _ = self.spi.write(&[cmd]).await;
        self.cs.set_high();
    }

    /// Write a single register
    async fn write_register(&mut self, addr: u8, value: u8) {
        // Format: CMD_WREG | addr, count-1, data
        let cmd = [reg::CMD_WREG | (addr & 0x1F), 0x00, value];
        self.cs.set_low();
        let _ = self.spi.write(&cmd).await;
        self.cs.set_high();
    }

    /// Read a single register
    async fn read_register(&mut self, addr: u8) -> u8 {
        // Format: CMD_RREG | addr, count-1, NOP (to clock out data)
        let tx = [reg::CMD_RREG | (addr & 0x1F), 0x00, 0x00];
        let mut rx = [0u8; 3];
        self.cs.set_low();
        let _ = self.spi.transfer(&mut rx, &tx).await;
        self.cs.set_high();
        rx[2]
    }

    /// Read multiple contiguous registers
    pub async fn read_registers(&mut self, start_addr: u8, count: u8, buf: &mut [u8]) {
        if buf.len() < count as usize {
            return;
        }
        // Format: CMD_RREG | addr, count-1, NOPs...
        let mut tx = [0u8; 32];
        tx[0] = reg::CMD_RREG | (start_addr & 0x1F);
        tx[1] = count.saturating_sub(1);
        let total_len = 2 + count as usize;

        let mut rx = [0u8; 32];
        self.cs.set_low();
        let _ = self.spi.transfer(&mut rx[..total_len], &tx[..total_len]).await;
        self.cs.set_high();

        buf[..count as usize].copy_from_slice(&rx[2..2 + count as usize]);
    }
}

/// Channel map constants for the SCADA solar monitoring PCB
///
/// These map physical measurement points to ADS1263 input configurations.
pub mod channel_map {
    /// MUX A voltage channels: selected via CD74HC4067 MUX A, read on AIN0
    pub const MUX_A_AIN: u8 = 0; // AIN0

    /// MUX B current channels: selected via CD74HC4067 MUX B, read on AIN1
    pub const MUX_B_AIN: u8 = 1; // AIN1

    /// Direct bus voltage input (after voltage divider)
    pub const BUS_VOLTAGE_AIN: u8 = 2; // AIN2

    /// Direct bus current input (via CT or hall sensor)
    pub const BUS_CURRENT_AIN: u8 = 3; // AIN3

    /// Irradiance sensor (pyranometer 4-20mA or 0-1V)
    pub const IRRADIANCE_AIN: u8 = 4; // AIN4

    /// Module temperature (backup to TMP117 I2C sensor)
    pub const MOD_TEMP_AIN: u8 = 5; // AIN5

    /// Number of MUX channels per MUX chip
    pub const MUX_CHANNELS: usize = 16;

    /// Number of direct (non-MUX) channels
    pub const DIRECT_CHANNELS: usize = 4;

    /// Total measurable channels
    pub const TOTAL_CHANNELS: usize = MUX_CHANNELS * 2 + DIRECT_CHANNELS;

    /// Voltage divider ratio for string voltage measurement
    /// 400k + 1k divider: ratio = 401:1
    /// At 800V open-circuit: 800V / 401 = 1.995V (within ±2.5V range)
    pub const VOLTAGE_DIVIDER_RATIO: f32 = 401.0;

    /// Current shunt transimpedance: mV per Amp
    /// 1.5 mV/A hall-effect sensor or 1 mOhm shunt
    pub const CURRENT_SHUNT_MV_PER_AMP: f32 = 1.5;
}
