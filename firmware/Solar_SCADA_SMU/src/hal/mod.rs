/// Hardware Abstraction Layer — SS-PCB-001 Pin Assignments
///
/// ┌─────────────────────────────────────────────────────────────┐
/// │  SS-PCB-001 — Solar SCADA String Monitoring Unit            │
/// │  MCU: STM32F407VGT6  (LQFP-100, 1MB Flash, 192KB RAM)     │
/// ├─────────────────────────────────────────────────────────────┤
/// │                                                             │
/// │  SPI1 — ADS1263 (32-bit Delta-Sigma ADC):                  │
/// │    PA5  = SPI1_SCK                                          │
/// │    PA6  = SPI1_MISO                                         │
/// │    PA7  = SPI1_MOSI                                         │
/// │    PA4  = ADS1263_CS    (chip select, active low)           │
/// │    PB0  = ADS1263_DRDY  (data ready, active low)           │
/// │    PB1  = ADS1263_RESET (active low)                       │
/// │                                                             │
/// │  MUX A — CD74HC4067 (U3, Voltage channels):                │
/// │    PC0  = MUX_A_S0                                          │
/// │    PC1  = MUX_A_S1                                          │
/// │    PC2  = MUX_A_S2                                          │
/// │    PC3  = MUX_A_S3                                          │
/// │    PC4  = MUX_A_EN  (active low)                           │
/// │    COM → ADS1263 AIN0                                       │
/// │                                                             │
/// │  MUX B — CD74HC4067 (U4, Current channels):                │
/// │    PC5  = MUX_B_S0                                          │
/// │    PC6  = MUX_B_S1                                          │
/// │    PC7  = MUX_B_S2                                          │
/// │    PC8  = MUX_B_S3                                          │
/// │    PC9  = MUX_B_EN  (active low)                           │
/// │    COM → ADS1263 AIN1                                       │
/// │                                                             │
/// │  I2C1 — TMP117 (Temperature Sensor):                       │
/// │    PB6  = I2C1_SCL                                          │
/// │    PB7  = I2C1_SDA                                          │
/// │    TMP117 address: 0x48                                     │
/// │                                                             │
/// │  SPI3 — MicroSD Card:                                       │
/// │    PB3  = SPI3_SCK                                          │
/// │    PB4  = SPI3_MISO                                         │
/// │    PB5  = SPI3_MOSI                                         │
/// │    PA15 = SD_CS     (chip select, active low)              │
/// │    PD2  = SD_DETECT (card present, active low)             │
/// │                                                             │
/// │  USART2 — RS-485 Modbus (SP3485):                          │
/// │    PA2  = USART2_TX                                         │
/// │    PA3  = USART2_RX                                         │
/// │    PA8  = RS485_DE/RE (direction: high=TX, low=RX)         │
/// │                                                             │
/// │  GPIO — Status LEDs:                                        │
/// │    PD12 = LED Green  (heartbeat)                            │
/// │    PD13 = LED Orange (acquisition active)                   │
/// │    PD14 = LED Red    (fault/alarm)                          │
/// │    PD15 = LED Blue   (SD card activity)                     │
/// │                                                             │
/// │  Direct ADC Channels (bypassing MUX):                      │
/// │    ADS1263 AIN2 = DC bus voltage (0-1000V via divider)     │
/// │    ADS1263 AIN3 = DC bus current (via hall sensor)         │
/// │    ADS1263 AIN4 = Pyranometer / irradiance sensor          │
/// │    ADS1263 AIN5 = Module temp RTD (backup to TMP117)       │
/// │                                                             │
/// │  String Monitoring (per MUX channel, 16 strings):          │
/// │    MUX A CH[n] = String [n+1] voltage (via resistive div)  │
/// │    MUX B CH[n] = String [n+1] current (via 50mV shunt)    │
/// └─────────────────────────────────────────────────────────────┘

pub mod pins {
    // Placeholder for pin type aliases if needed
}
