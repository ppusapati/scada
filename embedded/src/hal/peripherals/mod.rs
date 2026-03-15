//! Low-level peripheral initialization and access for STM32H743
//!
//! Configures all GPIO, SPI, UART, ADC, and timer peripherals
//! according to the PCB pin assignments.

pub mod adc_inputs;
pub mod digital_io;
pub mod spi_buses;
pub mod uart_ports;
pub mod watchdog;
