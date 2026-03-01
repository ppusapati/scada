#![no_std]
#![no_main]

/// Solar SCADA SMU Firmware — SS-PCB-001
///
/// P9e Microsystems Pvt Ltd
///
/// Board: SS-PCB-001  MCU: STM32F407VGT6
/// Key Peripherals:
///   - ADS1263 (SPI1): 32-bit delta-sigma ADC for high-precision measurement
///   - 2× CD74HC4067 (GPIO): 16-channel analog MUX (voltage + current)
///   - TMP117 (I2C1): High-accuracy ambient temperature sensor
///   - MicroSD (SPI3): Data logging (FAT32 CSV files)
///   - SP3485 (USART2): RS-485 transceiver for Modbus RTU slave
///
/// Task Structure:
///   ┌──────────────────────┬──────────┬────────────────────────────────────┐
///   │ Task                 │ Priority │ Function                           │
///   ├──────────────────────┼──────────┼────────────────────────────────────┤
///   │ string_scan          │ Highest  │ Scan 16 strings + 4 direct ch     │
///   │ modbus_slave         │ High     │ RS-485 Modbus RTU slave            │
///   │ data_logger          │ Normal   │ Log to MicroSD every 60s           │
///   │ diagnostics          │ Low      │ TMP117, LEDs, alarms               │
///   └──────────────────────┴──────────┴────────────────────────────────────┘

use embassy_executor::Spawner;
use embassy_stm32::{self, Config};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::usart::{self, Uart};
use embassy_stm32::i2c::I2c;
use embassy_stm32::time::Hertz;
use embassy_time::{Duration, Timer};
use defmt::*;
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

mod drivers;
mod hal;
mod modbus;
mod tasks;
mod storage;

use crate::drivers::cd74hc4067::Cd74hc4067;
use crate::tasks::shared::SharedState;

static SHARED: StaticCell<SharedState> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("═══════════════════════════════════════");
    info!("  Solar SCADA SMU — SS-PCB-001");
    info!("  P9e Microsystems Pvt Ltd");
    info!("  Firmware v1.0.0");
    info!("═══════════════════════════════════════");

    // ── Initialize MCU ──
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: Hertz(8_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll_src = PllSource::HSE;
        config.rcc.pll = Some(Pll {
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL168,
            divp: Some(PllPDiv::DIV2),
            divq: Some(PllQDiv::DIV7),
            divr: None,
        });
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV4;
        config.rcc.apb2_pre = APBPrescaler::DIV2;
        config.rcc.sys = Sysclk::PLL1_P;
    }
    let p = embassy_stm32::init(config);

    // ── Shared state ──
    let shared = SHARED.init(SharedState::new());

    // ── Load defaults and calibration ──
    {
        let mut regs = shared.registers.lock().await;
        regs.load_defaults();
    }
    // Attempt to load calibration from flash
    storage::flash_cal::load_calibration_from_flash(shared).await;

    // ── Status LEDs ──
    let mut led_green = Output::new(p.PD12, Level::Low, Speed::Low);
    let mut led_orange = Output::new(p.PD13, Level::Low, Speed::Low);
    let mut led_red = Output::new(p.PD14, Level::Low, Speed::Low);
    let mut led_blue = Output::new(p.PD15, Level::Low, Speed::Low);

    // Boot flash
    led_green.set_high();
    led_orange.set_high();
    led_red.set_high();
    led_blue.set_high();
    Timer::after(Duration::from_millis(500)).await;
    led_green.set_low();
    led_orange.set_low();
    led_red.set_low();
    led_blue.set_low();

    // ── SPI1: ADS1263 ADC ──
    let mut spi1_config = spi::Config::default();
    spi1_config.frequency = Hertz(4_000_000);
    let ads_spi = Spi::new(
        p.SPI1, p.PA5, p.PA7, p.PA6,
        p.DMA2_CH3, p.DMA2_CH0,
        spi1_config,
    );
    let ads_cs = Output::new(p.PA4, Level::High, Speed::High);
    let ads_drdy = embassy_stm32::gpio::Input::new(p.PB0, embassy_stm32::gpio::Pull::Up);
    let ads_reset = Output::new(p.PB1, Level::High, Speed::Low);

    // ── MUX A: CD74HC4067 (Voltage channels) ──
    let mux_a = Cd74hc4067::new(
        Output::new(p.PC0, Level::Low, Speed::Medium),  // S0
        Output::new(p.PC1, Level::Low, Speed::Medium),  // S1
        Output::new(p.PC2, Level::Low, Speed::Medium),  // S2
        Output::new(p.PC3, Level::Low, Speed::Medium),  // S3
        Output::new(p.PC4, Level::High, Speed::Medium), // EN (high = disabled)
    );

    // ── MUX B: CD74HC4067 (Current channels) ──
    let mux_b = Cd74hc4067::new(
        Output::new(p.PC5, Level::Low, Speed::Medium),
        Output::new(p.PC6, Level::Low, Speed::Medium),
        Output::new(p.PC7, Level::Low, Speed::Medium),
        Output::new(p.PC8, Level::Low, Speed::Medium),
        Output::new(p.PC9, Level::High, Speed::Medium),
    );

    // ── I2C1: TMP117 Temperature Sensor ──
    let i2c1 = I2c::new(
        p.I2C1, p.PB6, p.PB7,
        embassy_stm32::Irqs,
        p.DMA1_CH6, p.DMA1_CH0,
        Hertz(400_000),
        Default::default(),
    );

    // ── SPI3: MicroSD Card ──
    let mut spi3_config = spi::Config::default();
    spi3_config.frequency = Hertz(400_000); // Start slow, speed up after init
    let sd_spi = Spi::new(
        p.SPI3, p.PB3, p.PB5, p.PB4,
        p.DMA1_CH5, p.DMA1_CH2,
        spi3_config,
    );
    let sd_cs = Output::new(p.PA15, Level::High, Speed::High);
    let sd_detect = embassy_stm32::gpio::Input::new(p.PD2, embassy_stm32::gpio::Pull::Up);

    // ── USART2: RS-485 ──
    let usart_config = usart::Config::default();
    let rs485_uart = Uart::new(
        p.USART2, p.PA3, p.PA2,
        embassy_stm32::Irqs,
        p.DMA1_CH6, p.DMA1_CH5,
        usart_config,
    );
    let rs485_de = Output::new(p.PA8, Level::Low, Speed::High);

    info!("Peripherals initialized, spawning tasks...");

    // ── Spawn tasks ──

    spawner.must_spawn(tasks::scan_task::string_scan(
        ads_spi, ads_cs, ads_drdy, ads_reset,
        mux_a, mux_b, shared,
    ));

    spawner.must_spawn(tasks::modbus_task::modbus_slave(
        rs485_uart, rs485_de, shared,
    ));

    spawner.must_spawn(tasks::logging_task::data_logger(
        sd_spi, sd_cs, sd_detect, shared,
    ));

    spawner.must_spawn(tasks::diagnostics_task::diagnostics(
        i2c1, led_green, led_orange, led_red, led_blue, shared,
    ));

    info!("All tasks spawned. Solar SCADA SMU running.");
}
