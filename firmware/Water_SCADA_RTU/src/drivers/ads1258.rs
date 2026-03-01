/// ADS1258 — 16-Channel, 24-Bit Delta-Sigma ADC Driver
///
/// Datasheet: Texas Instruments SBAS506
///
/// The ADS1258 is a high-precision ADC with:
///   - 16 single-ended or 8 differential inputs
///   - 24-bit resolution, up to 23,739 SPS per channel
///   - Built-in multiplexer, PGA, and reference
///   - SPI interface (CPOL=0, CPHA=1 → SPI Mode 1)
///
/// On WS-PCB-001, we use 8 single-ended channels for 4-20mA inputs.
/// Each channel has a 250Ω precision shunt resistor:
///   4mA  × 250Ω = 1.000V
///   20mA × 250Ω = 5.000V
///
/// With Vref = 2.5V (internal) and PGA=1: full scale = ±2.5V
/// We use external Vref = 5.0V for full 1V-5V range.

use embassy_stm32::gpio::{Input, Output};
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::{info, warn, error};

/// ADS1258 SPI register addresses (command byte format)
mod reg {
    // Read/Write command prefix
    pub const CMD_READ: u8 = 0x40;   // 0b01_AAAAA
    pub const CMD_WRITE: u8 = 0x60;  // 0b011_AAAAA

    // Configuration registers
    pub const CONFIG0: u8 = 0x00;
    pub const CONFIG1: u8 = 0x01;
    pub const MUXSCH: u8 = 0x02;     // MUX scan channel enable (high byte)
    pub const MUXDIF: u8 = 0x03;     // MUX differential enable
    pub const MUXSG0: u8 = 0x04;     // MUX single-ended enable CH0-CH7
    pub const MUXSG1: u8 = 0x05;     // MUX single-ended enable CH8-CH15
    pub const SYSRED: u8 = 0x06;     // System readings enable
    pub const GPIOC: u8 = 0x07;      // GPIO config
    pub const GPIOD: u8 = 0x08;      // GPIO data

    // Status bits
    pub const STATUS_NEW: u8 = 0x80; // New data available
}

/// Channel identifiers for 8× 4-20mA inputs
#[derive(Clone, Copy, Debug, defmt::Format)]
#[repr(u8)]
pub enum Channel {
    Pressure    = 0, // CH0: 0-10 bar
    Ph          = 1, // CH1: 0-14 pH
    Turbidity   = 2, // CH2: 0-1000 NTU
    Chlorine    = 3, // CH3: 0-5 mg/L
    Flow        = 4, // CH4: flow rate
    TankLevel   = 5, // CH5: 0-100%
    Conductivity = 6, // CH6: 0-2000 µS/cm
    DissolvedO2 = 7, // CH7: 0-20 mg/L
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
}

/// Calibration data per channel (stored in flash sector 11)
#[derive(Clone, Copy)]
pub struct ChannelCalibration {
    /// ADC code at 4.00mA (zero reference)
    pub zero_code: i32,
    /// ADC code at 20.00mA (span reference)
    pub span_code: i32,
    /// Engineering unit minimum (at 4mA)
    pub eng_min: f32,
    /// Engineering unit maximum (at 20mA)
    pub eng_max: f32,
}

impl ChannelCalibration {
    /// Convert raw 24-bit ADC code to engineering units
    pub fn convert(&self, raw: i32) -> f32 {
        if self.span_code == self.zero_code {
            return 0.0;
        }
        let fraction = (raw - self.zero_code) as f32 / (self.span_code - self.zero_code) as f32;
        self.eng_min + fraction * (self.eng_max - self.eng_min)
    }

    /// Get raw current in mA from ADC code
    pub fn current_ma(&self, raw: i32) -> f32 {
        if self.span_code == self.zero_code {
            return 0.0;
        }
        let fraction = (raw - self.zero_code) as f32 / (self.span_code - self.zero_code) as f32;
        4.0 + fraction * 16.0
    }
}

/// Default calibration constants (before field calibration)
/// Assumes Vref=5.0V, 250Ω shunt, 24-bit resolution
/// 4mA  = 1.000V → code ≈ 1,677,722  (1V/5V × 2^23)
/// 20mA = 5.000V → code ≈ 8,388,608  (5V/5V × 2^23, full scale)
pub const DEFAULT_CALIBRATIONS: [ChannelCalibration; Channel::COUNT] = [
    // CH0: Pressure 0-10 bar
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 10.0 },
    // CH1: pH 0-14
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 14.0 },
    // CH2: Turbidity 0-1000 NTU
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 1000.0 },
    // CH3: Chlorine 0-5 mg/L
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 5.0 },
    // CH4: Flow 0-100 m³/h
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 100.0 },
    // CH5: Tank level 0-100%
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 100.0 },
    // CH6: Conductivity 0-2000 µS/cm
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 2000.0 },
    // CH7: Dissolved O₂ 0-20 mg/L
    ChannelCalibration { zero_code: 1_677_722, span_code: 8_388_608, eng_min: 0.0, eng_max: 20.0 },
];

/// Status codes per channel (matches spec table)
#[derive(Clone, Copy, PartialEq, defmt::Format)]
#[repr(u8)]
pub enum ChannelStatus {
    Normal       = 0, // Operating normally
    UnderRange   = 1, // < 3.8mA (broken wire suspect)
    OverRange    = 2, // > 20.5mA (sensor fault)
    OpenCircuit  = 3, // < 1mA (wire disconnected)
    ShortCircuit = 4, // > 24mA (wiring short)
}

impl ChannelStatus {
    /// Determine status from loop current in mA
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
}

/// Complete reading for one channel
#[derive(Clone, Copy)]
pub struct ChannelReading {
    pub raw_code: i32,
    pub current_ma: f32,
    pub eng_value: f32,
    pub status: ChannelStatus,
    pub alarm: u8,
}

/// ADS1258 driver
pub struct Ads1258<'a> {
    spi: Spi<'a, Async>,
    cs: Output<'a>,
    drdy: Input<'a>,
    start: Output<'a>,
    reset: Output<'a>,
    calibrations: [ChannelCalibration; Channel::COUNT],
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
        }
    }

    /// Load calibration from flash (if available)
    pub fn set_calibrations(&mut self, cals: [ChannelCalibration; Channel::COUNT]) {
        self.calibrations = cals;
    }

    /// Hardware reset and initialization
    pub async fn init(&mut self) -> bool {
        info!("ADS1258: Initializing...");

        // Hardware reset pulse (active low, min 4 CLKIN cycles)
        self.reset.set_low();
        Timer::after(Duration::from_millis(10)).await;
        self.reset.set_high();
        Timer::after(Duration::from_millis(50)).await;

        // Configure: auto-scan mode, internal reference, CHOP enabled
        // CONFIG0: SPIX=0, MUXMOD=0 (auto-scan), BYPAS=0, CLKENB=0, CHOP=1, STAT=1
        let config0: u8 = 0x03; // CHOP + STATUS byte enabled
        if !self.write_register(reg::CONFIG0, config0).await {
            error!("ADS1258: Failed to write CONFIG0");
            return false;
        }

        // CONFIG1: IDLMOD=0, DLY=010 (default delay), SCBCS=0, DRATE=00 (default)
        let config1: u8 = 0x20;
        if !self.write_register(reg::CONFIG1, config1).await {
            return false;
        }

        // Enable single-ended channels 0-7
        // MUXSG0: bits 7-0 correspond to channels 7-0
        let muxsg0: u8 = 0xFF; // All 8 channels enabled
        if !self.write_register(reg::MUXSG0, muxsg0).await {
            return false;
        }

        // Disable differential channels and channels 8-15
        if !self.write_register(reg::MUXDIF, 0x00).await {
            return false;
        }
        if !self.write_register(reg::MUXSG1, 0x00).await {
            return false;
        }

        // Start conversions
        self.start.set_high();
        Timer::after(Duration::from_millis(10)).await;

        // Verify by reading back CONFIG0
        let readback = self.read_register(reg::CONFIG0).await;
        if readback != config0 {
            error!("ADS1258: Config verify failed (got 0x{:02X}, expected 0x{:02X})", readback, config0);
            return false;
        }

        info!("ADS1258: Initialized, 8 channels scanning");
        true
    }

    /// Read all 8 channels (one complete scan)
    /// Returns readings in channel order [CH0..CH7]
    pub async fn read_all_channels(&mut self) -> [ChannelReading; Channel::COUNT] {
        let mut readings = [ChannelReading {
            raw_code: 0,
            current_ma: 0.0,
            eng_value: 0.0,
            status: ChannelStatus::OpenCircuit,
            alarm: 0,
        }; Channel::COUNT];

        for i in 0..Channel::COUNT {
            // Wait for DRDY (active low = data ready)
            let mut timeout = 0u32;
            while self.drdy.is_high() {
                Timer::after(Duration::from_micros(100)).await;
                timeout += 1;
                if timeout > 1000 {
                    warn!("ADS1258: DRDY timeout on channel {}", i);
                    readings[i].status = ChannelStatus::OpenCircuit;
                    break;
                }
            }

            if timeout <= 1000 {
                // Read conversion data: STATUS(1) + DATA(3) = 4 bytes
                let (channel_id, raw) = self.read_conversion().await;

                let ch_idx = (channel_id & 0x0F) as usize;
                if ch_idx < Channel::COUNT {
                    let cal = &self.calibrations[ch_idx];
                    let current = cal.current_ma(raw);
                    let eng = cal.convert(raw);
                    let status = ChannelStatus::from_current(current);

                    readings[ch_idx] = ChannelReading {
                        raw_code: raw,
                        current_ma: current,
                        eng_value: eng,
                        status,
                        alarm: if status != ChannelStatus::Normal { status as u8 } else { 0 },
                    };
                }
            }
        }

        readings
    }

    /// Read a single conversion result
    /// Returns (channel_id, raw_24bit_code)
    async fn read_conversion(&mut self) -> (u8, i32) {
        // With STATUS enabled: read 4 bytes (status + 3 data bytes)
        let mut buf = [0u8; 4];
        self.cs.set_low();
        let _ = self.spi.read(&mut buf).await;
        self.cs.set_high();

        let channel_id = buf[0] & 0x1F; // Channel ID from status byte
        let raw = ((buf[1] as i32) << 16) | ((buf[2] as i32) << 8) | (buf[3] as i32);

        // Sign extend from 24-bit to 32-bit
        let raw = if raw & 0x800000 != 0 {
            raw | (0xFF << 24) as i32
        } else {
            raw
        };

        (channel_id, raw)
    }

    async fn write_register(&mut self, addr: u8, value: u8) -> bool {
        let cmd = [reg::CMD_WRITE | (addr & 0x1F), value];
        self.cs.set_low();
        let result = self.spi.write(&cmd).await;
        self.cs.set_high();
        result.is_ok()
    }

    async fn read_register(&mut self, addr: u8) -> u8 {
        let cmd = [reg::CMD_READ | (addr & 0x1F), 0x00];
        let mut buf = [0u8; 2];
        self.cs.set_low();
        let _ = self.spi.transfer(&mut buf, &cmd).await;
        self.cs.set_high();
        buf[1]
    }
}
