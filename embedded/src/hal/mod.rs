/// STM32F407VGT6 Hardware Abstraction Layer for SCADA Sensor Nodes
///
/// ┌─────────────────────────────────────────────────┐
/// │  STM32F407VGT6 Pin Map -- Water/Solar Node      │
/// ├─────────────────────────────────────────────────┤
/// │  SPI1 (ADC):                                    │
/// │    PA5=SCK, PA6=MISO, PA7=MOSI, PA4=CS_ADC     │
/// │    → ADS1263 (Solar) or ADS1258 (Water)         │
/// │                                                  │
/// │  SPI2 (W5500 Ethernet):                         │
/// │    PB13=SCK, PB14=MISO, PB15=MOSI, PB12=CS     │
/// │    → W5500 Ethernet controller                   │
/// │                                                  │
/// │  SPI3 (SD Card):                                │
/// │    PB3=SCK, PB4=MISO, PB5=MOSI, PA15=CS_SD     │
/// │    → MicroSD card (FAT32 logging)                │
/// │                                                  │
/// │  I2C1 (PB6=SCL, PB7=SDA):                      │
/// │    → BME280 (temp/humidity/pressure, 0x76)       │
/// │    → INA219 (current/voltage, 0x40)              │
/// │    → TMP117 (high-accuracy temp, 0x48)           │
/// │                                                  │
/// │  USART2 (PA2=TX, PA3=RX):                       │
/// │    → Modbus RTU via SP3485 RS-485 transceiver    │
/// │                                                  │
/// │  ADC1 (12-bit, up to 2.4 MSPS):                │
/// │    PA0 (ADC1_IN0)  → Pressure sensor (4-20mA)   │
/// │    PA1 (ADC1_IN1)  → pH sensor analog            │
/// │    PA2 (ADC1_IN2)  → Turbidity sensor            │
/// │    PA3 (ADC1_IN3)  → Chlorine sensor             │
/// │    PA4 (ADC1_IN4)  → Flow sensor pulse/analog    │
/// │    PA5 (ADC1_IN5)  → Tank level (4-20mA)        │
/// │    PA6 (ADC1_IN6)  → Solar irradiance            │
/// │    PA7 (ADC1_IN7)  → Panel voltage divider       │
/// │                                                  │
/// │  Multiplexer (CD74HC4067):                       │
/// │    PC6=S0, PC7=S1, PC8=S2, PC9=S3, PC10=EN      │
/// │                                                  │
/// │  RS-485 (SP3485):                                │
/// │    PB0=DE, PB1=/RE                               │
/// │                                                  │
/// │  Ethernet (RMII):                                │
/// │    PA1=ETH_REF_CLK, PA2=ETH_MDIO                 │
/// │    PA7=ETH_CRS_DV, PC1=ETH_MDC                   │
/// │    PC4=ETH_RXD0, PC5=ETH_RXD1                    │
/// │    PB11=ETH_TX_EN, PB12=ETH_TXD0, PB13=ETH_TXD1 │
/// │                                                  │
/// │  GPIO Output:                                    │
/// │    PD12 (LED Green)  → Heartbeat                 │
/// │    PD13 (LED Orange) → Network connected          │
/// │    PD14 (LED Red)    → Fault/alarm               │
/// │    PD15 (LED Blue)   → Activity (TX/SD/ETH)      │
/// │    PE0  → Relay 1 (Pump control)                 │
/// │    PE1  → Relay 2 (Valve control)                │
/// │    PE2  → Relay 3 (Spare)                        │
/// │                                                  │
/// │  GPIO Input:                                     │
/// │    PC13 → User button                            │
/// │    PE3  → Float switch (tank overflow)           │
/// │    PE4  → Flow pulse counter input               │
/// └─────────────────────────────────────────────────┘

pub mod adc;
pub mod gpio;
pub mod i2c_sensors;
pub mod spi;
