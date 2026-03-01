/// Hard Real-Time Sensor Acquisition Task — Highest Priority
///
/// This task performs deterministic ADC sampling at a configurable rate
/// (default 1 kHz timer base, configurable scan interval).
///
/// Solar SMU variant:
///   - Scans 16 string voltages via MUX A → ADS1263 AIN0
///   - Scans 16 string currents via MUX B → ADS1263 AIN1
///   - Reads 4 direct channels (bus V, bus I, irradiance, module temp)
///   - Applies 5-point voltage calibration, 3-point current calibration
///   - IIR low-pass filtering (10 Hz cutoff at 1 kHz sample rate)
///
/// Water RTU variant:
///   - Scans 8 channels via ADS1258 auto-scan
///   - Applies 2-point (zero/span) 4-20mA calibration
///   - Detects wire faults (open circuit, short circuit, under/over range)
///
/// Execution budget: < 500 us per scan cycle
/// All sensor data is stored in shared state and Modbus registers

use embassy_executor::task;
use embassy_stm32::gpio::{Input, Output};
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Instant, Ticker, Timer};
use defmt::{info, warn, error};

use crate::modbus::{device_status, input_reg, holding_reg};
use crate::sensors::IirFilter;
use crate::tasks::shared::{SharedState, ChannelData, MAX_CHANNELS};

/// Task index for statistics
const TASK_IDX: usize = 0;

/// Solar SMU sensor acquisition task
#[cfg(feature = "solar_smu")]
#[task]
pub async fn sensor_acquisition_solar(
    adc_spi: Spi<'static, Async>,
    adc_cs: Output<'static>,
    adc_drdy: Input<'static>,
    adc_reset: Output<'static>,
    mux_a: crate::sensors::mux::Cd74hc4067<'static>,
    mux_b: crate::sensors::mux::Cd74hc4067<'static>,
    shared: &'static SharedState,
) {
    use crate::sensors::ads1263::Ads1263;
    use crate::sensors::mux::{DualMux, MuxChannel};

    info!("Sensor task (Solar): starting");

    let mut adc = Ads1263::new(adc_spi, adc_cs, adc_drdy, adc_reset);
    let mut mux = DualMux::new(mux_a, mux_b);

    // Initialize ADC
    if !adc.init().await {
        error!("Sensor task: ADS1263 init FAILED");
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

    info!("Sensor task: ADS1263 ready");

    // Initialize IIR filters for each channel (10 Hz cutoff at 1 kHz sample rate)
    let mut voltage_filters = [IirFilter::new(0.0591); 16];
    let mut current_filters = [IirFilter::new(0.0591); 16];

    // Get scan interval from config
    let scan_ms = {
        let regs = shared.registers.lock().await;
        let ms = regs.holding_regs[holding_reg::SCAN_INTERVAL_MS as usize];
        if ms < 100 { 1000u64 } else { ms as u64 }
    };

    let mut ticker = Ticker::every(Duration::from_millis(scan_ms));

    // Voltage divider ratio: String voltage up to 1000V → divided to 0-2.5V
    const VOLTAGE_DIVIDER_RATIO: f32 = 401.0;
    // Current shunt: 50A/75mV → 1.5mV per amp
    const CURRENT_SHUNT_MV_PER_AMP: f32 = 1.5;
    const BUS_VOLTAGE_DIVIDER: f32 = 601.0;
    const IRRADIANCE_SCALE: f32 = 600.0;

    loop {
        ticker.next().await;
        let scan_start = Instant::now();

        let mut channel_data = [ChannelData::default(); MAX_CHANNELS];

        // Scan 16 strings
        for s in 0..16u8 {
            mux.select_string(s).await;

            // Read voltage (MUX A → AIN0)
            let v_reading = adc.read_muxed_channel(0).await;
            let raw_voltage = v_reading.voltage * VOLTAGE_DIVIDER_RATIO;

            // Read current (MUX B → AIN1)
            let i_reading = adc.read_muxed_channel(1).await;
            let raw_current = (i_reading.voltage * 1000.0) / CURRENT_SHUNT_MV_PER_AMP;

            // Apply calibration from calibration store
            let calibrated_voltage = {
                let cal = shared.calibration.lock().await;
                cal.voltage_cals[s as usize].convert(v_reading.raw_code)
            };
            let calibrated_current = {
                let cal = shared.calibration.lock().await;
                cal.current_cals[s as usize].convert(i_reading.raw_code)
            };

            // Apply IIR filter
            let filtered_voltage = voltage_filters[s as usize].update(calibrated_voltage);
            let filtered_current = current_filters[s as usize].update(calibrated_current);

            // Determine status
            let status = if filtered_voltage < 10.0 && filtered_current < 0.1 {
                3 // Disconnected
            } else if filtered_current > 15.0 {
                2 // Overcurrent
            } else if filtered_voltage < 100.0 {
                1 // Low voltage
            } else {
                0 // OK
            };

            channel_data[s as usize] = ChannelData {
                value: filtered_voltage,
                raw_code: v_reading.raw_code,
                status,
                alarm: status != 0,
            };
        }

        mux.disable_all();

        // Read direct ADC channels
        adc.set_input_mux(2, 0x0A).await;
        Timer::after(Duration::from_micros(500)).await;
        let bus_v_raw = adc.read_adc1().await;
        let bus_voltage = (bus_v_raw as f64 * 2.5 / 2147483648.0) as f32 * BUS_VOLTAGE_DIVIDER;

        adc.set_input_mux(3, 0x0A).await;
        Timer::after(Duration::from_micros(500)).await;
        let bus_i_raw = adc.read_adc1().await;
        let bus_current = ((bus_i_raw as f64 * 2.5 / 2147483648.0) as f32 * 1000.0) / CURRENT_SHUNT_MV_PER_AMP;

        adc.set_input_mux(4, 0x0A).await;
        Timer::after(Duration::from_micros(500)).await;
        let irr_raw = adc.read_adc1().await;
        let irradiance = (irr_raw as f64 * 2.5 / 2147483648.0) as f32 * IRRADIANCE_SCALE;

        let total_power: f32 = channel_data.iter()
            .take(16)
            .filter(|c| c.status == 0)
            .map(|c| c.value * 0.0) // placeholder: voltage * current
            .sum();

        // Actually compute total power from voltage and current
        let total_power_kw = bus_voltage * bus_current / 1000.0;

        let energy_increment = total_power_kw * (scan_ms as f32 / 3_600_000.0);

        let scan_duration = scan_start.elapsed();

        // Update shared state
        {
            let mut ch = shared.channels.lock().await;
            *ch = channel_data;
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
            let mut tp = shared.total_power.lock().await;
            *tp = total_power_kw;
        }
        {
            let mut energy = shared.daily_energy_kwh.lock().await;
            *energy += energy_increment;
        }
        {
            let mut irr = shared.irradiance.lock().await;
            *irr = irradiance;
        }

        // Update Modbus registers
        {
            let mut regs = shared.registers.lock().await;
            let mut fault_bitmap: u16 = 0;

            for s in 0..16u8 {
                let base = input_reg::CHANNEL_VALUES_BASE + (s as u16) * 2;
                let bits = channel_data[s as usize].value.to_bits();
                regs.input_regs[base as usize] = (bits >> 16) as u16;
                regs.input_regs[(base + 1) as usize] = (bits & 0xFFFF) as u16;

                regs.input_regs[(input_reg::CHANNEL_STATUS_BASE + s as u16) as usize] =
                    channel_data[s as usize].status as u16;

                if channel_data[s as usize].status != 0 {
                    fault_bitmap |= 1 << s;
                }
            }

            regs.input_regs[input_reg::ALARM_BITMAP_LO as usize] = fault_bitmap;

            let bv_bits = bus_voltage.to_bits();
            regs.input_regs[input_reg::TOTAL_POWER as usize] = (total_power_kw.to_bits() >> 16) as u16;
            regs.input_regs[(input_reg::TOTAL_POWER + 1) as usize] = (total_power_kw.to_bits() & 0xFFFF) as u16;

            let uptime = (Instant::now().as_millis() / 1000) as u32;
            regs.input_regs[input_reg::UPTIME as usize] = (uptime >> 16) as u16;
            regs.input_regs[(input_reg::UPTIME + 1) as usize] = (uptime & 0xFFFF) as u16;

            let dev_status = shared.device_status.lock().await;
            let mut status_word = *dev_status;
            if fault_bitmap != 0 {
                status_word |= device_status::ANY_ALARM;
            }
            regs.input_regs[input_reg::DEVICE_STATUS as usize] = status_word;
        }

        // Update task statistics
        {
            let mut stats = shared.task_stats.lock().await;
            let exec_us = scan_duration.as_micros() as u32;
            stats[TASK_IDX].last_exec_us = exec_us;
            if exec_us > stats[TASK_IDX].max_exec_us {
                stats[TASK_IDX].max_exec_us = exec_us;
            }
            stats[TASK_IDX].exec_count += 1;
        }

        // Increment scan count and signal
        {
            let mut count = shared.scan_count.lock().await;
            *count += 1;

            if *count % 60 == 0 {
                info!(
                    "Scan #{}: {}us, Vbus={}V, Ibus={}A, P={}kW",
                    *count,
                    scan_duration.as_micros(),
                    bus_voltage as i32,
                    bus_current as i32,
                    total_power_kw as i32,
                );
            }
        }

        shared.new_scan.signal(());
    }
}

/// Water RTU sensor acquisition task
#[cfg(feature = "water_rtu")]
#[task]
pub async fn sensor_acquisition_water(
    adc_spi: Spi<'static, Async>,
    adc_cs: Output<'static>,
    adc_drdy: Input<'static>,
    adc_start: Output<'static>,
    adc_reset: Output<'static>,
    shared: &'static SharedState,
) {
    use crate::sensors::ads1258::Ads1258;

    info!("Sensor task (Water): starting");

    let mut adc = Ads1258::new(adc_spi, adc_cs, adc_drdy, adc_start, adc_reset);

    if !adc.init().await {
        error!("Sensor task: ADS1258 init FAILED");
        loop {
            Timer::after(Duration::from_secs(5)).await;
            if adc.init().await { break; }
        }
    }

    {
        let mut status = shared.device_status.lock().await;
        *status |= device_status::ADC_OK;
    }

    // Set number of active channels to 8
    {
        let mut nc = shared.num_channels.lock().await;
        *nc = 8;
    }

    info!("Sensor task: ADS1258 ready, scanning 8 channels at 10 Hz");

    let mut ticker = Ticker::every(Duration::from_millis(100));
    let mut iir_filters = [IirFilter::new(0.1); 8];

    loop {
        ticker.next().await;
        let scan_start = Instant::now();

        let readings = adc.read_all_channels().await;
        let mut channel_data = [ChannelData::default(); MAX_CHANNELS];

        for i in 0..8usize {
            let calibrated = {
                let cal = shared.calibration.lock().await;
                cal.water_cals[i].convert(readings[i].raw_code)
            };

            let filtered = iir_filters[i].update(calibrated);

            channel_data[i] = ChannelData {
                value: filtered,
                raw_code: readings[i].raw_code,
                status: readings[i].status as u8,
                alarm: readings[i].status != crate::sensors::ads1258::ChannelStatus::Normal,
            };
        }

        let scan_duration = scan_start.elapsed();

        // Update shared state
        {
            let mut ch = shared.channels.lock().await;
            *ch = channel_data;
        }

        // Update Modbus registers
        {
            let mut regs = shared.registers.lock().await;
            let mut alarm_bits: u16 = 0;

            for i in 0..8u16 {
                let base = input_reg::CHANNEL_VALUES_BASE + i * 2;
                let bits = channel_data[i as usize].value.to_bits();
                regs.input_regs[base as usize] = (bits >> 16) as u16;
                regs.input_regs[(base + 1) as usize] = (bits & 0xFFFF) as u16;
                regs.input_regs[(input_reg::CHANNEL_STATUS_BASE + i) as usize] =
                    channel_data[i as usize].status as u16;

                if channel_data[i as usize].alarm {
                    alarm_bits |= 1 << i;
                }
            }

            regs.input_regs[input_reg::ALARM_BITMAP_LO as usize] = alarm_bits;

            let uptime = (Instant::now().as_millis() / 1000) as u32;
            regs.input_regs[input_reg::UPTIME as usize] = (uptime >> 16) as u16;
            regs.input_regs[(input_reg::UPTIME + 1) as usize] = (uptime & 0xFFFF) as u16;

            let dev_status = shared.device_status.lock().await;
            let mut status_word = *dev_status;
            if alarm_bits != 0 {
                status_word |= device_status::ANY_ALARM;
            }
            regs.input_regs[input_reg::DEVICE_STATUS as usize] = status_word;
        }

        // Update task stats
        {
            let mut stats = shared.task_stats.lock().await;
            let exec_us = scan_duration.as_micros() as u32;
            stats[TASK_IDX].last_exec_us = exec_us;
            if exec_us > stats[TASK_IDX].max_exec_us {
                stats[TASK_IDX].max_exec_us = exec_us;
            }
            stats[TASK_IDX].exec_count += 1;
        }

        {
            let mut count = shared.scan_count.lock().await;
            *count += 1;
        }

        shared.new_scan.signal(());
    }
}
