/// Sensor Acquisition Drivers for STM32MP157 SCADA
///
/// Provides ADC drivers for both Solar SMU and Water RTU variants:
///   - ADS1263: 32-bit delta-sigma ADC (Solar SMU, string voltage/current)
///   - ADS1258: 16-channel 24-bit ADC (Water RTU, 4-20mA inputs)
///
/// Reuses the proven driver code from the existing STM32F407 firmware
/// with minor adaptations for the STM32MP157 M4 SPI peripheral.
///
/// Both drivers use Embassy async SPI with DMA for non-blocking transfers.

pub mod calibration;

#[cfg(feature = "solar_smu")]
pub mod ads1263;

#[cfg(feature = "solar_smu")]
pub mod mux;

#[cfg(feature = "water_rtu")]
pub mod ads1258;

/// Common sensor reading structure
#[derive(Clone, Copy, Default)]
pub struct SensorReading {
    pub raw_code: i32,
    pub voltage: f32,
    pub eng_value: f32,
    pub valid: bool,
    pub status: u8,
}

/// IIR low-pass filter for sensor data smoothing
///
/// Single-pole IIR filter: y[n] = alpha * x[n] + (1 - alpha) * y[n-1]
/// Alpha = dt / (RC + dt) where RC = 1 / (2*pi*fc)
///
/// For a 1 kHz sample rate and 10 Hz cutoff:
///   RC = 1 / (2*pi*10) = 0.01592
///   alpha = 0.001 / (0.01592 + 0.001) = 0.0591
#[derive(Clone, Copy)]
pub struct IirFilter {
    /// Filter coefficient (0.0 - 1.0, higher = less filtering)
    alpha: f32,
    /// Previous output value
    state: f32,
    /// Filter has been initialized with first sample
    initialized: bool,
}

impl IirFilter {
    /// Create a new IIR filter with given alpha coefficient
    pub const fn new(alpha: f32) -> Self {
        Self {
            alpha,
            state: 0.0,
            initialized: false,
        }
    }

    /// Create a filter from sample rate and cutoff frequency
    /// fs = sample frequency in Hz, fc = cutoff frequency in Hz
    pub fn from_cutoff(fs: f32, fc: f32) -> Self {
        let rc = 1.0 / (2.0 * core::f32::consts::PI * fc);
        let dt = 1.0 / fs;
        let alpha = dt / (rc + dt);
        Self::new(alpha)
    }

    /// Apply the filter to a new sample
    pub fn update(&mut self, input: f32) -> f32 {
        if !self.initialized {
            self.state = input;
            self.initialized = true;
            return input;
        }

        self.state = self.alpha * input + (1.0 - self.alpha) * self.state;
        self.state
    }

    /// Reset the filter state
    pub fn reset(&mut self) {
        self.state = 0.0;
        self.initialized = false;
    }

    /// Get the current filtered value without updating
    pub fn value(&self) -> f32 {
        self.state
    }
}

/// Oversampling averager — collects N samples and computes mean
#[derive(Clone, Copy)]
pub struct Oversampler {
    buf: [f32; 16],
    count: u8,
    max_samples: u8,
}

impl Oversampler {
    pub const fn new(max_samples: u8) -> Self {
        Self {
            buf: [0.0; 16],
            count: 0,
            max_samples: if max_samples > 16 { 16 } else { max_samples },
        }
    }

    /// Add a sample. Returns Some(average) when all samples collected.
    pub fn add(&mut self, sample: f32) -> Option<f32> {
        if self.count < self.max_samples {
            self.buf[self.count as usize] = sample;
            self.count += 1;
        }

        if self.count >= self.max_samples {
            let sum: f32 = self.buf[..self.max_samples as usize].iter().sum();
            let avg = sum / self.max_samples as f32;
            self.count = 0;
            Some(avg)
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.count = 0;
    }
}
