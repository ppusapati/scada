/// ADC Acquisition Task — Highest Priority
///
/// Reads all 8 channels from ADS1258 at 10 Hz (100ms period).
/// Updates shared register store with raw codes, engineering values, and status.
/// Signals other tasks when new data is available.

use embassy_executor::task;
use embassy_stm32::gpio::{Input, Output};
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Instant, Timer, Ticker};
use defmt::{info, warn, error};

use crate::drivers::ads1258::{Ads1258, Channel};
use crate::modbus::registers::device_status;
use crate::tasks::shared::SharedState;

#[task]
pub async fn adc_acquisition(
    spi: Spi<'static, Async>,
    cs: Output<'static>,
    drdy: Input<'static>,
    start: Output<'static>,
    reset: Output<'static>,
    shared: &'static SharedState,
) {
    info!("ADC task: starting");

    let mut adc = Ads1258::new(spi, cs, drdy, start, reset);

    // Initialize ADC hardware
    let adc_ok = adc.init().await;
    if !adc_ok {
        error!("ADC task: ADS1258 init FAILED — entering error state");
        // Set ADC fault in device status
        {
            let mut status = shared.device_status.lock().await;
            *status &= !device_status::ADC_OK;
        }
        // Keep task alive but in error state, retry periodically
        loop {
            Timer::after(Duration::from_secs(5)).await;
            if adc.init().await {
                info!("ADC task: ADS1258 recovered");
                break;
            }
        }
    }

    // Set ADC OK in device status
    {
        let mut status = shared.device_status.lock().await;
        *status |= device_status::ADC_OK;
    }

    // Load calibrations from holding registers
    {
        let regs = shared.registers.lock().await;
        adc.set_calibrations(regs.get_calibrations());
    }

    info!("ADC task: scanning 8 channels at 10 Hz");

    // 10 Hz acquisition loop (100ms period)
    let mut ticker = Ticker::every(Duration::from_millis(100));
    let mut scan_count: u32 = 0;

    loop {
        ticker.next().await;

        let readings = adc.read_all_channels().await;
        scan_count += 1;

        // Update shared register store
        {
            let mut regs = shared.registers.lock().await;
            regs.update_from_readings(&readings);

            // Update uptime
            let uptime_secs = (Instant::now().as_millis() / 1000) as u32;
            regs.update_uptime(uptime_secs);

            // Update device status
            let dev_status = shared.device_status.lock().await;
            regs.set_device_status(*dev_status);

            // Check for calibration save command
            if regs.holding_regs[crate::modbus::registers::holding_reg::CAL_SAVE_CMD as usize] == 0xCAFE {
                regs.holding_regs[crate::modbus::registers::holding_reg::CAL_SAVE_CMD as usize] = 0;
                shared.cal_save_requested.signal(());
                // Reload calibrations
                adc.set_calibrations(regs.get_calibrations());
            }
        }

        // Cache latest readings for diagnostics
        {
            let mut latest = shared.latest_readings.lock().await;
            *latest = readings;
        }

        // Signal new data available
        shared.new_data.signal(());

        // Periodic status log (every 10 seconds)
        if scan_count % 100 == 0 {
            let r = &readings;
            info!(
                "ADC scan #{}: CH0={} CH1={} CH2={} CH3={}",
                scan_count,
                r[0].eng_value as i32,
                r[1].eng_value as i32,
                r[2].eng_value as i32,
                r[3].eng_value as i32,
            );
        }
    }
}
