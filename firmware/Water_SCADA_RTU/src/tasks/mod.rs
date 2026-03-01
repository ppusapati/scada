/// Embassy task modules for Water SCADA RTU
///
/// Task priorities (Embassy cooperative scheduling):
///   1. adc_task       — 10 Hz acquisition, highest urgency
///   2. modbus_rtu_task — RS-485 slave response
///   3. modbus_tcp_task — Ethernet TCP server
///   4. diagnostics_task — LED heartbeat, alarm processing

pub mod shared;
pub mod adc_task;
pub mod modbus_rtu_task;
pub mod modbus_tcp_task;
pub mod diagnostics_task;
