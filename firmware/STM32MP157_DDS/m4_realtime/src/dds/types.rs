/// CDR Data Types for DDS Communication
///
/// These types are shared between the M4 firmware (no_std, CDR encoding)
/// and the A7 Linux application (std, serde+CDR). Both sides must use
/// identical CDR wire format for interoperability.
///
/// CDR (Common Data Representation) encoding rules:
///   - Little-endian byte order (XCDR1)
///   - 4-byte alignment for 32-bit types
///   - 8-byte alignment for 64-bit types
///   - Strings: 4-byte length prefix + UTF-8 data + null terminator
///   - Arrays: elements packed with natural alignment
///   - Booleans: 1 byte (0x00 = false, 0x01 = true)
///
/// Each type includes a topic name constant and CDR serialization methods.

use heapless::{String, Vec};

/// CDR header (4 bytes, prepended to every CDR payload)
pub const CDR_HEADER: [u8; 4] = [
    0x00, // Encapsulation kind: CDR_LE
    0x01, // Encapsulation options (little-endian)
    0x00, // Padding
    0x00, // Padding
];

/// Timestamp — IEEE 1588 PTP-compatible
#[derive(Clone, Copy, Default)]
pub struct Timestamp {
    pub sec: u32,
    pub nanosec: u32,
}

impl Timestamp {
    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&self.sec.to_le_bytes());
        let _ = buf.extend_from_slice(&self.nanosec.to_le_bytes());
    }

    pub fn decode_cdr(data: &[u8], offset: &mut usize) -> Option<Self> {
        if *offset + 8 > data.len() {
            return None;
        }
        let sec = u32::from_le_bytes([data[*offset], data[*offset+1], data[*offset+2], data[*offset+3]]);
        *offset += 4;
        let nanosec = u32::from_le_bytes([data[*offset], data[*offset+1], data[*offset+2], data[*offset+3]]);
        *offset += 4;
        Some(Self { sec, nanosec })
    }

    pub const CDR_SIZE: usize = 8;
}

/// Device identity — identifies a SCADA field device
#[derive(Clone)]
pub struct DeviceIdentity {
    /// Unique device ID (e.g., "SMU-PLANT01-STR01")
    pub device_id: String<32>,
    /// Device type: 0=Solar SMU, 1=Water RTU, 2=Gateway
    pub device_type: u8,
    /// Firmware version (major.minor.patch encoded as u32)
    pub firmware_ver: u32,
    /// Physical location descriptor
    pub location: String<64>,
}

impl DeviceIdentity {
    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        // device_id: string (4-byte length + data + null + padding)
        encode_cdr_string(buf, &self.device_id);
        // device_type: u8 + 3 bytes padding for alignment
        let _ = buf.push(self.device_type);
        let _ = buf.extend_from_slice(&[0u8; 3]); // Align to 4 bytes
        // firmware_ver: u32
        let _ = buf.extend_from_slice(&self.firmware_ver.to_le_bytes());
        // location: string
        encode_cdr_string(buf, &self.location);
    }
}

/// Solar string voltage data — 16 string voltages + status
///
/// Topic: "rt/solar/string_voltage"
pub struct StringVoltageData {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// 16 string voltages in volts
    pub voltages: [f32; 16],
    /// Per-string status bitmap (1 bit per string, 1=fault)
    pub status_bitmap: u16,
    /// Bus voltage (DC bus)
    pub bus_voltage: f32,
}

impl StringVoltageData {
    pub const TOPIC_NAME: &'static str = "rt/solar/string_voltage";
    pub const TYPE_NAME: &'static str = "scada::solar::StringVoltageData";

    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&CDR_HEADER);
        self.timestamp.encode_cdr(buf);
        encode_cdr_string(buf, &self.device_id);
        // Array of 16 floats (no length prefix for fixed-size arrays)
        for v in &self.voltages {
            let _ = buf.extend_from_slice(&v.to_le_bytes());
        }
        // status_bitmap: u16 + 2 bytes padding
        let _ = buf.extend_from_slice(&self.status_bitmap.to_le_bytes());
        let _ = buf.extend_from_slice(&[0u8; 2]);
        // bus_voltage: f32
        let _ = buf.extend_from_slice(&self.bus_voltage.to_le_bytes());
    }
}

/// Solar string current data — 16 string currents + status
///
/// Topic: "rt/solar/string_current"
pub struct StringCurrentData {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// 16 string currents in amps
    pub currents: [f32; 16],
    /// Per-string status bitmap
    pub status_bitmap: u16,
    /// Bus current (total DC bus current)
    pub bus_current: f32,
}

impl StringCurrentData {
    pub const TOPIC_NAME: &'static str = "rt/solar/string_current";
    pub const TYPE_NAME: &'static str = "scada::solar::StringCurrentData";

    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&CDR_HEADER);
        self.timestamp.encode_cdr(buf);
        encode_cdr_string(buf, &self.device_id);
        for c in &self.currents {
            let _ = buf.extend_from_slice(&c.to_le_bytes());
        }
        let _ = buf.extend_from_slice(&self.status_bitmap.to_le_bytes());
        let _ = buf.extend_from_slice(&[0u8; 2]);
        let _ = buf.extend_from_slice(&self.bus_current.to_le_bytes());
    }
}

/// DC bus power data — aggregated bus measurements
///
/// Topic: "rt/solar/bus_power" or "rt/water/bus_power"
pub struct BusPowerData {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// DC bus voltage (V)
    pub bus_voltage: f32,
    /// DC bus current (A)
    pub bus_current: f32,
    /// Total power (kW)
    pub total_power_kw: f32,
    /// Accumulated energy (kWh)
    pub energy_kwh: f32,
}

impl BusPowerData {
    pub const TOPIC_NAME: &'static str = "rt/solar/bus_power";
    pub const TYPE_NAME: &'static str = "scada::common::BusPowerData";

    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&CDR_HEADER);
        self.timestamp.encode_cdr(buf);
        encode_cdr_string(buf, &self.device_id);
        let _ = buf.extend_from_slice(&self.bus_voltage.to_le_bytes());
        let _ = buf.extend_from_slice(&self.bus_current.to_le_bytes());
        let _ = buf.extend_from_slice(&self.total_power_kw.to_le_bytes());
        let _ = buf.extend_from_slice(&self.energy_kwh.to_le_bytes());
    }
}

/// Water level data — 8 level sensor readings
///
/// Topic: "rt/water/level"
pub struct WaterLevelData {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// 8 level readings (percentage 0-100 or absolute meters)
    pub levels: [f32; 8],
    /// Raw 4-20mA values (mA × 100 as u16)
    pub raw_ma: [u16; 8],
    /// Per-channel status (0=normal, 1=under, 2=over, 3=open, 4=short)
    pub status: [u8; 8],
}

impl WaterLevelData {
    pub const TOPIC_NAME: &'static str = "rt/water/level";
    pub const TYPE_NAME: &'static str = "scada::water::WaterLevelData";

    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&CDR_HEADER);
        self.timestamp.encode_cdr(buf);
        encode_cdr_string(buf, &self.device_id);
        for l in &self.levels {
            let _ = buf.extend_from_slice(&l.to_le_bytes());
        }
        for r in &self.raw_ma {
            let _ = buf.extend_from_slice(&r.to_le_bytes());
        }
        let _ = buf.extend_from_slice(&self.status);
    }
}

/// Flow rate data — single flow meter reading
///
/// Topic: "rt/water/flow"
pub struct FlowRateData {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// Instantaneous flow rate (m^3/h)
    pub flow_rate: f32,
    /// Totalized volume (m^3)
    pub total_volume: f32,
    /// Flow velocity (m/s)
    pub velocity: f32,
    /// Raw 4-20mA value
    pub raw_ma: f32,
    /// Channel status
    pub status: u8,
}

impl FlowRateData {
    pub const TOPIC_NAME: &'static str = "rt/water/flow";
    pub const TYPE_NAME: &'static str = "scada::water::FlowRateData";

    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&CDR_HEADER);
        self.timestamp.encode_cdr(buf);
        encode_cdr_string(buf, &self.device_id);
        let _ = buf.extend_from_slice(&self.flow_rate.to_le_bytes());
        let _ = buf.extend_from_slice(&self.total_volume.to_le_bytes());
        let _ = buf.extend_from_slice(&self.velocity.to_le_bytes());
        let _ = buf.extend_from_slice(&self.raw_ma.to_le_bytes());
        let _ = buf.push(self.status);
        // Pad to 4-byte alignment
        let _ = buf.extend_from_slice(&[0u8; 3]);
    }
}

/// Pressure data — pressure transmitter reading
///
/// Topic: "rt/water/pressure"
pub struct PressureData {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// Pressure reading (bar)
    pub pressure_bar: f32,
    /// Raw 4-20mA value
    pub raw_ma: f32,
    /// Channel status
    pub status: u8,
}

impl PressureData {
    pub const TOPIC_NAME: &'static str = "rt/water/pressure";
    pub const TYPE_NAME: &'static str = "scada::water::PressureData";

    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&CDR_HEADER);
        self.timestamp.encode_cdr(buf);
        encode_cdr_string(buf, &self.device_id);
        let _ = buf.extend_from_slice(&self.pressure_bar.to_le_bytes());
        let _ = buf.extend_from_slice(&self.raw_ma.to_le_bytes());
        let _ = buf.push(self.status);
        let _ = buf.extend_from_slice(&[0u8; 3]);
    }
}

/// Water quality data — pH, turbidity, chlorine, conductivity, DO
///
/// Topic: "rt/water/quality"
pub struct WaterQualityData {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// pH value (0-14)
    pub ph: f32,
    /// Turbidity (NTU)
    pub turbidity_ntu: f32,
    /// Chlorine residual (mg/L)
    pub chlorine_mgl: f32,
    /// Conductivity (uS/cm)
    pub conductivity_uscm: f32,
    /// Dissolved oxygen (mg/L)
    pub dissolved_o2_mgl: f32,
    /// Per-parameter status array [pH, turb, Cl, cond, DO]
    pub status: [u8; 5],
}

impl WaterQualityData {
    pub const TOPIC_NAME: &'static str = "rt/water/quality";
    pub const TYPE_NAME: &'static str = "scada::water::WaterQualityData";

    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&CDR_HEADER);
        self.timestamp.encode_cdr(buf);
        encode_cdr_string(buf, &self.device_id);
        let _ = buf.extend_from_slice(&self.ph.to_le_bytes());
        let _ = buf.extend_from_slice(&self.turbidity_ntu.to_le_bytes());
        let _ = buf.extend_from_slice(&self.chlorine_mgl.to_le_bytes());
        let _ = buf.extend_from_slice(&self.conductivity_uscm.to_le_bytes());
        let _ = buf.extend_from_slice(&self.dissolved_o2_mgl.to_le_bytes());
        let _ = buf.extend_from_slice(&self.status);
        // Pad to 4-byte alignment (5 status bytes → 3 bytes padding)
        let _ = buf.extend_from_slice(&[0u8; 3]);
    }
}

/// Alarm event — critical condition notification
///
/// Topic: "rt/alarm" (reliable delivery)
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum AlarmSeverity {
    Info = 0,
    Warning = 1,
    Critical = 2,
    Emergency = 3,
}

pub struct AlarmEvent {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// Alarm code (application-specific, see alarm_codes module)
    pub alarm_code: u16,
    /// Severity level
    pub severity: AlarmSeverity,
    /// Human-readable description
    pub description: String<64>,
    /// Measured value that triggered the alarm
    pub value: f32,
    /// Threshold that was exceeded
    pub threshold: f32,
    /// Channel or string index (0xFF = device-level)
    pub channel: u8,
}

impl AlarmEvent {
    pub const TOPIC_NAME: &'static str = "rt/alarm";
    pub const TYPE_NAME: &'static str = "scada::common::AlarmEvent";

    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&CDR_HEADER);
        self.timestamp.encode_cdr(buf);
        encode_cdr_string(buf, &self.device_id);
        let _ = buf.extend_from_slice(&self.alarm_code.to_le_bytes());
        let _ = buf.push(self.severity as u8);
        let _ = buf.push(0); // padding
        encode_cdr_string(buf, &self.description);
        let _ = buf.extend_from_slice(&self.value.to_le_bytes());
        let _ = buf.extend_from_slice(&self.threshold.to_le_bytes());
        let _ = buf.push(self.channel);
        let _ = buf.extend_from_slice(&[0u8; 3]); // alignment
    }
}

/// Alarm code definitions
pub mod alarm_codes {
    // Solar SMU alarms (0x0100 - 0x01FF)
    pub const SOLAR_STRING_DISCONNECT: u16 = 0x0100;
    pub const SOLAR_STRING_LOW_VOLTAGE: u16 = 0x0101;
    pub const SOLAR_STRING_OVERCURRENT: u16 = 0x0102;
    pub const SOLAR_BUS_OVERVOLTAGE: u16 = 0x0103;
    pub const SOLAR_BUS_UNDERVOLTAGE: u16 = 0x0104;
    pub const SOLAR_OVERTEMPERATURE: u16 = 0x0105;

    // Water RTU alarms (0x0200 - 0x02FF)
    pub const WATER_LOOP_OPEN: u16 = 0x0200;
    pub const WATER_LOOP_SHORT: u16 = 0x0201;
    pub const WATER_UNDER_RANGE: u16 = 0x0202;
    pub const WATER_OVER_RANGE: u16 = 0x0203;
    pub const WATER_HIGH_PRESSURE: u16 = 0x0210;
    pub const WATER_LOW_LEVEL: u16 = 0x0211;
    pub const WATER_HIGH_TURBIDITY: u16 = 0x0212;
    pub const WATER_LOW_CHLORINE: u16 = 0x0213;

    // Device-level alarms (0x0300 - 0x03FF)
    pub const DEVICE_ADC_FAULT: u16 = 0x0300;
    pub const DEVICE_COMM_LOSS: u16 = 0x0301;
    pub const DEVICE_OVERTEMP: u16 = 0x0302;
    pub const DEVICE_WATCHDOG: u16 = 0x0303;
    pub const DEVICE_MEMORY_LOW: u16 = 0x0304;
}

/// Heartbeat data — periodic system health status
///
/// Topic: "rt/heartbeat" (best-effort delivery)
pub struct HeartbeatData {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// Uptime in seconds since boot
    pub uptime_secs: u32,
    /// CPU usage estimate (0-100%)
    pub cpu_usage: u8,
    /// MCU die temperature (degrees C)
    pub temperature: f32,
    /// Free memory estimate (bytes)
    pub free_memory: u32,
    /// Task execution stats: [sensor, dds, modbus, diag] in microseconds
    pub task_exec_us: [u32; 4],
    /// Device status flags (see device_status module)
    pub device_status: u16,
}

impl HeartbeatData {
    pub const TOPIC_NAME: &'static str = "rt/heartbeat";
    pub const TYPE_NAME: &'static str = "scada::common::HeartbeatData";

    pub fn encode_cdr(&self, buf: &mut Vec<u8, 512>) {
        let _ = buf.extend_from_slice(&CDR_HEADER);
        self.timestamp.encode_cdr(buf);
        encode_cdr_string(buf, &self.device_id);
        let _ = buf.extend_from_slice(&self.uptime_secs.to_le_bytes());
        let _ = buf.push(self.cpu_usage);
        let _ = buf.extend_from_slice(&[0u8; 3]); // alignment
        let _ = buf.extend_from_slice(&self.temperature.to_le_bytes());
        let _ = buf.extend_from_slice(&self.free_memory.to_le_bytes());
        for t in &self.task_exec_us {
            let _ = buf.extend_from_slice(&t.to_le_bytes());
        }
        let _ = buf.extend_from_slice(&self.device_status.to_le_bytes());
        let _ = buf.extend_from_slice(&[0u8; 2]); // alignment
    }
}

/// Configuration command — received from SCADA master via DDS
///
/// Topic: "rt/config/command" (reliable delivery)
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum CommandType {
    /// Set scan interval (parameter[0] = interval in ms)
    SetScanInterval = 0x01,
    /// Set Modbus slave address (parameter[0] = address)
    SetModbusAddress = 0x02,
    /// Trigger calibration save to flash
    SaveCalibration = 0x03,
    /// Factory reset calibration to defaults
    ResetCalibration = 0x04,
    /// Set alarm threshold (parameter[0] = channel, parameter[1:4] = threshold as f32)
    SetAlarmThreshold = 0x05,
    /// Enable/disable a channel (parameter[0] = channel, parameter[1] = enable)
    SetChannelEnable = 0x06,
    /// Request immediate sensor reading
    ForceScan = 0x07,
    /// Restart M4 firmware
    Restart = 0xFF,
}

pub struct ConfigCommand {
    pub timestamp: Timestamp,
    pub device_id: String<32>,
    /// Command type
    pub command_type: u8,
    /// Command parameters (variable use depending on command_type)
    pub parameters: [u8; 16],
}

impl ConfigCommand {
    pub const TOPIC_NAME: &'static str = "rt/config/command";
    pub const TYPE_NAME: &'static str = "scada::common::ConfigCommand";

    pub fn decode_cdr(data: &[u8]) -> Option<Self> {
        if data.len() < 4 + Timestamp::CDR_SIZE {
            return None;
        }
        let mut offset = 4; // Skip CDR header
        let timestamp = Timestamp::decode_cdr(data, &mut offset)?;
        let device_id = decode_cdr_string(data, &mut offset)?;
        if offset >= data.len() {
            return None;
        }
        let command_type = data[offset];
        offset += 4; // command_type + 3 bytes padding

        let mut parameters = [0u8; 16];
        let copy_len = core::cmp::min(16, data.len().saturating_sub(offset));
        if copy_len > 0 {
            parameters[..copy_len].copy_from_slice(&data[offset..offset + copy_len]);
        }

        Some(Self {
            timestamp,
            device_id,
            command_type,
            parameters,
        })
    }
}

// ── CDR string encoding/decoding helpers ──

/// Encode a string in CDR format: 4-byte length (including null) + data + null + padding
fn encode_cdr_string<const N: usize>(buf: &mut Vec<u8, 512>, s: &String<N>) {
    let len_with_null = (s.len() + 1) as u32;
    let _ = buf.extend_from_slice(&len_with_null.to_le_bytes());
    let _ = buf.extend_from_slice(s.as_bytes());
    let _ = buf.push(0); // null terminator

    // Pad to 4-byte boundary
    let pad = (4 - ((s.len() + 1) % 4)) % 4;
    for _ in 0..pad {
        let _ = buf.push(0);
    }
}

/// Decode a CDR string
fn decode_cdr_string(data: &[u8], offset: &mut usize) -> Option<String<32>> {
    if *offset + 4 > data.len() {
        return None;
    }
    let len = u32::from_le_bytes([
        data[*offset], data[*offset+1], data[*offset+2], data[*offset+3]
    ]) as usize;
    *offset += 4;

    if *offset + len > data.len() || len == 0 {
        return None;
    }

    // len includes null terminator
    let str_bytes = &data[*offset..*offset + len - 1];
    *offset += len;

    // Align to 4 bytes
    let pad = (4 - (len % 4)) % 4;
    *offset += pad;

    let mut s = String::new();
    for &b in str_bytes {
        if s.push(b as char).is_err() {
            break;
        }
    }
    Some(s)
}
