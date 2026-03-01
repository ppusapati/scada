/// Networking stack for STM32F407VGT6
///
/// Uses the on-chip Ethernet MAC (RMII) with embassy-net for TCP/IP
/// and rust-mqtt for MQTT v3.1.1 communication with the SCADA backend.
///
/// Network topology:
///   STM32 ETH MAC (RMII) → PHY (e.g. LAN8720) → Switch → MQTT Broker
///
/// DHCP is used by default; static IP fallback is configurable.

pub mod eth;
pub mod mqtt;
