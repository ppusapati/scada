//! Data Logger Engine
//!
//! Core data acquisition and logging engine that coordinates:
//! - Periodic sampling of analog and digital inputs
//! - Data storage to flash ring buffer and SD card
//! - Alarm evaluation and notification
//! - Telemetry transmission via communication manager

use heapless::{String, Vec};

/// Data logger operating state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoggerState {
    /// Not started
    Idle,
    /// Running, actively logging
    Running,
    /// Paused (by user command)
    Paused,
    /// Error condition, logging suspended
    Error,
}

/// Sample interval presets
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SampleRate {
    /// 100ms (10 Hz) - fast acquisition
    Hz10,
    /// 1 second - standard SCADA
    Hz1,
    /// 5 seconds
    Sec5,
    /// 10 seconds
    Sec10,
    /// 30 seconds
    Sec30,
    /// 1 minute
    Min1,
    /// 5 minutes
    Min5,
}

impl SampleRate {
    pub fn interval_ms(&self) -> u32 {
        match self {
            Self::Hz10 => 100,
            Self::Hz1 => 1_000,
            Self::Sec5 => 5_000,
            Self::Sec10 => 10_000,
            Self::Sec30 => 30_000,
            Self::Min1 => 60_000,
            Self::Min5 => 300_000,
        }
    }
}

/// Data logger configuration
#[derive(Clone)]
pub struct LoggerConfig {
    pub sample_rate: SampleRate,
    /// Telemetry upload interval (multiple of sample_rate)
    pub telemetry_interval_ms: u32,
    /// Log to flash ring buffer
    pub log_to_flash: bool,
    /// Log to SD card (CSV)
    pub log_to_sd: bool,
    /// Device name for identification
    pub device_name: String<16>,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            sample_rate: SampleRate::Hz1,
            telemetry_interval_ms: 10_000,
            log_to_flash: true,
            log_to_sd: true,
            device_name: String::new(),
        }
    }
}

/// A single timestamped data record
#[derive(Clone, Debug)]
pub struct DataRecord {
    /// Milliseconds since boot (or RTC timestamp)
    pub timestamp_ms: u64,
    /// Sequence number
    pub sequence: u32,
    /// 4 analog channels (engineering units)
    pub analog: [f32; 4],
    /// Digital input bitmask (8 channels)
    pub digital_in: u8,
    /// Digital output bitmask (4 channels)
    pub digital_out: u8,
    /// MCU internal temperature (°C)
    pub mcu_temp: f32,
    /// Supply voltage (V)
    pub supply_voltage: f32,
}

impl DataRecord {
    /// Serialize to binary (40 bytes, compact for flash storage)
    pub fn to_binary(&self) -> [u8; 40] {
        let mut buf = [0u8; 40];
        buf[0..8].copy_from_slice(&self.timestamp_ms.to_le_bytes());
        buf[8..12].copy_from_slice(&self.sequence.to_le_bytes());
        for i in 0..4 {
            let offset = 12 + i * 4;
            buf[offset..offset + 4].copy_from_slice(&self.analog[i].to_le_bytes());
        }
        buf[28] = self.digital_in;
        buf[29] = self.digital_out;
        buf[30..34].copy_from_slice(&self.mcu_temp.to_le_bytes());
        buf[34..38].copy_from_slice(&self.supply_voltage.to_le_bytes());
        // CRC-16 over first 38 bytes
        let crc = crc16(&buf[..38]);
        buf[38..40].copy_from_slice(&crc.to_le_bytes());
        buf
    }

    /// Deserialize from binary, verifying CRC
    pub fn from_binary(data: &[u8; 40]) -> Option<Self> {
        let stored_crc = u16::from_le_bytes([data[38], data[39]]);
        if crc16(&data[..38]) != stored_crc {
            return None;
        }
        Some(Self {
            timestamp_ms: u64::from_le_bytes(data[0..8].try_into().ok()?),
            sequence: u32::from_le_bytes(data[8..12].try_into().ok()?),
            analog: [
                f32::from_le_bytes(data[12..16].try_into().ok()?),
                f32::from_le_bytes(data[16..20].try_into().ok()?),
                f32::from_le_bytes(data[20..24].try_into().ok()?),
                f32::from_le_bytes(data[24..28].try_into().ok()?),
            ],
            digital_in: data[28],
            digital_out: data[29],
            mcu_temp: f32::from_le_bytes(data[30..34].try_into().ok()?),
            supply_voltage: f32::from_le_bytes(data[34..38].try_into().ok()?),
        })
    }

    /// Format as CSV line
    pub fn to_csv_line(&self) -> String<128> {
        let mut line = String::new();
        let _ = core::fmt::write(
            &mut line,
            format_args!(
                "{},{:.3},{:.3},{:.3},{:.3},{:#04X},{:#04X},{:.1},{:.2}\r\n",
                self.timestamp_ms,
                self.analog[0],
                self.analog[1],
                self.analog[2],
                self.analog[3],
                self.digital_in,
                self.digital_out,
                self.mcu_temp,
                self.supply_voltage,
            ),
        );
        line
    }
}

/// Alarm threshold configuration per channel
#[derive(Clone, Copy, Debug)]
pub struct AlarmThreshold {
    pub high_high: f32,
    pub high: f32,
    pub low: f32,
    pub low_low: f32,
    pub deadband: f32,
    pub enabled: bool,
}

impl Default for AlarmThreshold {
    fn default() -> Self {
        Self {
            high_high: f32::MAX,
            high: f32::MAX,
            low: f32::MIN,
            low_low: f32::MIN,
            deadband: 0.0,
            enabled: false,
        }
    }
}

/// Alarm state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlarmLevel {
    Normal,
    Low,
    LowLow,
    High,
    HighHigh,
}

impl AlarmThreshold {
    /// Evaluate a value against this threshold
    pub fn evaluate(&self, value: f32, current_state: AlarmLevel) -> AlarmLevel {
        if !self.enabled {
            return AlarmLevel::Normal;
        }
        // Apply deadband to prevent chattering
        match current_state {
            AlarmLevel::Normal => {
                if value >= self.high_high {
                    AlarmLevel::HighHigh
                } else if value >= self.high {
                    AlarmLevel::High
                } else if value <= self.low_low {
                    AlarmLevel::LowLow
                } else if value <= self.low {
                    AlarmLevel::Low
                } else {
                    AlarmLevel::Normal
                }
            }
            AlarmLevel::High => {
                if value >= self.high_high {
                    AlarmLevel::HighHigh
                } else if value < self.high - self.deadband {
                    AlarmLevel::Normal
                } else {
                    AlarmLevel::High
                }
            }
            AlarmLevel::HighHigh => {
                if value < self.high_high - self.deadband {
                    if value >= self.high {
                        AlarmLevel::High
                    } else {
                        AlarmLevel::Normal
                    }
                } else {
                    AlarmLevel::HighHigh
                }
            }
            AlarmLevel::Low => {
                if value <= self.low_low {
                    AlarmLevel::LowLow
                } else if value > self.low + self.deadband {
                    AlarmLevel::Normal
                } else {
                    AlarmLevel::Low
                }
            }
            AlarmLevel::LowLow => {
                if value > self.low_low + self.deadband {
                    if value <= self.low {
                        AlarmLevel::Low
                    } else {
                        AlarmLevel::Normal
                    }
                } else {
                    AlarmLevel::LowLow
                }
            }
        }
    }
}

/// Data logger engine
pub struct DataLogger {
    state: LoggerState,
    config: LoggerConfig,
    sequence: u32,
    total_records: u64,
    alarm_thresholds: [AlarmThreshold; 4],
    alarm_states: [AlarmLevel; 4],
}

impl DataLogger {
    pub fn new(config: LoggerConfig) -> Self {
        Self {
            state: LoggerState::Idle,
            config,
            sequence: 0,
            total_records: 0,
            alarm_thresholds: [AlarmThreshold::default(); 4],
            alarm_states: [AlarmLevel::Normal; 4],
        }
    }

    pub fn state(&self) -> LoggerState {
        self.state
    }

    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    pub fn total_records(&self) -> u64 {
        self.total_records
    }

    pub fn alarm_states(&self) -> &[AlarmLevel; 4] {
        &self.alarm_states
    }

    pub fn set_alarm_threshold(&mut self, channel: usize, threshold: AlarmThreshold) {
        if channel < 4 {
            self.alarm_thresholds[channel] = threshold;
        }
    }

    /// Create a data record from current sensor readings
    pub fn create_record(
        &mut self,
        timestamp_ms: u64,
        analog: [f32; 4],
        digital_in: u8,
        digital_out: u8,
        mcu_temp: f32,
        supply_voltage: f32,
    ) -> DataRecord {
        let record = DataRecord {
            timestamp_ms,
            sequence: self.sequence,
            analog,
            digital_in,
            digital_out,
            mcu_temp,
            supply_voltage,
        };
        self.sequence = self.sequence.wrapping_add(1);
        self.total_records += 1;

        // Evaluate alarms
        for i in 0..4 {
            self.alarm_states[i] =
                self.alarm_thresholds[i].evaluate(analog[i], self.alarm_states[i]);
        }

        record
    }

    /// Check if any alarm is active
    pub fn any_alarm_active(&self) -> bool {
        self.alarm_states.iter().any(|s| *s != AlarmLevel::Normal)
    }

    pub fn start(&mut self) {
        self.state = LoggerState::Running;
    }

    pub fn pause(&mut self) {
        self.state = LoggerState::Paused;
    }

    pub fn stop(&mut self) {
        self.state = LoggerState::Idle;
    }
}

/// CRC-16 for data records
fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}
