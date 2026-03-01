/// ADS1263 — 32-Bit, 38.4 kSPS Delta-Sigma ADC Driver
///
/// Datasheet: Texas Instruments SBAS661
///
/// The ADS1263 is a high-precision ADC with:
///   - 32-bit primary ADC (ADC1) + 24-bit auxiliary ADC (ADC2)
///   - 10 differential / 20 single-ended inputs on ADC1
///   - Programmable data rates: 2.5 SPS to 38,400 SPS
///   - PGA with gains 1-32
///   - SPI interface (CPOL=0, CPHA=1 → SPI Mode 1)
///
/// On SS-PCB-001, the ADS1263 reads through 2× CD74HC4067 16:1 MUX.
/// Combined, this provides 40 analog channels (20 voltage + 20 current).
/// The MUX outputs feed ADS1263 AIN0 (MUX A) and AIN1 (MUX B).

use embassy_stm32::gpio::Output;
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::{info, warn, error};

/// ADS1263 register addresses
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
    pub const CMD_RREG: u8 = 0x20;    // Read register (0x20 + addr)
    pub const CMD_WREG: u8 = 0x40;    // Write register (0x40 + addr)

    // Expected device ID
    pub const DEVICE_ID: u8 = 0x01;   // ADS1263 ID
}

/// Data rate selections for MODE2 register
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum DataRate {
    Sps2_5   = 0x00,
    Sps5     = 0x01,
    Sps10    = 0x02,
    Sps16_6  = 0x03,
    Sps20    = 0x04,
    Sps50    = 0x05,
    Sps60    = 0x06,
    Sps100   = 0x07,
    Sps400   = 0x08,
    Sps1200  = 0x09,
    Sps2400  = 0x0A,
    Sps4800  = 0x0B,
    Sps7200  = 0x0C,
    Sps14400 = 0x0D,
    Sps19200 = 0x0E,
    Sps38400 = 0x0F,
}

/// PGA gain selections
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum PgaGain {
    Gain1  = 0x00,
    Gain2  = 0x01,
    Gain4  = 0x02,
    Gain8  = 0x03,
    Gain16 = 0x04,
    Gain32 = 0x05,
}

/// Calibration for one solar channel
#[derive(Clone, Copy)]
pub struct SolarCalibration {
    pub offset: f32,
    pub gain: f32,
    pub eng_min: f32,
    pub eng_max: f32,
}

impl Default for SolarCalibration {
    fn default() -> Self {
        Self {
            offset: 0.0,
            gain: 1.0,
            eng_min: 0.0,
            eng_max: 1000.0,
        }
    }
}

/// Channel reading from the ADS1263
#[derive(Clone, Copy, Default)]
pub struct SolarReading {
    pub raw_code: i32,
    pub voltage: f32,
    pub eng_value: f32,
    pub valid: bool,
}

/// ADS1263 driver
pub struct Ads1263<'a> {
    spi: Spi<'a, Async>,
    cs: Output<'a>,
    drdy: embassy_stm32::gpio::Input<'a>,
    reset: Output<'a>,
}

impl<'a> Ads1263<'a> {
    pub fn new(
        spi: Spi<'a, Async>,
        cs: Output<'a>,
        drdy: embassy_stm32::gpio::Input<'a>,
        reset: Output<'a>,
    ) -> Self {
        Self { spi, cs, drdy, reset }
    }

    /// Hardware reset and initialization
    pub async fn init(&mut self) -> bool {
        info!("ADS1263: Initializing...");

        // Hardware reset
        self.reset.set_low();
        Timer::after(Duration::from_millis(10)).await;
        self.reset.set_high();
        Timer::after(Duration::from_millis(100)).await;

        // Software reset
        self.send_command(reg::CMD_RESET).await;
        Timer::after(Duration::from_millis(50)).await;

        // Verify device ID
        let id = self.read_register(reg::ID).await;
        if (id >> 5) != reg::DEVICE_ID {
            error!("ADS1263: ID mismatch (got 0x{:02X})", id);
            return false;
        }

        // Configure MODE0: continuous conversion, no delay
        self.write_register(reg::MODE0, 0x00).await;

        // Configure MODE1: sinc4 filter, single-shot
        self.write_register(reg::MODE1, 0x80).await;

        // Configure MODE2: PGA gain=1, data rate=100 SPS
        // Bits [7:4] = gain, [3:0] = data rate
        let mode2 = (PgaGain::Gain1 as u8) << 4 | DataRate::Sps100 as u8;
        self.write_register(reg::MODE2, mode2).await;

        // Configure REFMUX: use internal 2.5V reference
        self.write_register(reg::REFMUX, 0x00).await;

        // Configure POWER: enable internal reference
        self.write_register(reg::POWER, 0x01).await;

        // Set input mux to AIN0+ / AINCOM- (default channel)
        self.write_register(reg::INPMUX, 0x0A).await; // AIN0 vs AINCOM

        info!("ADS1263: Initialized, ID=0x{:02X}", id);
        true
    }

    /// Set input multiplexer to specific positive/negative inputs
    /// positive: AIN0-AIN9, negative: AIN0-AIN9 or AINCOM (0x0A)
    pub async fn set_input_mux(&mut self, positive: u8, negative: u8) {
        let mux_val = (positive << 4) | (negative & 0x0F);
        self.write_register(reg::INPMUX, mux_val).await;
    }

    /// Read a single conversion from ADC1 (32-bit)
    pub async fn read_adc1(&mut self) -> i32 {
        // Start conversion
        self.send_command(reg::CMD_START1).await;

        // Wait for DRDY
        let mut timeout = 0u32;
        while self.drdy.is_high() {
            Timer::after(Duration::from_micros(100)).await;
            timeout += 1;
            if timeout > 5000 {
                warn!("ADS1263: DRDY timeout");
                self.send_command(reg::CMD_STOP1).await;
                return 0;
            }
        }

        // Read data: send RDATA1, receive status(1) + data(4) + CRC(1)
        let mut tx = [reg::CMD_RDATA1, 0, 0, 0, 0, 0];
        let mut rx = [0u8; 6];
        self.cs.set_low();
        let _ = self.spi.transfer(&mut rx, &tx).await;
        self.cs.set_high();

        // Parse 32-bit result (bytes 2-5, big-endian signed)
        let raw = ((rx[2] as i32) << 24)
            | ((rx[3] as i32) << 16)
            | ((rx[4] as i32) << 8)
            | (rx[5] as i32);

        raw
    }

    /// Read a single conversion from ADC2 (24-bit auxiliary)
    pub async fn read_adc2(&mut self) -> i32 {
        self.send_command(reg::CMD_START2).await;

        let mut timeout = 0u32;
        while self.drdy.is_high() {
            Timer::after(Duration::from_micros(100)).await;
            timeout += 1;
            if timeout > 5000 {
                self.send_command(reg::CMD_STOP2).await;
                return 0;
            }
        }

        let mut tx = [reg::CMD_RDATA2, 0, 0, 0];
        let mut rx = [0u8; 4];
        self.cs.set_low();
        let _ = self.spi.transfer(&mut rx, &tx).await;
        self.cs.set_high();

        // 24-bit result
        let raw = ((rx[1] as i32) << 16) | ((rx[2] as i32) << 8) | (rx[3] as i32);
        // Sign extend
        if raw & 0x800000 != 0 {
            raw | (0xFF << 24) as i32
        } else {
            raw
        }
    }

    /// Read one multiplexed channel (MUX A or B output on AIN0/AIN1)
    /// ain_pin: 0 = MUX A output (AIN0), 1 = MUX B output (AIN1)
    pub async fn read_muxed_channel(&mut self, ain_pin: u8) -> SolarReading {
        // Set input to AINx vs AINCOM
        self.set_input_mux(ain_pin, 0x0A).await;

        // Small settling delay for MUX + ADC
        Timer::after(Duration::from_micros(500)).await;

        let raw = self.read_adc1().await;

        // Convert to voltage: Vref=2.5V, PGA=1, 32-bit
        // V = raw × Vref / (2^31 × PGA)
        let voltage = (raw as f64 * 2.5 / 2147483648.0) as f32;

        SolarReading {
            raw_code: raw,
            voltage,
            eng_value: voltage, // Caller applies calibration
            valid: true,
        }
    }

    async fn send_command(&mut self, cmd: u8) {
        self.cs.set_low();
        let _ = self.spi.write(&[cmd]).await;
        self.cs.set_high();
    }

    async fn write_register(&mut self, addr: u8, value: u8) {
        let cmd = [reg::CMD_WREG | addr, 0x00, value]; // addr, count-1=0, data
        self.cs.set_low();
        let _ = self.spi.write(&cmd).await;
        self.cs.set_high();
    }

    async fn read_register(&mut self, addr: u8) -> u8 {
        let tx = [reg::CMD_RREG | addr, 0x00, 0x00];
        let mut rx = [0u8; 3];
        self.cs.set_low();
        let _ = self.spi.transfer(&mut rx, &tx).await;
        self.cs.set_high();
        rx[2]
    }
}
