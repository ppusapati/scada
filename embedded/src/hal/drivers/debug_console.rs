//! UART Debug Console Driver
//!
//! Provides diagnostic output via USART1 on the J12 header.
//! Connected to PA9(TX) and PA10(RX) at 115200 baud 8N1.
//! Compatible with standard FTDI USB-UART cables (3.3V level).
//!
//! Features:
//! - System boot messages and firmware version
//! - Periodic status reports (sensor values, alarm states)
//! - SD card and flash storage diagnostics
//! - Communication link status
//! - Simple command interface for field diagnostics

use heapless::String;

/// Debug console message categories
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Error => "[ERR]",
            Self::Warn  => "[WRN]",
            Self::Info  => "[INF]",
            Self::Debug => "[DBG]",
            Self::Trace => "[TRC]",
        }
    }
}

/// Debug console configuration
pub struct DebugConsoleConfig {
    /// Minimum log level to output
    pub min_level: LogLevel,
    /// Include timestamp prefix
    pub timestamps: bool,
    /// Periodic status report interval in seconds (0 = disabled)
    pub status_interval_secs: u16,
}

impl Default for DebugConsoleConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            timestamps: true,
            status_interval_secs: 30,
        }
    }
}

/// Debug console driver state
pub struct DebugConsole {
    config: DebugConsoleConfig,
    tx_count: u32,
    boot_timestamp_ms: u64,
}

impl DebugConsole {
    pub fn new(config: DebugConsoleConfig) -> Self {
        Self {
            config,
            tx_count: 0,
            boot_timestamp_ms: 0,
        }
    }

    /// Format a log message for UART transmission
    /// Returns the formatted bytes to send
    pub fn format_message(&mut self, level: LogLevel, timestamp_ms: u64, msg: &str) -> String<256> {
        let mut output = String::new();

        if self.should_log(level) {
            if self.config.timestamps {
                let secs = timestamp_ms / 1000;
                let ms = timestamp_ms % 1000;
                let _ = core::fmt::write(
                    &mut output,
                    format_args!("[{:6}.{:03}] {} {}\r\n", secs, ms, level.prefix(), msg),
                );
            } else {
                let _ = core::fmt::write(
                    &mut output,
                    format_args!("{} {}\r\n", level.prefix(), msg),
                );
            }
            self.tx_count += 1;
        }

        output
    }

    /// Format the boot banner
    pub fn boot_banner() -> &'static str {
        "\r\n\
         ========================================\r\n\
         SCADA DataLogger STM32H743 v0.2.0\r\n\
         ARM Cortex-M7 @ 480MHz\r\n\
         ========================================\r\n\
         \r\n\
         Programming interfaces:\r\n\
           J10: SWD 10-pin (Samtec 1.27mm)\r\n\
           J11: USB-C (DFU mode)\r\n\
           J12: UART Console (this port)\r\n\
           J13: BOOT0 jumper (GND=flash, 3V3=DFU)\r\n\
           J14: Tag-Connect TC2050 (production)\r\n\
         \r\n"
    }

    /// Format a periodic system status report
    pub fn format_status_report(
        &mut self,
        timestamp_ms: u64,
        analog: &[f32; 4],
        digital_in: u8,
        digital_out: u8,
        mcu_temp: f32,
        supply_v: f32,
        sd_active: bool,
        sd_records: u64,
    ) -> String<256> {
        let mut output = String::new();
        let secs = timestamp_ms / 1000;
        let _ = core::fmt::write(
            &mut output,
            format_args!(
                "[{:6}s] STATUS: AI=[{:.2},{:.2},{:.2},{:.2}] DI={:#04X} DO={:#04X} T={:.1}C V={:.2}V SD={} rec={}\r\n",
                secs,
                analog[0], analog[1], analog[2], analog[3],
                digital_in, digital_out,
                mcu_temp, supply_v,
                if sd_active { "OK" } else { "--" },
                sd_records,
            ),
        );
        output
    }

    /// Check if a message at this level should be logged
    fn should_log(&self, level: LogLevel) -> bool {
        (level as u8) <= (self.config.min_level as u8)
    }

    pub fn tx_count(&self) -> u32 {
        self.tx_count
    }

    pub fn status_interval_secs(&self) -> u16 {
        self.config.status_interval_secs
    }
}

/// Simple console command parser for field diagnostics
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConsoleCommand {
    /// Print system status
    Status,
    /// Print SD card info
    SdInfo,
    /// Print alarm thresholds
    Alarms,
    /// Print communication link status
    Links,
    /// Print firmware version
    Version,
    /// Reset the MCU
    Reset,
    /// Print help text
    Help,
    /// Unknown command
    Unknown,
}

impl ConsoleCommand {
    /// Parse a received line into a command
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        match trimmed {
            "status" | "s" => Self::Status,
            "sdinfo" | "sd" => Self::SdInfo,
            "alarms" | "a" => Self::Alarms,
            "links" | "l" => Self::Links,
            "version" | "v" => Self::Version,
            "reset" => Self::Reset,
            "help" | "h" | "?" => Self::Help,
            _ => Self::Unknown,
        }
    }

    /// Help text for all commands
    pub fn help_text() -> &'static str {
        "Commands:\r\n\
         \x20 status (s)  - System status\r\n\
         \x20 sdinfo (sd) - SD card info\r\n\
         \x20 alarms (a)  - Alarm thresholds\r\n\
         \x20 links  (l)  - Comm link status\r\n\
         \x20 version (v) - Firmware version\r\n\
         \x20 reset       - Reset MCU\r\n\
         \x20 help (h/?)  - This help\r\n"
    }
}
