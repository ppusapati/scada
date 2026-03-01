# SS-PCB-001 MCU Pin Assignment Table

**Document:** SS-PCB-001 Pin Assignment Reference
**MCU:** STM32F407VGT6 (LQFP-100)
**Board:** Solar String Monitoring Unit (SMU)
**Revision:** A
**Date:** 2026-02-28

---

## Pin Assignment Summary

| Category             | Pins Used | Interface      |
|----------------------|-----------|----------------|
| SPI1 (ADS1263)       | 4         | PA4-PA7        |
| SPI3 (SD Card)       | 4         | PB3-PB5, PA15  |
| I2C1 (TMP117)        | 2         | PB6-PB7        |
| USART2 (RS-485)      | 3         | PA2-PA3, PA8   |
| ADC Control          | 2         | PB0-PB1        |
| MUX A Control        | 5         | PC0-PC4        |
| MUX B Control        | 5         | PC5-PC9        |
| SD Card Detect       | 1         | PD2            |
| LEDs                 | 4         | PD12-PD15      |
| SWD Debug            | 2         | PA13-PA14      |
| HSE Oscillator       | 2         | PH0-PH1        |
| LSE Oscillator       | 2         | PC14-PC15      |
| Power / Reset / Boot | 16        | Various        |
| **Total Assigned**   | **52**    |                |
| NC / Reserved        | 48        | Connect via 10K to GND |

---

## Complete LQFP-100 Pin Assignment Table

All 100 pins of the STM32F407VGT6 LQFP-100 package are listed below in pin-number order. Unused pins are marked NC/Reserved with a recommendation to connect to GND via a 10K resistor and configure as input with internal pull-down in firmware.

| Pin | Pin Name       | Function Assigned       | Direction | Notes                                                      |
|-----|----------------|-------------------------|-----------|------------------------------------------------------------|
| 1   | PE2            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 2   | PE3            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 3   | PE4            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 4   | PE5            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 5   | PE6            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 6   | VBAT           | Battery Backup Supply   | PWR       | Connected to 3.3V; 100nF + 4.7uF decoupling               |
| 7   | PC13           | NC/Reserved             | --        | Connect to GND via 10K resistor; RTC domain pin            |
| 8   | PC14/OSC32_IN  | LSE Oscillator Input    | IN        | 32.768 kHz crystal Y2; 6.8 pF load cap                    |
| 9   | PC15/OSC32_OUT | LSE Oscillator Output   | OUT       | 32.768 kHz crystal Y2; 6.8 pF load cap                    |
| 10  | PH0/OSC_IN     | HSE Oscillator Input    | IN        | 8 MHz crystal Y1; 20 pF load cap                          |
| 11  | PH1/OSC_OUT    | HSE Oscillator Output   | OUT       | 8 MHz crystal Y1; 20 pF load cap                          |
| 12  | NRST           | System Reset            | IN        | 100nF cap to GND + 10K pull-up to 3.3V; routed to J5 pin 5|
| 13  | PC0            | MUX_A_S0 (GPIO Output)  | OUT       | CD74HC4067 (U5) address select bit 0                      |
| 14  | PC1            | MUX_A_S1 (GPIO Output)  | OUT       | CD74HC4067 (U5) address select bit 1                      |
| 15  | PC2            | MUX_A_S2 (GPIO Output)  | OUT       | CD74HC4067 (U5) address select bit 2                      |
| 16  | PC3            | MUX_A_S3 (GPIO Output)  | OUT       | CD74HC4067 (U5) address select bit 3                      |
| 17  | VSSA           | Analog Ground           | PWR       | Connected to AGND plane (star-point to DGND)               |
| 18  | VDDA           | Analog Power Supply     | PWR       | 3.3V via ferrite bead; 1uF + 100nF decoupling to VSSA     |
| 19  | PA0            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 20  | PA1            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 21  | PA2            | USART2_TX (AF7)         | OUT       | RS-485 transmit data to SP3485 DI pin                      |
| 22  | PA3            | USART2_RX (AF7)         | IN        | RS-485 receive data from SP3485 RO pin                     |
| 23  | VSS            | Digital Ground           | PWR       | Connected to GND plane                                     |
| 24  | VDD            | Digital Power Supply     | PWR       | 3.3V; 100nF MLCC decoupling                               |
| 25  | PA4            | SPI1_NSS / ADS1263 CS   | OUT       | ADS1263 chip select, active low; GPIO software control     |
| 26  | PA5            | SPI1_SCK (AF5)          | OUT       | ADS1263 SPI clock; max 8 MHz; 33R series resistor          |
| 27  | PA6            | SPI1_MISO (AF5)         | IN        | ADS1263 DOUT; 33R series resistor                          |
| 28  | PA7            | SPI1_MOSI (AF5)         | OUT       | ADS1263 DIN; 33R series resistor                           |
| 29  | PC4            | MUX_A_EN (GPIO Output)  | OUT       | CD74HC4067 (U5) enable, active low                         |
| 30  | PC5            | MUX_B_S0 (GPIO Output)  | OUT       | CD74HC4067 (U6) address select bit 0                      |
| 31  | PB0            | ADS1263_DRDY (GPIO In)  | IN        | ADS1263 data ready, active low; 10K external pull-up       |
| 32  | PB1            | ADS1263_RESET (GPIO Out)| OUT       | ADS1263 hardware reset, active low; 10K external pull-up   |
| 33  | PB2/BOOT1      | BOOT1 Configuration     | IN        | 10K pull-down to GND; boot from Flash (BOOT1=0)           |
| 34  | PE7            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 35  | PE8            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 36  | PE9            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 37  | PE10           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 38  | PE11           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 39  | PE12           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 40  | PE13           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 41  | PE14           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 42  | PE15           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 43  | PB10           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 44  | PB11           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 45  | VSS            | Digital Ground           | PWR       | Connected to GND plane                                     |
| 46  | VDD            | Digital Power Supply     | PWR       | 3.3V; 100nF MLCC decoupling                               |
| 47  | PB12           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 48  | PB13           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 49  | PB14           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 50  | PB15           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 51  | PD8            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 52  | PD9            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 53  | PD10           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 54  | PD11           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 55  | PD12           | LED1_GREEN (GPIO Out PP)| OUT       | Green LED; heartbeat 1 Hz; 1K series resistor              |
| 56  | PD13           | LED2_ORANGE (GPIO Out PP)| OUT      | Orange LED; RS-485 comm activity; 1K series resistor       |
| 57  | PD14           | LED3_RED (GPIO Out PP)  | OUT       | Red LED; fault/alarm indicator; 1K series resistor         |
| 58  | PD15           | LED4_BLUE (GPIO Out PP) | OUT       | Blue LED; SD card write activity; 1K series resistor       |
| 59  | PC6            | MUX_B_S1 (GPIO Output)  | OUT       | CD74HC4067 (U6) address select bit 1                      |
| 60  | PC7            | MUX_B_S2 (GPIO Output)  | OUT       | CD74HC4067 (U6) address select bit 2                      |
| 61  | PC8            | MUX_B_S3 (GPIO Output)  | OUT       | CD74HC4067 (U6) address select bit 3                      |
| 62  | PC9            | MUX_B_EN (GPIO Output)  | OUT       | CD74HC4067 (U6) enable, active low                         |
| 63  | PA8            | RS485_DE (GPIO Output)  | OUT       | SP3485 DE + /RE tied together; HIGH=TX, LOW=RX             |
| 64  | PA9            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 65  | PA10           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 66  | PA11           | NC/Reserved             | --        | Connect to GND via 10K resistor; USB_DM capable            |
| 67  | PA12           | NC/Reserved             | --        | Connect to GND via 10K resistor; USB_DP capable            |
| 68  | PA13           | SWDIO (AF0)             | BIDIR     | SWD debug data; J5 pin 2; do not repurpose                |
| 69  | VSS            | Digital Ground           | PWR       | Connected to GND plane                                     |
| 70  | VDD            | Digital Power Supply     | PWR       | 3.3V; 100nF MLCC decoupling                               |
| 71  | PA14           | SWCLK (AF0)             | IN        | SWD debug clock; J5 pin 4; do not repurpose               |
| 72  | PA15           | SPI3_NSS / SD Card CS   | OUT       | MicroSD chip select, active low; GPIO software control     |
| 73  | PC10           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 74  | PC11           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 75  | PC12           | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 76  | PD0            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 77  | PD1            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 78  | PD2            | SD_CARD_DETECT (GPIO In)| IN        | SD card detect switch, active low; 10K external pull-up    |
| 79  | PD3            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 80  | PD4            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 81  | PD5            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 82  | PD6            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 83  | PD7            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 84  | PB3            | SPI3_SCK (AF6)          | OUT       | SD card SPI clock; 400 kHz init, 12 MHz normal            |
| 85  | PB4            | SPI3_MISO (AF6)         | IN        | SD card DAT0 (data out); 10K external pull-up              |
| 86  | PB5            | SPI3_MOSI (AF6)         | OUT       | SD card CMD (data in)                                      |
| 87  | PB6            | I2C1_SCL (AF4)          | BIDIR     | TMP117 SCL; open-drain; 4.7K external pull-up to 3.3V     |
| 88  | PB7            | I2C1_SDA (AF4)          | BIDIR     | TMP117 SDA; open-drain; 4.7K external pull-up to 3.3V     |
| 89  | BOOT0          | Boot Mode Select        | IN        | 10K pull-down to GND; boot from Flash (BOOT0=0)           |
| 90  | PB8            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 91  | PB9            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 92  | PE0            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 93  | PE1            | NC/Reserved             | --        | Connect to GND via 10K resistor; input with pull-down      |
| 94  | VSS            | Digital Ground           | PWR       | Connected to GND plane                                     |
| 95  | VDD            | Digital Power Supply     | PWR       | 3.3V; 100nF MLCC decoupling                               |
| 96  | VCAP1          | Internal Regulator Out   | PWR       | 1uF + 100nF ceramic to GND; place within 2mm of pin       |
| 97  | VCAP2          | Internal Regulator Out   | PWR       | 1uF + 100nF ceramic to GND; place within 2mm of pin       |
| 98  | VSS            | Digital Ground           | PWR       | Connected to GND plane                                     |
| 99  | VDD            | Digital Power Supply     | PWR       | 3.3V; 100nF MLCC decoupling                               |
| 100 | VDD            | Digital Power Supply     | PWR       | 3.3V; 100nF MLCC decoupling                               |

---

## Pin Assignments Grouped by Port

### PORT A (PA0 - PA15)

| LQFP Pin | Pin Name | Function Assigned       | AF/Mode         | Direction | Speed     | Pull     | Notes                                     |
|----------|----------|-------------------------|-----------------|-----------|-----------|----------|--------------------------------------------|
| 19       | PA0      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 20       | PA1      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 21       | PA2      | USART2_TX               | AF7 (USART2)   | OUT       | High      | None     | RS-485 transmit to SP3485 DI               |
| 22       | PA3      | USART2_RX               | AF7 (USART2)   | IN        | --        | Pull-up  | RS-485 receive from SP3485 RO              |
| 25       | PA4      | SPI1_NSS (ADS1263 CS)   | GPIO Output PP  | OUT       | High      | None     | ADS1263 chip select, active low            |
| 26       | PA5      | SPI1_SCK                | AF5 (SPI1)      | OUT       | Very High | None     | ADS1263 SPI clock                          |
| 27       | PA6      | SPI1_MISO               | AF5 (SPI1)      | IN        | Very High | None     | ADS1263 DOUT                               |
| 28       | PA7      | SPI1_MOSI               | AF5 (SPI1)      | OUT       | Very High | None     | ADS1263 DIN                                |
| 63       | PA8      | RS485_DE                | GPIO Output PP  | OUT       | High      | None     | SP3485 DE+/RE; HIGH=TX, LOW=RX             |
| 64       | PA9      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 65       | PA10     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 66       | PA11     | NC/Reserved             | Input           | --        | --        | Pull-down| USB_DM capable; connect to GND via 10K     |
| 67       | PA12     | NC/Reserved             | Input           | --        | --        | Pull-down| USB_DP capable; connect to GND via 10K     |
| 68       | PA13     | SWDIO                   | AF0 (SWD)       | BIDIR     | --        | Pull-up  | SWD data; J5 pin 2; do not repurpose      |
| 71       | PA14     | SWCLK                   | AF0 (SWD)       | IN        | --        | Pull-down| SWD clock; J5 pin 4; do not repurpose     |
| 72       | PA15     | SPI3_NSS (SD Card CS)   | GPIO Output PP  | OUT       | High      | None     | MicroSD chip select, active low            |

### PORT B (PB0 - PB15)

| LQFP Pin | Pin Name | Function Assigned       | AF/Mode         | Direction | Speed     | Pull     | Notes                                     |
|----------|----------|-------------------------|-----------------|-----------|-----------|----------|--------------------------------------------|
| 31       | PB0      | ADS1263_DRDY            | GPIO Input      | IN        | --        | None     | Data ready, active low; 10K ext pull-up    |
| 32       | PB1      | ADS1263_RESET           | GPIO Output PP  | OUT       | Low       | None     | ADC reset, active low; 10K ext pull-up     |
| 33       | PB2      | BOOT1                   | Input           | IN        | --        | Pull-down| 10K to GND; boot from Flash (BOOT1=0)     |
| 84       | PB3      | SPI3_SCK                | AF6 (SPI3)      | OUT       | Very High | None     | SD card SPI clock                          |
| 85       | PB4      | SPI3_MISO               | AF6 (SPI3)      | IN        | Very High | Pull-up  | SD card DAT0; 10K ext pull-up              |
| 86       | PB5      | SPI3_MOSI               | AF6 (SPI3)      | OUT       | Very High | None     | SD card CMD line                           |
| 87       | PB6      | I2C1_SCL                | AF4 (I2C1)      | BIDIR     | High      | None     | TMP117 SCL; open-drain; 4.7K ext pull-up   |
| 88       | PB7      | I2C1_SDA                | AF4 (I2C1)      | BIDIR     | High      | None     | TMP117 SDA; open-drain; 4.7K ext pull-up   |
| 90       | PB8      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 91       | PB9      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 43       | PB10     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 44       | PB11     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 47       | PB12     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 48       | PB13     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 49       | PB14     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 50       | PB15     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |

### PORT C (PC0 - PC15)

| LQFP Pin | Pin Name | Function Assigned       | AF/Mode         | Direction | Speed     | Pull     | Notes                                     |
|----------|----------|-------------------------|-----------------|-----------|-----------|----------|--------------------------------------------|
| 13       | PC0      | MUX_A_S0                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U5) address bit 0             |
| 14       | PC1      | MUX_A_S1                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U5) address bit 1             |
| 15       | PC2      | MUX_A_S2                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U5) address bit 2             |
| 16       | PC3      | MUX_A_S3                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U5) address bit 3             |
| 29       | PC4      | MUX_A_EN                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U5) enable, active low        |
| 30       | PC5      | MUX_B_S0                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U6) address bit 0             |
| 59       | PC6      | MUX_B_S1                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U6) address bit 1             |
| 60       | PC7      | MUX_B_S2                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U6) address bit 2             |
| 61       | PC8      | MUX_B_S3                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U6) address bit 3             |
| 62       | PC9      | MUX_B_EN                | GPIO Output PP  | OUT       | Low       | None     | CD74HC4067 (U6) enable, active low        |
| 73       | PC10     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 74       | PC11     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 75       | PC12     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 7        | PC13     | NC/Reserved             | Input           | --        | --        | Pull-down| RTC domain; connect to GND via 10K         |
| 8        | PC14     | OSC32_IN (LSE)          | Oscillator      | IN        | --        | None     | 32.768 kHz crystal Y2; 6.8 pF load cap    |
| 9        | PC15     | OSC32_OUT (LSE)         | Oscillator      | OUT       | --        | None     | 32.768 kHz crystal Y2; 6.8 pF load cap    |

### PORT D (PD0 - PD15)

| LQFP Pin | Pin Name | Function Assigned       | AF/Mode         | Direction | Speed     | Pull     | Notes                                     |
|----------|----------|-------------------------|-----------------|-----------|-----------|----------|--------------------------------------------|
| 76       | PD0      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 77       | PD1      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 78       | PD2      | SD_CARD_DETECT          | GPIO Input      | IN        | --        | Pull-up  | Card detect switch, active low; 10K ext pull-up |
| 79       | PD3      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 80       | PD4      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 81       | PD5      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 82       | PD6      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 83       | PD7      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 51       | PD8      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 52       | PD9      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 53       | PD10     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 54       | PD11     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 55       | PD12     | LED1_GREEN              | GPIO Output PP  | OUT       | Low       | None     | Green LED; heartbeat 1 Hz; 1K series       |
| 56       | PD13     | LED2_ORANGE             | GPIO Output PP  | OUT       | Low       | None     | Orange LED; RS-485 activity; 1K series     |
| 57       | PD14     | LED3_RED                | GPIO Output PP  | OUT       | Low       | None     | Red LED; fault indicator; 1K series        |
| 58       | PD15     | LED4_BLUE               | GPIO Output PP  | OUT       | Low       | None     | Blue LED; SD write activity; 1K series     |

### PORT E (PE0 - PE15)

| LQFP Pin | Pin Name | Function Assigned       | AF/Mode         | Direction | Speed     | Pull     | Notes                                     |
|----------|----------|-------------------------|-----------------|-----------|-----------|----------|--------------------------------------------|
| 92       | PE0      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 93       | PE1      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 1        | PE2      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 2        | PE3      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 3        | PE4      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 4        | PE5      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 5        | PE6      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 34       | PE7      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 35       | PE8      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 36       | PE9      | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 37       | PE10     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 38       | PE11     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 39       | PE12     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 40       | PE13     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 41       | PE14     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |
| 42       | PE15     | NC/Reserved             | Input           | --        | --        | Pull-down| Connect to GND via 10K resistor            |

### PORT H (PH0 - PH1)

| LQFP Pin | Pin Name | Function Assigned       | AF/Mode         | Direction | Speed     | Pull     | Notes                                     |
|----------|----------|-------------------------|-----------------|-----------|-----------|----------|--------------------------------------------|
| 10       | PH0      | OSC_IN (HSE)            | Oscillator      | IN        | --        | None     | 8 MHz crystal Y1; 20 pF load cap          |
| 11       | PH1      | OSC_OUT (HSE)           | Oscillator      | OUT       | --        | None     | 8 MHz crystal Y1; 20 pF load cap          |

### Power Pins

| LQFP Pin | Pin Name | Function                | Connection                                          |
|----------|----------|-------------------------|-----------------------------------------------------|
| 6        | VBAT     | Battery Backup          | 3.3V via Schottky diode; 100nF + 4.7uF decoupling  |
| 17       | VSSA     | Analog Ground           | AGND plane, star-point connection to DGND           |
| 18       | VDDA     | Analog Power            | 3.3V via ferrite bead; 1uF + 100nF to VSSA         |
| 23       | VSS      | Digital Ground           | GND plane                                           |
| 24       | VDD      | Digital Power            | 3.3V; 100nF decoupling                             |
| 45       | VSS      | Digital Ground           | GND plane                                           |
| 46       | VDD      | Digital Power            | 3.3V; 100nF decoupling                             |
| 69       | VSS      | Digital Ground           | GND plane                                           |
| 70       | VDD      | Digital Power            | 3.3V; 100nF decoupling                             |
| 94       | VSS      | Digital Ground           | GND plane                                           |
| 95       | VDD      | Digital Power            | 3.3V; 100nF decoupling                             |
| 96       | VCAP1    | Internal Regulator       | 1uF + 100nF ceramic to GND (< 2mm from pin)        |
| 97       | VCAP2    | Internal Regulator       | 1uF + 100nF ceramic to GND (< 2mm from pin)        |
| 98       | VSS      | Digital Ground           | GND plane                                           |
| 99       | VDD      | Digital Power            | 3.3V; 100nF decoupling                             |
| 100      | VDD      | Digital Power            | 3.3V; 100nF decoupling                             |

### Special Function Pins

| LQFP Pin | Pin Name | Function                | Connection                                          |
|----------|----------|-------------------------|-----------------------------------------------------|
| 12       | NRST     | System Reset            | 100nF to GND + 10K pull-up to 3.3V; J5 pin 5       |
| 89       | BOOT0    | Boot Configuration      | 10K pull-down to GND (always boot from Flash)        |

---

## Pin Map by Peripheral

### SPI1 -- ADS1263 ADC

| Signal | MCU Pin | LQFP Pin | ADS1263 Pin | AF   | Notes                           |
|--------|---------|----------|-------------|------|---------------------------------|
| CS     | PA4     | 25       | /CS         | GPIO | Software controlled, active low |
| SCK    | PA5     | 26       | SCLK        | AF5  | Max 8 MHz; 33R series           |
| MISO   | PA6     | 27       | DOUT        | AF5  | 33R series resistor             |
| MOSI   | PA7     | 28       | DIN         | AF5  | 33R series resistor             |
| DRDY   | PB0     | 31       | /DRDY       | GPIO | Input, 10K ext pull-up, EXTI0   |
| RESET  | PB1     | 32       | /RESET      | GPIO | Output, active low              |

### SPI3 -- MicroSD Card

| Signal  | MCU Pin | LQFP Pin | SD Card Pin  | AF   | Notes                           |
|---------|---------|----------|--------------|------|---------------------------------|
| SCK     | PB3     | 84       | CLK (pin 5)  | AF6  | 400 kHz init, 12 MHz normal     |
| MISO    | PB4     | 85       | DAT0 (pin 7) | AF6  | 10K ext pull-up                 |
| MOSI    | PB5     | 86       | CMD (pin 2)  | AF6  |                                 |
| CS      | PA15    | 72       | DAT3 (pin 1) | GPIO | Software controlled, active low |
| DETECT  | PD2     | 78       | CD switch     | GPIO | Input, active low, 10K pull-up  |

### I2C1 -- TMP117 Temperature Sensor

| Signal | MCU Pin | LQFP Pin | TMP117 Pin | AF   | Notes                           |
|--------|---------|----------|------------|------|---------------------------------|
| SCL    | PB6     | 87       | SCL        | AF4  | Open-drain; 4.7K ext pull-up    |
| SDA    | PB7     | 88       | SDA        | AF4  | Open-drain; 4.7K ext pull-up    |

TMP117 I2C address: 0x48 (ADD0 = GND)

### USART2 + RS-485 (SP3485)

| Signal | MCU Pin | LQFP Pin | SP3485 Pin | AF   | Notes                           |
|--------|---------|----------|------------|------|---------------------------------|
| TX     | PA2     | 21       | DI         | AF7  | Modbus RTU transmit             |
| RX     | PA3     | 22       | RO         | AF7  | Modbus RTU receive              |
| DE     | PA8     | 63       | DE + /RE   | GPIO | HIGH = transmit, LOW = receive  |

### MUX A -- CD74HC4067 (U5) -- String Voltages

| Signal | MCU Pin | LQFP Pin | MUX Pin | Notes                           |
|--------|---------|----------|---------|---------------------------------|
| S0     | PC0     | 13       | S0      | Address select bit 0            |
| S1     | PC1     | 14       | S1      | Address select bit 1            |
| S2     | PC2     | 15       | S2      | Address select bit 2            |
| S3     | PC3     | 16       | S3      | Address select bit 3            |
| EN     | PC4     | 29       | /EN     | Enable, active low              |

MUX A common output (Z) connects to ADS1263 AIN0.

### MUX B -- CD74HC4067 (U6) -- String Currents

| Signal | MCU Pin | LQFP Pin | MUX Pin | Notes                           |
|--------|---------|----------|---------|---------------------------------|
| S0     | PC5     | 30       | S0      | Address select bit 0            |
| S1     | PC6     | 59       | S1      | Address select bit 1            |
| S2     | PC7     | 60       | S2      | Address select bit 2            |
| S3     | PC8     | 61       | S3      | Address select bit 3            |
| EN     | PC9     | 62       | /EN     | Enable, active low              |

MUX B common output (Z) connects to ADS1263 AIN1.

### LED Indicators

| Signal      | MCU Pin | LQFP Pin | Color  | Function                        |
|-------------|---------|----------|--------|---------------------------------|
| LED1        | PD12    | 55       | Green  | Heartbeat (1 Hz blink = OK)     |
| LED2        | PD13    | 56       | Orange | RS-485 TX/RX activity           |
| LED3        | PD14    | 57       | Red    | Fault / alarm indicator         |
| LED4        | PD15    | 58       | Blue   | SD card write in progress       |

All LEDs driven through 1K series resistors, active high (push-pull output).

### SWD Debug Header (J5)

| Signal | MCU Pin | LQFP Pin | J5 Pin | Notes                           |
|--------|---------|----------|--------|---------------------------------|
| SWDIO  | PA13    | 68       | Pin 2  | Do not repurpose in production  |
| SWCLK  | PA14    | 71       | Pin 4  | Do not repurpose in production  |
| NRST   | NRST    | 12       | Pin 5  | Active low reset                |
| 3.3V   | --      | --       | Pin 1  | Power reference for debugger    |
| GND    | --      | --       | Pin 3,9,10 | Ground connections          |

### Oscillators

| Signal    | MCU Pin | LQFP Pin | Crystal | Frequency   | Load Cap |
|-----------|---------|----------|---------|-------------|----------|
| OSC_IN    | PH0     | 10       | Y1      | 8 MHz       | 20 pF    |
| OSC_OUT   | PH1     | 11       | Y1      | 8 MHz       | 20 pF    |
| OSC32_IN  | PC14    | 8        | Y2      | 32.768 kHz  | 6.8 pF   |
| OSC32_OUT | PC15    | 9        | Y2      | 32.768 kHz  | 6.8 pF   |

---

## Pin Usage Statistics

| Category            | Count | Percentage |
|---------------------|-------|------------|
| Power (VDD/VSS)     | 10    | 10%        |
| Analog Power        | 2     | 2%         |
| VCAP (internal reg) | 2     | 2%         |
| VBAT                | 1     | 1%         |
| Oscillator (HSE)    | 2     | 2%         |
| Oscillator (LSE)    | 2     | 2%         |
| Reset (NRST)        | 1     | 1%         |
| Boot (BOOT0, BOOT1) | 2     | 2%         |
| SPI1 (ADS1263)      | 4     | 4%         |
| SPI3 (SD card)      | 4     | 4%         |
| USART2 (RS-485)     | 2     | 2%         |
| I2C1 (TMP117)       | 2     | 2%         |
| ADS1263 Control     | 2     | 2%         |
| MUX A Control       | 5     | 5%         |
| MUX B Control       | 5     | 5%         |
| RS-485 DE/RE        | 1     | 1%         |
| SD Card Detect      | 1     | 1%         |
| LEDs                | 4     | 4%         |
| SWD Debug           | 2     | 2%         |
| **NC/Reserved**     | **46**| **46%**    |
| **Total**           |**100**| **100%**   |

---

## Peripheral Conflict Check

The following table verifies there are no alternate function (AF) conflicts on assigned pins.

| Pin  | AF0       | AF4       | AF5       | AF6       | AF7        | Assigned AF | Conflict |
|------|-----------|-----------|-----------|-----------|------------|-------------|----------|
| PA2  | --        | --        | --        | --        | USART2_TX  | AF7         | None     |
| PA3  | --        | --        | --        | --        | USART2_RX  | AF7         | None     |
| PA4  | --        | --        | SPI1_NSS  | --        | --         | GPIO (CS)   | None     |
| PA5  | --        | --        | SPI1_SCK  | --        | --         | AF5         | None     |
| PA6  | --        | --        | SPI1_MISO | --        | --         | AF5         | None     |
| PA7  | --        | --        | SPI1_MOSI | --        | --         | AF5         | None     |
| PA13 | SWDIO     | --        | --        | --        | --         | AF0         | None     |
| PA14 | SWCLK     | --        | --        | --        | --         | AF0         | None     |
| PA15 | --        | --        | --        | SPI3_NSS  | --         | GPIO (CS)   | None     |
| PB3  | --        | --        | --        | SPI3_SCK  | --         | AF6         | None     |
| PB4  | --        | --        | --        | SPI3_MISO | --         | AF6         | None     |
| PB5  | --        | --        | --        | SPI3_MOSI | --         | AF6         | None     |
| PB6  | --        | I2C1_SCL  | --        | --        | --         | AF4         | None     |
| PB7  | --        | I2C1_SDA  | --        | --        | --         | AF4         | None     |

No alternate function conflicts detected. All peripheral assignments are valid.

---

## Interrupt Assignments

| IRQ Source         | Pin  | EXTI Line | Priority | Description                          |
|--------------------|------|-----------|----------|--------------------------------------|
| ADS1263 DRDY       | PB0  | EXTI0     | High (1) | Data ready, triggers SPI read cycle  |

---

## DMA Channel Assignments

| DMA    | Stream   | Channel | Peripheral  | Direction | Purpose                     |
|--------|----------|---------|-------------|-----------|-----------------------------|
| DMA2   | Stream 0 | Ch 3    | SPI1_RX     | P-to-M    | ADS1263 data receive        |
| DMA2   | Stream 3 | Ch 3    | SPI1_TX     | M-to-P    | ADS1263 command transmit    |
| DMA1   | Stream 0 | Ch 0    | SPI3_RX     | P-to-M    | SD card data receive        |
| DMA1   | Stream 7 | Ch 0    | SPI3_TX     | M-to-P    | SD card data transmit       |
| DMA1   | Stream 6 | Ch 4    | USART2_TX   | M-to-P    | Modbus RTU frame transmit   |
| DMA1   | Stream 5 | Ch 4    | USART2_RX   | P-to-M    | Modbus RTU frame receive    |

---

## Unused Pin Handling

All unused GPIO pins (marked "NC/Reserved" in the tables above) must be handled as follows per STM32 design guidelines (AN4488):

1. **Hardware (PCB):** Connect each unused pin to GND through a 10K resistor. This provides a defined state during power-up before firmware configures the pin, and prevents floating inputs from coupling noise.

2. **Firmware:** Configure as GPIO input with internal pull-down enabled. This provides double protection (external 10K + internal pull-down).

3. **Alternative:** If board space is constrained, unused pins may be left unconnected externally and configured as GPIO output push-pull driven low in firmware. However, the 10K-to-GND approach is preferred for maximum robustness.

4. **Never leave floating:** Floating inputs increase power consumption and may cause EMC issues.

### Complete List of Unused Pins (46 pins)

| Port A (6 pins)     | Port B (8 pins)    | Port C (4 pins)    | Port D (8 pins)    | Port E (16 pins)     |
|----------------------|--------------------|--------------------|--------------------|-----------------------|
| PA0 (pin 19)         | PB2 (pin 33)*     | PC10 (pin 73)      | PD0 (pin 76)       | PE0 (pin 92)          |
| PA1 (pin 20)         | PB8 (pin 90)      | PC11 (pin 74)      | PD1 (pin 77)       | PE1 (pin 93)          |
| PA9 (pin 64)         | PB9 (pin 91)      | PC12 (pin 75)      | PD3 (pin 79)       | PE2 (pin 1)           |
| PA10 (pin 65)        | PB10 (pin 43)     | PC13 (pin 7)       | PD4 (pin 80)       | PE3 (pin 2)           |
| PA11 (pin 66)        | PB11 (pin 44)     |                    | PD5 (pin 81)       | PE4 (pin 3)           |
| PA12 (pin 67)        | PB12 (pin 47)     |                    | PD6 (pin 82)       | PE5 (pin 4)           |
|                      | PB13 (pin 48)     |                    | PD7 (pin 83)       | PE6 (pin 5)           |
|                      | PB14 (pin 49)     |                    | PD8 (pin 51)       | PE7 (pin 34)          |
|                      | PB15 (pin 50)     |                    | PD9 (pin 52)       | PE8 (pin 35)          |
|                      |                    |                    | PD10 (pin 53)      | PE9 (pin 36)          |
|                      |                    |                    | PD11 (pin 54)      | PE10 (pin 37)         |
|                      |                    |                    |                    | PE11 (pin 38)         |
|                      |                    |                    |                    | PE12 (pin 39)         |
|                      |                    |                    |                    | PE13 (pin 40)         |
|                      |                    |                    |                    | PE14 (pin 41)         |
|                      |                    |                    |                    | PE15 (pin 42)         |

*PB2 is BOOT1 with 10K to GND; counted as unused for GPIO purposes but has dedicated boot function.*

---

## GPIO Initialization Quick Reference (Firmware)

```c
/*
 * SS-PCB-001 GPIO Initialization Summary
 * STM32F407VGT6 - Solar String Monitoring Unit
 *
 * Generated from pinout document rev A, 2026-02-28
 */

/* === Port A === */
// PA0  = NC/Reserved    -> Input, Pull-Down
// PA1  = NC/Reserved    -> Input, Pull-Down
// PA2  = USART2_TX      -> AF7, Push-Pull, High Speed
// PA3  = USART2_RX      -> AF7, Input, Pull-Up
// PA4  = ADS1263_CS     -> Output Push-Pull, Init HIGH (deselected), High Speed
// PA5  = SPI1_SCK       -> AF5, Push-Pull, Very High Speed
// PA6  = SPI1_MISO      -> AF5, Input, No Pull
// PA7  = SPI1_MOSI      -> AF5, Push-Pull, Very High Speed
// PA8  = RS485_DE       -> Output Push-Pull, Init LOW (RX mode), High Speed
// PA9  = NC/Reserved    -> Input, Pull-Down
// PA10 = NC/Reserved    -> Input, Pull-Down
// PA11 = NC/Reserved    -> Input, Pull-Down
// PA12 = NC/Reserved    -> Input, Pull-Down
// PA13 = SWDIO          -> AF0 (default after reset, do not modify)
// PA14 = SWCLK          -> AF0 (default after reset, do not modify)
// PA15 = SD_CS          -> Output Push-Pull, Init HIGH (deselected), High Speed

/* === Port B === */
// PB0  = ADS1263_DRDY   -> Input, No Pull (external 10K pull-up)
// PB1  = ADS1263_RST    -> Output Push-Pull, Init HIGH (not in reset)
// PB2  = BOOT1          -> Input, Pull-Down (external 10K to GND)
// PB3  = SPI3_SCK       -> AF6, Push-Pull, Very High Speed
// PB4  = SPI3_MISO      -> AF6, Input, Pull-Up
// PB5  = SPI3_MOSI      -> AF6, Push-Pull, Very High Speed
// PB6  = I2C1_SCL       -> AF4, Open-Drain (external 4.7K pull-up)
// PB7  = I2C1_SDA       -> AF4, Open-Drain (external 4.7K pull-up)
// PB8  = NC/Reserved    -> Input, Pull-Down
// PB9  = NC/Reserved    -> Input, Pull-Down
// PB10 = NC/Reserved    -> Input, Pull-Down
// PB11 = NC/Reserved    -> Input, Pull-Down
// PB12 = NC/Reserved    -> Input, Pull-Down
// PB13 = NC/Reserved    -> Input, Pull-Down
// PB14 = NC/Reserved    -> Input, Pull-Down
// PB15 = NC/Reserved    -> Input, Pull-Down

/* === Port C === */
// PC0  = MUX_A_S0       -> Output Push-Pull, Init LOW, Low Speed
// PC1  = MUX_A_S1       -> Output Push-Pull, Init LOW, Low Speed
// PC2  = MUX_A_S2       -> Output Push-Pull, Init LOW, Low Speed
// PC3  = MUX_A_S3       -> Output Push-Pull, Init LOW, Low Speed
// PC4  = MUX_A_EN       -> Output Push-Pull, Init HIGH (disabled), Low Speed
// PC5  = MUX_B_S0       -> Output Push-Pull, Init LOW, Low Speed
// PC6  = MUX_B_S1       -> Output Push-Pull, Init LOW, Low Speed
// PC7  = MUX_B_S2       -> Output Push-Pull, Init LOW, Low Speed
// PC8  = MUX_B_S3       -> Output Push-Pull, Init LOW, Low Speed
// PC9  = MUX_B_EN       -> Output Push-Pull, Init HIGH (disabled), Low Speed
// PC10 = NC/Reserved    -> Input, Pull-Down
// PC11 = NC/Reserved    -> Input, Pull-Down
// PC12 = NC/Reserved    -> Input, Pull-Down
// PC13 = NC/Reserved    -> Input, Pull-Down
// PC14 = OSC32_IN       -> Configured by RCC for LSE
// PC15 = OSC32_OUT      -> Configured by RCC for LSE

/* === Port D === */
// PD0  = NC/Reserved    -> Input, Pull-Down
// PD1  = NC/Reserved    -> Input, Pull-Down
// PD2  = SD_DETECT      -> Input, Pull-Up (external 10K pull-up)
// PD3  = NC/Reserved    -> Input, Pull-Down
// PD4  = NC/Reserved    -> Input, Pull-Down
// PD5  = NC/Reserved    -> Input, Pull-Down
// PD6  = NC/Reserved    -> Input, Pull-Down
// PD7  = NC/Reserved    -> Input, Pull-Down
// PD8  = NC/Reserved    -> Input, Pull-Down
// PD9  = NC/Reserved    -> Input, Pull-Down
// PD10 = NC/Reserved    -> Input, Pull-Down
// PD11 = NC/Reserved    -> Input, Pull-Down
// PD12 = LED_GREEN      -> Output Push-Pull, Init LOW (off), Low Speed
// PD13 = LED_ORANGE     -> Output Push-Pull, Init LOW (off), Low Speed
// PD14 = LED_RED        -> Output Push-Pull, Init LOW (off), Low Speed
// PD15 = LED_BLUE       -> Output Push-Pull, Init LOW (off), Low Speed

/* === Port E === */
// PE0  through PE15     -> All Input, Pull-Down (NC/Reserved)

/* === Port H === */
// PH0  = OSC_IN         -> Configured by RCC for HSE
// PH1  = OSC_OUT        -> Configured by RCC for HSE
```

---

## Design Notes

1. **PA4 and PA15** (SPI chip selects) are driven as GPIO outputs rather than hardware NSS. This provides precise software control of chip select timing required for ADS1263 command framing and SD card SPI mode operation.

2. **PA13 and PA14** are reserved for SWD debug access and must not be repurposed in production firmware without providing an alternative programming method (e.g., UART bootloader).

3. **PB3 (SPI3_SCK)** shares the SWO trace output function. When SPI3 is active for SD card communication, SWO trace is unavailable. During debug sessions without SD card access, PB3 may be reconfigured for SWO via J5 pin 6.

4. **PB2 (BOOT1)** is dedicated to boot configuration with a 10K pull-down. The ADS1263 RESET is assigned to PB1 to avoid conflict with the boot function.

5. **MUX enable pins (PC4, PC9)** initialize HIGH (disabled). Firmware must explicitly enable the MUX before measurement and disable it afterward to reduce crosstalk and power consumption.

6. **I2C1 (PB6/PB7)** requires external 4.7K pull-up resistors. The internal pull-ups are not used because they are too weak (~40K) for reliable I2C operation at 400 kHz.

7. **All 46 reserved pins** are routed to test pads on the PCB bottom layer for future expansion (additional sensors, display interface, Ethernet, etc.).

---

*End of SS-PCB-001 MCU Pin Assignment Table*
