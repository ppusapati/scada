//! SIM7600G-H GSM/LTE Module Driver
//!
//! AT command interface over UART4 for cellular data.
//! Supports LTE Cat-1 + 2G/3G fallback, GNSS, SMS.
//!
//! Hardware: UART4 (PA0=TX, PC11=RX)
//! Control:  PC3=PWR_KEY, PD11=STATUS

use embassy_stm32::gpio::{Input, Level, Output};
use embassy_time::{Duration, Timer};
use heapless::{String, Vec};

/// Cellular network registration state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NetworkState {
    Off,
    PoweringOn,
    SimReady,
    Searching,
    Registered,
    Roaming,
    DataConnected,
    Error,
}

/// GSM module configuration
#[derive(Clone)]
pub struct GsmConfig {
    pub apn: String<64>,
    pub apn_user: String<32>,
    pub apn_password: String<32>,
    /// Enable GNSS (GPS/GLONASS)
    pub enable_gnss: bool,
    /// MQTT broker for cellular link
    pub mqtt_broker: String<64>,
    pub mqtt_port: u16,
}

impl Default for GsmConfig {
    fn default() -> Self {
        Self {
            apn: String::new(),
            apn_user: String::new(),
            apn_password: String::new(),
            enable_gnss: true,
            mqtt_broker: String::new(),
            mqtt_port: 1883,
        }
    }
}

/// Signal quality info
#[derive(Clone, Debug, Default)]
pub struct SignalInfo {
    pub rssi: i8,
    pub ber: u8,
    pub operator: String<32>,
    pub network_type: String<8>,
}

/// GNSS position
#[derive(Clone, Debug, Default)]
pub struct GnssPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f32,
    pub speed_kmh: f32,
    pub satellites: u8,
    pub fix_valid: bool,
}

/// SIM7600 driver
pub struct Sim7600Driver {
    state: NetworkState,
    config: GsmConfig,
    signal: SignalInfo,
    position: GnssPosition,
    imei: String<20>,
    iccid: String<24>,
}

impl Sim7600Driver {
    pub fn new(config: GsmConfig) -> Self {
        Self {
            state: NetworkState::Off,
            config,
            signal: SignalInfo::default(),
            position: GnssPosition::default(),
            imei: String::new(),
            iccid: String::new(),
        }
    }

    pub fn state(&self) -> NetworkState {
        self.state
    }

    pub fn signal_info(&self) -> &SignalInfo {
        &self.signal
    }

    pub fn gnss_position(&self) -> &GnssPosition {
        &self.position
    }

    pub fn is_data_connected(&self) -> bool {
        self.state == NetworkState::DataConnected
    }

    /// Power on the SIM7600 module via PWR_KEY pin
    /// PWR_KEY must be held low for >500ms then released
    pub async fn power_on(pwr_key: &mut Output<'static>, status: &Input<'static>) {
        if status.is_high() {
            return; // Already powered on
        }
        pwr_key.set_low();
        Timer::after(Duration::from_millis(600)).await;
        pwr_key.set_high();
        // Wait for module to boot (up to 15 seconds)
        for _ in 0..30 {
            Timer::after(Duration::from_millis(500)).await;
            if status.is_high() {
                return;
            }
        }
    }

    /// Power off the module gracefully
    pub async fn power_off(pwr_key: &mut Output<'static>) {
        pwr_key.set_low();
        Timer::after(Duration::from_millis(3000)).await;
        pwr_key.set_high();
    }

    /// Check if module is powered on via STATUS pin
    pub fn is_powered_on(status: &Input<'static>) -> bool {
        status.is_high()
    }
}

/// Common AT commands for SIM7600G-H
pub mod at_commands {
    pub const AT: &str = "AT\r\n";
    pub const AT_OK: &str = "OK";
    pub const ATE0: &str = "ATE0\r\n"; // Disable echo
    pub const AT_CPIN: &str = "AT+CPIN?\r\n"; // SIM status
    pub const AT_CSQ: &str = "AT+CSQ\r\n"; // Signal quality
    pub const AT_CREG: &str = "AT+CREG?\r\n"; // Network registration
    pub const AT_CGATT: &str = "AT+CGATT?\r\n"; // GPRS attach status
    pub const AT_CGSN: &str = "AT+CGSN\r\n"; // IMEI
    pub const AT_CCID: &str = "AT+CCID\r\n"; // SIM ICCID
    pub const AT_COPS: &str = "AT+COPS?\r\n"; // Operator info
    pub const AT_CGPS_ON: &str = "AT+CGPS=1\r\n"; // Enable GNSS
    pub const AT_CGPSINFO: &str = "AT+CGPSINFO\r\n"; // Get GNSS position
    pub const AT_CMQTTSTART: &str = "AT+CMQTTSTART\r\n"; // Start MQTT
    pub const AT_CMQTTCONNECT: &str = "AT+CMQTTCONNECT="; // MQTT connect
    pub const AT_CMQTTPUB: &str = "AT+CMQTTPUB="; // MQTT publish
    pub const AT_CMQTTSUB: &str = "AT+CMQTTSUB="; // MQTT subscribe
}
