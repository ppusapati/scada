//! ATWINC1500 WiFi Module Driver
//!
//! Microchip WiFi 802.11 b/g/n with built-in TCP/IP + TLS 1.2 stack.
//! SPI slave interface - no second MCU needed.
//!
//! Hardware: SPI2 (PB13=SCK, PB14=MISO, PB15=MOSI, PB12=CS)
//! Control:  PB1=CHIP_EN, PB2=RST_N, PD10=IRQ_N

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_time::{Duration, Timer};
use heapless::String;

/// WiFi security mode
#[derive(Clone, Copy, Debug)]
pub enum WifiSecurity {
    Open,
    Wpa2Personal,
    WpaWpa2Personal,
    Wpa2Enterprise,
}

/// WiFi connection configuration
#[derive(Clone)]
pub struct WifiConfig {
    pub ssid: String<32>,
    pub password: String<64>,
    pub security: WifiSecurity,
    pub use_dhcp: bool,
    pub static_ip: Option<[u8; 4]>,
    pub auto_reconnect: bool,
    pub power_save: bool,
}

impl Default for WifiConfig {
    fn default() -> Self {
        Self {
            ssid: String::new(),
            password: String::new(),
            security: WifiSecurity::Wpa2Personal,
            use_dhcp: true,
            static_ip: None,
            auto_reconnect: true,
            power_save: false,
        }
    }
}

/// WiFi connection state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WifiState {
    Off,
    Initializing,
    Scanning,
    Connecting,
    Connected,
    Disconnected,
    Error,
}

/// ATWINC1500 driver
pub struct Atwinc1500Driver {
    state: WifiState,
    config: WifiConfig,
    ip_address: [u8; 4],
    rssi: i8,
    firmware_version: String<16>,
}

impl Atwinc1500Driver {
    pub fn new(config: WifiConfig) -> Self {
        Self {
            state: WifiState::Off,
            config,
            ip_address: [0; 4],
            rssi: 0,
            firmware_version: String::new(),
        }
    }

    pub fn state(&self) -> WifiState {
        self.state
    }

    pub fn is_connected(&self) -> bool {
        self.state == WifiState::Connected
    }

    pub fn rssi(&self) -> i8 {
        self.rssi
    }

    pub fn ip_address(&self) -> [u8; 4] {
        self.ip_address
    }

    /// Hardware reset via RST_N pin
    pub async fn hardware_reset(rst_pin: &mut Output<'static>) {
        rst_pin.set_low();
        Timer::after(Duration::from_millis(10)).await;
        rst_pin.set_high();
        Timer::after(Duration::from_millis(500)).await; // ATWINC1500 boot time
    }

    /// Enable/disable chip via CHIP_EN pin
    pub fn set_chip_enable(en_pin: &mut Output<'static>, enable: bool) {
        if enable {
            en_pin.set_high();
        } else {
            en_pin.set_low();
        }
    }
}
