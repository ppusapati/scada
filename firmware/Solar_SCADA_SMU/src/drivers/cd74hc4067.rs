/// CD74HC4067 — 16-Channel Analog Multiplexer Driver
///
/// Datasheet: Texas Instruments CD74HC4067
///
/// The CD74HC4067 is a CMOS 16:1 analog MUX/DEMUX with:
///   - 16 channels selected via 4 digital control pins (S0-S3)
///   - Enable pin (active low)
///   - Single common I/O (COM)
///   - Break-before-make switching
///
/// On SS-PCB-001, two MUX chips are used:
///   MUX A (U3): 16 voltage channels → ADS1263 AIN0
///     S0=PC0, S1=PC1, S2=PC2, S3=PC3, EN=PC4
///   MUX B (U4): 16 current channels → ADS1263 AIN1
///     S0=PC5, S1=PC6, S2=PC7, S3=PC8, EN=PC9
///
/// Combined with 4 additional direct ADS1263 inputs, this provides
/// 40 total channels (2×16 MUX + 8 direct).

use embassy_stm32::gpio::Output;
use embassy_time::{Duration, Timer};

/// MUX channel selection (0-15)
#[derive(Clone, Copy)]
pub struct MuxChannel(pub u8);

impl MuxChannel {
    pub const fn new(ch: u8) -> Self {
        Self(ch & 0x0F) // Clamp to 0-15
    }
}

/// Single CD74HC4067 multiplexer
pub struct Cd74hc4067<'a> {
    s0: Output<'a>,
    s1: Output<'a>,
    s2: Output<'a>,
    s3: Output<'a>,
    en: Output<'a>,
}

impl<'a> Cd74hc4067<'a> {
    pub fn new(
        s0: Output<'a>,
        s1: Output<'a>,
        s2: Output<'a>,
        s3: Output<'a>,
        en: Output<'a>,
    ) -> Self {
        let mut mux = Self { s0, s1, s2, s3, en };
        mux.disable();
        mux
    }

    /// Select a channel (0-15) and enable the MUX
    pub fn select(&mut self, ch: MuxChannel) {
        let bits = ch.0;
        if bits & 0x01 != 0 { self.s0.set_high(); } else { self.s0.set_low(); }
        if bits & 0x02 != 0 { self.s1.set_high(); } else { self.s1.set_low(); }
        if bits & 0x04 != 0 { self.s2.set_high(); } else { self.s2.set_low(); }
        if bits & 0x08 != 0 { self.s3.set_high(); } else { self.s3.set_low(); }

        // Enable (active low)
        self.en.set_low();
    }

    /// Disable the MUX (high-impedance output)
    pub fn disable(&mut self) {
        self.en.set_high();
    }

    /// Select channel with settling time
    /// The CD74HC4067 has ~70ns propagation delay, but we add margin
    /// for the analog signal to settle through the external filter.
    pub async fn select_and_settle(&mut self, ch: MuxChannel) {
        self.select(ch);
        // Allow analog settling: RC filter on PCB typically needs 10-50µs
        Timer::after(Duration::from_micros(50)).await;
    }
}

/// Dual MUX manager for SS-PCB-001
/// MUX A handles voltage channels (string voltages)
/// MUX B handles current channels (string currents via shunts)
pub struct DualMux<'a> {
    pub mux_a: Cd74hc4067<'a>,  // Voltage MUX → AIN0
    pub mux_b: Cd74hc4067<'a>,  // Current MUX → AIN1
}

impl<'a> DualMux<'a> {
    pub fn new(mux_a: Cd74hc4067<'a>, mux_b: Cd74hc4067<'a>) -> Self {
        Self { mux_a, mux_b }
    }

    /// Select the same channel on both MUXes (voltage + current for one string)
    pub async fn select_string(&mut self, string_id: u8) {
        let ch = MuxChannel::new(string_id);
        self.mux_a.select_and_settle(ch).await;
        self.mux_b.select_and_settle(ch).await;
    }

    /// Disable both MUXes
    pub fn disable_all(&mut self) {
        self.mux_a.disable();
        self.mux_b.disable();
    }
}

/// Total channel map for SS-PCB-001:
///   Channels 0-15:  MUX A (string voltages) → ADS1263 AIN0
///   Channels 16-31: MUX B (string currents) → ADS1263 AIN1
///   Channels 32-35: Direct ADS1263 inputs (AIN2-AIN5)
///     32 = Bus voltage (AIN2)
///     33 = Bus current (AIN3)
///     34 = Irradiance sensor (AIN4)
///     35 = Module temperature (AIN5) [backup to TMP117]
///   Channels 36-39: Reserved / spare
pub const TOTAL_CHANNELS: usize = 40;
pub const MUX_A_CHANNELS: usize = 16;
pub const MUX_B_CHANNELS: usize = 16;
pub const DIRECT_CHANNELS: usize = 4;
