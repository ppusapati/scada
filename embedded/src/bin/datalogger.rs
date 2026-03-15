//! STM32H743 PLC-Grade Data Logger - Main Firmware
//!
//! Embassy async runtime with concurrent tasks for:
//! - Data acquisition (ADC sampling, digital I/O)
//! - Data logging (flash ring buffer, SD card)
//! - Communication (Ethernet/WiFi/GSM/LoRa/BLE)
//! - Fieldbus polling (Modbus RTU, CAN FD)
//! - System health monitoring (watchdog, temperature)

#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::usart::{self, Uart};
use embassy_stm32::Config;
use embassy_time::{Duration, Ticker, Timer};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

use scada_embedded::hal::peripherals::watchdog;
use scada_embedded::datalogger::{DataLogger, LoggerConfig, SampleRate, DataRecord};
use scada_embedded::comm::CommManager;
use scada_embedded::storage::StorageHealth;
use scada_embedded::storage::microsd::{MicroSdDriver, SdLogConfig, SdCardInfo, SdCardType, SdState};
use scada_embedded::storage::sd_logger::{SdLogger, SdLoggerMode};
use scada_embedded::hal::drivers::debug_console::{DebugConsole, DebugConsoleConfig, LogLevel};
use scada_embedded::security::flash_protection::{FlashProtection, ProvisioningManager, ProvisioningState};
use scada_embedded::ota::firmware_update::{FirmwareUpdater, OtaState, OtaSource};

/// Shared channel for passing data records from acquisition to logging task
static DATA_CHANNEL: Channel<CriticalSectionRawMutex, DataRecord, 8> = Channel::new();

/// System clock configuration for STM32H743 (480 MHz)
fn system_config() -> Config {
    let mut config = Config::default();
    // HSE 25MHz -> PLL1 -> 480MHz SYSCLK
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: embassy_stm32::time::Hertz(25_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV5,   // 25MHz / 5 = 5MHz
            mul: PllMul::MUL192,       // 5MHz * 192 = 960MHz VCO
            divp: Some(PllDiv::DIV2),  // 960MHz / 2 = 480MHz SYSCLK
            divq: Some(PllDiv::DIV4),  // 240MHz for SPI clocks
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV2;  // 240MHz AHB
        config.rcc.apb1_pre = APBPrescaler::DIV2;  // 120MHz APB1
        config.rcc.apb2_pre = APBPrescaler::DIV2;  // 120MHz APB2
        config.rcc.apb3_pre = APBPrescaler::DIV2;  // 120MHz APB3
        config.rcc.apb4_pre = APBPrescaler::DIV2;  // 120MHz APB4
    }
    config
}

// ============================================================
// Embassy Tasks
// ============================================================

/// Watchdog kick task - highest priority, must never block
#[embassy_executor::task]
async fn watchdog_task(mut wdi_pin: Output<'static>) {
    info!("Watchdog task started (TPS3823-33, 1.6s timeout)");
    let mut ticker = Ticker::every(Duration::from_millis(500));
    loop {
        watchdog::kick_watchdog(&mut wdi_pin);
        ticker.next().await;
    }
}

/// Data acquisition task - samples analog and digital inputs
#[embassy_executor::task]
async fn acquisition_task() {
    info!("Data acquisition task started");
    let mut ticker = Ticker::every(Duration::from_millis(1000));
    let mut logger = DataLogger::new(LoggerConfig::default());
    logger.start();
    let mut tick_count: u64 = 0;

    loop {
        ticker.next().await;
        tick_count += 1;

        // ADC sampling via DMA (placeholder values until ADC driver is connected)
        // In production, these come from the OPA2376 buffered analog front-end
        let analog_values: [f32; 4] = [4.0, 4.0, 0.0, 0.0]; // 4mA idle, 0V idle
        let digital_in: u8 = 0x00;
        let digital_out: u8 = 0x00;
        let mcu_temp: f32 = 25.0;
        let supply_voltage: f32 = 3.3;

        let record = logger.create_record(
            tick_count * 1000, // timestamp_ms
            analog_values,
            digital_in,
            digital_out,
            mcu_temp,
            supply_voltage,
        );

        // Send record to logging task via shared channel
        // try_send avoids blocking the acquisition task if logger is slow
        match DATA_CHANNEL.try_send(record) {
            Ok(()) => {}
            Err(_) => {
                defmt::warn!("Data channel full, record dropped");
            }
        }
    }
}

/// Data logging task - writes records to flash and SD card
///
/// Receives DataRecords from the acquisition task via channel,
/// buffers them as CSV, and periodically flushes to SD card.
/// Handles file rotation when size limit is reached.
#[embassy_executor::task]
async fn logging_task() {
    info!("SD card logging task started");

    // Initialize SD logger with default config
    // 10MB per file, up to 1000 files, flush every 10 records
    let sd_config = SdLogConfig::default();
    let mut sd_logger = SdLogger::new(sd_config);

    // Wait for SD card initialization
    // In production, the SDMMC1 peripheral init would happen here
    Timer::after(Duration::from_millis(1000)).await;

    // Attempt to start logging
    // TODO: Replace with actual SDMMC1 hardware init sequence:
    //   1. Send CMD0 (GO_IDLE) at 400kHz
    //   2. Send CMD8 (SEND_IF_COND) to detect SDHC
    //   3. Send ACMD41 until card ready
    //   4. Switch to 25MHz 4-bit mode
    //   5. Mount FAT32 filesystem via embedded-sdmmc
    //   6. Create/open log directory
    sd_logger.start_logging();
    info!("SD logger active, writing to {}", sd_logger.current_filename().as_str());

    loop {
        // Wait for a data record from acquisition task
        let record = DATA_CHANNEL.receive().await;

        // Write CSV header if this is a new file
        if let Some(header) = sd_logger.get_header_if_needed() {
            // TODO: Write header bytes to SD card via SDMMC1
            // sdmmc_write(current_file, header).await;
            defmt::trace!("CSV header written ({} bytes)", header.len());
        }

        // Buffer the record as CSV
        if let Some(flush_data) = sd_logger.log_record(&record) {
            let bytes_to_write = flush_data.len();

            // Flush buffer to SD card
            // TODO: Replace with actual SDMMC1 write:
            //   volume_mgr.write(file_handle, flush_data)
            //   volume_mgr.flush(file_handle)
            //
            // For now, log the write operation
            info!(
                "SD flush: {} bytes -> {} (file size: {} KB)",
                bytes_to_write,
                sd_logger.current_filename().as_str(),
                sd_logger.current_file_size() / 1024
            );

            // Confirm write succeeded
            sd_logger.confirm_flush();

            // Check if file needs rotation
            if sd_logger.needs_file_rotation() {
                // Close current file, open next
                sd_logger.rotate_file(1000);
                info!("File rotated -> {}", sd_logger.current_filename().as_str());
            }
        }
    }
}

/// Communication task - manages all network interfaces
#[embassy_executor::task]
async fn comm_task() {
    info!("Communication task started");
    let mut ticker = Ticker::every(Duration::from_millis(10_000));

    loop {
        // Check link status for all interfaces
        // Select best available link
        // Transmit queued telemetry via MQTT
        // Process incoming commands
        ticker.next().await;
    }
}

/// Modbus RTU master task - polls field devices
#[embassy_executor::task]
async fn modbus_task() {
    info!("Modbus RTU task started");
    let mut ticker = Ticker::every(Duration::from_millis(500));

    loop {
        // Poll Modbus slave devices on RS485 bus
        // Read holding registers
        // Update shared data store
        ticker.next().await;
    }
}

/// BLE service task - handles local configuration via BLE
#[embassy_executor::task]
async fn ble_task() {
    info!("BLE service task started");
    let mut ticker = Ticker::every(Duration::from_millis(200));

    loop {
        // Check RN4870 connection status
        // If connected, send sensor data notifications
        // Process configuration commands from BLE central
        ticker.next().await;
    }
}

/// Debug console task - outputs diagnostics on USART1 (J12 header)
/// Sends boot banner, then periodic status reports at configured interval
#[embassy_executor::task]
async fn debug_console_task() {
    info!("Debug console task started (USART1 PA9/PA10, 115200 baud)");

    let mut console = DebugConsole::new(DebugConsoleConfig::default());

    // Send boot banner
    // TODO: Replace with actual USART1 write:
    //   uart1_tx.write_all(DebugConsole::boot_banner().as_bytes()).await;
    let banner = DebugConsole::boot_banner();
    info!("Debug banner: {} bytes queued", banner.len());

    let status_secs = console.status_interval_secs();
    let mut ticker = Ticker::every(Duration::from_secs(status_secs as u64));

    loop {
        ticker.next().await;

        // Format and send periodic status report
        // TODO: Read actual sensor values from shared state
        let report = console.format_status_report(
            0, // TODO: actual timestamp
            &[4.0, 4.0, 0.0, 0.0],
            0x00, 0x00,
            25.0, 3.3,
            true, 0,
        );

        // TODO: Replace with actual USART1 write:
        //   uart1_tx.write_all(report.as_bytes()).await;
        defmt::trace!("Console status: {} bytes", report.len());
    }
}

/// OTA firmware update task
///
/// Listens for firmware update commands via any communication channel,
/// manages the download/verify/apply lifecycle, and handles rollback.
#[embassy_executor::task]
async fn ota_task() {
    info!("OTA update task started");

    // Initialize updater in production mode (rejects debug builds when locked)
    let mut updater = FirmwareUpdater::new(false); // Set true after provisioning

    // On first boot after OTA, confirm the update succeeded
    // If this isn't called within watchdog timeout, bootloader rolls back
    // TODO: Only call after all subsystem checks pass
    updater.confirm_boot();
    info!("Boot confirmed for current firmware");

    let mut ticker = Ticker::every(Duration::from_millis(5000));

    loop {
        ticker.next().await;

        match updater.state() {
            OtaState::Idle => {
                // Check for update notifications from comm channels
                // Check SD card for /SCADA_FW/UPDATE.BIN
                // TODO: Integrate with comm_task for MQTT OTA commands
            }
            OtaState::Downloading => {
                // Check for download timeout
                // TODO: Pass actual timestamp
                if updater.check_timeout(0) {
                    defmt::warn!("OTA download timed out");
                }
            }
            OtaState::Verifying => {
                match updater.verify_download() {
                    Ok(()) => {
                        info!("OTA image verified, ready to apply");
                    }
                    Err(e) => {
                        defmt::error!("OTA verification failed: {:?}", defmt::Debug2Format(&e));
                    }
                }
            }
            OtaState::ReadyToApply => {
                // Apply the update (bank swap + reboot)
                match updater.apply_update() {
                    Ok(config) => {
                        info!(
                            "Applying OTA update v{}.{}.{}, rebooting...",
                            config.new_version[0],
                            config.new_version[1],
                            config.new_version[2],
                        );
                        // TODO: Program option bytes for bank swap
                        // TODO: Trigger system reset
                        //   cortex_m::peripheral::SCB::sys_reset();
                    }
                    Err(e) => {
                        defmt::error!("OTA apply failed: {:?}", defmt::Debug2Format(&e));
                    }
                }
            }
            OtaState::Error => {
                // Log error and return to idle
                if let Some(err) = updater.error() {
                    defmt::error!("OTA error: {:?}", defmt::Debug2Format(&err));
                }
                updater.abort();
            }
            _ => {}
        }
    }
}

/// System monitor task - temperature, voltage, diagnostics
#[embassy_executor::task]
async fn system_monitor_task() {
    info!("System monitor task started");
    let mut ticker = Ticker::every(Duration::from_millis(5000));

    loop {
        // Read MCU internal temperature sensor
        // Read supply voltage via ADC
        // Check storage health
        // Log diagnostics
        ticker.next().await;
    }
}

// ============================================================
// Main Entry Point
// ============================================================

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("=== SCADA Data Logger v0.2.0 ===");
    info!("STM32H743VIT6 @ 480MHz, Embassy async runtime");

    // Initialize peripherals with clock configuration
    let p = embassy_stm32::init(system_config());

    // ---- GPIO Setup ----

    // Watchdog (TPS3823-33 WDI pin)
    let wdi_pin = Output::new(p.PB8, Level::Low, Speed::Low);

    // SX1276 LoRa control pins
    let _lora_cs = Output::new(p.PA4, Level::High, Speed::VeryHigh);
    let _lora_rst = Output::new(p.PC2, Level::High, Speed::Low);
    let _lora_dio0 = Input::new(p.PC13, Pull::Down);

    // ATWINC1500 WiFi control pins
    let _wifi_cs = Output::new(p.PB12, Level::High, Speed::VeryHigh);
    let _wifi_rst = Output::new(p.PC0, Level::High, Speed::Low); // Moved from PB2 (now SPI3_MOSI)
    let _wifi_en = Output::new(p.PB1, Level::High, Speed::Low);
    let _wifi_irq = Input::new(p.PD10, Pull::Up);

    // W25Q64JV Flash CS (SPI3 remapped: PB3=SCK, PB4=MISO, PB2=MOSI)
    let _flash_cs = Output::new(p.PA15, Level::High, Speed::VeryHigh);

    // W5500 Ethernet control pins
    let _eth_cs = Output::new(p.PE4, Level::High, Speed::VeryHigh);
    let mut eth_rst = Output::new(p.PE3, Level::High, Speed::Low);
    let _eth_int = Input::new(p.PB0, Pull::Up);

    // SIM7600 GSM control pins
    let _gsm_pwr_key = Output::new(p.PC3, Level::Low, Speed::Low);
    let _gsm_status = Input::new(p.PD11, Pull::Down);

    // RN4870 BLE control pins
    let _ble_rst = Output::new(p.PD12, Level::High, Speed::Low);
    let _ble_status = Input::new(p.PD13, Pull::Down);

    // RS485 DE/RE pin
    let _rs485_de = Output::new(p.PD4, Level::Low, Speed::High);

    // Digital outputs (relay drivers via ULN2003)
    let _do0 = Output::new(p.PD14, Level::Low, Speed::Low);
    let _do1 = Output::new(p.PD15, Level::Low, Speed::Low);
    let _do2 = Output::new(p.PC6, Level::Low, Speed::Low);
    let _do3 = Output::new(p.PC7, Level::Low, Speed::Low);

    info!("GPIO initialized");

    // ---- SDMMC1 SD Card Pins ----
    // Native SDMMC1 peripheral in 4-bit mode for high-speed SD card access
    // Pins configured as alternate function by embassy-stm32 SDMMC driver:
    //   PC12 = SDMMC1_CLK
    //   PD2  = SDMMC1_CMD
    //   PC8  = SDMMC1_D0
    //   PC9  = SDMMC1_D1
    //   PC10 = SDMMC1_D2
    //   PC11 = SDMMC1_D3
    // Card detect via Molex 104031-0811 connector CD pin
    info!("SDMMC1 pins reserved for SD card (4-bit mode)");

    // ---- Security: Read Flash Protection Status ----
    let mut provisioning = ProvisioningManager::new();
    provisioning.init();
    match provisioning.state() {
        ProvisioningState::Unprovisioned => {
            info!("SECURITY: Device is UNPROVISIONED (SWD/DFU open)");
            info!("  -> Run lockdown after verifying OTA works");
        }
        ProvisioningState::Locked => {
            info!("SECURITY: Device is LOCKED (OTA-only updates)");
        }
        state => {
            info!("SECURITY: Provisioning state: {:?}", defmt::Debug2Format(&state));
        }
    }
    info!("Protection: {}", provisioning.protection().status().summary().as_str());

    // ---- Hardware Reset Sequence ----

    // Reset W5500 Ethernet
    use scada_embedded::hal::drivers::w5500_ethernet::W5500Driver;
    W5500Driver::hardware_reset(&mut eth_rst).await;
    info!("W5500 reset complete");

    // ---- Spawn Tasks ----

    // Watchdog must start first (1.6s timeout)
    spawner.must_spawn(watchdog_task(wdi_pin));
    info!("Watchdog task spawned");

    // Core tasks
    spawner.must_spawn(acquisition_task());
    spawner.must_spawn(logging_task());
    spawner.must_spawn(comm_task());
    spawner.must_spawn(modbus_task());
    spawner.must_spawn(ble_task());
    spawner.must_spawn(system_monitor_task());
    spawner.must_spawn(debug_console_task());
    spawner.must_spawn(ota_task());

    info!("All tasks spawned, system running");

    // Main loop - LED heartbeat
    let mut led_state = false;
    let mut ticker = Ticker::every(Duration::from_millis(1000));
    loop {
        led_state = !led_state;
        // Status LED would toggle here
        defmt::trace!("heartbeat");
        ticker.next().await;
    }
}
