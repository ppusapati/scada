//! UART Port Configuration for STM32H743
//!
//! USART1 (Debug Console): PA9=TX, PA10=RX - 115200 baud (J12 header)
//! UART4 (GSM SIM7600G-H): PA0=TX, PC11=RX - 115200 baud default
//! UART7 (BLE RN4870):     PE8=TX, PE7=RX  - 115200 baud default
//! USART2 (RS485 ISO3082): PD5=TX, PD6=RX, PD4=DE/RE - 9600-115200 baud

use embassy_stm32::usart;
use embassy_stm32::time::Hertz;

/// Standard baud rates
pub struct BaudRates;

impl BaudRates {
    pub const DEBUG_CONSOLE: u32 = 115200;
    pub const GSM_DEFAULT: u32 = 115200;
    pub const BLE_DEFAULT: u32 = 115200;
    pub const RS485_DEFAULT: u32 = 9600;
    pub const RS485_FAST: u32 = 115200;
}

/// UART configuration for debug console (USART1, J12 header)
/// 115200 8N1, no flow control, TX only by default
pub fn uart_config_debug_console() -> usart::Config {
    let mut config = usart::Config::default();
    config.baudrate = BaudRates::DEBUG_CONSOLE;
    config
}

/// UART configuration for AT command modules (GSM, BLE)
pub fn uart_config_at_command(baud: u32) -> usart::Config {
    let mut config = usart::Config::default();
    config.baudrate = baud;
    config
}

/// UART configuration for RS485 half-duplex with DE/RE control
pub fn uart_config_rs485(baud: u32) -> usart::Config {
    let mut config = usart::Config::default();
    config.baudrate = baud;
    // STM32H743 USART2 supports hardware DE control
    config
}
