#![no_std]
#![no_main]

/// Water SCADA RTU Firmware — WS-PCB-001
///
/// P9e Microsystems Pvt Ltd
///
/// Board: WS-PCB-001  MCU: STM32F407VGT6
/// Key Peripherals:
///   - ADS1258 (SPI1): 16-ch delta-sigma ADC for 8× 4-20mA inputs at 10 Hz
///   - SP3485 (USART2): RS-485 transceiver for Modbus RTU slave
///   - W5500  (SPI2): Hardwired TCP/IP Ethernet for Modbus TCP server
///   - SWD (J5): ST-Link V2 debug interface
///
/// Embassy Task Structure (mirroring FreeRTOS priorities):
///   ┌──────────────────────┬──────────┬─────────────────────────────────────┐
///   │ Task                 │ Priority │ Function                            │
///   ├──────────────────────┼──────────┼─────────────────────────────────────┤
///   │ adc_acquisition      │ Highest  │ Read 8× 4-20mA channels at 10 Hz   │
///   │ modbus_rtu           │ High     │ RS-485 slave, respond to polls      │
///   │ modbus_tcp           │ Normal   │ Ethernet TCP/502 server             │
///   │ diagnostics          │ Low      │ LED management, alarm processing    │
///   └──────────────────────┴──────────┴─────────────────────────────────────┘

use embassy_executor::Spawner;
use embassy_stm32::{self, Config};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::usart::{self, Uart};
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

use hal::pins;
use tasks::shared::SharedState;

/// Global shared state between tasks
static SHARED: StaticCell<SharedState> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("═══════════════════════════════════════");
    info!("  Water SCADA RTU — WS-PCB-001");
    info!("  P9e Microsystems Pvt Ltd");
    info!("  Firmware v1.0.0");
    info!("═══════════════════════════════════════");

    // ── Initialize MCU ──
    let mut config = Config::default();
    // Configure clocks: HSE 8MHz → PLL → 168MHz SYSCLK
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
            divp: Some(PllPDiv::DIV2),  // 168 MHz
            divq: Some(PllQDiv::DIV7),  // 48 MHz for USB
            divr: None,
        });
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV4;  // 42 MHz
        config.rcc.apb2_pre = APBPrescaler::DIV2;  // 84 MHz
        config.rcc.sys = Sysclk::PLL1_P;
    }
    let p = embassy_stm32::init(config);

    // ── Initialize shared state ──
    let shared = SHARED.init(SharedState::new());

    // ── Status LEDs ──
    let mut led_green = Output::new(p.PD12, Level::Low, Speed::Low);
    let mut led_orange = Output::new(p.PD13, Level::Low, Speed::Low);
    let mut led_red = Output::new(p.PD14, Level::Low, Speed::Low);
    let mut led_blue = Output::new(p.PD15, Level::Low, Speed::Low);

    // Boot indication: all LEDs on briefly
    led_green.set_high();
    led_orange.set_high();
    led_red.set_high();
    led_blue.set_high();
    Timer::after(Duration::from_millis(500)).await;
    led_green.set_low();
    led_orange.set_low();
    led_red.set_low();
    led_blue.set_low();

    info!("Peripherals initialized, starting tasks...");

    // ── SPI1: ADS1258 ADC ──
    // PA5=SCK, PA6=MISO, PA7=MOSI, PA4=CS (directly driven)
    let mut spi1_config = spi::Config::default();
    spi1_config.frequency = Hertz(4_000_000); // 4 MHz SPI clock
    let ads_spi = Spi::new(
        p.SPI1, p.PA5, p.PA7, p.PA6,
        p.DMA2_CH3, p.DMA2_CH0,
        spi1_config,
    );
    let ads_cs = Output::new(p.PA4, Level::High, Speed::High);
    let ads_drdy = embassy_stm32::gpio::Input::new(p.PB0, embassy_stm32::gpio::Pull::Up);
    let ads_start = Output::new(p.PB1, Level::Low, Speed::Low);
    let ads_reset = Output::new(p.PB2, Level::High, Speed::Low);

    // ── SPI2: W5500 Ethernet ──
    // PB13=SCK, PB14=MISO, PB15=MOSI, PB12=CS
    let mut spi2_config = spi::Config::default();
    spi2_config.frequency = Hertz(20_000_000); // 20 MHz for W5500
    let w5500_spi = Spi::new(
        p.SPI2, p.PB13, p.PB15, p.PB14,
        p.DMA1_CH4, p.DMA1_CH3,
        spi2_config,
    );
    let w5500_cs = Output::new(p.PB12, Level::High, Speed::High);
    let w5500_rst = Output::new(p.PC6, Level::High, Speed::Low);

    // ── USART2: RS-485 via SP3485 ──
    // PA2=TX, PA3=RX, PA8=DE/RE (direction control)
    let usart_config = usart::Config::default();
    let rs485_uart = Uart::new(
        p.USART2, p.PA3, p.PA2,
        embassy_stm32::Irqs,
        p.DMA1_CH6, p.DMA1_CH5,
        usart_config,
    );
    let rs485_de = Output::new(p.PA8, Level::Low, Speed::High); // DE/RE pin

    // ── Spawn tasks ──

    // ADC acquisition — highest priority, 10 Hz sampling
    spawner.must_spawn(tasks::adc_task::adc_acquisition(
        ads_spi, ads_cs, ads_drdy, ads_start, ads_reset, shared,
    ));

    // Modbus RTU — high priority, RS-485 slave
    spawner.must_spawn(tasks::modbus_rtu_task::modbus_rtu_slave(
        rs485_uart, rs485_de, shared,
    ));

    // Modbus TCP — normal priority, W5500 Ethernet
    spawner.must_spawn(tasks::modbus_tcp_task::modbus_tcp_server(
        w5500_spi, w5500_cs, w5500_rst, shared,
    ));

    // Diagnostics — lowest priority, LED + alarms
    spawner.must_spawn(tasks::diagnostics_task::diagnostics(
        led_green, led_orange, led_red, led_blue, shared,
    ));

    info!("All tasks spawned. Water SCADA RTU running.");
}
