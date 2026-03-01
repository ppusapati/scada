/// System Diagnostics Task — Lowest Priority
///
/// Manages:
///   - LED indicators (heartbeat, fault, TX, link)
///   - TMP117 ambient temperature readings
///   - Memory usage monitoring
///   - Task execution time monitoring
///   - Watchdog feeding
///   - PTP sync status updates

use embassy_executor::task;
use embassy_stm32::gpio::Output;
use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Instant, Timer};
use defmt::{info, warn};

use crate::modbus::{device_status, input_reg};
use crate::tasks::shared::SharedState;

/// Task index for statistics
const TASK_IDX: usize = 3;

/// TMP117 I2C address (ADD0 = GND)
const TMP117_ADDR: u8 = 0x48;
const TMP117_REG_TEMP: u8 = 0x00;
const TMP117_REG_CONFIG: u8 = 0x01;
const TMP117_REG_DEVICE_ID: u8 = 0x0F;
const TMP117_EXPECTED_ID: u16 = 0x0117;

/// IWDG (Independent Watchdog) registers for STM32MP157
const IWDG2_BASE: u32 = 0x4900_2000; // IWDG2 is for M4
const IWDG_KR: u32 = 0x00;
const IWDG_KEY_RELOAD: u32 = 0xAAAA;

#[task]
pub async fn diagnostics(
    i2c: I2c<'static, Async>,
    mut led_heartbeat: Output<'static>,
    mut led_fault: Output<'static>,
    mut led_tx: Output<'static>,
    mut led_link: Output<'static>,
    shared: &'static SharedState,
) {
    info!("Diagnostics task: starting");

    // Initialize TMP117
    let mut tmp117_ok = false;
    let mut i2c = i2c;

    // Read TMP117 device ID
    let mut id_buf = [0u8; 2];
    if let Ok(()) = i2c.write_read(TMP117_ADDR, &[TMP117_REG_DEVICE_ID], &mut id_buf).await {
        let id = u16::from_be_bytes(id_buf);
        if id == TMP117_EXPECTED_ID {
            // Configure: continuous conversion, 1s cycle, 8x averaging
            let config: u16 = 0x0620; // CC mode, 1s cycle, 8x avg
            let config_bytes = config.to_be_bytes();
            if let Ok(()) = i2c.write(TMP117_ADDR, &[TMP117_REG_CONFIG, config_bytes[0], config_bytes[1]]).await {
                tmp117_ok = true;
                let mut status = shared.device_status.lock().await;
                *status |= device_status::TMP_OK;
                info!("TMP117: Initialized (ID=0x{:04X})", id);
            }
        } else {
            warn!("TMP117: ID mismatch (got 0x{:04X}, expected 0x{:04X})", id, TMP117_EXPECTED_ID);
        }
    } else {
        warn!("TMP117: I2C communication failed");
    }

    let mut heartbeat_state = false;
    let mut cycle: u32 = 0;

    loop {
        Timer::after(Duration::from_millis(500)).await;
        cycle += 1;

        let exec_start = Instant::now();

        // ── Heartbeat LED (green): 1 Hz blink ──
        heartbeat_state = !heartbeat_state;
        if heartbeat_state {
            led_heartbeat.set_high();
        } else {
            led_heartbeat.set_low();
        }

        // ── Feed watchdog ──
        feed_watchdog();

        // ── Temperature reading (every 2 cycles = 1 Hz) ──
        if cycle % 2 == 0 && tmp117_ok {
            let mut temp_buf = [0u8; 2];
            if let Ok(()) = i2c.write_read(TMP117_ADDR, &[TMP117_REG_TEMP], &mut temp_buf).await {
                let raw = i16::from_be_bytes(temp_buf);
                let celsius = raw as f32 * 0.0078125;

                let mut temp = shared.temperature.lock().await;
                *temp = celsius;

                // Update Modbus register
                let mut regs = shared.registers.lock().await;
                let bits = celsius.to_bits();
                regs.input_regs[input_reg::TEMPERATURE as usize] = (bits >> 16) as u16;
                regs.input_regs[(input_reg::TEMPERATURE + 1) as usize] = (bits & 0xFFFF) as u16;
            }
        }

        // ── Update PTP sync status (every 2 seconds) ──
        if cycle % 4 == 0 {
            let mut ptp = shared.ptp_clock.lock().await;
            ptp.update_sync_status();

            if ptp.is_synchronized() {
                let mut status = shared.device_status.lock().await;
                *status |= device_status::PTP_SYNC;
            } else {
                let mut status = shared.device_status.lock().await;
                *status &= !device_status::PTP_SYNC;
            }
        }

        // ── Fault LED (red): alarm indicator ──
        let alarm_active = {
            let regs = shared.registers.lock().await;
            regs.input_regs[input_reg::ALARM_BITMAP_LO as usize] != 0
        };

        if alarm_active {
            // Fast blink red when alarms active
            if cycle % 2 == 0 {
                led_fault.set_high();
            } else {
                led_fault.set_low();
            }
        } else {
            led_fault.set_low();
        }

        // ── TX activity LED (orange): brief flash on DDS publish ──
        let dds_ok = {
            let status = shared.device_status.lock().await;
            *status & device_status::DDS_OK != 0
        };
        if dds_ok {
            led_tx.set_high();
            Timer::after(Duration::from_millis(50)).await;
            led_tx.set_low();
        }

        // ── Link LED (blue): RPMsg/DDS connection status ──
        let rpmsg_ok = {
            let status = shared.device_status.lock().await;
            *status & device_status::RPMSG_OK != 0
        };
        if rpmsg_ok {
            led_link.set_low(); // Solid off = connected (blinks on traffic)
        } else {
            // Slow blink = not connected
            if cycle % 4 < 2 {
                led_link.set_high();
            } else {
                led_link.set_low();
            }
        }

        // ── Periodic system summary (every 30 seconds) ──
        if cycle % 60 == 0 {
            let temp = { *shared.temperature.lock().await };
            let dev_status = { *shared.device_status.lock().await };
            let scan_count = { *shared.scan_count.lock().await };
            let task_stats = { *shared.task_stats.lock().await };

            let ptp_state = {
                let ptp = shared.ptp_clock.lock().await;
                ptp.sync_state()
            };

            info!("=== System Health ===");
            info!("  Temp: {}C, Scans: {}", temp as i32, scan_count);
            info!("  Status: 0x{:04X}, PTP: {}", dev_status, ptp_state);
            info!("  Task exec (us): sensor={}, dds={}, modbus={}, diag={}",
                task_stats[0].last_exec_us,
                task_stats[1].last_exec_us,
                task_stats[2].last_exec_us,
                task_stats[3].last_exec_us,
            );
            info!("  Task max (us): sensor={}, dds={}, modbus={}, diag={}",
                task_stats[0].max_exec_us,
                task_stats[1].max_exec_us,
                task_stats[2].max_exec_us,
                task_stats[3].max_exec_us,
            );
        }

        // Update task stats
        {
            let mut stats = shared.task_stats.lock().await;
            let exec_us = exec_start.elapsed().as_micros() as u32;
            stats[TASK_IDX].last_exec_us = exec_us;
            if exec_us > stats[TASK_IDX].max_exec_us {
                stats[TASK_IDX].max_exec_us = exec_us;
            }
            stats[TASK_IDX].exec_count += 1;
        }
    }
}

/// Feed the independent watchdog (IWDG2 for M4 on STM32MP157)
fn feed_watchdog() {
    unsafe {
        core::ptr::write_volatile(
            (IWDG2_BASE + IWDG_KR) as *mut u32,
            IWDG_KEY_RELOAD,
        );
    }
}
