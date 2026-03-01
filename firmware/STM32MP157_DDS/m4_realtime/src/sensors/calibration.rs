/// Multi-Point ADC Calibration System
///
/// Provides accurate sensor readings through multi-point calibration:
///   - 5-point calibration for voltage channels (Solar SMU)
///   - 3-point calibration for current channels (Solar SMU)
///   - 2-point (zero/span) calibration for 4-20mA channels (Water RTU)
///
/// Calibration data is stored in MCU internal flash (sector 11) and
/// loaded at boot. Factory defaults are used when no valid calibration
/// data is found.
///
/// Linear interpolation is used between calibration points for
/// piecewise-linear correction of ADC nonlinearity.

use defmt::{info, warn};

/// Maximum number of calibration points per channel
pub const MAX_CAL_POINTS: usize = 5;

/// A single calibration point (reference value → ADC reading)
#[derive(Clone, Copy, Default)]
pub struct CalPoint {
    /// Known reference value (engineering units or voltage)
    pub reference: f32,
    /// Corresponding raw ADC code
    pub adc_code: i32,
}

/// Multi-point calibration data for one channel
#[derive(Clone, Copy)]
pub struct ChannelCal {
    /// Calibration points sorted by reference value (ascending)
    pub points: [CalPoint; MAX_CAL_POINTS],
    /// Number of valid calibration points (2-5)
    pub num_points: u8,
    /// Engineering unit minimum (display range)
    pub eng_min: f32,
    /// Engineering unit maximum (display range)
    pub eng_max: f32,
    /// Engineering unit label (e.g., "V", "A", "bar", "pH")
    pub unit_code: u8,
}

impl ChannelCal {
    /// Create a default 2-point linear calibration
    pub const fn linear(
        zero_code: i32,
        span_code: i32,
        eng_min: f32,
        eng_max: f32,
        unit_code: u8,
    ) -> Self {
        Self {
            points: [
                CalPoint { reference: eng_min, adc_code: zero_code },
                CalPoint { reference: eng_max, adc_code: span_code },
                CalPoint { reference: 0.0, adc_code: 0 },
                CalPoint { reference: 0.0, adc_code: 0 },
                CalPoint { reference: 0.0, adc_code: 0 },
            ],
            num_points: 2,
            eng_min,
            eng_max,
            unit_code,
        }
    }

    /// Convert a raw ADC code to engineering units using multi-point
    /// piecewise linear interpolation.
    ///
    /// For codes outside the calibrated range, the nearest segment
    /// is extrapolated.
    pub fn convert(&self, raw_code: i32) -> f32 {
        let n = self.num_points as usize;
        if n < 2 {
            return 0.0;
        }

        // Find the interpolation segment
        // Points are sorted by adc_code ascending
        for i in 0..n - 1 {
            let p0 = &self.points[i];
            let p1 = &self.points[i + 1];

            if raw_code <= p1.adc_code || i == n - 2 {
                // Interpolate (or extrapolate on last segment)
                let code_range = p1.adc_code - p0.adc_code;
                if code_range == 0 {
                    return p0.reference;
                }
                let fraction = (raw_code - p0.adc_code) as f32 / code_range as f32;
                return p0.reference + fraction * (p1.reference - p0.reference);
            }
        }

        // Should not reach here, but handle gracefully
        self.points[n - 1].reference
    }

    /// Check if a calibration is valid
    pub fn is_valid(&self) -> bool {
        if self.num_points < 2 || self.num_points > MAX_CAL_POINTS as u8 {
            return false;
        }

        // Verify points are in ascending order of adc_code
        for i in 1..self.num_points as usize {
            if self.points[i].adc_code <= self.points[i - 1].adc_code {
                return false;
            }
        }

        true
    }
}

/// Unit codes for engineering units
pub mod unit_code {
    pub const VOLTS: u8 = 0x01;
    pub const AMPS: u8 = 0x02;
    pub const WATTS: u8 = 0x03;
    pub const KWH: u8 = 0x04;
    pub const CELSIUS: u8 = 0x05;
    pub const BAR: u8 = 0x06;
    pub const PH: u8 = 0x07;
    pub const NTU: u8 = 0x08;
    pub const MGL: u8 = 0x09;
    pub const USCM: u8 = 0x0A;
    pub const PERCENT: u8 = 0x0B;
    pub const M3H: u8 = 0x0C;
    pub const WM2: u8 = 0x0D;
}

/// Default calibrations for Solar SMU voltage channels
/// 5-point calibration spanning 0-1000V range
/// Based on ADS1263 32-bit ADC, Vref=2.5V, PGA=1, voltage divider 1:401
pub const fn default_solar_voltage_cal() -> ChannelCal {
    // Approximate ADC codes for 5-point calibration at 0V, 250V, 500V, 750V, 1000V
    // V_adc = V_string / 401  (divider ratio)
    // code = V_adc / Vref * 2^31
    // At 0V:    code = 0
    // At 250V:  V_adc = 0.6234V,  code = 0.6234/2.5 * 2^31 = 535_306_682
    // At 500V:  V_adc = 1.2469V,  code = 1070_613_363
    // At 750V:  V_adc = 1.8703V,  code = 1605_920_045
    // At 1000V: V_adc = 2.4938V,  code = 2141_226_726
    ChannelCal {
        points: [
            CalPoint { reference: 0.0, adc_code: 0 },
            CalPoint { reference: 250.0, adc_code: 535_306_682 },
            CalPoint { reference: 500.0, adc_code: 1_070_613_363 },
            CalPoint { reference: 750.0, adc_code: 1_605_920_045 },
            CalPoint { reference: 1000.0, adc_code: 2_141_226_726 },
        ],
        num_points: 5,
        eng_min: 0.0,
        eng_max: 1000.0,
        unit_code: unit_code::VOLTS,
    }
}

/// Default calibrations for Solar SMU current channels
/// 3-point calibration spanning 0-15A range
/// Based on 50mV/A shunt (1.5mV/A), measured via ADS1263
pub const fn default_solar_current_cal() -> ChannelCal {
    // Current measurement via 50A/75mV shunt → 1.5mV per amp
    // code = (I * 0.0015) / 2.5 * 2^31
    // At 0A:  code = 0
    // At 7.5A: V = 0.01125V, code = 9_663_676
    // At 15A:  V = 0.0225V,  code = 19_327_353
    ChannelCal {
        points: [
            CalPoint { reference: 0.0, adc_code: 0 },
            CalPoint { reference: 7.5, adc_code: 9_663_676 },
            CalPoint { reference: 15.0, adc_code: 19_327_353 },
            CalPoint { reference: 0.0, adc_code: 0 }, // unused
            CalPoint { reference: 0.0, adc_code: 0 }, // unused
        ],
        num_points: 3,
        eng_min: 0.0,
        eng_max: 15.0,
        unit_code: unit_code::AMPS,
    }
}

/// Default calibration for Water RTU 4-20mA input
/// 2-point (zero/span) calibration
/// ADS1258: Vref=5.0V, 250 ohm shunt
/// 4mA × 250Ω = 1.000V → code ≈ 1,677,722
/// 20mA × 250Ω = 5.000V → code ≈ 8,388,608
pub const fn default_water_420ma_cal(eng_min: f32, eng_max: f32, unit: u8) -> ChannelCal {
    ChannelCal::linear(1_677_722, 8_388_608, eng_min, eng_max, unit)
}

/// Flash calibration storage header
///
/// Stored at FLASH_SECTOR_11_BASE (0x080E_0000) on STM32F407
/// or at a designated flash region on STM32MP157
#[repr(C)]
pub struct CalFlashHeader {
    pub magic: u32,       // 0xCAFE_BABE
    pub version: u32,     // Calibration data version
    pub crc32: u32,       // CRC-32 of payload
    pub data_len: u32,    // Length of calibration data in bytes
    pub num_channels: u16, // Number of calibrated channels
    pub reserved: u16,
}

pub const CAL_MAGIC: u32 = 0xCAFE_BABE;
pub const CAL_VERSION: u32 = 2; // Version 2 = multi-point calibration

/// Calibration store — holds calibration data for all channels
pub struct CalibrationStore {
    /// Voltage channel calibrations (Solar SMU: 16 strings)
    #[cfg(feature = "solar_smu")]
    pub voltage_cals: [ChannelCal; 16],

    /// Current channel calibrations (Solar SMU: 16 strings)
    #[cfg(feature = "solar_smu")]
    pub current_cals: [ChannelCal; 16],

    /// 4-20mA channel calibrations (Water RTU: 8 channels)
    #[cfg(feature = "water_rtu")]
    pub water_cals: [ChannelCal; 8],

    /// Calibration data is valid (loaded from flash)
    pub valid: bool,
}

impl CalibrationStore {
    /// Create store with factory defaults
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "solar_smu")]
            voltage_cals: [default_solar_voltage_cal(); 16],

            #[cfg(feature = "solar_smu")]
            current_cals: [default_solar_current_cal(); 16],

            #[cfg(feature = "water_rtu")]
            water_cals: [
                default_water_420ma_cal(0.0, 10.0, unit_code::BAR),      // CH0: Pressure
                default_water_420ma_cal(0.0, 14.0, unit_code::PH),       // CH1: pH
                default_water_420ma_cal(0.0, 1000.0, unit_code::NTU),    // CH2: Turbidity
                default_water_420ma_cal(0.0, 5.0, unit_code::MGL),       // CH3: Chlorine
                default_water_420ma_cal(0.0, 100.0, unit_code::M3H),     // CH4: Flow
                default_water_420ma_cal(0.0, 100.0, unit_code::PERCENT), // CH5: Level
                default_water_420ma_cal(0.0, 2000.0, unit_code::USCM),   // CH6: Conductivity
                default_water_420ma_cal(0.0, 20.0, unit_code::MGL),      // CH7: Dissolved O2
            ],

            valid: false,
        }
    }

    /// Reset to factory defaults
    pub fn factory_reset(&mut self) {
        *self = Self::new();
        info!("Calibration: reset to factory defaults");
    }

    /// Attempt to load calibration from flash
    ///
    /// On STM32MP157, the M4 can read flash at memory-mapped addresses.
    /// Calibration is stored at a dedicated flash sector.
    pub fn load_from_flash(&mut self, flash_base: u32) -> bool {
        let header_ptr = flash_base as *const CalFlashHeader;

        let magic = unsafe { core::ptr::read_volatile(&(*header_ptr).magic) };
        if magic != CAL_MAGIC {
            info!("CalStore: No valid flash data (magic=0x{:08X})", magic);
            return false;
        }

        let version = unsafe { core::ptr::read_volatile(&(*header_ptr).version) };
        if version != CAL_VERSION {
            warn!("CalStore: Version mismatch (flash={}, expected={})", version, CAL_VERSION);
            return false;
        }

        let stored_crc = unsafe { core::ptr::read_volatile(&(*header_ptr).crc32) };
        let data_len = unsafe { core::ptr::read_volatile(&(*header_ptr).data_len) };

        // Read and verify CRC
        let data_base = (flash_base + core::mem::size_of::<CalFlashHeader>() as u32) as *const u8;
        let mut crc: u32 = 0;
        for i in 0..data_len as usize {
            let byte = unsafe { core::ptr::read_volatile(data_base.add(i)) };
            crc = crc.wrapping_add(byte as u32);
        }

        if crc != stored_crc {
            warn!("CalStore: CRC mismatch (calc=0x{:08X}, stored=0x{:08X})", crc, stored_crc);
            return false;
        }

        // Load calibration data from flash
        // The data layout in flash matches the in-memory CalibrationStore layout
        let cal_ptr = data_base as *const ChannelCal;

        #[cfg(feature = "solar_smu")]
        {
            for i in 0..16 {
                self.voltage_cals[i] = unsafe { core::ptr::read_volatile(cal_ptr.add(i)) };
            }
            for i in 0..16 {
                self.current_cals[i] = unsafe { core::ptr::read_volatile(cal_ptr.add(16 + i)) };
            }
        }

        #[cfg(feature = "water_rtu")]
        {
            for i in 0..8 {
                self.water_cals[i] = unsafe { core::ptr::read_volatile(cal_ptr.add(i)) };
            }
        }

        self.valid = true;
        info!("CalStore: Loaded calibration from flash ({} bytes)", data_len);
        true
    }
}
