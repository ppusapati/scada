/// Data Logging Task — MicroSD Card
///
/// Logs solar string data to MicroSD card at configurable interval.
/// Default: every 60 seconds.
/// Format: CSV files in /LOG/YYYYMMDD.CSV (simplified to sequential)
///
/// Each log entry contains all 16 string measurements plus bus/plant data.

use embassy_executor::task;
use embassy_stm32::gpio::{Input, Output};
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Instant, Timer};
use defmt::{info, warn, error};

use crate::drivers::sdcard::{SdLogger, SdState, LogEntry};
use crate::modbus::registers::{device_status, holding_reg};
use crate::tasks::shared::SharedState;

#[task]
pub async fn data_logger(
    spi: Spi<'static, Async>,
    cs: Output<'static>,
    detect: Input<'static>,
    shared: &'static SharedState,
) {
    info!("Logging task: starting");

    let mut sd = SdLogger::new(spi, cs, detect);

    // Try to initialize SD card
    if sd.card_present() {
        if sd.init().await {
            let mut status = shared.device_status.lock().await;
            *status |= device_status::SD_OK;
            info!("Logging: SD card ready");
        } else {
            warn!("Logging: SD card init failed");
        }
    } else {
        info!("Logging: No SD card detected");
    }

    // Get log interval from config
    let log_interval_s = {
        let regs = shared.registers.lock().await;
        let s = regs.holding_regs[holding_reg::LOG_INTERVAL_S as usize];
        if s < 5 { 60u64 } else { s as u64 }
    };

    info!("Logging: interval = {}s", log_interval_s);

    let mut log_count: u32 = 0;

    loop {
        Timer::after(Duration::from_secs(log_interval_s)).await;

        // Check if SD card is still present
        if !sd.card_present() {
            if sd.state() != SdState::NoCard {
                warn!("Logging: SD card removed");
                let mut status = shared.device_status.lock().await;
                *status &= !device_status::SD_OK;
            }
            continue;
        }

        // Re-init if needed
        if sd.state() != SdState::Ready {
            if sd.init().await {
                let mut status = shared.device_status.lock().await;
                *status |= device_status::SD_OK;
            } else {
                continue;
            }
        }

        // Read current data
        let strings = {
            let s = shared.strings.lock().await;
            *s
        };
        let irradiance = {
            let i = shared.irradiance.lock().await;
            *i
        };
        let temperature = {
            let t = shared.ambient_temp.lock().await;
            *t
        };

        let timestamp = (Instant::now().as_millis() / 1000) as u32;

        // Log each string
        for s in 0..16u8 {
            let entry = LogEntry {
                timestamp_secs: timestamp,
                string_id: s + 1,
                voltage: strings[s as usize].voltage,
                current: strings[s as usize].current,
                power: strings[s as usize].power,
                irradiance,
                temperature,
            };

            if !sd.write_log(&entry).await {
                warn!("Logging: write failed for string {}", s + 1);
                break;
            }
        }

        log_count += 1;
        if log_count % 10 == 0 {
            info!("Logging: {} cycles written to SD", log_count);
        }
    }
}
