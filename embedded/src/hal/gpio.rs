/// GPIO driver for STM32F407VGT6 -- LEDs, relays, multiplexer, RS-485, and digital inputs
///
/// LED pins (active high, on Discovery board):
///   PD12 = Green  (heartbeat)
///   PD13 = Orange (MQTT/DDS connected)
///   PD14 = Red    (fault/alarm)
///   PD15 = Blue   (data TX blink / SD/ETH activity)
///
/// Relay outputs (active high via transistor driver):
///   PE0 = Relay 1 (pump)
///   PE1 = Relay 2 (valve)
///   PE2 = Relay 3 (spare)
///
/// CD74HC4067 analog multiplexer (16:1):
///   PC6 = S0 (select bit 0)
///   PC7 = S1 (select bit 1)
///   PC8 = S2 (select bit 2)
///   PC9 = S3 (select bit 3)
///   PC10 = EN (active low enable)
///
/// RS-485 transceiver (SP3485):
///   PB0 = DE  (driver enable, active high)
///   PB1 = /RE (receiver enable, active low)
///
/// Boot configuration:
///   BOOT0 = PH3 (directly maps to chip pin)
///
/// Digital inputs (active low with internal pull-up):
///   PC13 = User button
///   PE3  = Float switch (tank overflow)
///   PE4  = Flow pulse counter

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::peripherals;
use defmt::{info, warn, Format};

// ============================================================================
// CD74HC4067 Analog Multiplexer (16-channel)
// ============================================================================

/// CD74HC4067 16-channel analog multiplexer control
///
/// Select pins S0-S3 choose one of 16 input channels routed to the common output.
/// Enable pin (active low) gates the output; when disabled, output is high-Z.
///
/// Channel selection truth table:
///   Channel = S3*8 + S2*4 + S1*2 + S0
///
/// Used to expand the number of analog channels readable by a single ADC pin.
pub struct AnalogMux {
    s0: Output<'static>,   // PC6
    s1: Output<'static>,   // PC7
    s2: Output<'static>,   // PC8
    s3: Output<'static>,   // PC9
    en: Output<'static>,   // PC10 (active low)
    current_channel: u8,
}

impl AnalogMux {
    /// Create a new multiplexer driver with all pins configured
    pub fn new(
        pc6: peripherals::PC6,
        pc7: peripherals::PC7,
        pc8: peripherals::PC8,
        pc9: peripherals::PC9,
        pc10: peripherals::PC10,
    ) -> Self {
        Self {
            s0: Output::new(pc6, Level::Low, Speed::Low),
            s1: Output::new(pc7, Level::Low, Speed::Low),
            s2: Output::new(pc8, Level::Low, Speed::Low),
            s3: Output::new(pc9, Level::Low, Speed::Low),
            en: Output::new(pc10, Level::High, Speed::Low), // Start disabled (active low)
            current_channel: 0,
        }
    }

    /// Select a channel (0-15) on the multiplexer
    ///
    /// The analog output will be available after the settling time (~50ns for CD74HC4067).
    /// For high-precision measurements, allow at least 1us after switching before reading.
    pub fn select_channel(&mut self, channel: u8) {
        let ch = channel & 0x0F; // Clamp to 0-15

        if ch & 0x01 != 0 { self.s0.set_high(); } else { self.s0.set_low(); }
        if ch & 0x02 != 0 { self.s1.set_high(); } else { self.s1.set_low(); }
        if ch & 0x04 != 0 { self.s2.set_high(); } else { self.s2.set_low(); }
        if ch & 0x08 != 0 { self.s3.set_high(); } else { self.s3.set_low(); }

        self.current_channel = ch;
    }

    /// Enable the multiplexer output (active low)
    pub fn enable(&mut self) {
        self.en.set_low();
    }

    /// Disable the multiplexer output (high impedance)
    pub fn disable(&mut self) {
        self.en.set_high();
    }

    /// Select channel and enable in one call
    pub fn select_and_enable(&mut self, channel: u8) {
        self.select_channel(channel);
        self.enable();
    }

    /// Get the currently selected channel
    pub fn current_channel(&self) -> u8 {
        self.current_channel
    }

    /// Check if the mux is currently enabled
    pub fn is_enabled(&self) -> bool {
        self.en.is_set_low()
    }
}

// ============================================================================
// Status LEDs
// ============================================================================

/// LED identifiers for status reporting
#[derive(Clone, Copy, PartialEq, Format)]
pub enum Led {
    Heartbeat,  // PD12 green
    Network,    // PD13 orange (MQTT/DDS connection)
    Fault,      // PD14 red
    Activity,   // PD15 blue (TX blink / SD / ETH)
}

/// Status LEDs on the STM32F4 Discovery board
///
/// Four LEDs provide visual indication of system state:
///   Green  (heartbeat): 1Hz blink = running, solid = booting
///   Orange (network):   Solid = connected, off = disconnected
///   Red    (fault):     Solid = active alarm, blink = warning
///   Blue   (activity):  Blink on TX/RX, solid = SD card write
pub struct StatusLeds {
    pub heartbeat: Output<'static>,   // PD12 green
    pub network:   Output<'static>,   // PD13 orange
    pub fault:     Output<'static>,   // PD14 red
    pub activity:  Output<'static>,   // PD15 blue
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
            network:   Output::new(pd13, Level::Low, Speed::Low),
            fault:     Output::new(pd14, Level::Low, Speed::Low),
            activity:  Output::new(pd15, Level::Low, Speed::Low),
        }
    }

    /// Set all LEDs to off
    pub fn all_off(&mut self) {
        self.heartbeat.set_low();
        self.network.set_low();
        self.fault.set_low();
        self.activity.set_low();
    }

    /// Boot sequence: flash all LEDs in sequence
    pub fn boot_sequence(&mut self) {
        self.heartbeat.set_high();
        self.network.set_high();
        self.fault.set_high();
        self.activity.set_high();
    }

    /// Set a specific LED on or off
    pub fn set(&mut self, led: Led, on: bool) {
        let output = match led {
            Led::Heartbeat => &mut self.heartbeat,
            Led::Network => &mut self.network,
            Led::Fault => &mut self.fault,
            Led::Activity => &mut self.activity,
        };
        if on {
            output.set_high();
        } else {
            output.set_low();
        }
    }

    /// Toggle a specific LED
    pub fn toggle(&mut self, led: Led) {
        match led {
            Led::Heartbeat => self.heartbeat.toggle(),
            Led::Network => self.network.toggle(),
            Led::Fault => self.fault.toggle(),
            Led::Activity => self.activity.toggle(),
        }
    }

    /// Set LED pattern for a given system state
    pub fn set_system_state(&mut self, state: SystemState) {
        match state {
            SystemState::Booting => {
                self.boot_sequence();
            }
            SystemState::Running => {
                // Heartbeat handled by heartbeat task
                self.fault.set_low();
            }
            SystemState::NetworkConnected => {
                self.network.set_high();
            }
            SystemState::NetworkDisconnected => {
                self.network.set_low();
            }
            SystemState::Fault => {
                self.fault.set_high();
            }
            SystemState::FaultCleared => {
                self.fault.set_low();
            }
        }
    }
}

/// System states for LED indication
#[derive(Clone, Copy, PartialEq, Format)]
pub enum SystemState {
    Booting,
    Running,
    NetworkConnected,
    NetworkDisconnected,
    Fault,
    FaultCleared,
}

// ============================================================================
// RS-485 direction control (SP3485)
// ============================================================================

/// RS-485 transceiver direction mode
#[derive(Clone, Copy, PartialEq, Format)]
pub enum Rs485Direction {
    /// Receiver enabled, driver disabled (listening)
    Receive,
    /// Driver enabled, receiver disabled (transmitting)
    Transmit,
    /// Both driver and receiver disabled (high-Z / power save)
    Disabled,
}

/// RS-485 transceiver direction control for SP3485
///
/// The SP3485 has two control pins:
///   DE  (PB0): Driver Enable, active high - asserted to transmit
///   /RE (PB1): Receiver Enable, active low - asserted to receive
///
/// In a typical half-duplex RS-485 configuration:
///   Receive:  DE=low,  /RE=low  (driver off, receiver on)
///   Transmit: DE=high, /RE=high (driver on, receiver off)
///   Disabled: DE=low,  /RE=high (both off)
pub struct Rs485Control {
    de:  Output<'static>,  // PB0 - Driver Enable (active high)
    nre: Output<'static>,  // PB1 - Receiver Enable (active low)
    direction: Rs485Direction,
}

impl Rs485Control {
    /// Create a new RS-485 control driver, starting in Receive mode
    pub fn new(
        pb0: peripherals::PB0,
        pb1: peripherals::PB1,
    ) -> Self {
        let mut ctrl = Self {
            de:  Output::new(pb0, Level::Low, Speed::High),
            nre: Output::new(pb1, Level::Low, Speed::High),  // Active low = enabled
            direction: Rs485Direction::Receive,
        };
        ctrl.set_direction(Rs485Direction::Receive);
        ctrl
    }

    /// Set the transceiver direction
    pub fn set_direction(&mut self, dir: Rs485Direction) {
        match dir {
            Rs485Direction::Receive => {
                self.de.set_low();   // Driver OFF
                self.nre.set_low();  // Receiver ON (active low)
            }
            Rs485Direction::Transmit => {
                self.nre.set_high(); // Receiver OFF
                self.de.set_high();  // Driver ON
            }
            Rs485Direction::Disabled => {
                self.de.set_low();   // Driver OFF
                self.nre.set_high(); // Receiver OFF
            }
        }
        self.direction = dir;
    }

    /// Switch to transmit mode
    pub fn enable_tx(&mut self) {
        self.set_direction(Rs485Direction::Transmit);
    }

    /// Switch to receive mode
    pub fn enable_rx(&mut self) {
        self.set_direction(Rs485Direction::Receive);
    }

    /// Get current direction
    pub fn direction(&self) -> Rs485Direction {
        self.direction
    }

    /// Check if currently in transmit mode
    pub fn is_transmitting(&self) -> bool {
        self.direction == Rs485Direction::Transmit
    }
}

// ============================================================================
// Boot pin management
// ============================================================================

/// Boot configuration manager
///
/// On STM32F407, BOOT0 (pin PH3 on some boards, or hardware strap) determines
/// the boot source:
///   BOOT0=0: Boot from main flash (normal operation)
///   BOOT0=1: Boot from system memory (bootloader) or SRAM
///
/// This driver manages the BOOT0 pin and provides DFU entry support.
pub struct BootControl {
    /// Track whether we're in application mode or bootloader request pending
    bootloader_requested: bool,
}

impl BootControl {
    pub fn new() -> Self {
        Self {
            bootloader_requested: false,
        }
    }

    /// Request entry to the system bootloader on next reset
    ///
    /// This writes a magic value to a known RAM location that the bootloader
    /// checks on startup. The actual reset is performed separately.
    pub fn request_bootloader(&mut self) {
        // Write magic value to backup register or a known SRAM location
        // that survives a soft reset. The bootloader stub checks this.
        // Address 0x2001FFF0 is at the top of SRAM on STM32F407 (128KB SRAM)
        #[cfg(not(feature = "simulator"))]
        unsafe {
            let magic_addr = 0x2001_FFF0 as *mut u32;
            core::ptr::write_volatile(magic_addr, 0xDEAD_BEEF);
        }
        self.bootloader_requested = true;
        info!("Bootloader entry requested - reset to activate");
    }

    /// Check if bootloader was requested (called by early startup code)
    pub fn check_bootloader_request() -> bool {
        #[cfg(not(feature = "simulator"))]
        unsafe {
            let magic_addr = 0x2001_FFF0 as *const u32;
            let val = core::ptr::read_volatile(magic_addr);
            if val == 0xDEAD_BEEF {
                // Clear the magic value so we don't loop
                let magic_addr = 0x2001_FFF0 as *mut u32;
                core::ptr::write_volatile(magic_addr, 0x0000_0000);
                return true;
            }
        }
        false
    }

    /// Perform a software reset (NVIC system reset)
    pub fn software_reset() -> ! {
        info!("Performing software reset...");
        #[cfg(not(feature = "simulator"))]
        {
            cortex_m::peripheral::SCB::sys_reset();
        }
        #[cfg(feature = "simulator")]
        {
            panic!("Software reset called in simulator");
        }
    }
}

// ============================================================================
// Relay outputs
// ============================================================================

/// Relay outputs for actuator control
pub struct Relays {
    pub pump:  Output<'static>,   // PE0
    pub valve: Output<'static>,   // PE1
    pub spare: Output<'static>,   // PE2
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
        if on { self.pump.set_high(); } else { self.pump.set_low(); }
    }

    pub fn set_valve(&mut self, on: bool) {
        if on { self.valve.set_high(); } else { self.valve.set_low(); }
    }

    pub fn set_spare(&mut self, on: bool) {
        if on { self.spare.set_high(); } else { self.spare.set_low(); }
    }

    /// Set a relay by index (0=pump, 1=valve, 2=spare)
    pub fn set_by_index(&mut self, index: u8, on: bool) {
        match index {
            0 => self.set_pump(on),
            1 => self.set_valve(on),
            2 => self.set_spare(on),
            _ => warn!("Invalid relay index: {}", index),
        }
    }

    /// Get relay state by index
    pub fn get_state(&self, index: u8) -> bool {
        match index {
            0 => self.pump.is_set_high(),
            1 => self.valve.is_set_high(),
            2 => self.spare.is_set_high(),
            _ => false,
        }
    }
}

// ============================================================================
// Digital inputs
// ============================================================================

/// Digital input sensors
pub struct DigitalInputs {
    pub user_button:  Input<'static>,   // PC13
    pub float_switch: Input<'static>,   // PE3 (tank overflow)
    pub flow_pulse:   Input<'static>,   // PE4 (flow meter pulse)
}

impl DigitalInputs {
    pub fn new(
        pc13: peripherals::PC13,
        pe3: peripherals::PE3,
        pe4: peripherals::PE4,
    ) -> Self {
        Self {
            user_button:  Input::new(pc13, Pull::Up),    // Active low
            float_switch: Input::new(pe3, Pull::Up),     // Active low (NC contact)
            flow_pulse:   Input::new(pe4, Pull::Down),   // Pulse input
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

// ============================================================================
// Flow pulse counter (software-based, for use with EXTI interrupt)
// ============================================================================

/// Flow pulse counter state
///
/// Tracks pulses from a flow meter and converts to flow rate.
/// Typically each pulse represents a fixed volume (e.g., 1 pulse = 1 liter).
pub struct FlowPulseCounter {
    /// Total accumulated pulses since reset
    pub total_pulses: u32,
    /// Pulses since last rate calculation
    pub period_pulses: u32,
    /// Calculated flow rate (pulses per second)
    pub flow_rate_pps: f32,
    /// Pulse-to-volume conversion factor (liters per pulse)
    pub liters_per_pulse: f32,
    /// Last rate calculation timestamp (ms)
    pub last_calc_ms: u64,
}

impl FlowPulseCounter {
    pub const fn new(liters_per_pulse: f32) -> Self {
        Self {
            total_pulses: 0,
            period_pulses: 0,
            flow_rate_pps: 0.0,
            liters_per_pulse,
            last_calc_ms: 0,
        }
    }

    /// Called from EXTI interrupt on each rising edge
    pub fn on_pulse(&mut self) {
        self.total_pulses = self.total_pulses.wrapping_add(1);
        self.period_pulses += 1;
    }

    /// Calculate flow rate from accumulated pulses over a time period
    pub fn calculate_rate(&mut self, now_ms: u64) {
        let elapsed_ms = now_ms.saturating_sub(self.last_calc_ms);
        if elapsed_ms > 0 {
            let elapsed_s = elapsed_ms as f32 / 1000.0;
            self.flow_rate_pps = self.period_pulses as f32 / elapsed_s;
            self.period_pulses = 0;
            self.last_calc_ms = now_ms;
        }
    }

    /// Get flow rate in liters per minute
    pub fn flow_rate_lpm(&self) -> f32 {
        self.flow_rate_pps * self.liters_per_pulse * 60.0
    }

    /// Get total volume in liters
    pub fn total_volume_liters(&self) -> f32 {
        self.total_pulses as f32 * self.liters_per_pulse
    }

    /// Reset the total counter
    pub fn reset_total(&mut self) {
        self.total_pulses = 0;
    }
}
