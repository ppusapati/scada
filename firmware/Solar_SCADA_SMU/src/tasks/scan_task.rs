/// String Scan Task — Highest Priority
///
/// Scans all 16 string voltages (MUX A) and 16 string currents (MUX B)
/// through the ADS1263 ADC, plus 4 direct channels (bus V, bus I, irradiance, temp).
/// Total: 36 channels per scan cycle.
///
/// Default scan interval: 1000ms (configurable via Modbus holding register).
/// MUX switching + ADC settling per channel: ~1ms
/// Full 36-channel scan takes ~40ms, leaving plenty of margin.

use embassy_executor::task;
use embassy_stm32::gpio::{Input, Output};
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Instant, Ticker, Timer};
use defmt::{info, warn};

use crate::drivers::ads1263::Ads1263;
use crate::drivers::cd74hc4067::{Cd74hc4067, DualMux, MuxChannel};
use crate::modbus::registers::{device_status, input_reg, holding_reg};
use crate::tasks::shared::{SharedState, StringData};

/// Voltage divider ratio for string voltage measurement
/// String voltage up to 1000V → divided to 0-2.5V range
/// R1 = 1MΩ, R2 = 2.5kΩ → ratio = 2500 / (1000000 + 2500) = 1/401
const VOLTAGE_DIVIDER_RATIO: f32 = 401.0;

/// Current shunt conversion: 50mV shunt at rated current
/// Shunt: 50A / 75mV → 1A = 1.5mV
const CURRENT_SHUNT_MV_PER_AMP: f32 = 1.5;

/// Bus voltage divider (higher ratio for 0-1500V bus)
const BUS_VOLTAGE_DIVIDER: f32 = 601.0;

/// Irradiance sensor scaling: 0-2.5V = 0-1500 W/m²
const IRRADIANCE_SCALE: f32 = 600.0; // W/m² per volt

/// Fault detection thresholds with hysteresis
const DISCONNECTED_VOLTAGE_THRESHOLD: f32 = 10.0;
const DISCONNECTED_VOLTAGE_RECOVERY: f32 = 15.0; // Must exceed this to clear disconnect
const DISCONNECTED_CURRENT_THRESHOLD: f32 = 0.1;
const LOW_VOLTAGE_THRESHOLD: f32 = 100.0;
const LOW_VOLTAGE_RECOVERY: f32 = 110.0;
const OVERCURRENT_THRESHOLD: f32 = 15.0;
const OVERCURRENT_RECOVERY: f32 = 14.0;

/// Number of consecutive fault readings before triggering a fault
const FAULT_DEBOUNCE_COUNT: u8 = 3;

/// Per-string fault debounce counter
struct FaultDebounce {
    counters: [u8; 16],       // consecutive fault count per string
    current_status: [u8; 16], // confirmed status per string
}

impl FaultDebounce {
    const fn new() -> Self {
        Self {
            counters: [0; 16],
            current_status: [0; 16], // 0 = OK
        }
    }

    /// Update the fault status for a string with debouncing + hysteresis
    fn update(&mut self, string_id: usize, raw_status: u8) -> u8 {
        if raw_status == self.current_status[string_id] {
            // Status unchanged, reset debounce counter
            self.counters[string_id] = 0;
            return self.current_status[string_id];
        }

        // Status changed — increment debounce counter
        self.counters[string_id] += 1;
        if self.counters[string_id] >= FAULT_DEBOUNCE_COUNT {
            // Confirmed status change after N consecutive readings
            self.current_status[string_id] = raw_status;
            self.counters[string_id] = 0;
        }

        self.current_status[string_id]
    }
}

#[task]
pub async fn string_scan(
    adc_spi: Spi<'static, Async>,
    adc_cs: Output<'static>,
    adc_drdy: Input<'static>,
    adc_reset: Output<'static>,
    mux_a: Cd74hc4067<'static>,
    mux_b: Cd74hc4067<'static>,
    shared: &'static SharedState,
) {
    info!("Scan task: starting");

    let mut adc = Ads1263::new(adc_spi, adc_cs, adc_drdy, adc_reset);
    let mut mux = DualMux::new(mux_a, mux_b);

    // Initialize ADC
    if !adc.init().await {
        error!("Scan task: ADS1263 init FAILED");
        loop {
            Timer::after(Duration::from_secs(5)).await;
            if adc.init().await {
                break;
            }
        }
    }

    {
        let mut status = shared.device_status.lock().await;
        *status |= device_status::ADC_OK;
    }

    info!("Scan task: ADS1263 ready, scanning 16 strings + 4 direct channels");

    // Get scan interval from config
    let scan_ms = {
        let regs = shared.registers.lock().await;
        let ms = regs.holding_regs[holding_reg::SCAN_INTERVAL_MS as usize];
        if ms < 100 { 1000u64 } else { ms as u64 }
    };

    let mut ticker = Ticker::every(Duration::from_millis(scan_ms));
    let mut scan_count: u32 = 0;
    let mut fault_debounce = FaultDebounce::new();

    // Load calibration offsets from holding registers
    let (vcal_offsets, ical_offsets) = {
        let regs = shared.registers.lock().await;
        let mut vcal = [0.0f32; 16];
        let mut ical = [0.0f32; 16];
        for s in 0..16usize {
            vcal[s] = regs.get_float(&regs.holding_regs, holding_reg::VCAL_BASE + (s as u16) * 2);
            ical[s] = regs.get_float(&regs.holding_regs, holding_reg::ICAL_BASE + (s as u16) * 2);
        }
        (vcal, ical)
    };

    loop {
        ticker.next().await;
        scan_count += 1;

        let scan_start = Instant::now();

        // ── Scan 16 strings: voltage (MUX A → AIN0) + current (MUX B → AIN1) ──
        let mut string_data = [StringData::default(); 16];

        for s in 0..16u8 {
            // Select MUX channel (both A and B simultaneously)
            mux.select_string(s).await;

            // Read voltage on AIN0 (MUX A output)
            let v_reading = adc.read_muxed_channel(0).await;
            let string_voltage = v_reading.voltage * VOLTAGE_DIVIDER_RATIO + vcal_offsets[s as usize];

            // Read current on AIN1 (MUX B output)
            let i_reading = adc.read_muxed_channel(1).await;
            let string_current = (i_reading.voltage * 1000.0) / CURRENT_SHUNT_MV_PER_AMP + ical_offsets[s as usize];

            let power = string_voltage * string_current;

            // Determine raw fault status with hysteresis
            let prev_status = fault_debounce.current_status[s as usize];
            let raw_status = if string_voltage < DISCONNECTED_VOLTAGE_THRESHOLD && string_current < DISCONNECTED_CURRENT_THRESHOLD {
                3 // Disconnected
            } else if prev_status == 3 && string_voltage < DISCONNECTED_VOLTAGE_RECOVERY {
                3 // Still disconnected (hysteresis: need to exceed recovery threshold to clear)
            } else if string_current > OVERCURRENT_THRESHOLD {
                2 // Overcurrent
            } else if prev_status == 2 && string_current > OVERCURRENT_RECOVERY {
                2 // Still overcurrent (hysteresis)
            } else if string_voltage < LOW_VOLTAGE_THRESHOLD {
                1 // Low voltage
            } else if prev_status == 1 && string_voltage < LOW_VOLTAGE_RECOVERY {
                1 // Still low voltage (hysteresis)
            } else {
                0 // OK
            };

            // Apply debounce — require N consecutive readings before changing status
            let status = fault_debounce.update(s as usize, raw_status);

            string_data[s as usize] = StringData {
                voltage: string_voltage,
                current: string_current,
                power,
                status,
            };
        }

        // Disable MUXes before direct readings
        mux.disable_all();

        // ── Read direct ADC channels ──

        // AIN2: Bus voltage
        adc.set_input_mux(2, 0x0A).await;
        Timer::after(Duration::from_micros(500)).await;
        let bus_v_raw = adc.read_adc1().await;
        let bus_voltage = (bus_v_raw as f64 * 2.5 / 2147483648.0) as f32 * BUS_VOLTAGE_DIVIDER;

        // AIN3: Bus current
        adc.set_input_mux(3, 0x0A).await;
        Timer::after(Duration::from_micros(500)).await;
        let bus_i_raw = adc.read_adc1().await;
        let bus_current = ((bus_i_raw as f64 * 2.5 / 2147483648.0) as f32 * 1000.0) / CURRENT_SHUNT_MV_PER_AMP;

        // AIN4: Irradiance
        adc.set_input_mux(4, 0x0A).await;
        Timer::after(Duration::from_micros(500)).await;
        let irr_raw = adc.read_adc1().await;
        let irradiance = (irr_raw as f64 * 2.5 / 2147483648.0) as f32 * IRRADIANCE_SCALE;

        // AIN5: Module temperature (backup RTD)
        adc.set_input_mux(5, 0x0A).await;
        Timer::after(Duration::from_micros(500)).await;
        let temp_raw = adc.read_adc1().await;
        // Approximate PT100: V = I_exc × R, R ≈ 100 + 0.385 × T
        let temp_voltage = (temp_raw as f64 * 2.5 / 2147483648.0) as f32;
        let module_temp = (temp_voltage / 0.001 - 100.0) / 0.385; // 1mA excitation assumed

        let scan_duration = scan_start.elapsed();

        // ── Compute totals ──
        let total_power: f32 = string_data.iter().map(|s| s.power).sum();

        // Accumulate energy (Wh)
        let energy_increment = total_power * (scan_ms as f32 / 3_600_000.0);

        // ── Update shared state ──
        {
            let mut strings = shared.strings.lock().await;
            *strings = string_data;
        }
        {
            let mut bv = shared.bus_voltage.lock().await;
            *bv = bus_voltage;
        }
        {
            let mut bi = shared.bus_current.lock().await;
            *bi = bus_current;
        }
        {
            let mut irr = shared.irradiance.lock().await;
            *irr = irradiance;
        }
        {
            let mut mt = shared.module_temp.lock().await;
            *mt = module_temp;
        }
        {
            let mut energy = shared.daily_energy_wh.lock().await;
            *energy += energy_increment;
        }

        // ── Update Modbus registers ──
        {
            let mut regs = shared.registers.lock().await;

            // String measurements
            let mut fault_bitmap: u16 = 0;
            for s in 0..16u8 {
                regs.set_string_voltage(s, string_data[s as usize].voltage);
                regs.set_string_current(s, string_data[s as usize].current);
                regs.set_string_power(s, string_data[s as usize].power);
                if string_data[s as usize].status != 0 {
                    fault_bitmap |= 1 << s;
                }
            }

            // Bus / plant measurements
            regs.set_bus_voltage(bus_voltage);
            regs.set_bus_current(bus_current);
            regs.set_total_power(total_power);

            // Irradiance
            let irr_bits = irradiance.to_bits();
            regs.input_regs[input_reg::IRRADIANCE as usize] = (irr_bits >> 16) as u16;
            regs.input_regs[(input_reg::IRRADIANCE + 1) as usize] = (irr_bits & 0xFFFF) as u16;

            // Module temp
            let mt_bits = module_temp.to_bits();
            regs.input_regs[input_reg::MODULE_TEMP as usize] = (mt_bits >> 16) as u16;
            regs.input_regs[(input_reg::MODULE_TEMP + 1) as usize] = (mt_bits & 0xFFFF) as u16;

            // String fault bitmap
            regs.input_regs[input_reg::STRING_STATUS as usize] = fault_bitmap;

            // Uptime
            let uptime = (Instant::now().as_millis() / 1000) as u32;
            regs.input_regs[input_reg::UPTIME as usize] = (uptime >> 16) as u16;
            regs.input_regs[(input_reg::UPTIME + 1) as usize] = (uptime & 0xFFFF) as u16;

            // Daily energy
            let energy_kwh = {
                let e = shared.daily_energy_wh.lock().await;
                *e / 1000.0
            };
            let ek_bits = energy_kwh.to_bits();
            regs.input_regs[input_reg::DAILY_ENERGY as usize] = (ek_bits >> 16) as u16;
            regs.input_regs[(input_reg::DAILY_ENERGY + 1) as usize] = (ek_bits & 0xFFFF) as u16;

            // Device status
            let dev_status = shared.device_status.lock().await;
            let mut status_word = *dev_status;
            if fault_bitmap != 0 {
                status_word |= device_status::ANY_ALARM;
            }
            regs.input_regs[input_reg::DEVICE_STATUS as usize] = status_word;
        }

        // Signal other tasks
        shared.new_scan.signal(());

        // Log timing every 60 scans
        if scan_count % 60 == 0 {
            info!(
                "Scan #{}: {}ms, Ptotal={}W, Vbus={}V, Ibus={}A, Irr={}W/m²",
                scan_count,
                scan_duration.as_millis(),
                total_power as i32,
                bus_voltage as i32,
                bus_current as i32,
                irradiance as i32,
            );
        }
    }
}
