/// DDS Publish Task — High Priority
///
/// Reads shared sensor data, attaches PTP timestamps, serializes to CDR,
/// and sends via RPMsg to the A7 XRCE-DDS agent for publication onto
/// the DDS network.
///
/// Topic publish rates are configurable per-topic. Default rates:
///   - String voltage/current: 1 Hz
///   - Bus power: 1 Hz
///   - Alarms: event-driven (immediate on state change)
///   - Heartbeat: 0.1 Hz (every 10 seconds)
///
/// Reliability:
///   - Sensor data uses best-effort delivery (tolerate drops)
///   - Alarms use reliable delivery (must be delivered)
///   - Heartbeats use best-effort delivery
///
/// CDR serialization is performed in-place using heapless::Vec buffers
/// to avoid heap allocation on the M4.

use embassy_executor::task;
use embassy_time::{Duration, Instant, Timer};
use defmt::{info, warn, debug};
use heapless::String;

use crate::dds::XrceClient;
use crate::dds::entity_id;
use crate::dds::types::*;
use crate::dds::qos::*;
use crate::modbus::device_status;
use crate::rpmsg::RpmsgEndpoint;
use crate::tasks::shared::SharedState;

/// Task index for statistics
const TASK_IDX: usize = 1;

/// Device ID string (built at compile time based on feature)
#[cfg(feature = "solar_smu")]
const DEVICE_ID: &str = "SMU-MP157-001";
#[cfg(feature = "water_rtu")]
const DEVICE_ID: &str = "RTU-MP157-001";

#[task]
pub async fn dds_publish(
    rpmsg: &'static RpmsgEndpoint,
    shared: &'static SharedState,
) {
    info!("DDS publish task: starting");

    // Wait for RPMsg connection to A7
    while !rpmsg.is_connected() {
        Timer::after(Duration::from_millis(100)).await;
    }

    info!("DDS publish: RPMsg connected to A7");

    // Initialize XRCE-DDS client
    let mut xrce = XrceClient::new();

    // Create session with XRCE agent
    if !xrce.create_session(rpmsg) {
        warn!("DDS publish: Failed to create XRCE session, retrying...");
        Timer::after(Duration::from_secs(1)).await;
        xrce.create_session(rpmsg);
    }

    // Create DDS entities
    Timer::after(Duration::from_millis(100)).await;
    xrce.create_participant(rpmsg, 0); // Domain 0

    Timer::after(Duration::from_millis(100)).await;
    xrce.create_publisher(rpmsg);
    xrce.create_subscriber(rpmsg);

    // Create topics and data writers based on variant
    Timer::after(Duration::from_millis(100)).await;

    #[cfg(feature = "solar_smu")]
    {
        xrce.create_topic(rpmsg, entity_id::TOPIC_STRING_VOLTAGE,
            StringVoltageData::TOPIC_NAME, StringVoltageData::TYPE_NAME);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_topic(rpmsg, entity_id::TOPIC_STRING_CURRENT,
            StringCurrentData::TOPIC_NAME, StringCurrentData::TYPE_NAME);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_topic(rpmsg, entity_id::TOPIC_BUS_POWER,
            BusPowerData::TOPIC_NAME, BusPowerData::TYPE_NAME);
        Timer::after(Duration::from_millis(50)).await;

        // Create data writers with QoS
        xrce.create_datawriter(rpmsg, entity_id::WRITER_STRING_VOLTAGE,
            entity_id::TOPIC_STRING_VOLTAGE, &QOS_SENSOR_DATA);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_datawriter(rpmsg, entity_id::WRITER_STRING_CURRENT,
            entity_id::TOPIC_STRING_CURRENT, &QOS_SENSOR_DATA);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_datawriter(rpmsg, entity_id::WRITER_BUS_POWER,
            entity_id::TOPIC_BUS_POWER, &QOS_SENSOR_DATA);
        Timer::after(Duration::from_millis(50)).await;
    }

    #[cfg(feature = "water_rtu")]
    {
        xrce.create_topic(rpmsg, entity_id::TOPIC_WATER_LEVEL,
            WaterLevelData::TOPIC_NAME, WaterLevelData::TYPE_NAME);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_topic(rpmsg, entity_id::TOPIC_FLOW_RATE,
            FlowRateData::TOPIC_NAME, FlowRateData::TYPE_NAME);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_topic(rpmsg, entity_id::TOPIC_PRESSURE,
            PressureData::TOPIC_NAME, PressureData::TYPE_NAME);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_topic(rpmsg, entity_id::TOPIC_WATER_QUALITY,
            WaterQualityData::TOPIC_NAME, WaterQualityData::TYPE_NAME);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_datawriter(rpmsg, entity_id::WRITER_WATER_LEVEL,
            entity_id::TOPIC_WATER_LEVEL, &QOS_SENSOR_DATA);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_datawriter(rpmsg, entity_id::WRITER_FLOW_RATE,
            entity_id::TOPIC_FLOW_RATE, &QOS_SENSOR_DATA);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_datawriter(rpmsg, entity_id::WRITER_PRESSURE,
            entity_id::TOPIC_PRESSURE, &QOS_SENSOR_DATA);
        Timer::after(Duration::from_millis(50)).await;

        xrce.create_datawriter(rpmsg, entity_id::WRITER_WATER_QUALITY,
            entity_id::TOPIC_WATER_QUALITY, &QOS_SENSOR_DATA);
        Timer::after(Duration::from_millis(50)).await;
    }

    // Common topics
    xrce.create_topic(rpmsg, entity_id::TOPIC_ALARM,
        AlarmEvent::TOPIC_NAME, AlarmEvent::TYPE_NAME);
    xrce.create_datawriter(rpmsg, entity_id::WRITER_ALARM,
        entity_id::TOPIC_ALARM, &QOS_ALARM);
    Timer::after(Duration::from_millis(50)).await;

    xrce.create_topic(rpmsg, entity_id::TOPIC_HEARTBEAT,
        HeartbeatData::TOPIC_NAME, HeartbeatData::TYPE_NAME);
    xrce.create_datawriter(rpmsg, entity_id::WRITER_HEARTBEAT,
        entity_id::TOPIC_HEARTBEAT, &QOS_HEARTBEAT);
    Timer::after(Duration::from_millis(50)).await;

    // Create command reader
    xrce.create_topic(rpmsg, entity_id::TOPIC_CONFIG_COMMAND,
        ConfigCommand::TOPIC_NAME, ConfigCommand::TYPE_NAME);
    xrce.create_datareader(rpmsg, entity_id::READER_CONFIG_COMMAND,
        entity_id::TOPIC_CONFIG_COMMAND);

    // Set DDS OK status
    {
        let mut status = shared.device_status.lock().await;
        *status |= device_status::DDS_OK | device_status::RPMSG_OK;
    }

    info!("DDS publish: All entities created, starting publish loop");

    let rates = PublishRates::default();
    let mut last_sensor_publish = Instant::now();
    let mut last_heartbeat = Instant::now();
    let mut prev_alarm_bitmap: u16 = 0;

    let mut device_id: String<32> = String::new();
    let _ = device_id.push_str(DEVICE_ID);

    loop {
        // Wait for new scan data
        shared.new_scan.wait().await;

        let exec_start = Instant::now();

        // Get PTP timestamp
        let ptp_ts = {
            let ptp = shared.ptp_clock.lock().await;
            ptp.read_timestamp()
        };
        let timestamp = Timestamp {
            sec: ptp_ts.seconds,
            nanosec: ptp_ts.nanoseconds,
        };

        let now = Instant::now();

        // ── Publish sensor data at configured rate ──
        if now.duration_since(last_sensor_publish).as_millis() >= rates.string_voltage_ms as u64 {
            last_sensor_publish = now;

            #[cfg(feature = "solar_smu")]
            {
                // Publish string voltage data
                let channels = shared.channels.lock().await;
                let mut voltages = [0.0f32; 16];
                let mut status_bitmap: u16 = 0;
                for i in 0..16 {
                    voltages[i] = channels[i].value;
                    if channels[i].status != 0 {
                        status_bitmap |= 1 << i;
                    }
                }
                drop(channels);

                let bus_v = { *shared.bus_voltage.lock().await };

                let voltage_data = StringVoltageData {
                    timestamp,
                    device_id: device_id.clone(),
                    voltages,
                    status_bitmap,
                    bus_voltage: bus_v,
                };

                let mut cdr_buf: heapless::Vec<u8, 512> = heapless::Vec::new();
                voltage_data.encode_cdr(&mut cdr_buf);
                xrce.write_data(rpmsg, entity_id::WRITER_STRING_VOLTAGE, &cdr_buf, false);

                // Publish bus power
                let bus_i = { *shared.bus_current.lock().await };
                let total_p = { *shared.total_power.lock().await };
                let energy = { *shared.daily_energy_kwh.lock().await };

                let power_data = BusPowerData {
                    timestamp,
                    device_id: device_id.clone(),
                    bus_voltage: bus_v,
                    bus_current: bus_i,
                    total_power_kw: total_p,
                    energy_kwh: energy,
                };

                let mut cdr_buf: heapless::Vec<u8, 512> = heapless::Vec::new();
                power_data.encode_cdr(&mut cdr_buf);
                xrce.write_data(rpmsg, entity_id::WRITER_BUS_POWER, &cdr_buf, false);
            }

            #[cfg(feature = "water_rtu")]
            {
                let channels = shared.channels.lock().await;
                let mut levels = [0.0f32; 8];
                let mut raw_ma = [0u16; 8];
                let mut ch_status = [0u8; 8];

                for i in 0..8 {
                    levels[i] = channels[i].value;
                    raw_ma[i] = (channels[i].value * 100.0) as u16;
                    ch_status[i] = channels[i].status;
                }
                drop(channels);

                let level_data = WaterLevelData {
                    timestamp,
                    device_id: device_id.clone(),
                    levels,
                    raw_ma,
                    status: ch_status,
                };

                let mut cdr_buf: heapless::Vec<u8, 512> = heapless::Vec::new();
                level_data.encode_cdr(&mut cdr_buf);
                xrce.write_data(rpmsg, entity_id::WRITER_WATER_LEVEL, &cdr_buf, false);
            }
        }

        // ── Publish alarms on state change (reliable delivery) ──
        {
            let regs = shared.registers.lock().await;
            let current_alarms = regs.input_regs[crate::modbus::input_reg::ALARM_BITMAP_LO as usize];

            // Detect new alarms (bits that changed from 0 to 1)
            let new_alarms = current_alarms & !prev_alarm_bitmap;
            if new_alarms != 0 {
                for bit in 0..16u8 {
                    if new_alarms & (1 << bit) != 0 {
                        let alarm = AlarmEvent {
                            timestamp,
                            device_id: device_id.clone(),
                            alarm_code: 0x0100 + bit as u16,
                            severity: AlarmSeverity::Warning,
                            description: {
                                let mut s = String::new();
                                let _ = s.push_str("Channel fault CH");
                                s
                            },
                            value: 0.0,
                            threshold: 0.0,
                            channel: bit,
                        };

                        let mut cdr_buf: heapless::Vec<u8, 512> = heapless::Vec::new();
                        alarm.encode_cdr(&mut cdr_buf);
                        xrce.write_data(rpmsg, entity_id::WRITER_ALARM, &cdr_buf, true);
                    }
                }
            }
            prev_alarm_bitmap = current_alarms;
        }

        // ── Publish heartbeat at configured rate ──
        if now.duration_since(last_heartbeat).as_millis() >= rates.heartbeat_ms as u64 {
            last_heartbeat = now;

            let uptime = (Instant::now().as_millis() / 1000) as u32;
            let temp = { *shared.temperature.lock().await };
            let dev_status = { *shared.device_status.lock().await };
            let task_stats = { *shared.task_stats.lock().await };

            let heartbeat = HeartbeatData {
                timestamp,
                device_id: device_id.clone(),
                uptime_secs: uptime,
                cpu_usage: 0, // TODO: compute from task stats
                temperature: temp,
                free_memory: 0, // TODO: compute
                task_exec_us: [
                    task_stats[0].last_exec_us,
                    task_stats[1].last_exec_us,
                    task_stats[2].last_exec_us,
                    task_stats[3].last_exec_us,
                ],
                device_status: dev_status,
            };

            let mut cdr_buf: heapless::Vec<u8, 512> = heapless::Vec::new();
            heartbeat.encode_cdr(&mut cdr_buf);
            xrce.write_data(rpmsg, entity_id::WRITER_HEARTBEAT, &cdr_buf, false);
        }

        // ── Check for incoming DDS commands ──
        let mut cmd_buf = [0u8; 256];
        if let Some((_reader_id, cmd_len)) = xrce.read_data(rpmsg, &mut cmd_buf) {
            if let Some(cmd) = ConfigCommand::decode_cdr(&cmd_buf[..cmd_len]) {
                info!("DDS: Received command type=0x{:02X}", cmd.command_type);
                handle_config_command(&cmd, shared).await;
            }
        }

        // Update task statistics
        {
            let mut stats = shared.task_stats.lock().await;
            let exec_us = exec_start.elapsed().as_micros() as u32;
            stats[TASK_IDX].last_exec_us = exec_us;
            if exec_us > stats[TASK_IDX].max_exec_us {
                stats[TASK_IDX].max_exec_us = exec_us;
            }
            stats[TASK_IDX].exec_count += 1;
        }
    }
}

/// Handle a configuration command received via DDS
async fn handle_config_command(cmd: &ConfigCommand, shared: &SharedState) {
    match cmd.command_type {
        0x01 => {
            // SetScanInterval
            let interval = u16::from_le_bytes([cmd.parameters[0], cmd.parameters[1]]);
            let mut regs = shared.registers.lock().await;
            regs.holding_regs[crate::modbus::holding_reg::SCAN_INTERVAL_MS as usize] = interval;
            info!("DDS cmd: Scan interval set to {}ms", interval);
        }
        0x02 => {
            // SetModbusAddress
            let addr = cmd.parameters[0];
            let mut regs = shared.registers.lock().await;
            regs.holding_regs[crate::modbus::holding_reg::SLAVE_ADDR as usize] = addr as u16;
            info!("DDS cmd: Modbus address set to {}", addr);
        }
        0x03 => {
            // SaveCalibration
            shared.cal_save_requested.signal(());
            info!("DDS cmd: Calibration save requested");
        }
        0x04 => {
            // ResetCalibration
            let mut cal = shared.calibration.lock().await;
            cal.factory_reset();
            info!("DDS cmd: Calibration reset to factory defaults");
        }
        0x07 => {
            // ForceScan
            info!("DDS cmd: Force scan requested");
        }
        _ => {
            warn!("DDS cmd: Unknown command type 0x{:02X}", cmd.command_type);
        }
    }
}
