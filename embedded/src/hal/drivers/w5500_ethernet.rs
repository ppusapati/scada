//! W5500 Ethernet Controller Driver
//!
//! Provides TCP/UDP socket interface via SPI4.
//! W5500 has hardwired TCP/IP stack - no lwIP needed.
//!
//! Hardware: SPI4 (PE2=SCK, PE5=MISO, PE6=MOSI, PE4=CS)
//! Control:  PE3=RST_N, PB0=INT_N

use embassy_net_wiznet::chip::W5500;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::{Duration, Timer};
use heapless::String;

/// W5500 network configuration
#[derive(Clone)]
pub struct EthernetConfig {
    pub mac_address: [u8; 6],
    pub use_dhcp: bool,
    pub static_ip: Option<[u8; 4]>,
    pub subnet_mask: [u8; 4],
    pub gateway: [u8; 4],
    pub dns_server: [u8; 4],
}

impl Default for EthernetConfig {
    fn default() -> Self {
        Self {
            // MAC address derived from STM32 unique ID in production
            mac_address: [0x02, 0x00, 0x00, 0x5C, 0xAD, 0xA1],
            use_dhcp: true,
            static_ip: None,
            subnet_mask: [255, 255, 255, 0],
            gateway: [192, 168, 1, 1],
            dns_server: [8, 8, 8, 8],
        }
    }
}

/// W5500 connection state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EthernetState {
    Uninitialized,
    LinkDown,
    WaitingDhcp,
    Connected,
    Error,
}

/// W5500 driver wrapper
pub struct W5500Driver {
    state: EthernetState,
    config: EthernetConfig,
    ip_address: [u8; 4],
    link_up: bool,
}

impl W5500Driver {
    pub fn new(config: EthernetConfig) -> Self {
        Self {
            state: EthernetState::Uninitialized,
            config,
            ip_address: [0; 4],
            link_up: false,
        }
    }

    pub fn state(&self) -> EthernetState {
        self.state
    }

    pub fn ip_address(&self) -> [u8; 4] {
        self.ip_address
    }

    pub fn is_connected(&self) -> bool {
        self.state == EthernetState::Connected
    }

    /// Reset W5500 via RST_N pin
    pub async fn hardware_reset(rst_pin: &mut Output<'static>) {
        rst_pin.set_low();
        Timer::after(Duration::from_millis(2)).await;
        rst_pin.set_high();
        Timer::after(Duration::from_millis(50)).await; // Wait for W5500 PLL lock
    }
}
