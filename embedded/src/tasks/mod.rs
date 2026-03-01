/// Embassy async tasks for SCADA sensor nodes
///
/// Each task runs as a cooperative async coroutine on the Embassy executor.
/// Tasks communicate through shared state using embassy-sync primitives.
///
/// Task architecture:
///   sensor_read    -> reads ADC/I2C sensors, updates SharedSensorData
///   mqtt_publish   -> publishes telemetry to MQTT broker
///   dds_publish    -> publishes sensor data via DDS-XRCE (when enabled)
///   modbus_task    -> handles Modbus RTU slave requests (when enabled)
///   command_handler -> processes incoming commands from MQTT/DDS/Modbus
///   heartbeat      -> LED blink and health status reporting
///   watchdog       -> monitors task health, feeds IWDG

pub mod sensor_read;
pub mod mqtt_publish;
pub mod command_handler;
pub mod heartbeat;
pub mod watchdog;

#[cfg(feature = "dds")]
pub mod dds_publish;

#[cfg(feature = "modbus")]
pub mod modbus_task;
