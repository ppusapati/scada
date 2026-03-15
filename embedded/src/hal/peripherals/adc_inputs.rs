//! 4-Channel Analog Input Driver
//!
//! Reads STM32H743 ADC1 channels PA0-PA3 (16-bit resolution).
//! Supports 4-20mA (249R shunt) and 0-10V (20k/10k divider) modes.
//!
//! Hardware:
//!   - ADC1_IN0 (PA0) → AI Channel 0
//!   - ADC1_IN1 (PA1) → AI Channel 1
//!   - ADC1_IN2 (PA2) → AI Channel 2
//!   - ADC1_IN3 (PA3) → AI Channel 3
//!   - VREF powered from TPS7A4533 ultra-low-noise 3.3VA rail

use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::peripherals::{ADC1, PA0, PA1, PA2, PA3};
use embassy_time::Timer;

use crate::hal::{AnalogChannel, AnalogMode};

/// Shunt resistor value for 4-20mA mode (ohms)
const SHUNT_RESISTANCE: f32 = 249.0;

/// Voltage divider ratio for 0-10V mode: Vout = Vin * R2/(R1+R2) = Vin * 10k/30k
const VOLTAGE_DIVIDER_RATIO: f32 = 3.0;

/// ADC reference voltage (3.3VA rail)
const VREF: f32 = 3.3;

/// ADC full-scale value (16-bit)
const ADC_MAX: f32 = 65535.0;

/// Number of samples for oversampling/averaging
const OVERSAMPLE_COUNT: u32 = 16;

/// Analog input reading result
#[derive(Clone, Debug)]
pub struct AnalogReading {
    pub channel: AnalogChannel,
    pub raw_adc: u16,
    pub voltage: f32,
    pub engineering_value: f32,
    pub mode: AnalogMode,
    pub quality: DataQuality,
}

/// Data quality indicator (IEC 62541 / OPC UA style)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataQuality {
    Good,
    Uncertain,
    Bad,
}

/// Channel configuration
pub struct ChannelConfig {
    pub mode: AnalogMode,
    /// Engineering unit low range (e.g., 0.0 for 0-100%)
    pub range_low: f32,
    /// Engineering unit high range (e.g., 100.0 for 0-100%)
    pub range_high: f32,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            mode: AnalogMode::Current4To20mA,
            range_low: 0.0,
            range_high: 100.0,
        }
    }
}

/// 4-channel analog input manager
pub struct AnalogInputs {
    adc: Adc<'static, ADC1>,
    pin_ch0: PA0,
    pin_ch1: PA1,
    pin_ch2: PA2,
    pin_ch3: PA3,
    config: [ChannelConfig; 4],
}

impl AnalogInputs {
    pub fn new(
        adc: Adc<'static, ADC1>,
        ch0: PA0,
        ch1: PA1,
        ch2: PA2,
        ch3: PA3,
    ) -> Self {
        Self {
            adc,
            pin_ch0: ch0,
            pin_ch1: ch1,
            pin_ch2: ch2,
            pin_ch3: ch3,
            config: [
                ChannelConfig::default(),
                ChannelConfig::default(),
                ChannelConfig::default(),
                ChannelConfig::default(),
            ],
        }
    }

    /// Configure a specific channel's mode and engineering range
    pub fn configure_channel(&mut self, ch: AnalogChannel, config: ChannelConfig) {
        self.config[ch as usize] = config;
    }

    /// Read a single channel with oversampling
    pub async fn read_channel(&mut self, ch: AnalogChannel) -> AnalogReading {
        let raw = self.read_raw_oversampled(ch).await;
        let voltage = (raw as f32 / ADC_MAX) * VREF;
        let cfg = &self.config[ch as usize];

        let (engineering_value, quality) = match cfg.mode {
            AnalogMode::Current4To20mA => {
                let current_ma = (voltage / SHUNT_RESISTANCE) * 1000.0;
                let quality = if current_ma < 3.8 {
                    DataQuality::Bad // Wire break detection (<3.8mA)
                } else if current_ma < 4.0 || current_ma > 20.5 {
                    DataQuality::Uncertain
                } else {
                    DataQuality::Good
                };
                // Scale 4-20mA to engineering range
                let normalized = (current_ma - 4.0) / 16.0;
                let value = cfg.range_low + normalized * (cfg.range_high - cfg.range_low);
                (value, quality)
            }
            AnalogMode::Voltage0To10V => {
                let actual_voltage = voltage * VOLTAGE_DIVIDER_RATIO;
                let quality = if actual_voltage < -0.1 || actual_voltage > 10.5 {
                    DataQuality::Bad
                } else if actual_voltage < 0.0 || actual_voltage > 10.1 {
                    DataQuality::Uncertain
                } else {
                    DataQuality::Good
                };
                // Scale 0-10V to engineering range
                let normalized = actual_voltage / 10.0;
                let value = cfg.range_low + normalized * (cfg.range_high - cfg.range_low);
                (value, quality)
            }
        };

        AnalogReading {
            channel: ch,
            raw_adc: raw,
            voltage,
            engineering_value,
            mode: cfg.mode,
            quality,
        }
    }

    /// Read all 4 channels
    pub async fn read_all(&mut self) -> [AnalogReading; 4] {
        let ch0 = self.read_channel(AnalogChannel::Ch0).await;
        let ch1 = self.read_channel(AnalogChannel::Ch1).await;
        let ch2 = self.read_channel(AnalogChannel::Ch2).await;
        let ch3 = self.read_channel(AnalogChannel::Ch3).await;
        [ch0, ch1, ch2, ch3]
    }

    /// Read raw ADC with software oversampling for noise reduction
    async fn read_raw_oversampled(&mut self, ch: AnalogChannel) -> u16 {
        let mut sum: u32 = 0;
        for _ in 0..OVERSAMPLE_COUNT {
            let sample = match ch {
                AnalogChannel::Ch0 => self.adc.read(&mut self.pin_ch0) as u32,
                AnalogChannel::Ch1 => self.adc.read(&mut self.pin_ch1) as u32,
                AnalogChannel::Ch2 => self.adc.read(&mut self.pin_ch2) as u32,
                AnalogChannel::Ch3 => self.adc.read(&mut self.pin_ch3) as u32,
            };
            sum += sample;
        }
        (sum / OVERSAMPLE_COUNT) as u16
    }
}
