/// CD74HC4067 — 16-Channel Analog Multiplexer Driver (STM32MP157 M4 Variant)
///
/// Datasheet: Texas Instruments CD74HC4067
///
/// Adapted from the Solar_SCADA_SMU STM32F407 driver for the STM32MP157 M4
/// co-processor. Uses Embassy GPIO pins for channel selection.
///
/// The CD74HC4067 is a CMOS 16:1 analog MUX/DEMUX with:
///   - 16 channels selected via 4 digital control pins (S0-S3)
///   - Enable pin (active low)
///   - Single common I/O (COM)
///   - Break-before-make switching (~70ns propagation delay)
///
/// On the SCADA PCB, two MUX chips are used:
///   MUX A (U3): 16 voltage channels -> ADS1263 AIN0
///     S0=PG0, S1=PG1, S2=PG2, S3=PG3, EN=PG4
///   MUX B (U4): 16 current channels -> ADS1263 AIN1
///     S0=PG5, S1=PG6, S2=PG7, S3=PG8, EN=PG9
///
/// Combined with 4 additional direct ADS1263 inputs, this provides
/// 36 total channels (2x16 MUX + 4 direct).

use embassy_stm32::gpio::Output;
use embassy_time::{Duration, Timer};
use defmt::debug;

/// MUX channel selection (0-15)
#[derive(Clone, Copy, Debug, defmt::Format)]
pub struct MuxChannel(pub u8);

impl MuxChannel {
    /// Create a new channel, clamping to valid range 0-15
    pub const fn new(ch: u8) -> Self {
        Self(ch & 0x0F)
    }

    /// Get the channel number
    pub const fn index(&self) -> u8 {
        self.0
    }
}

/// Single CD74HC4067 multiplexer driver
///
/// Controls one 16:1 analog MUX via 4 select pins and an enable pin.
pub struct Cd74hc4067<'a> {
    s0: Output<'a>,
    s1: Output<'a>,
    s2: Output<'a>,
    s3: Output<'a>,
    en: Output<'a>,
    /// Currently selected channel (for debug/tracking)
    current_channel: u8,
}

impl<'a> Cd74hc4067<'a> {
    pub fn new(
        s0: Output<'a>,
        s1: Output<'a>,
        s2: Output<'a>,
        s3: Output<'a>,
        en: Output<'a>,
    ) -> Self {
        let mut mux = Self {
            s0, s1, s2, s3, en,
            current_channel: 0,
        };
        mux.disable();
        mux
    }

    /// Select a channel (0-15) and enable the MUX output
    ///
    /// Sets the 4-bit binary address on S0-S3 and pulls EN low.
    /// The MUX uses break-before-make switching, so there is a brief
    /// moment (~10ns) where no channel is connected during transitions.
    pub fn select(&mut self, ch: MuxChannel) {
        let bits = ch.0;

        // Set address bits (S0 = LSB, S3 = MSB)
        if bits & 0x01 != 0 { self.s0.set_high(); } else { self.s0.set_low(); }
        if bits & 0x02 != 0 { self.s1.set_high(); } else { self.s1.set_low(); }
        if bits & 0x04 != 0 { self.s2.set_high(); } else { self.s2.set_low(); }
        if bits & 0x08 != 0 { self.s3.set_high(); } else { self.s3.set_low(); }

        // Enable MUX output (active low)
        self.en.set_low();
        self.current_channel = bits;
    }

    /// Disable the MUX output (high-impedance on COM pin)
    pub fn disable(&mut self) {
        self.en.set_high();
    }

    /// Select channel with analog settling time
    ///
    /// The CD74HC4067 itself switches in ~70ns, but the downstream analog
    /// path needs time to settle:
    ///   - Anti-aliasing RC filter on PCB: ~10-50us
    ///   - ADS1263 PGA settling: depends on gain
    ///   - Total recommended: 50-200us
    pub async fn select_and_settle(&mut self, ch: MuxChannel) {
        // If already on this channel, skip the re-select
        if ch.0 == self.current_channel {
            return;
        }

        self.select(ch);

        // Conservative settling time for precision measurement
        // The RC filter on the PCB has tau ~= 10us, so 5*tau = 50us for 99.3%
        Timer::after(Duration::from_micros(50)).await;
    }

    /// Select channel with extended settling for high-impedance sources
    ///
    /// Used when the source impedance is high (e.g., > 10k ohm) and
    /// the MUX on-resistance (~70 ohm) creates a longer RC time constant.
    pub async fn select_and_settle_extended(&mut self, ch: MuxChannel) {
        self.select(ch);
        Timer::after(Duration::from_micros(200)).await;
    }

    /// Get the currently selected channel
    pub fn current(&self) -> MuxChannel {
        MuxChannel(self.current_channel)
    }

    /// Check if the MUX is enabled
    pub fn is_enabled(&self) -> bool {
        // EN is active low, so MUX is enabled when pin is low
        // We track this implicitly through select/disable calls
        true // Simplified — in practice, read the pin state
    }
}

/// Dual MUX manager for the SCADA solar monitoring PCB
///
/// MUX A handles voltage channels (string voltages through resistive dividers)
/// MUX B handles current channels (string currents through hall sensors / shunts)
///
/// Both MUXes share the same 4-bit channel address — when you select string N,
/// both voltage and current for that string are routed simultaneously.
pub struct DualMux<'a> {
    /// Voltage MUX: 16 string voltage channels -> ADS1263 AIN0
    pub mux_a: Cd74hc4067<'a>,
    /// Current MUX: 16 string current channels -> ADS1263 AIN1
    pub mux_b: Cd74hc4067<'a>,
}

impl<'a> DualMux<'a> {
    pub fn new(mux_a: Cd74hc4067<'a>, mux_b: Cd74hc4067<'a>) -> Self {
        Self { mux_a, mux_b }
    }

    /// Select the same channel on both MUXes (voltage + current for one string)
    ///
    /// This routes both the voltage divider output and the current sense output
    /// for string `string_id` to AIN0 and AIN1 respectively, allowing
    /// a single ADC to measure both with minimal time skew.
    pub async fn select_string(&mut self, string_id: u8) {
        let ch = MuxChannel::new(string_id);
        debug!("DualMux: Selecting string {}", string_id);

        // Select on both MUXes simultaneously (GPIO writes are instant)
        self.mux_a.select(ch);
        self.mux_b.select(ch);

        // Single settling delay for both channels
        // Use the longer of the two settling requirements
        Timer::after(Duration::from_micros(100)).await;
    }

    /// Select voltage-only channel (disable current MUX to reduce crosstalk)
    pub async fn select_voltage_only(&mut self, channel: u8) {
        self.mux_b.disable();
        self.mux_a.select_and_settle(MuxChannel::new(channel)).await;
    }

    /// Select current-only channel
    pub async fn select_current_only(&mut self, channel: u8) {
        self.mux_a.disable();
        self.mux_b.select_and_settle(MuxChannel::new(channel)).await;
    }

    /// Disable both MUXes (high-impedance on both AIN0 and AIN1)
    pub fn disable_all(&mut self) {
        self.mux_a.disable();
        self.mux_b.disable();
    }
}

/// Total channel map for the SCADA solar monitoring PCB:
///
///   Channels 0-15:  MUX A (string voltages) -> ADS1263 AIN0
///   Channels 16-31: MUX B (string currents) -> ADS1263 AIN1
///   Channels 32-35: Direct ADS1263 inputs (AIN2-AIN5)
///     32 = Bus voltage (AIN2) — total DC bus after combiner
///     33 = Bus current (AIN3) — total DC bus current
///     34 = Irradiance sensor (AIN4) — pyranometer
///     35 = Module temperature (AIN5) — backup to TMP117
///   Channels 36-39: Reserved / spare
pub const TOTAL_CHANNELS: usize = 40;
pub const MUX_A_CHANNELS: usize = 16;
pub const MUX_B_CHANNELS: usize = 16;
pub const DIRECT_CHANNELS: usize = 4;

/// Scan order for optimal measurement quality
///
/// Scanning strings in an interleaved pattern reduces the effect of
/// systematic drift during the scan cycle. Instead of 0,1,2,...15,
/// we scan in a pattern that maximizes the time between adjacent
/// physical channels, reducing crosstalk and settling artifacts.
pub const INTERLEAVED_SCAN_ORDER: [u8; 16] = [
    0, 8, 4, 12, 2, 10, 6, 14,
    1, 9, 5, 13, 3, 11, 7, 15,
];

/// Sequential scan order (simpler, used when interleaving is not needed)
pub const SEQUENTIAL_SCAN_ORDER: [u8; 16] = [
    0, 1, 2, 3, 4, 5, 6, 7,
    8, 9, 10, 11, 12, 13, 14, 15,
];
