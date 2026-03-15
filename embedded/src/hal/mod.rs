//! Hardware Abstraction Layer for STM32H743 PLC Data Logger
//!
//! Provides driver abstractions for all onboard peripherals:
//! - SPI bus management (4 SPI buses)
//! - ADC analog input reading (4ch, 16-bit)
//! - Digital I/O (8 DI + 4 DO)
//! - UART management for GSM and BLE modules

pub mod drivers;
pub mod peripherals;

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::spi::Spi;
use embassy_stm32::usart::Uart;
use embassy_stm32::adc::Adc;

/// System-wide peripheral handles initialized at startup
pub struct BoardPeripherals {
    pub status_led: Output<'static>,
    pub error_led: Output<'static>,
}

impl BoardPeripherals {
    pub fn set_status(&mut self, on: bool) {
        if on {
            self.status_led.set_high();
        } else {
            self.status_led.set_low();
        }
    }

    pub fn set_error(&mut self, on: bool) {
        if on {
            self.error_led.set_high();
        } else {
            self.error_led.set_low();
        }
    }
}

/// Analog input channel identifier
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnalogChannel {
    Ch0 = 0,
    Ch1 = 1,
    Ch2 = 2,
    Ch3 = 3,
}

/// Analog input mode
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnalogMode {
    /// 4-20mA current loop (249 ohm shunt)
    Current4To20mA,
    /// 0-10V voltage input (resistive divider)
    Voltage0To10V,
}

/// Digital input state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DigitalInputState {
    Low = 0,
    High = 1,
}

/// Relay output command
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelayState {
    Open = 0,
    Closed = 1,
}

/// SPI bus identifier for the 4 SPI peripherals
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpiBus {
    /// SPI1 - SX1276 LoRa transceiver
    Spi1Lora,
    /// SPI2 - ATWINC1500 WiFi module
    Spi2Wifi,
    /// SPI3 - W25Q64JV external flash
    Spi3Flash,
    /// SPI4 - W5500 Ethernet controller
    Spi4Ethernet,
}

/// UART identifier
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UartBus {
    /// UART4 - SIM7600G-H GSM/LTE module
    Uart4Gsm,
    /// UART7 - RN4870 BLE module
    Uart7Ble,
    /// USART2 - RS485 Modbus (with DE/RE control)
    Usart2Rs485,
}
