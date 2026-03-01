#![no_std]
#![no_main]

/// STM32MP157 Cortex-M4 Real-Time Firmware — DDS-Enabled SCADA
///
/// P9e Microsystems Pvt Ltd
///
/// MCU: STM32MP157C Cortex-M4 co-processor (209 MHz)
/// Communication: OpenAMP RPMsg ↔ A7 Linux ↔ DDS network
///
/// This firmware runs on the M4 core of the dual-core STM32MP157.
/// It performs hard real-time sensor acquisition and forwards data
/// to the A7 Linux core via RPMsg/OpenAMP. The A7 core runs an
/// XRCE-DDS agent that publishes data onto the DDS network.
///
/// Peripheral Configuration:
///   - SPI1: ADC (ADS1263 for Solar, ADS1258 for Water)
///   - I2C1: TMP117 temperature sensor
///   - USART2: RS-485 Modbus RTU slave (legacy interface)
///   - IPCC: Inter-Processor Communication Controller (RPMsg notifications)
///   - ETH MAC: IEEE 1588 PTP timestamp counter (read-only from M4)
///   - TIM2: Sensor acquisition timer (1 kHz base tick)
///   - GPIOs: Status LEDs, MUX control, transceiver DE/RE
///
/// Task Architecture:
///   ┌──────────────────────────┬──────────┬────────────────────────────────────────┐
///   │ Task                     │ Priority │ Function                               │
///   ├──────────────────────────┼──────────┼────────────────────────────────────────┤
///   │ sensor_acquisition       │ Highest  │ ADC scan + calibration + filtering     │
///   │ dds_publish              │ High     │ CDR serialize → RPMsg → A7 XRCE Agent  │
///   │ modbus_slave             │ Normal   │ RS-485 RTU slave (legacy Modbus)       │
///   │ diagnostics              │ Low      │ LEDs, temperature, watchdog, health    │
///   └──────────────────────────┴──────────┴────────────────────────────────────────┘
///
/// Build variants (Cargo features):
///   --features solar_smu  → ADS1263 + 2× CD74HC4067 MUX, 16 string channels
///   --features water_rtu  → ADS1258, 8× 4-20mA channels

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

mod rpmsg;
mod dds;
mod sensors;
mod modbus;
mod ptp;
mod tasks;

use crate::tasks::shared::SharedState;
use crate::rpmsg::RpmsgEndpoint;

/// Global shared state between all tasks
static SHARED: StaticCell<SharedState> = StaticCell::new();

/// Global RPMsg endpoint for DDS communication with A7
static RPMSG: StaticCell<RpmsgEndpoint> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("═══════════════════════════════════════════════");
    info!("  STM32MP157 M4 Real-Time Firmware");
    info!("  DDS-Enabled SCADA Co-Processor");
    info!("  P9e Microsystems Pvt Ltd");
    info!("  Firmware v1.0.0");
    #[cfg(feature = "solar_smu")]
    info!("  Variant: Solar SMU (ADS1263)");
    #[cfg(feature = "water_rtu")]
    info!("  Variant: Water RTU (ADS1258)");
    info!("═══════════════════════════════════════════════");

    // ── Initialize MCU (M4 subsystem clocks) ──
    // On STM32MP157, the M4 clock tree is configured by the A7 (TF-A/U-Boot).
    // The M4 typically runs at 209 MHz from PLL3.
    // We configure only M4-local peripherals here.
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        // M4 core clock is pre-configured by A7 bootloader at 209 MHz
        // APB1 = 104.5 MHz, APB2 = 104.5 MHz
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV2;
        config.rcc.apb2_pre = APBPrescaler::DIV2;
    }
    let p = embassy_stm32::init(config);

    // ── Initialize shared state ──
    let shared = SHARED.init(SharedState::new());

    // ── Initialize RPMsg endpoint ──
    let rpmsg = RPMSG.init(RpmsgEndpoint::new());
    rpmsg.init().await;
    info!("RPMsg: OpenAMP endpoint initialized");

    // ── Status LEDs ──
    // On the SCADA carrier board:
    //   PE0 = Heartbeat (green)
    //   PE1 = Fault (red)
    //   PE2 = TX activity (orange)
    //   PE3 = Link status (blue)
    let mut led_heartbeat = Output::new(p.PE0, Level::Low, Speed::Low);
    let mut led_fault = Output::new(p.PE1, Level::Low, Speed::Low);
    let mut led_tx = Output::new(p.PE2, Level::Low, Speed::Low);
    let mut led_link = Output::new(p.PE3, Level::Low, Speed::Low);

    // Boot indication: all LEDs on briefly
    led_heartbeat.set_high();
    led_fault.set_high();
    led_tx.set_high();
    led_link.set_high();
    Timer::after(Duration::from_millis(500)).await;
    led_heartbeat.set_low();
    led_fault.set_low();
    led_tx.set_low();
    led_link.set_low();

    // ── SPI1: ADC ──
    // SPI1 is assigned to M4 via device tree (m4_spi1)
    let mut spi1_config = spi::Config::default();
    spi1_config.frequency = Hertz(4_000_000);
    let adc_spi = Spi::new(
        p.SPI1, p.PA5, p.PA7, p.PA6,
        p.DMA2_CH3, p.DMA2_CH0,
        spi1_config,
    );
    let adc_cs = Output::new(p.PA4, Level::High, Speed::High);
    let adc_drdy = embassy_stm32::gpio::Input::new(p.PB0, embassy_stm32::gpio::Pull::Up);
    let adc_reset = Output::new(p.PB1, Level::High, Speed::Low);

    // ── Conditional: Solar MUX control pins ──
    #[cfg(feature = "solar_smu")]
    let (mux_a, mux_b) = {
        use crate::sensors::mux::Cd74hc4067;
        let mux_a = Cd74hc4067::new(
            Output::new(p.PC0, Level::Low, Speed::Medium),
            Output::new(p.PC1, Level::Low, Speed::Medium),
            Output::new(p.PC2, Level::Low, Speed::Medium),
            Output::new(p.PC3, Level::Low, Speed::Medium),
            Output::new(p.PC4, Level::High, Speed::Medium),
        );
        let mux_b = Cd74hc4067::new(
            Output::new(p.PC5, Level::Low, Speed::Medium),
            Output::new(p.PC6, Level::Low, Speed::Medium),
            Output::new(p.PC7, Level::Low, Speed::Medium),
            Output::new(p.PC8, Level::Low, Speed::Medium),
            Output::new(p.PC9, Level::High, Speed::Medium),
        );
        (mux_a, mux_b)
    };

    // ── Conditional: Water RTU start pin ──
    #[cfg(feature = "water_rtu")]
    let adc_start = Output::new(p.PB2, Level::Low, Speed::Low);

    // ── I2C1: TMP117 Temperature Sensor ──
    let i2c1 = I2c::new(
        p.I2C1, p.PB6, p.PB7,
        embassy_stm32::Irqs,
        p.DMA1_CH6, p.DMA1_CH0,
        Hertz(400_000),
        Default::default(),
    );

    // ── USART2: RS-485 (Legacy Modbus RTU) ──
    let usart_config = usart::Config::default();
    let rs485_uart = Uart::new(
        p.USART2, p.PA3, p.PA2,
        embassy_stm32::Irqs,
        p.DMA1_CH6, p.DMA1_CH5,
        usart_config,
    );
    let rs485_de = Output::new(p.PA8, Level::Low, Speed::High);

    info!("Peripherals initialized, spawning tasks...");

    // ── Spawn sensor acquisition task ──
    #[cfg(feature = "solar_smu")]
    spawner.must_spawn(tasks::sensor_acquisition::sensor_acquisition_solar(
        adc_spi, adc_cs, adc_drdy, adc_reset,
        mux_a, mux_b, shared,
    ));

    #[cfg(feature = "water_rtu")]
    spawner.must_spawn(tasks::sensor_acquisition::sensor_acquisition_water(
        adc_spi, adc_cs, adc_drdy, adc_start, adc_reset, shared,
    ));

    // ── Spawn DDS publish task ──
    spawner.must_spawn(tasks::dds_publish::dds_publish(rpmsg, shared));

    // ── Spawn Modbus RTU slave task ──
    spawner.must_spawn(tasks::mod_task::modbus_slave(
        rs485_uart, rs485_de, shared,
    ));

    // ── Spawn diagnostics task ──
    spawner.must_spawn(tasks::diagnostics::diagnostics(
        i2c1, led_heartbeat, led_fault, led_tx, led_link, shared,
    ));

    info!("All tasks spawned. STM32MP157 M4 co-processor running.");
}
