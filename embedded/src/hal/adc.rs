/// ADC driver for STM32F407 — reads analog sensors
///
/// The STM32F407 has 3x 12-bit ADCs capable of 2.4 MSPS.
/// We use ADC1 with DMA for continuous multi-channel scanning.
///
/// Sensor signal conditioning:
///   4-20mA sensors → 250Ω resistor → 1.0V-5.0V → voltage divider → 0-3.3V ADC range
///   0-10V sensors  → voltage divider (3:1) → 0-3.3V ADC range

use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::peripherals;
use defmt::info;

/// Raw ADC reading to engineering unit conversion factors
pub struct AdcCalibration {
    pub offset: f32,    // Zero offset in ADC counts
    pub scale: f32,     // Counts-to-engineering-unit scale factor
    pub min_valid: f32, // Minimum valid engineering value (fault detection)
    pub max_valid: f32, // Maximum valid engineering value
}

impl AdcCalibration {
    pub fn convert(&self, raw: u16) -> f32 {
        (raw as f32 - self.offset) * self.scale
    }

    pub fn is_valid(&self, value: f32) -> bool {
        value >= self.min_valid && value <= self.max_valid
    }
}

/// Calibration constants for water sensors (4-20mA into 250Ω = 1V-5V range)
/// ADC 12-bit: 0-4095 maps to 0-3.3V
/// With voltage divider 5V→3.3V: 4mA=~800 counts, 20mA=~4000 counts
pub static PRESSURE_CAL: AdcCalibration = AdcCalibration {
    offset: 800.0,    // 4mA zero
    scale: 0.00313,   // (10.0 bar) / (4000-800) counts
    min_valid: -0.5,
    max_valid: 12.0,  // bar
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
    max_valid: 15.0,  // NTU
};

pub static CHLORINE_CAL: AdcCalibration = AdcCalibration {
    offset: 800.0,
    scale: 0.00156,   // (5.0 mg/L) / 3200
    min_valid: 0.0,
    max_valid: 6.0,   // mg/L
};

pub static TANK_LEVEL_CAL: AdcCalibration = AdcCalibration {
    offset: 800.0,
    scale: 0.03125,   // (100%) / 3200
    min_valid: -1.0,
    max_valid: 105.0,  // %
};

pub static IRRADIANCE_CAL: AdcCalibration = AdcCalibration {
    offset: 0.0,
    scale: 0.366,     // (1500 W/m²) / 4095
    min_valid: 0.0,
    max_valid: 1600.0, // W/m²
};

pub static PANEL_VOLTAGE_CAL: AdcCalibration = AdcCalibration {
    offset: 0.0,
    scale: 0.122,     // (500V) / 4095 (with voltage divider)
    min_valid: 0.0,
    max_valid: 600.0,  // V
};

/// Read a single ADC channel with oversampling for noise reduction
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
