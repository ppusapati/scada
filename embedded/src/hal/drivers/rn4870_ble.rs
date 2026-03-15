//! RN4870 BLE 5.0 Module Driver
//!
//! Microchip BLE module with ASCII command interface over UART.
//! Built-in antenna, no external RF components needed.
//!
//! Hardware: UART7 (PE8=TX, PE7=RX)
//! Control:  PD12=RST_N, PD13=STATUS (connection indicator)

use embassy_stm32::gpio::{Input, Level, Output, Pull};
use embassy_time::{Duration, Timer};
use heapless::{String, Vec};

/// BLE advertising mode
#[derive(Clone, Copy, Debug)]
pub enum BleAdvMode {
    /// Connectable, scannable undirected
    ConnectableUndirected,
    /// Non-connectable, non-scannable (beacon mode)
    NonConnectable,
}

/// BLE connection state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BleState {
    Off,
    Initializing,
    Advertising,
    Connected,
    Disconnected,
    Error,
}

/// RN4870 configuration
#[derive(Clone)]
pub struct BleConfig {
    pub device_name: String<20>,
    pub adv_interval_ms: u16,
    pub adv_mode: BleAdvMode,
    /// Enable UART transparent service (data pipe)
    pub transparent_uart: bool,
    /// TX power level (0-5, where 5 is max)
    pub tx_power: u8,
}

impl Default for BleConfig {
    fn default() -> Self {
        Self {
            device_name: String::try_from("SCADA-DL").unwrap_or_default(),
            adv_interval_ms: 200,
            adv_mode: BleAdvMode::ConnectableUndirected,
            transparent_uart: true,
            tx_power: 5,
        }
    }
}

/// RN4870 AT command responses
const CMD_ENTER: &[u8] = b"$$$";
const CMD_EXIT: &[u8] = b"---\r\n";
const CMD_REBOOT: &[u8] = b"R,1\r\n";
const CMD_GET_NAME: &[u8] = b"GN\r\n";
const CMD_SET_NAME: &[u8] = b"SN,";
const RESPONSE_OK: &[u8] = b"AOK";
const RESPONSE_ERR: &[u8] = b"ERR";

/// RN4870 BLE driver
pub struct Rn4870Driver {
    state: BleState,
    config: BleConfig,
    connected_device: String<20>,
    rx_buffer: Vec<u8, 256>,
}

impl Rn4870Driver {
    pub fn new(config: BleConfig) -> Self {
        Self {
            state: BleState::Off,
            config,
            connected_device: String::new(),
            rx_buffer: Vec::new(),
        }
    }

    pub fn state(&self) -> BleState {
        self.state
    }

    pub fn is_connected(&self) -> bool {
        self.state == BleState::Connected
    }

    /// Check connection status via STATUS pin (PD13)
    pub fn check_status_pin(status_pin: &Input<'static>) -> bool {
        status_pin.is_high()
    }

    /// Hardware reset
    pub async fn hardware_reset(rst_pin: &mut Output<'static>) {
        rst_pin.set_low();
        Timer::after(Duration::from_millis(5)).await;
        rst_pin.set_high();
        Timer::after(Duration::from_millis(300)).await; // RN4870 boot time
    }

    /// Format a BLE characteristic notification payload
    /// For SCADA: sends sensor data as compact binary to connected BLE central
    pub fn format_sensor_payload(
        analog: &[f32; 4],
        digital_in: u8,
        digital_out: u8,
    ) -> Vec<u8, 32> {
        let mut payload = Vec::new();
        // Header byte
        let _ = payload.push(0xDA); // SCADA data marker
        // 4x analog values as 16-bit fixed point (value * 100)
        for &val in analog.iter() {
            let fixed = (val * 100.0) as i16;
            let _ = payload.extend_from_slice(&fixed.to_le_bytes());
        }
        // Digital I/O packed
        let _ = payload.push(digital_in);
        let _ = payload.push(digital_out);
        payload
    }
}
