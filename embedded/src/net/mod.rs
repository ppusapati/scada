/// Networking stack for SCADA sensor nodes
///
/// Supports multiple communication protocols:
///
///   Ethernet (RMII or W5500):
///     - MQTT v3.1.1 for telemetry publishing and command subscription
///     - DDS-XRCE over UDP for real-time sensor data distribution
///     - Modbus TCP server for SCADA HMI integration
///
///   RS-485 (SP3485):
///     - Modbus RTU slave for local bus communication
///
///   RPMsg (STM32MP157 only):
///     - DDS-XRCE via shared memory for M4<->A7 communication
///
/// Network topology:
///   STM32 ETH/W5500 -> Switch -> MQTT Broker + DDS Agent
///   STM32 RS-485 -> Modbus Master (PLC/HMI)

pub mod eth;
pub mod mqtt;

#[cfg(feature = "dds")]
pub mod dds_xrce;

#[cfg(feature = "modbus")]
pub mod modbus;
