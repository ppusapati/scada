/// Embassy async tasks for SCADA sensor nodes
///
/// Each task runs as a cooperative async coroutine on the Embassy executor.
/// Tasks communicate through shared state using embassy-sync primitives.

pub mod sensor_read;
pub mod mqtt_publish;
pub mod command_handler;
pub mod heartbeat;
pub mod watchdog;
