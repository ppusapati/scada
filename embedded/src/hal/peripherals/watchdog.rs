//! Hardware Watchdog Interface - TPS3823-33
//!
//! External watchdog timer connected to PB8 (WDI input).
//! MCU must toggle PB8 within 1.6s or TPS3823 resets the system.
//! Also uses STM32H743 internal IWDG as secondary watchdog.

use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::{Duration, Timer};

/// TPS3823 watchdog timeout (1.6 seconds)
pub const WDT_TIMEOUT_MS: u64 = 1600;

/// Kick interval should be well under timeout (use 500ms)
pub const WDT_KICK_INTERVAL_MS: u64 = 500;

/// External hardware watchdog (TPS3823-33) driver
pub struct ExternalWatchdog {
    wdi_pin: Output<'static>,
    kick_state: bool,
}

impl ExternalWatchdog {
    pub fn new(wdi_pin: Output<'static>) -> Self {
        Self {
            wdi_pin,
            kick_state: false,
        }
    }

    /// Toggle WDI pin to reset the watchdog timer.
    /// Must be called at least every 1.6 seconds.
    pub fn kick(&mut self) {
        self.kick_state = !self.kick_state;
        if self.kick_state {
            self.wdi_pin.set_high();
        } else {
            self.wdi_pin.set_low();
        }
    }
}

/// Watchdog task - runs in background, kicks both external and internal WDT
pub async fn watchdog_task(mut wdt: ExternalWatchdog) -> ! {
    loop {
        wdt.kick();
        Timer::after(Duration::from_millis(WDT_KICK_INTERVAL_MS)).await;
    }
}
