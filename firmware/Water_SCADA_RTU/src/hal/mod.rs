/// Hardware Abstraction Layer — WS-PCB-001 Pin Assignments
///
/// ┌─────────────────────────────────────────────────────────────┐
/// │  WS-PCB-001 — Water SCADA RTU                              │
/// │  MCU: STM32F407VGT6  (LQFP-100, 1MB Flash, 192KB RAM)     │
/// ├─────────────────────────────────────────────────────────────┤
/// │                                                             │
/// │  SPI1 — ADS1258 (16-ch Delta-Sigma ADC):                   │
/// │    PA5 = SPI1_SCK                                           │
/// │    PA6 = SPI1_MISO                                          │
/// │    PA7 = SPI1_MOSI                                          │
/// │    PA4 = ADS1258_CS   (chip select, active low)            │
/// │    PB0 = ADS1258_DRDY (data ready, active low)             │
/// │    PB1 = ADS1258_START (conversion start)                  │
/// │    PB2 = ADS1258_RESET (active low)                        │
/// │                                                             │
/// │  SPI2 — W5500 (Hardwired TCP/IP Ethernet):                 │
/// │    PB13 = SPI2_SCK                                          │
/// │    PB14 = SPI2_MISO                                         │
/// │    PB15 = SPI2_MOSI                                         │
/// │    PB12 = W5500_CS     (chip select, active low)           │
/// │    PC6  = W5500_RST    (reset, active low)                 │
/// │    PC7  = W5500_INT    (interrupt, active low)              │
/// │                                                             │
/// │  USART2 — SP3485 (RS-485 Transceiver):                     │
/// │    PA2  = USART2_TX                                         │
/// │    PA3  = USART2_RX                                         │
/// │    PA8  = SP3485_DE/RE (direction: high=TX, low=RX)        │
/// │                                                             │
/// │  GPIO — Status LEDs:                                        │
/// │    PD12 = LED Green  (heartbeat)                            │
/// │    PD13 = LED Orange (Modbus activity)                      │
/// │    PD14 = LED Red    (fault/alarm)                          │
/// │    PD15 = LED Blue   (data acquisition)                     │
/// │                                                             │
/// │  SWD Debug (J5 header):                                     │
/// │    PA13 = SWDIO                                              │
/// │    PA14 = SWCLK                                              │
/// │                                                             │
/// │  4-20mA Sensor Inputs (via ADS1258):                        │
/// │    CH0 = Pressure transmitter (0-10 bar)                    │
/// │    CH1 = pH sensor (0-14 pH)                                │
/// │    CH2 = Turbidity sensor (0-1000 NTU)                      │
/// │    CH3 = Chlorine residual (0-5 mg/L)                       │
/// │    CH4 = Flow meter (4-20mA proportional)                   │
/// │    CH5 = Tank level (0-100%)                                │
/// │    CH6 = Conductivity (0-2000 µS/cm)                        │
/// │    CH7 = Dissolved oxygen (0-20 mg/L)                       │
/// │                                                             │
/// │  Each 4-20mA input circuit:                                 │
/// │    Sensor → 250Ω precision resistor → ADS1258 diff input    │
/// │    4mA = 1.000V, 20mA = 5.000V (with Vref = 5.0V)          │
/// └─────────────────────────────────────────────────────────────┘

pub mod pins {
    // Placeholder module for pin type aliases if needed
}
