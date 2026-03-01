/// GPIO driver for STM32F407VGT6 — LEDs, relays, and digital inputs
///
/// LED pins (active high, on Discovery board):
///   PD12 = Green  (heartbeat)
///   PD13 = Orange (MQTT connected)
///   PD14 = Red    (fault/alarm)
///   PD15 = Blue   (data TX blink)
///
/// Relay outputs (active high via transistor driver):
///   PE0 = Relay 1 (pump)
///   PE1 = Relay 2 (valve)
///   PE2 = Relay 3 (spare)
///
/// Digital inputs (active low with internal pull-up):
///   PC13 = User button
///   PE3  = Float switch (tank overflow)
///   PE4  = Flow pulse counter

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::peripherals;

/// Status LEDs on the STM32F4 Discovery board
pub struct StatusLeds {
    pub heartbeat: Output<'static>,  // PD12 green
    pub mqtt:      Output<'static>,  // PD13 orange
    pub fault:     Output<'static>,  // PD14 red
    pub tx:        Output<'static>,  // PD15 blue
}

impl StatusLeds {
    pub fn new(
        pd12: peripherals::PD12,
        pd13: peripherals::PD13,
        pd14: peripherals::PD14,
        pd15: peripherals::PD15,
    ) -> Self {
        Self {
            heartbeat: Output::new(pd12, Level::Low, Speed::Low),
            mqtt:      Output::new(pd13, Level::Low, Speed::Low),
            fault:     Output::new(pd14, Level::Low, Speed::Low),
            tx:        Output::new(pd15, Level::Low, Speed::Low),
        }
    }

    /// Set all LEDs to off
    pub fn all_off(&mut self) {
        self.heartbeat.set_low();
        self.mqtt.set_low();
        self.fault.set_low();
        self.tx.set_low();
    }

    /// Boot sequence: flash all LEDs in sequence
    pub fn boot_sequence(&mut self) {
        self.heartbeat.set_high();
        self.mqtt.set_high();
        self.fault.set_high();
        self.tx.set_high();
    }
}

/// Relay outputs for actuator control
pub struct Relays {
    pub pump:  Output<'static>,  // PE0
    pub valve: Output<'static>,  // PE1
    pub spare: Output<'static>,  // PE2
}

impl Relays {
    pub fn new(
        pe0: peripherals::PE0,
        pe1: peripherals::PE1,
        pe2: peripherals::PE2,
    ) -> Self {
        // Initialize all relays OFF for safety
        Self {
            pump:  Output::new(pe0, Level::Low, Speed::Low),
            valve: Output::new(pe1, Level::Low, Speed::Low),
            spare: Output::new(pe2, Level::Low, Speed::Low),
        }
    }

    /// Emergency stop: all relays off
    pub fn emergency_stop(&mut self) {
        self.pump.set_low();
        self.valve.set_low();
        self.spare.set_low();
    }

    pub fn set_pump(&mut self, on: bool) {
        if on {
            self.pump.set_high();
        } else {
            self.pump.set_low();
        }
    }

    pub fn set_valve(&mut self, on: bool) {
        if on {
            self.valve.set_high();
        } else {
            self.valve.set_low();
        }
    }
}

/// Digital input sensors
pub struct DigitalInputs {
    pub user_button:  Input<'static>,  // PC13
    pub float_switch: Input<'static>,  // PE3 (tank overflow)
    pub flow_pulse:   Input<'static>,  // PE4 (flow meter pulse)
}

impl DigitalInputs {
    pub fn new(
        pc13: peripherals::PC13,
        pe3: peripherals::PE3,
        pe4: peripherals::PE4,
    ) -> Self {
        Self {
            user_button:  Input::new(pc13, Pull::Up),   // Active low
            float_switch: Input::new(pe3, Pull::Up),    // Active low (NC contact)
            flow_pulse:   Input::new(pe4, Pull::Down),  // Pulse input
        }
    }

    /// Returns true if user button is pressed (active low)
    pub fn button_pressed(&self) -> bool {
        self.user_button.is_low()
    }

    /// Returns true if float switch indicates overflow (active low = triggered)
    pub fn tank_overflow(&self) -> bool {
        self.float_switch.is_low()
    }

    /// Read flow pulse state (for edge counting in interrupt)
    pub fn flow_pulse_high(&self) -> bool {
        self.flow_pulse.is_high()
    }
}
