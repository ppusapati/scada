//! High-level drivers for onboard communication modules
//!
//! Each driver wraps its SPI/UART peripheral and provides
//! a clean async API for the application layer.

pub mod w5500_ethernet;
pub mod sx1276_lora;
pub mod atwinc1500_wifi;
pub mod rn4870_ble;
pub mod sim7600_gsm;
pub mod rs485_modbus;
pub mod canfd;
