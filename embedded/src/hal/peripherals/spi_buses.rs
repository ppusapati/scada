//! SPI Bus Configuration for STM32H743
//!
//! SPI1 (LoRa SX1276):    PA5=SCK, PA6=MISO, PB5=MOSI, PA4=CS
//! SPI2 (WiFi ATWINC1500): PB13=SCK, PB14=MISO, PB15=MOSI, PB12=CS
//! SPI3 (Flash W25Q64JV):  PB3=SCK, PB4=MISO, PB2=MOSI, PA15=CS
//!   (PC10-PC12 freed for SDMMC1 4-bit SD card)
//! SPI4 (Ethernet W5500):  PE2=SCK, PE5=MISO, PE6=MOSI, PE4=CS
//!
//! SDMMC1 (microSD 4-bit): PC12=CLK, PD2=CMD, PC8=D0, PC9=D1, PC10=D2, PC11=D3

use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;

/// SPI clock speeds for each peripheral
pub struct SpiClocks;

impl SpiClocks {
    /// SX1276 LoRa: max 10MHz SPI clock
    pub const LORA: Hertz = Hertz(10_000_000);

    /// ATWINC1500 WiFi: max 48MHz, use 20MHz for reliability
    pub const WIFI: Hertz = Hertz(20_000_000);

    /// W25Q64JV Flash: max 133MHz, use 50MHz
    pub const FLASH: Hertz = Hertz(50_000_000);

    /// W5500 Ethernet: max 80MHz, use 33MHz for signal integrity
    pub const ETHERNET: Hertz = Hertz(33_000_000);
}

/// Common SPI configuration (Mode 0: CPOL=0, CPHA=0)
pub fn spi_config_mode0(freq: Hertz) -> spi::Config {
    let mut config = spi::Config::default();
    config.frequency = freq;
    config
}

/// SPI Mode 0 (CPOL=0, CPHA=0) - used by W5500, SX1276, W25Q64JV
pub fn spi_mode0() -> spi::Config {
    spi::Config::default()
}
