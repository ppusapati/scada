/// SPI HAL abstraction for STM32F407VGT6
///
/// Three SPI peripherals are used:
///   SPI1: External ADC (ADS1263 or ADS1258) -- high-precision sensor data
///     PA5 = SCK, PA6 = MISO, PA7 = MOSI
///     PA4 = CS_ADC (directly managed GPIO)
///     Clock: up to 8 MHz for ADS1263, 16 MHz for ADS1258
///
///   SPI2: W5500 Ethernet controller -- network connectivity
///     PB13 = SCK, PB14 = MISO, PB15 = MOSI
///     PB12 = CS_W5500
///     Clock: up to 33 MHz (W5500 max = 80 MHz)
///
///   SPI3: SD card (SPI mode) -- data logging
///     PB3 = SCK, PB4 = MISO, PB5 = MOSI
///     PA15 = CS_SD
///     Clock: 400 kHz for init, then up to 25 MHz

use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals;
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::mode::Async;
use embassy_stm32::time::Hertz;
use defmt::{info, warn, error, Format};

// ============================================================================
// Error types
// ============================================================================

/// SPI subsystem errors
#[derive(Clone, Copy, Debug, Format)]
pub enum SpiError {
    /// SPI peripheral error (overrun, mode fault, CRC error)
    BusError,
    /// Chip select assertion failed
    CsError,
    /// DMA transfer error
    DmaError,
    /// Transfer timeout
    Timeout,
    /// Invalid configuration parameter
    InvalidConfig,
    /// Buffer too small for requested operation
    BufferOverflow,
}

// ============================================================================
// SPI clock configuration
// ============================================================================

/// SPI clock speed presets for each peripheral
#[derive(Clone, Copy, PartialEq, Format)]
pub enum SpiClock {
    /// 400 kHz - SD card initialization
    Slow400k,
    /// 1 MHz - conservative ADC speed
    Adc1M,
    /// 4 MHz - standard ADC speed
    Adc4M,
    /// 8 MHz - ADS1263 maximum
    Adc8M,
    /// 16 MHz - ADS1258 maximum
    Adc16M,
    /// 25 MHz - SD card normal operation
    Sd25M,
    /// 33 MHz - W5500 Ethernet
    Eth33M,
}

impl SpiClock {
    /// Get the clock frequency in Hz
    pub fn to_hertz(self) -> Hertz {
        match self {
            SpiClock::Slow400k => Hertz(400_000),
            SpiClock::Adc1M => Hertz(1_000_000),
            SpiClock::Adc4M => Hertz(4_000_000),
            SpiClock::Adc8M => Hertz(8_000_000),
            SpiClock::Adc16M => Hertz(16_000_000),
            SpiClock::Sd25M => Hertz(25_000_000),
            SpiClock::Eth33M => Hertz(33_000_000),
        }
    }
}

/// SPI mode (CPOL/CPHA combination)
#[derive(Clone, Copy, PartialEq, Format)]
pub enum SpiMode {
    /// Mode 0: CPOL=0, CPHA=0 (clock idle low, sample on rising edge)
    Mode0,
    /// Mode 1: CPOL=0, CPHA=1 (clock idle low, sample on falling edge)
    Mode1,
    /// Mode 2: CPOL=1, CPHA=0 (clock idle high, sample on falling edge)
    Mode2,
    /// Mode 3: CPOL=1, CPHA=1 (clock idle high, sample on rising edge)
    Mode3,
}

impl SpiMode {
    /// Convert to embassy-stm32 SPI polarity
    pub fn polarity(self) -> spi::Polarity {
        match self {
            SpiMode::Mode0 | SpiMode::Mode1 => spi::Polarity::IdleLow,
            SpiMode::Mode2 | SpiMode::Mode3 => spi::Polarity::IdleHigh,
        }
    }

    /// Convert to embassy-stm32 SPI phase
    pub fn phase(self) -> spi::Phase {
        match self {
            SpiMode::Mode0 | SpiMode::Mode2 => spi::Phase::CaptureOnFirstTransition,
            SpiMode::Mode1 | SpiMode::Mode3 => spi::Phase::CaptureOnSecondTransition,
        }
    }
}

// ============================================================================
// Chip select management
// ============================================================================

/// Which SPI device is selected
#[derive(Clone, Copy, PartialEq, Format)]
pub enum SpiDevice {
    /// External ADC on SPI1
    Adc,
    /// W5500 Ethernet on SPI2
    Ethernet,
    /// SD card on SPI3
    SdCard,
    /// No device selected
    None,
}

/// Chip select pin manager for a single SPI device
///
/// Manages the active-low chip select line. The CS pin is deasserted (high)
/// on creation, and can be asserted/deasserted for transactions.
pub struct ChipSelect {
    pin: Output<'static>,
    device: SpiDevice,
}

impl ChipSelect {
    /// Assert chip select (drive low)
    pub fn assert(&mut self) {
        self.pin.set_low();
    }

    /// Deassert chip select (drive high)
    pub fn deassert(&mut self) {
        self.pin.set_high();
    }

    /// Check if CS is currently asserted
    pub fn is_asserted(&self) -> bool {
        self.pin.is_set_low()
    }

    /// Get the device type this CS manages
    pub fn device(&self) -> SpiDevice {
        self.device
    }
}

// ============================================================================
// SPI1: External ADC (ADS1263 / ADS1258)
// ============================================================================

/// SPI1 peripheral bundle for ADC communication
///
/// ADS1263 uses SPI Mode 1 (CPOL=0, CPHA=1) at up to 8 MHz
/// ADS1258 uses SPI Mode 1 (CPOL=0, CPHA=1) at up to 16 MHz
pub struct Spi1Adc {
    pub spi: Spi<'static, Async>,
    pub cs: ChipSelect,
}

impl Spi1Adc {
    /// Create and configure SPI1 for ADC communication
    pub fn new(
        spi_peri: peripherals::SPI1,
        sck: peripherals::PA5,
        miso: peripherals::PA6,
        mosi: peripherals::PA7,
        cs_pin: peripherals::PA4,
        tx_dma: peripherals::DMA2_CH3,
        rx_dma: peripherals::DMA2_CH0,
        clock: SpiClock,
    ) -> Self {
        let mut config = spi::Config::default();
        config.frequency = clock.to_hertz();
        config.mode = spi::MODE_1; // CPOL=0, CPHA=1 (ADS126x default)
        config.bit_order = spi::BitOrder::MsbFirst;

        let spi = Spi::new(spi_peri, sck, mosi, miso, tx_dma, rx_dma, config);
        let cs = ChipSelect {
            pin: Output::new(cs_pin, Level::High, Speed::VeryHigh),
            device: SpiDevice::Adc,
        };

        info!("SPI1 (ADC): initialized at {} Hz", clock.to_hertz().0);

        Self { spi, cs }
    }

    /// Perform a full-duplex SPI transfer with CS management
    pub async fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), SpiError> {
        self.cs.assert();
        let result = self.spi.transfer(rx, tx).await;
        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }

    /// Write-only SPI transfer with CS management
    pub async fn write(&mut self, data: &[u8]) -> Result<(), SpiError> {
        self.cs.assert();
        let result = self.spi.write(data).await;
        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }

    /// Read-only SPI transfer (sends zeros, reads response)
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<(), SpiError> {
        self.cs.assert();
        let result = self.spi.read(buf).await;
        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }

    /// Perform transfer with CS left asserted (for multi-part transactions)
    pub async fn transfer_keep_cs(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), SpiError> {
        self.cs.assert();
        self.spi.transfer(rx, tx).await.map_err(|_| SpiError::BusError)
    }

    /// Finish a multi-part transaction by deasserting CS
    pub fn finish_transaction(&mut self) {
        self.cs.deassert();
    }
}

// ============================================================================
// SPI2: W5500 Ethernet Controller
// ============================================================================

/// SPI2 peripheral bundle for W5500 Ethernet communication
///
/// W5500 uses SPI Mode 0 (CPOL=0, CPHA=0) at up to 80 MHz.
/// We run at 33 MHz due to STM32F407 SPI2 max prescaler limits.
pub struct Spi2Eth {
    pub spi: Spi<'static, Async>,
    pub cs: ChipSelect,
}

impl Spi2Eth {
    /// Create and configure SPI2 for W5500 communication
    pub fn new(
        spi_peri: peripherals::SPI2,
        sck: peripherals::PB13,
        miso: peripherals::PB14,
        mosi: peripherals::PB15,
        cs_pin: peripherals::PB12,
        tx_dma: peripherals::DMA1_CH4,
        rx_dma: peripherals::DMA1_CH3,
        clock: SpiClock,
    ) -> Self {
        let mut config = spi::Config::default();
        config.frequency = clock.to_hertz();
        config.mode = spi::MODE_0; // CPOL=0, CPHA=0 (W5500)
        config.bit_order = spi::BitOrder::MsbFirst;

        let spi = Spi::new(spi_peri, sck, mosi, miso, tx_dma, rx_dma, config);
        let cs = ChipSelect {
            pin: Output::new(cs_pin, Level::High, Speed::VeryHigh),
            device: SpiDevice::Ethernet,
        };

        info!("SPI2 (W5500): initialized at {} Hz", clock.to_hertz().0);

        Self { spi, cs }
    }

    /// Perform a full-duplex SPI transfer with CS management
    pub async fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), SpiError> {
        self.cs.assert();
        let result = self.spi.transfer(rx, tx).await;
        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }

    /// Write-only SPI transfer
    pub async fn write(&mut self, data: &[u8]) -> Result<(), SpiError> {
        self.cs.assert();
        let result = self.spi.write(data).await;
        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }

    /// Read-only SPI transfer
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<(), SpiError> {
        self.cs.assert();
        let result = self.spi.read(buf).await;
        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }

    /// W5500 variable-length data phase: write address+control then transfer data
    /// The W5500 protocol is: [addr_hi, addr_lo, control_byte, data...]
    pub async fn w5500_transfer(
        &mut self,
        addr: u16,
        control: u8,
        data_tx: &[u8],
        data_rx: &mut [u8],
    ) -> Result<(), SpiError> {
        let header = [
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            control,
        ];

        self.cs.assert();

        // Send header
        if self.spi.write(&header).await.is_err() {
            self.cs.deassert();
            return Err(SpiError::BusError);
        }

        // Transfer data phase
        let result = if !data_tx.is_empty() {
            self.spi.write(data_tx).await
        } else if !data_rx.is_empty() {
            self.spi.read(data_rx).await
        } else {
            Ok(())
        };

        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }
}

// ============================================================================
// SPI3: SD Card (SPI mode)
// ============================================================================

/// SPI3 peripheral bundle for SD card communication
///
/// SD cards in SPI mode use Mode 0 (CPOL=0, CPHA=0).
/// Initialization requires 400 kHz clock, then speed up to 25 MHz.
pub struct Spi3Sd {
    pub spi: Spi<'static, Async>,
    pub cs: ChipSelect,
}

impl Spi3Sd {
    /// Create and configure SPI3 for SD card communication
    ///
    /// Starts at slow clock (400 kHz) for initialization.
    pub fn new(
        spi_peri: peripherals::SPI3,
        sck: peripherals::PB3,
        miso: peripherals::PB4,
        mosi: peripherals::PB5,
        cs_pin: peripherals::PA15,
        tx_dma: peripherals::DMA1_CH5,
        rx_dma: peripherals::DMA1_CH0,
        clock: SpiClock,
    ) -> Self {
        let mut config = spi::Config::default();
        config.frequency = clock.to_hertz();
        config.mode = spi::MODE_0; // CPOL=0, CPHA=0 (SD card)
        config.bit_order = spi::BitOrder::MsbFirst;

        let spi = Spi::new(spi_peri, sck, mosi, miso, tx_dma, rx_dma, config);
        let cs = ChipSelect {
            pin: Output::new(cs_pin, Level::High, Speed::VeryHigh),
            device: SpiDevice::SdCard,
        };

        info!("SPI3 (SD): initialized at {} Hz", clock.to_hertz().0);

        Self { spi, cs }
    }

    /// Perform a full-duplex SPI transfer with CS management
    pub async fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), SpiError> {
        self.cs.assert();
        let result = self.spi.transfer(rx, tx).await;
        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }

    /// Write-only SPI transfer
    pub async fn write(&mut self, data: &[u8]) -> Result<(), SpiError> {
        self.cs.assert();
        let result = self.spi.write(data).await;
        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }

    /// Read-only SPI transfer
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<(), SpiError> {
        self.cs.assert();
        let result = self.spi.read(buf).await;
        self.cs.deassert();
        result.map_err(|_| SpiError::BusError)
    }

    /// Send clock pulses with CS deasserted (needed for SD card init)
    pub async fn send_clocks(&mut self, count: usize) -> Result<(), SpiError> {
        self.cs.deassert();
        let dummy = [0xFF; 1];
        for _ in 0..count {
            self.spi.write(&dummy).await.map_err(|_| SpiError::BusError)?;
        }
        Ok(())
    }

    /// Send an SD command and receive R1 response
    /// SD commands are 6 bytes: [0x40|cmd, arg[3:0], crc]
    pub async fn sd_command(&mut self, cmd: u8, arg: u32, crc: u8) -> Result<u8, SpiError> {
        let cmd_buf = [
            0x40 | cmd,
            (arg >> 24) as u8,
            (arg >> 16) as u8,
            (arg >> 8) as u8,
            arg as u8,
            crc,
        ];

        self.cs.assert();
        self.spi.write(&cmd_buf).await.map_err(|_| SpiError::BusError)?;

        // Wait for response (R1): poll up to 8 bytes for non-0xFF
        let mut response: u8 = 0xFF;
        for _ in 0..8 {
            let mut rx = [0xFF; 1];
            self.spi.read(&mut rx).await.map_err(|_| SpiError::BusError)?;
            if rx[0] != 0xFF {
                response = rx[0];
                break;
            }
        }

        self.cs.deassert();

        // Send one more byte to complete the transaction
        self.spi.write(&[0xFF]).await.map_err(|_| SpiError::BusError)?;

        Ok(response)
    }
}
