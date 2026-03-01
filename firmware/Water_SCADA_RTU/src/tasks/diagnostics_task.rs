/// Diagnostics Task — Lowest Priority
///
/// Manages status LEDs:
///   PD12 (Green)  — heartbeat, 1 Hz blink when healthy
///   PD13 (Orange) — Modbus activity indicator
///   PD14 (Red)    — fault/alarm active
///   PD15 (Blue)   — ADC acquisition indicator
///
/// Also processes alarm conditions and calculates derived values.

use embassy_executor::task;
use embassy_stm32::gpio::Output;
use embassy_time::{Duration, Timer};
use defmt::{info, warn};

use crate::drivers::ads1258::ChannelStatus;
use crate::modbus::registers::{device_status, input_reg};
use crate::tasks::shared::SharedState;

#[task]
pub async fn diagnostics(
    mut led_green: Output<'static>,
    mut led_orange: Output<'static>,
    mut led_red: Output<'static>,
    mut led_blue: Output<'static>,
    shared: &'static SharedState,
) {
    info!("Diagnostics task: starting");

    let mut heartbeat = false;
    let mut cycle: u32 = 0;

    loop {
        // 500ms cycle → 1 Hz heartbeat
        Timer::after(Duration::from_millis(500)).await;
        cycle += 1;

        // ── Heartbeat LED (green) ──
        heartbeat = !heartbeat;
        if heartbeat {
            led_green.set_high();
        } else {
            led_green.set_low();
        }

        // ── Check device status ──
        let dev_status = {
            let s = shared.device_status.lock().await;
            *s
        };

        // ── Alarm LED (red) ──
        let alarm_active = {
            let regs = shared.registers.lock().await;
            regs.input_regs[input_reg::ALARM_BITMAP_LO as usize] != 0
        };
        if alarm_active {
            // Fast blink red LED when alarm active
            if cycle % 2 == 0 {
                led_red.set_high();
            } else {
                led_red.set_low();
            }
        } else {
            led_red.set_low();
        }

        // ── ADC status LED (blue) ──
        if dev_status & device_status::ADC_OK != 0 {
            // Brief flash on each acquisition cycle
            led_blue.set_high();
            Timer::after(Duration::from_millis(50)).await;
            led_blue.set_low();
        } else {
            // Solid blue = ADC fault
            led_blue.set_high();
        }

        // ── Ethernet status (orange) ──
        if dev_status & device_status::ETH_LINK != 0 {
            led_orange.set_low(); // Link up = LED off (blinks on traffic elsewhere)
        } else {
            // Slow blink = no Ethernet link
            if cycle % 4 < 2 {
                led_orange.set_high();
            } else {
                led_orange.set_low();
            }
        }

        // ── Sensor diagnostics (every 5 seconds) ──
        if cycle % 10 == 0 {
            let readings = shared.latest_readings.lock().await;
            let mut fault_count = 0u8;

            for (i, r) in readings.iter().enumerate() {
                if r.status != ChannelStatus::Normal {
                    fault_count += 1;
                    warn!(
                        "CH{} fault: status={}, current={}mA",
                        i,
                        r.status,
                        r.current_ma as i32,
                    );
                }
            }

            if fault_count > 0 {
                warn!("Diagnostics: {} channel fault(s) active", fault_count);
            }

            // Update device status alarm flag
            let mut status = shared.device_status.lock().await;
            if alarm_active {
                *status |= device_status::ANY_ALARM;
            } else {
                *status &= !device_status::ANY_ALARM;
            }
        }
    }
}
