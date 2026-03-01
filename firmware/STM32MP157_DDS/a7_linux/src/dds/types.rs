/// DDS Data Types — Standard Library Version
///
/// These types mirror the M4 no_std CDR types with serde derive macros
/// for convenient serialization/deserialization on the A7 Linux side.
///
/// CDR wire format is compatible between M4 (manual CDR) and A7 (serde CDR).

use serde::{Deserialize, Serialize};

/// Timestamp — IEEE 1588 PTP-compatible
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Timestamp {
    pub sec: u32,
    pub nanosec: u32,
}

/// Device identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub device_type: u8,
    pub firmware_ver: u32,
    pub location: String,
}

/// Solar string voltage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringVoltageData {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub voltages: [f32; 16],
    pub status_bitmap: u16,
    pub bus_voltage: f32,
}

impl StringVoltageData {
    pub const TOPIC_NAME: &'static str = "rt/solar/string_voltage";
    pub const TYPE_NAME: &'static str = "scada::solar::StringVoltageData";
}

/// Solar string current data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringCurrentData {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub currents: [f32; 16],
    pub status_bitmap: u16,
    pub bus_current: f32,
}

impl StringCurrentData {
    pub const TOPIC_NAME: &'static str = "rt/solar/string_current";
    pub const TYPE_NAME: &'static str = "scada::solar::StringCurrentData";
}

/// DC bus power data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusPowerData {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub bus_voltage: f32,
    pub bus_current: f32,
    pub total_power_kw: f32,
    pub energy_kwh: f32,
}

impl BusPowerData {
    pub const TOPIC_NAME: &'static str = "rt/solar/bus_power";
    pub const TYPE_NAME: &'static str = "scada::common::BusPowerData";
}

/// Water level data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterLevelData {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub levels: [f32; 8],
    pub raw_ma: [u16; 8],
    pub status: [u8; 8],
}

impl WaterLevelData {
    pub const TOPIC_NAME: &'static str = "rt/water/level";
    pub const TYPE_NAME: &'static str = "scada::water::WaterLevelData";
}

/// Flow rate data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRateData {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub flow_rate: f32,
    pub total_volume: f32,
    pub velocity: f32,
    pub raw_ma: f32,
    pub status: u8,
}

impl FlowRateData {
    pub const TOPIC_NAME: &'static str = "rt/water/flow";
    pub const TYPE_NAME: &'static str = "scada::water::FlowRateData";
}

/// Pressure data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureData {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub pressure_bar: f32,
    pub raw_ma: f32,
    pub status: u8,
}

impl PressureData {
    pub const TOPIC_NAME: &'static str = "rt/water/pressure";
    pub const TYPE_NAME: &'static str = "scada::water::PressureData";
}

/// Water quality data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterQualityData {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub ph: f32,
    pub turbidity_ntu: f32,
    pub chlorine_mgl: f32,
    pub conductivity_uscm: f32,
    pub dissolved_o2_mgl: f32,
    pub status: [u8; 5],
}

impl WaterQualityData {
    pub const TOPIC_NAME: &'static str = "rt/water/quality";
    pub const TYPE_NAME: &'static str = "scada::water::WaterQualityData";
}

/// Alarm severity levels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum AlarmSeverity {
    Info = 0,
    Warning = 1,
    Critical = 2,
    Emergency = 3,
}

/// Alarm event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmEvent {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub alarm_code: u16,
    pub severity: AlarmSeverity,
    pub description: String,
    pub value: f32,
    pub threshold: f32,
    pub channel: u8,
}

impl AlarmEvent {
    pub const TOPIC_NAME: &'static str = "rt/alarm";
    pub const TYPE_NAME: &'static str = "scada::common::AlarmEvent";
}

/// Heartbeat data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatData {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub uptime_secs: u32,
    pub cpu_usage: u8,
    pub temperature: f32,
    pub free_memory: u32,
    pub task_exec_us: [u32; 4],
    pub device_status: u16,
}

impl HeartbeatData {
    pub const TOPIC_NAME: &'static str = "rt/heartbeat";
    pub const TYPE_NAME: &'static str = "scada::common::HeartbeatData";
}

/// Configuration command types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CommandType {
    SetScanInterval = 0x01,
    SetModbusAddress = 0x02,
    SaveCalibration = 0x03,
    ResetCalibration = 0x04,
    SetAlarmThreshold = 0x05,
    SetChannelEnable = 0x06,
    ForceScan = 0x07,
    Restart = 0xFF,
}

/// Configuration command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCommand {
    pub timestamp: Timestamp,
    pub device_id: String,
    pub command_type: CommandType,
    pub parameters: Vec<u8>,
}

impl ConfigCommand {
    pub const TOPIC_NAME: &'static str = "rt/config/command";
    pub const TYPE_NAME: &'static str = "scada::common::ConfigCommand";
}
