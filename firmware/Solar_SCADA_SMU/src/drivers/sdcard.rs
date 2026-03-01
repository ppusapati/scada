/// MicroSD Card — SPI Mode Data Logger
///
/// On SS-PCB-001:
///   SPI3_SCK  = PB3
///   SPI3_MISO = PB4
///   SPI3_MOSI = PB5
///   SD_CS     = PA15
///   SD_DETECT = PD2 (card detect switch, active low)
///
/// Uses embedded-sdmmc crate for FAT32 filesystem.
/// Logs solar performance data as CSV files:
///   /LOG/YYYYMMDD.CSV — daily log files
///   /CFG/CONFIG.BIN  — configuration backup
///
/// CSV format:
///   timestamp,string_id,voltage,current,power,irradiance,temp

use embassy_stm32::gpio::{Input, Output};
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Instant, Timer};
use defmt::{info, warn, error};
use heapless::String;

/// SD card state
#[derive(Clone, Copy, PartialEq, defmt::Format)]
pub enum SdState {
    NoCard,
    Initializing,
    Ready,
    Error,
    Full,
}

/// A log entry for one scan cycle
#[derive(Clone)]
pub struct LogEntry {
    pub timestamp_secs: u32,
    pub string_id: u8,
    pub voltage: f32,
    pub current: f32,
    pub power: f32,
    pub irradiance: f32,
    pub temperature: f32,
}

impl LogEntry {
    /// Format as CSV line
    pub fn to_csv(&self) -> String<128> {
        let mut s = String::new();
        // Simple integer formatting to avoid float formatting overhead
        let v_int = (self.voltage * 100.0) as i32;
        let i_int = (self.current * 1000.0) as i32;
        let p_int = (self.power * 10.0) as i32;
        let irr_int = (self.irradiance * 10.0) as i32;
        let t_int = (self.temperature * 10.0) as i32;

        // Format: timestamp,string,voltage*100,current*1000,power*10,irradiance*10,temp*10
        use core::fmt::Write;
        let _ = core::write!(
            s,
            "{},{},{},{},{},{},{}\n",
            self.timestamp_secs, self.string_id,
            v_int, i_int, p_int, irr_int, t_int
        );
        s
    }
}

/// Starting sector for log data (4 MB offset = sector 8192)
const LOG_START_SECTOR: u32 = 8192;

/// SD card logger (SPI mode)
pub struct SdLogger<'a> {
    spi: Spi<'a, Async>,
    cs: Output<'a>,
    detect: Input<'a>,
    state: SdState,
    initialized: bool,
    write_buf: [u8; 512],
    write_offset: usize,
    current_sector: u32,
}

impl<'a> SdLogger<'a> {
    pub fn new(
        spi: Spi<'a, Async>,
        cs: Output<'a>,
        detect: Input<'a>,
    ) -> Self {
        Self {
            spi,
            cs,
            detect,
            state: SdState::NoCard,
            initialized: false,
            write_buf: [0u8; 512],
            write_offset: 0,
            current_sector: LOG_START_SECTOR,
        }
    }

    /// Check if SD card is physically present
    pub fn card_present(&self) -> bool {
        self.detect.is_low() // Active low detect switch
    }

    pub fn state(&self) -> SdState {
        self.state
    }

    /// Initialize SD card in SPI mode
    pub async fn init(&mut self) -> bool {
        if !self.card_present() {
            self.state = SdState::NoCard;
            return false;
        }

        info!("SD: Initializing card...");
        self.state = SdState::Initializing;

        // SD card SPI initialization sequence:
        // 1. Send 80+ clock pulses with CS high (card enters SPI mode)
        self.cs.set_high();
        let dummy = [0xFF; 10]; // 80 clocks
        let _ = self.spi.write(&dummy).await;

        // 2. CMD0 (GO_IDLE_STATE) — reset card
        let resp = self.send_cmd(0, 0x00000000, 0x95).await;
        if resp != 0x01 {
            error!("SD: CMD0 failed (resp=0x{:02X})", resp);
            self.state = SdState::Error;
            return false;
        }

        // 3. CMD8 (SEND_IF_COND) — check voltage range (SDv2)
        let resp = self.send_cmd(8, 0x000001AA, 0x87).await;
        if resp != 0x01 {
            warn!("SD: Not SDv2 card");
        }
        // Read 4 response bytes
        let mut r7 = [0xFF; 4];
        self.cs.set_low();
        let _ = self.spi.read(&mut r7).await;
        self.cs.set_high();

        // 4. ACMD41 — initialize card (repeat until ready)
        for _ in 0..100 {
            // CMD55 (prefix for ACMD)
            self.send_cmd(55, 0, 0x65).await;
            // ACMD41 (SD_SEND_OP_COND) with HCS bit
            let resp = self.send_cmd(41, 0x40000000, 0x77).await;
            if resp == 0x00 {
                info!("SD: Card initialized");
                self.state = SdState::Ready;
                self.initialized = true;

                // Set block size to 512
                self.send_cmd(16, 512, 0xFF).await;

                return true;
            }
            Timer::after(Duration::from_millis(10)).await;
        }

        error!("SD: ACMD41 timeout");
        self.state = SdState::Error;
        false
    }

    /// Write a log entry to the SD card using raw sector writes.
    ///
    /// Uses a simple append-only log in raw sectors starting from sector 8192
    /// (4 MB offset, leaving room for a potential FAT32 partition table).
    /// Each 512-byte sector holds multiple CSV lines. When a sector fills up,
    /// we advance to the next sector.
    pub async fn write_log(&mut self, entry: &LogEntry) -> bool {
        if self.state != SdState::Ready {
            return false;
        }

        let csv = entry.to_csv();
        let data = csv.as_bytes();

        if data.is_empty() {
            return true;
        }

        // Check if current write buffer has room; if not, flush and advance sector
        if self.write_offset + data.len() > 512 {
            // Flush current sector
            if !self.flush_sector().await {
                warn!("SD: sector flush failed");
                self.state = SdState::Error;
                return false;
            }
            self.current_sector += 1;
            self.write_offset = 0;
            self.write_buf = [0u8; 512];
        }

        // Copy data into write buffer
        let end = self.write_offset + data.len();
        if end > 512 {
            return false; // Single entry too large for sector
        }
        self.write_buf[self.write_offset..end].copy_from_slice(data);
        self.write_offset = end;

        true
    }

    /// Flush the write buffer to the current sector on the SD card
    async fn flush_sector(&mut self) -> bool {
        if self.write_offset == 0 {
            return true;
        }

        // CMD24: WRITE_SINGLE_BLOCK
        // For SDHC/SDXC cards, the address is the sector number (not byte address)
        let sector = self.current_sector;
        let resp = self.send_cmd(24, sector, 0xFF).await;
        if resp != 0x00 {
            error!("SD: CMD24 failed (resp=0x{:02X})", resp);
            return false;
        }

        self.cs.set_low();

        // Send data token (0xFE)
        let _ = self.spi.write(&[0xFE]).await;

        // Send 512 bytes of data
        let _ = self.spi.write(&self.write_buf).await;

        // Send dummy CRC (2 bytes)
        let _ = self.spi.write(&[0xFF, 0xFF]).await;

        // Read data response token
        let mut resp_byte = [0xFF; 1];
        let _ = self.spi.read(&mut resp_byte).await;

        self.cs.set_high();
        let _ = self.spi.write(&[0xFF]).await;

        // Data response: 0bxxx00101 = accepted
        if resp_byte[0] & 0x1F != 0x05 {
            error!("SD: write rejected (resp=0x{:02X})", resp_byte[0]);
            return false;
        }

        // Wait for card to finish programming (busy = MISO low)
        self.cs.set_low();
        for _ in 0..1000 {
            let mut busy = [0u8; 1];
            let _ = self.spi.read(&mut busy).await;
            if busy[0] == 0xFF {
                break;
            }
            Timer::after(Duration::from_millis(1)).await;
        }
        self.cs.set_high();

        true
    }

    /// Send SD SPI command
    async fn send_cmd(&mut self, cmd: u8, arg: u32, crc: u8) -> u8 {
        let cmd_buf = [
            0x40 | cmd,           // Command index
            (arg >> 24) as u8,    // Argument byte 3
            (arg >> 16) as u8,    // Argument byte 2
            (arg >> 8) as u8,     // Argument byte 1
            (arg & 0xFF) as u8,   // Argument byte 0
            crc,                   // CRC7 + stop bit
        ];

        self.cs.set_low();
        let _ = self.spi.write(&cmd_buf).await;

        // Wait for response (R1 format)
        let mut response = 0xFF;
        for _ in 0..10 {
            let mut buf = [0xFF; 1];
            let _ = self.spi.read(&mut buf).await;
            if buf[0] != 0xFF {
                response = buf[0];
                break;
            }
        }

        self.cs.set_high();

        // Send 8 clock pulses (required between commands)
        let _ = self.spi.write(&[0xFF]).await;

        response
    }
}
