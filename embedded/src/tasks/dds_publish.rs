/// DDS Publish Task -- periodic sensor data publishing via XRCE-DDS
///
/// This task reads sensor data from the shared state and publishes it
/// to the DDS-XRCE Agent. It supports multiple topics with different
/// QoS settings:
///
///   - Sensor telemetry: best-effort, high frequency (1-10 Hz)
///   - Alarm events: reliable delivery, on-change
///   - Heartbeat/status: best-effort, low frequency (0.1 Hz)
///
/// PTP timestamps are attached to each measurement for time-synchronized
/// data fusion across multiple sensor nodes.

#[cfg(feature = "dds")]
use crate::net::dds_xrce::{
    XrceClient, XrceError, ObjectId, QosProfile,
    StringVoltageData, StringCurrentData, BusPowerData,
    WaterLevelData, FlowRateData, PressureData,
    AlarmEvent, HeartbeatData,
};
use embassy_time::{Duration, Ticker, Instant};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use defmt::{info, warn, error, Format};

// ============================================================================
// DDS Topic definitions
// ============================================================================

/// Well-known DDS topic names for the SCADA system
pub mod topics {
    /// Solar string voltage measurements
    pub const SOLAR_STRING_VOLTAGE: &str = "rt/scada/solar/string_voltage";
    /// Solar string current measurements
    pub const SOLAR_STRING_CURRENT: &str = "rt/scada/solar/string_current";
    /// Solar bus power measurements
    pub const SOLAR_BUS_POWER: &str = "rt/scada/solar/bus_power";
    /// Water tank level measurements
    pub const WATER_LEVEL: &str = "rt/scada/water/level";
    /// Water flow rate measurements
    pub const WATER_FLOW: &str = "rt/scada/water/flow";
    /// Water pressure measurements
    pub const WATER_PRESSURE: &str = "rt/scada/water/pressure";
    /// Alarm events (both solar and water)
    pub const ALARM_EVENT: &str = "rt/scada/alarms";
    /// Device heartbeat / health status
    pub const HEARTBEAT: &str = "rt/scada/heartbeat";
    /// Configuration commands (subscribe)
    pub const CONFIG_CMD: &str = "rt/scada/config/cmd";
}

/// DDS type names for CDR serialization
pub mod type_names {
    pub const STRING_VOLTAGE: &str = "scada_msgs::msg::StringVoltageData";
    pub const STRING_CURRENT: &str = "scada_msgs::msg::StringCurrentData";
    pub const BUS_POWER: &str = "scada_msgs::msg::BusPowerData";
    pub const WATER_LEVEL: &str = "scada_msgs::msg::WaterLevelData";
    pub const FLOW_RATE: &str = "scada_msgs::msg::FlowRateData";
    pub const PRESSURE: &str = "scada_msgs::msg::PressureData";
    pub const ALARM_EVENT: &str = "scada_msgs::msg::AlarmEvent";
    pub const HEARTBEAT: &str = "scada_msgs::msg::HeartbeatData";
}

// ============================================================================
// DDS Publisher Configuration
// ============================================================================

/// DDS publish task configuration
pub struct DdsPublishConfig {
    /// Sensor telemetry publish interval (milliseconds)
    pub telemetry_interval_ms: u64,
    /// Heartbeat publish interval (milliseconds)
    pub heartbeat_interval_ms: u64,
    /// Device ID for heartbeat and alarm source identification
    pub device_id: u16,
    /// DDS domain ID
    pub domain_id: u16,
}

impl Default for DdsPublishConfig {
    fn default() -> Self {
        Self {
            telemetry_interval_ms: 1000,
            heartbeat_interval_ms: 10_000,
            device_id: 0x0001,
            domain_id: 0,
        }
    }
}

// ============================================================================
// DDS Publisher State
// ============================================================================

/// DDS publish task statistics
pub struct DdsPublishStats {
    pub messages_published: u32,
    pub publish_errors: u32,
    pub alarms_published: u32,
    pub heartbeats_published: u32,
    pub last_publish_ms: u64,
    pub session_reconnects: u32,
}

impl DdsPublishStats {
    pub const fn new() -> Self {
        Self {
            messages_published: 0,
            publish_errors: 0,
            alarms_published: 0,
            heartbeats_published: 0,
            last_publish_ms: 0,
            session_reconnects: 0,
        }
    }
}

/// Signal to trigger an immediate alarm publish
pub type AlarmSignal = Signal<CriticalSectionRawMutex, AlarmEvent>;

// ============================================================================
// DDS Object IDs for pre-created entities
// ============================================================================

/// Pre-defined object IDs for DDS entities
pub mod obj_ids {
    use super::ObjectId;

    // Topics
    pub const TOPIC_STRING_VOLTAGE: ObjectId = ObjectId::topic(1);
    pub const TOPIC_STRING_CURRENT: ObjectId = ObjectId::topic(2);
    pub const TOPIC_BUS_POWER: ObjectId = ObjectId::topic(3);
    pub const TOPIC_WATER_LEVEL: ObjectId = ObjectId::topic(4);
    pub const TOPIC_WATER_FLOW: ObjectId = ObjectId::topic(5);
    pub const TOPIC_WATER_PRESSURE: ObjectId = ObjectId::topic(6);
    pub const TOPIC_ALARM: ObjectId = ObjectId::topic(7);
    pub const TOPIC_HEARTBEAT: ObjectId = ObjectId::topic(8);
    pub const TOPIC_CONFIG_CMD: ObjectId = ObjectId::topic(9);

    // DataWriters
    pub const DW_STRING_VOLTAGE: ObjectId = ObjectId::datawriter(1);
    pub const DW_STRING_CURRENT: ObjectId = ObjectId::datawriter(2);
    pub const DW_BUS_POWER: ObjectId = ObjectId::datawriter(3);
    pub const DW_WATER_LEVEL: ObjectId = ObjectId::datawriter(4);
    pub const DW_WATER_FLOW: ObjectId = ObjectId::datawriter(5);
    pub const DW_WATER_PRESSURE: ObjectId = ObjectId::datawriter(6);
    pub const DW_ALARM: ObjectId = ObjectId::datawriter(7);
    pub const DW_HEARTBEAT: ObjectId = ObjectId::datawriter(8);

    // DataReaders
    pub const DR_CONFIG_CMD: ObjectId = ObjectId::datareader(1);
}

// ============================================================================
// PTP Timestamp helper
// ============================================================================

/// Get a PTP-synchronized timestamp in nanoseconds
///
/// On hardware with PTP support (STM32MP157, STM32H7), this reads the
/// IEEE 1588 PTP clock. On other platforms, falls back to the Embassy
/// monotonic timer.
pub fn get_ptp_timestamp_ns() -> u64 {
    // TODO: Integrate with actual PTP hardware clock when available
    // For now, use the Embassy monotonic timer (microseconds -> nanoseconds)
    Instant::now().as_micros() * 1000
}

// ============================================================================
// DDS Publish helpers
// ============================================================================

/// Publish a solar string voltage reading via DDS
#[cfg(feature = "dds")]
pub fn publish_string_voltage<S>(
    client: &mut XrceClient,
    send_fn: &mut S,
    string_id: u16,
    voltage_v: f32,
    quality: u8,
) -> Result<(), XrceError>
where
    S: FnMut(&[u8]) -> Result<(), XrceError>,
{
    let data = StringVoltageData {
        string_id,
        voltage_v,
        timestamp_ns: get_ptp_timestamp_ns(),
        quality,
    };
    let mut buf = [0u8; 64];
    let len = data.serialize(&mut buf);
    client.write_data(send_fn, obj_ids::DW_STRING_VOLTAGE, &buf[..len], QosProfile::BestEffort)
}

/// Publish a solar string current reading via DDS
#[cfg(feature = "dds")]
pub fn publish_string_current<S>(
    client: &mut XrceClient,
    send_fn: &mut S,
    string_id: u16,
    current_a: f32,
    quality: u8,
) -> Result<(), XrceError>
where
    S: FnMut(&[u8]) -> Result<(), XrceError>,
{
    let data = StringCurrentData {
        string_id,
        current_a,
        timestamp_ns: get_ptp_timestamp_ns(),
        quality,
    };
    let mut buf = [0u8; 64];
    let len = data.serialize(&mut buf);
    client.write_data(send_fn, obj_ids::DW_STRING_CURRENT, &buf[..len], QosProfile::BestEffort)
}

/// Publish a bus power reading via DDS
#[cfg(feature = "dds")]
pub fn publish_bus_power<S>(
    client: &mut XrceClient,
    send_fn: &mut S,
    bus_id: u16,
    voltage_v: f32,
    current_a: f32,
    power_w: f32,
    energy_wh: f32,
    quality: u8,
) -> Result<(), XrceError>
where
    S: FnMut(&[u8]) -> Result<(), XrceError>,
{
    let data = BusPowerData {
        bus_id,
        voltage_v,
        current_a,
        power_w,
        energy_wh,
        timestamp_ns: get_ptp_timestamp_ns(),
        quality,
    };
    let mut buf = [0u8; 64];
    let len = data.serialize(&mut buf);
    client.write_data(send_fn, obj_ids::DW_BUS_POWER, &buf[..len], QosProfile::BestEffort)
}

/// Publish a water level reading via DDS
#[cfg(feature = "dds")]
pub fn publish_water_level<S>(
    client: &mut XrceClient,
    send_fn: &mut S,
    tank_id: u16,
    level_pct: f32,
    level_m: f32,
    overflow: bool,
    quality: u8,
) -> Result<(), XrceError>
where
    S: FnMut(&[u8]) -> Result<(), XrceError>,
{
    let data = WaterLevelData {
        tank_id,
        level_pct,
        level_m,
        timestamp_ns: get_ptp_timestamp_ns(),
        quality,
        overflow,
    };
    let mut buf = [0u8; 64];
    let len = data.serialize(&mut buf);
    client.write_data(send_fn, obj_ids::DW_WATER_LEVEL, &buf[..len], QosProfile::BestEffort)
}

/// Publish a flow rate reading via DDS
#[cfg(feature = "dds")]
pub fn publish_flow_rate<S>(
    client: &mut XrceClient,
    send_fn: &mut S,
    sensor_id: u16,
    flow_lpm: f32,
    total_volume_l: f32,
    quality: u8,
) -> Result<(), XrceError>
where
    S: FnMut(&[u8]) -> Result<(), XrceError>,
{
    let data = FlowRateData {
        sensor_id,
        flow_lpm,
        total_volume_l,
        timestamp_ns: get_ptp_timestamp_ns(),
        quality,
    };
    let mut buf = [0u8; 64];
    let len = data.serialize(&mut buf);
    client.write_data(send_fn, obj_ids::DW_WATER_FLOW, &buf[..len], QosProfile::BestEffort)
}

/// Publish a pressure reading via DDS
#[cfg(feature = "dds")]
pub fn publish_pressure<S>(
    client: &mut XrceClient,
    send_fn: &mut S,
    sensor_id: u16,
    pressure_bar: f32,
    quality: u8,
) -> Result<(), XrceError>
where
    S: FnMut(&[u8]) -> Result<(), XrceError>,
{
    let data = PressureData {
        sensor_id,
        pressure_bar,
        timestamp_ns: get_ptp_timestamp_ns(),
        quality,
    };
    let mut buf = [0u8; 64];
    let len = data.serialize(&mut buf);
    client.write_data(send_fn, obj_ids::DW_WATER_PRESSURE, &buf[..len], QosProfile::BestEffort)
}

/// Publish an alarm event via DDS (reliable delivery)
#[cfg(feature = "dds")]
pub fn publish_alarm<S>(
    client: &mut XrceClient,
    send_fn: &mut S,
    alarm: &AlarmEvent,
) -> Result<(), XrceError>
where
    S: FnMut(&[u8]) -> Result<(), XrceError>,
{
    let mut buf = [0u8; 128];
    let len = alarm.serialize(&mut buf);
    client.write_data(send_fn, obj_ids::DW_ALARM, &buf[..len], QosProfile::Reliable)
}

/// Publish a heartbeat/status message via DDS
#[cfg(feature = "dds")]
pub fn publish_heartbeat<S>(
    client: &mut XrceClient,
    send_fn: &mut S,
    heartbeat: &HeartbeatData,
) -> Result<(), XrceError>
where
    S: FnMut(&[u8]) -> Result<(), XrceError>,
{
    let mut buf = [0u8; 64];
    let len = heartbeat.serialize(&mut buf);
    client.write_data(send_fn, obj_ids::DW_HEARTBEAT, &buf[..len], QosProfile::BestEffort)
}
