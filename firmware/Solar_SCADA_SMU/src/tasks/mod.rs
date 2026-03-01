/// Embassy task modules for Solar SCADA SMU
///
/// Task priorities (Embassy cooperative scheduling):
///   1. scan_task      — MUX+ADC scan of 40 channels, highest urgency
///   2. modbus_task    — RS-485 Modbus RTU slave response
///   3. logging_task   — MicroSD data logging
///   4. diagnostics_task — LED heartbeat, temperature, alarms

pub mod shared;
pub mod scan_task;
pub mod modbus_task;
pub mod logging_task;
pub mod diagnostics_task;
