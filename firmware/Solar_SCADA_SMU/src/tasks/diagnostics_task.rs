/// Diagnostics Task — Solar SCADA SMU
///
/// Manages:
///   - TMP117 ambient temperature readings (every 1s)
///   - Status LEDs (heartbeat, alarms, SD activity)
///   - Alarm condition processing
///   - Performance ratio calculation

use embassy_executor::task;
use embassy_stm32::gpio::Output;
use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::{info, warn};

use crate::drivers::tmp117::Tmp117;
use crate::modbus::registers::{device_status, input_reg};
use crate::tasks::shared::SharedState;

#[task]
pub async fn diagnostics(
    i2c: I2c<'static, Async>,
    mut led_green: Output<'static>,
    mut led_orange: Output<'static>,
    mut led_red: Output<'static>,
    mut led_blue: Output<'static>,
    shared: &'static SharedState,
) {
    info!("Diagnostics task (Solar): starting");

    let mut tmp117 = Tmp117::new(i2c);

    // Initialize TMP117
    if tmp117.init().await {
        let mut status = shared.device_status.lock().await;
        *status |= device_status::TMP_OK;
        info!("TMP117: initialized");
    } else {
        warn!("TMP117: init failed, temperature data unavailable");
    }

    let mut heartbeat = false;
    let mut cycle: u32 = 0;

    loop {
        Timer::after(Duration::from_millis(500)).await;
        cycle += 1;

        // ── Heartbeat (green) ──
        heartbeat = !heartbeat;
        if heartbeat {
            led_green.set_high();
        } else {
            led_green.set_low();
        }

        // ── Temperature reading (every 1s = every 2 cycles) ──
        if cycle % 2 == 0 {
            let temp = tmp117.read_temperature().await;
            if temp.valid {
                let mut at = shared.ambient_temp.lock().await;
                *at = temp.celsius;

                // Update Modbus register
                let mut regs = shared.registers.lock().await;
                let bits = temp.celsius.to_bits();
                regs.input_regs[input_reg::AMBIENT_TEMP as usize] = (bits >> 16) as u16;
                regs.input_regs[(input_reg::AMBIENT_TEMP + 1) as usize] = (bits & 0xFFFF) as u16;
            }
        }

        // ── String fault LED (red) ──
        let string_faults = {
            let regs = shared.registers.lock().await;
            regs.input_regs[input_reg::STRING_STATUS as usize]
        };
        if string_faults != 0 {
            if cycle % 2 == 0 {
                led_red.set_high();
            } else {
                led_red.set_low();
            }
        } else {
            led_red.set_low();
        }

        // ── Acquisition LED (orange) ── blinks when scan active
        let dev_status = {
            let s = shared.device_status.lock().await;
            *s
        };
        if dev_status & device_status::ADC_OK != 0 {
            led_orange.set_high();
            Timer::after(Duration::from_millis(50)).await;
            led_orange.set_low();
        }

        // ── SD card LED (blue) ──
        if dev_status & device_status::SD_OK != 0 {
            led_blue.set_low(); // Off when idle
        } else {
            // Slow blink = no SD card
            if cycle % 6 < 3 {
                led_blue.set_high();
            } else {
                led_blue.set_low();
            }
        }

        // ── Periodic summary (every 30 seconds) ──
        if cycle % 60 == 0 {
            let ambient = {
                let t = shared.ambient_temp.lock().await;
                *t
            };
            let total_power = {
                let regs = shared.registers.lock().await;
                let bits = ((regs.input_regs[input_reg::TOTAL_POWER as usize] as u32) << 16)
                    | regs.input_regs[(input_reg::TOTAL_POWER + 1) as usize] as u32;
                f32::from_bits(bits)
            };
            let energy = {
                let e = shared.daily_energy_wh.lock().await;
                *e / 1000.0 // kWh
            };

            info!(
                "Solar: Ptotal={}W, Energy={}kWh, Tamb={}°C, Faults=0x{:04X}",
                total_power as i32,
                energy as i32,
                ambient as i32,
                string_faults,
            );
        }
    }
}
