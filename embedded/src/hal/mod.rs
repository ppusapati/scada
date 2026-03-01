/// STM32F407VGT6 Pin Assignments for SCADA Sensor Nodes
///
/// ┌─────────────────────────────────────────────────┐
/// │  STM32F407VGT6 Pin Map — Water/Solar Node       │
/// ├─────────────────────────────────────────────────┤
/// │  ADC1 (12-bit, up to 2.4 MSPS):                │
/// │    PA0 (ADC1_IN0)  → Pressure sensor (4-20mA)  │
/// │    PA1 (ADC1_IN1)  → pH sensor analog           │
/// │    PA2 (ADC1_IN2)  → Turbidity sensor           │
/// │    PA3 (ADC1_IN3)  → Chlorine sensor            │
/// │    PA4 (ADC1_IN4)  → Flow sensor pulse/analog   │
/// │    PA5 (ADC1_IN5)  → Tank level (4-20mA)       │
/// │    PA6 (ADC1_IN6)  → Solar irradiance           │
/// │    PA7 (ADC1_IN7)  → Panel voltage divider      │
/// │                                                  │
/// │  I2C1 (PB6=SCL, PB7=SDA):                      │
/// │    → Temperature sensor (DS18B20 / BME280)      │
/// │    → Current sensor (INA219)                     │
/// │                                                  │
/// │  SPI1 (PA5=SCK, PA6=MISO, PA7=MOSI, PA4=CS):   │
/// │    → External ADC (ADS1256) for high-precision   │
/// │                                                  │
/// │  UART2 (PA2=TX, PA3=RX):                        │
/// │    → Modbus RTU to PLCs / flow meters            │
/// │                                                  │
/// │  Ethernet (RMII):                                │
/// │    PA1=ETH_REF_CLK, PA2=ETH_MDIO                │
/// │    PA7=ETH_CRS_DV, PC1=ETH_MDC                  │
/// │    PC4=ETH_RXD0, PC5=ETH_RXD1                   │
/// │    PB11=ETH_TX_EN, PB12=ETH_TXD0, PB13=ETH_TXD1│
/// │                                                  │
/// │  GPIO Output:                                    │
/// │    PD12 (LED Green)  → Heartbeat                 │
/// │    PD13 (LED Orange) → MQTT connected            │
/// │    PD14 (LED Red)    → Fault/alarm               │
/// │    PD15 (LED Blue)   → Data transmit             │
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
