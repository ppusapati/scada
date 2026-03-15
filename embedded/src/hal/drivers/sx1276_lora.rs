//! SX1276 LoRa Transceiver Driver
//!
//! Provides LoRa/LoRaWAN communication via SPI1.
//! Bare IC with 32MHz TCXO and pi-matching network.
//!
//! Hardware: SPI1 (PA5=SCK, PA6=MISO, PB5=MOSI, PA4=CS)
//! Control:  PC13=DIO0 (RX/TX done IRQ), PC2=RST

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_time::{Duration, Timer};
use heapless::Vec;

/// LoRa frequency bands
#[derive(Clone, Copy, Debug)]
pub enum LoRaFrequency {
    /// EU868 band (863-870 MHz)
    Eu868,
    /// US915 band (902-928 MHz)
    Us915,
    /// Custom frequency in Hz
    Custom(u32),
}

impl LoRaFrequency {
    pub fn to_hz(self) -> u32 {
        match self {
            Self::Eu868 => 868_100_000,
            Self::Us915 => 915_000_000,
            Self::Custom(f) => f,
        }
    }
}

/// LoRa spreading factor (SF7-SF12)
#[derive(Clone, Copy, Debug)]
pub enum SpreadingFactor {
    SF7 = 7,
    SF8 = 8,
    SF9 = 9,
    SF10 = 10,
    SF11 = 11,
    SF12 = 12,
}

/// LoRa bandwidth
#[derive(Clone, Copy, Debug)]
pub enum Bandwidth {
    Bw125kHz = 7,
    Bw250kHz = 8,
    Bw500kHz = 9,
}

/// LoRa coding rate
#[derive(Clone, Copy, Debug)]
pub enum CodingRate {
    Cr45 = 1,
    Cr46 = 2,
    Cr47 = 3,
    Cr48 = 4,
}

/// LoRa radio configuration
#[derive(Clone)]
pub struct LoRaConfig {
    pub frequency: LoRaFrequency,
    pub spreading_factor: SpreadingFactor,
    pub bandwidth: Bandwidth,
    pub coding_rate: CodingRate,
    pub tx_power_dbm: i8,
    pub sync_word: u8,
    pub preamble_length: u16,
    pub enable_crc: bool,
}

impl Default for LoRaConfig {
    fn default() -> Self {
        Self {
            frequency: LoRaFrequency::Us915,
            spreading_factor: SpreadingFactor::SF7,
            bandwidth: Bandwidth::Bw125kHz,
            coding_rate: CodingRate::Cr45,
            tx_power_dbm: 14,
            sync_word: 0x34, // LoRaWAN public network
            preamble_length: 8,
            enable_crc: true,
        }
    }
}

/// SX1276 operating mode
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RadioState {
    Sleep,
    Standby,
    Transmitting,
    Receiving,
    CadDetect,
}

/// SX1276 LoRa driver
pub struct SX1276Driver {
    state: RadioState,
    config: LoRaConfig,
    rssi: i16,
    snr: i8,
}

impl SX1276Driver {
    pub fn new(config: LoRaConfig) -> Self {
        Self {
            state: RadioState::Sleep,
            config,
            rssi: 0,
            snr: 0,
        }
    }

    pub fn state(&self) -> RadioState {
        self.state
    }

    pub fn last_rssi(&self) -> i16 {
        self.rssi
    }

    pub fn last_snr(&self) -> i8 {
        self.snr
    }

    /// Reset SX1276 via RST pin
    pub async fn hardware_reset(rst_pin: &mut Output<'static>) {
        rst_pin.set_low();
        Timer::after(Duration::from_millis(1)).await;
        rst_pin.set_high();
        Timer::after(Duration::from_millis(10)).await;
    }

    /// Calculate time on air for a given payload size (milliseconds)
    pub fn time_on_air_ms(&self, payload_len: u8) -> u32 {
        let sf = self.config.spreading_factor as u32;
        let bw = match self.config.bandwidth {
            Bandwidth::Bw125kHz => 125000u32,
            Bandwidth::Bw250kHz => 250000,
            Bandwidth::Bw500kHz => 500000,
        };
        let cr = self.config.coding_rate as u32;

        // Symbol duration
        let t_sym = (1u32 << sf) * 1000 / bw;

        // Preamble duration
        let t_preamble = (self.config.preamble_length as u32 + 4) * t_sym + t_sym / 4;

        // Payload symbols (simplified calculation)
        let payload_symbols = 8u32.max(
            ((8 * payload_len as u32 - 4 * sf + 28 + 16) as f32
                / (4 * (sf - 2)) as f32)
                .ceil() as u32
                * (cr + 4),
        );

        t_preamble + payload_symbols * t_sym
    }
}

/// Maximum LoRa payload size
pub const LORA_MAX_PAYLOAD: usize = 255;

/// LoRa received packet
pub struct LoRaPacket {
    pub data: Vec<u8, 256>,
    pub rssi: i16,
    pub snr: i8,
    pub frequency_error: i32,
}
