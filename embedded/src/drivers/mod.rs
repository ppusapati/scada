/// High-level sensor/actuator/peripheral drivers built on top of HAL
///
/// These drivers combine multiple HAL peripherals into domain-specific
/// abstractions for water treatment and solar monitoring.
///
/// Peripheral drivers:
///   - ads1263: 32-bit delta-sigma ADC for solar string monitoring
///   - ads1258: 24-bit multi-channel ADC for water sensor scanning
///   - w5500:   Ethernet controller for TCP/IP connectivity
///   - sp3485:  RS-485 transceiver for Modbus RTU
///   - sdcard:  SD card for local data logging
///
/// Domain drivers:
///   - water: Water treatment telemetry aggregation
///   - solar: Solar energy monitoring telemetry aggregation

pub mod water;
pub mod solar;

pub mod ads1263;
pub mod ads1258;
pub mod w5500;
pub mod sp3485;
pub mod sdcard;
