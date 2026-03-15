//! Digital I/O Driver - 8 Isolated Inputs + 4 Relay Outputs
//!
//! Digital Inputs: 24VDC via TLP293 optocouplers → GPIO (active low)
//!   DI0: PD3, DI1: PD7, DI2: PE0, DI3: PE1
//!   DI4: PE9, DI5: PE10, DI6: PE11, DI7: PE12
//!
//! Digital Outputs: GPIO → ULN2003 → Relay coils
//!   DO0: PE13, DO1: PE14, DO2: PE15, DO3: PD14

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use crate::hal::{DigitalInputState, RelayState};

/// Number of digital input channels
pub const DI_COUNT: usize = 8;
/// Number of digital output (relay) channels
pub const DO_COUNT: usize = 4;

/// Debounce sample count (reads across multiple cycles)
const DEBOUNCE_SAMPLES: u8 = 3;

/// Digital input channel with debouncing
pub struct DigitalInput {
    /// Last stable state
    state: DigitalInputState,
    /// Debounce counter
    debounce_count: u8,
    /// Raw reading from last sample
    raw: bool,
}

impl DigitalInput {
    pub fn new() -> Self {
        Self {
            state: DigitalInputState::Low,
            debounce_count: 0,
            raw: false,
        }
    }

    /// Update with a new raw reading, returns true if state changed
    pub fn update(&mut self, raw_high: bool) -> bool {
        self.raw = raw_high;
        let target = if raw_high {
            DigitalInputState::High
        } else {
            DigitalInputState::Low
        };

        if target != self.state {
            self.debounce_count += 1;
            if self.debounce_count >= DEBOUNCE_SAMPLES {
                self.state = target;
                self.debounce_count = 0;
                return true; // State changed
            }
        } else {
            self.debounce_count = 0;
        }
        false
    }

    pub fn state(&self) -> DigitalInputState {
        self.state
    }

    pub fn is_high(&self) -> bool {
        self.state == DigitalInputState::High
    }
}

/// Relay output channel with safety interlock
pub struct RelayOutput {
    state: RelayState,
    /// Safety: maximum ON time in seconds (0 = unlimited)
    max_on_time_s: u32,
    /// Tick counter when relay was turned on
    on_since_tick: u32,
}

impl RelayOutput {
    pub fn new() -> Self {
        Self {
            state: RelayState::Open,
            max_on_time_s: 0,
            on_since_tick: 0,
        }
    }

    pub fn set_max_on_time(&mut self, seconds: u32) {
        self.max_on_time_s = seconds;
    }

    pub fn set(&mut self, state: RelayState, current_tick: u32) {
        self.state = state;
        if state == RelayState::Closed {
            self.on_since_tick = current_tick;
        }
    }

    /// Check safety timeout, returns true if relay was force-opened
    pub fn check_timeout(&mut self, current_tick: u32) -> bool {
        if self.state == RelayState::Closed && self.max_on_time_s > 0 {
            if current_tick.wrapping_sub(self.on_since_tick) >= self.max_on_time_s {
                self.state = RelayState::Open;
                return true;
            }
        }
        false
    }

    pub fn state(&self) -> RelayState {
        self.state
    }

    pub fn is_closed(&self) -> bool {
        self.state == RelayState::Closed
    }
}

/// Complete digital I/O manager
pub struct DigitalIoManager {
    pub inputs: [DigitalInput; DI_COUNT],
    pub outputs: [RelayOutput; DO_COUNT],
}

impl DigitalIoManager {
    pub fn new() -> Self {
        Self {
            inputs: core::array::from_fn(|_| DigitalInput::new()),
            outputs: core::array::from_fn(|_| RelayOutput::new()),
        }
    }

    /// Get packed digital input state as u8 bitmask (DI0=bit0, DI7=bit7)
    pub fn input_bitmask(&self) -> u8 {
        let mut mask: u8 = 0;
        for (i, di) in self.inputs.iter().enumerate() {
            if di.is_high() {
                mask |= 1 << i;
            }
        }
        mask
    }

    /// Get packed relay output state as u8 bitmask (DO0=bit0, DO3=bit3)
    pub fn output_bitmask(&self) -> u8 {
        let mut mask: u8 = 0;
        for (i, dout) in self.outputs.iter().enumerate() {
            if dout.is_closed() {
                mask |= 1 << i;
            }
        }
        mask
    }

    /// Emergency stop - open all relays
    pub fn emergency_stop(&mut self) {
        for relay in self.outputs.iter_mut() {
            relay.state = RelayState::Open;
        }
    }
}
