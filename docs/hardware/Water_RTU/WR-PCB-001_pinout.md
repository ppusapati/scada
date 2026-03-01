# WR-PCB-001 MCU Pin Assignment Table

## STM32F407VGT6 -- LQFP-100 Package

| Field            | Value                          |
|------------------|--------------------------------|
| Document ID      | WR-PCB-001-PIN-REV1            |
| Board            | WR-PCB-001                     |
| MCU              | STM32F407VGT6                  |
| Package          | LQFP-100                       |
| Revision         | 1.0                            |
| Date             | 2026-02-28                     |

---

## Pin Assignment Summary

| Peripheral     | Pins Used                              | Connected To     |
|----------------|----------------------------------------|------------------|
| SPI1           | PA4(CS), PA5(SCK), PA6(MISO), PA7(MOSI) | ADS1258 ADC    |
| SPI2           | PB12(CS), PB13(SCK), PB14(MISO), PB15(MOSI) | W5500 Ethernet |
| USART2         | PA2(TX), PA3(RX)                       | SP3485 RS-485    |
| GPIO (RS-485)  | PA8(DE/RE)                             | SP3485 DE+RE     |
| GPIO (ADS1258) | PB0(DRDY), PB1(START), PB2(RESET)     | ADS1258 control  |
| GPIO (W5500)   | PC6(RST), PC7(INT)                     | W5500 control    |
| GPIO (LEDs)    | PD12, PD13, PD14, PD15                | Status LEDs      |
| SWD            | PA13(SWDIO), PA14(SWCLK)              | Debug header J5  |
| HSE            | PH0(OSC_IN), PH1(OSC_OUT)             | 8 MHz crystal Y1 |
| LSE            | PC14(OSC32_IN), PC15(OSC32_OUT)        | 32.768 kHz Y2    |

---

## Complete LQFP-100 Pin Assignment Table

| Pin | Pin Name      | Function Assigned      | Direction | Notes                                      |
|-----|---------------|------------------------|-----------|--------------------------------------------|
| 1   | PE2           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 2   | PE3           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 3   | PE4           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 4   | PE5           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 5   | PE6           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 6   | VBAT          | VBAT                   | PWR       | Connected to +3.3V (no battery backup)     |
| 7   | PC13          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 8   | PC14/OSC32_IN | LSE Oscillator Input   | Input     | 32.768 kHz crystal Y2, load cap 6.8pF     |
| 9   | PC15/OSC32_OUT| LSE Oscillator Output  | Output    | 32.768 kHz crystal Y2, load cap 6.8pF     |
| 10  | PH0/OSC_IN    | HSE Oscillator Input   | Input     | 8 MHz crystal Y1, load cap 22pF           |
| 11  | PH1/OSC_OUT   | HSE Oscillator Output  | Output    | 8 MHz crystal Y1, load cap 22pF           |
| 12  | NRST          | System Reset           | Input     | 100nF to GND + 10K pull-up to 3.3V        |
| 13  | PC0           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 14  | PC1           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 15  | PC2           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 16  | PC3           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 17  | VSSA          | Analog Ground          | PWR       | Connected to GND plane                     |
| 18  | VDDA          | Analog VDD             | PWR       | +3.3V with 1uF + 100nF decoupling to VSSA |
| 19  | PA0           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 20  | PA1           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 21  | PA2           | USART2_TX              | Output    | SP3485 DI pin, Modbus RTU transmit         |
| 22  | PA3           | USART2_RX              | Input     | SP3485 RO pin, Modbus RTU receive          |
| 23  | VSS           | Ground                 | PWR       | Connected to GND plane                     |
| 24  | VDD           | Digital VDD            | PWR       | +3.3V with 100nF decoupling               |
| 25  | PA4           | SPI1_NSS (GPIO)        | Output    | ADS1258 CS (active low), software control  |
| 26  | PA5           | SPI1_SCK               | Output    | ADS1258 SCLK, 10.5 MHz, 33R series        |
| 27  | PA6           | SPI1_MISO              | Input     | ADS1258 DOUT, 33R series                   |
| 28  | PA7           | SPI1_MOSI              | Output    | ADS1258 DIN, 33R series                    |
| 29  | PC4           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 30  | PC5           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 31  | PB0           | GPIO Input (DRDY)      | Input     | ADS1258 DRDY, active low, 10K pull-up      |
| 32  | PB1           | GPIO Output (START)    | Output    | ADS1258 START, active high                 |
| 33  | PB2/BOOT1     | GPIO Output (RESET)    | Output    | ADS1258 RESET, active low, 10K pull-up     |
| 34  | PE7           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 35  | PE8           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 36  | PE9           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 37  | PE10          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 38  | PE11          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 39  | PE12          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 40  | PE13          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 41  | PE14          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 42  | PE15          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 43  | PB10          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 44  | PB11          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 45  | VSS           | Ground                 | PWR       | Connected to GND plane                     |
| 46  | VDD           | Digital VDD            | PWR       | +3.3V with 100nF decoupling               |
| 47  | PB12          | SPI2_NSS (GPIO)        | Output    | W5500 SCSn (active low), software control  |
| 48  | PB13          | SPI2_SCK               | Output    | W5500 SCLK, 21 MHz, 33R series            |
| 49  | PB14          | SPI2_MISO              | Input     | W5500 MISO, 33R series                    |
| 50  | PB15          | SPI2_MOSI              | Output    | W5500 MOSI, 33R series                    |
| 51  | PD8           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 52  | PD9           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 53  | PD10          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 54  | PD11          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 55  | PD12          | GPIO Output (LED1)     | Output    | Green LED -- System OK / Heartbeat         |
| 56  | PD13          | GPIO Output (LED2)     | Output    | Orange LED -- Modbus TX activity           |
| 57  | PD14          | GPIO Output (LED3)     | Output    | Red LED -- Fault / Sensor alarm            |
| 58  | PD15          | GPIO Output (LED4)     | Output    | Blue LED -- Ethernet link active           |
| 59  | PC6           | GPIO Output (W5500 RST)| Output    | W5500 RSTn, active low, 10K pull-up        |
| 60  | PC7           | GPIO Input (W5500 INT) | Input     | W5500 INTn, active low, 10K pull-up        |
| 61  | PC8           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 62  | PC9           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 63  | PA8           | GPIO Output (DE/RE)    | Output    | SP3485 DE and RE tied together             |
| 64  | PA9           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 65  | PA10          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 66  | PA11          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 67  | PA12          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 68  | PA13          | SWDIO                  | Bidir     | SWD debug data, J5 pin 2                  |
| 69  | VSS           | Ground                 | PWR       | Connected to GND plane                     |
| 70  | VDD           | Digital VDD            | PWR       | +3.3V with 100nF decoupling               |
| 71  | PA14          | SWCLK                  | Input     | SWD debug clock, J5 pin 4                 |
| 72  | PA15          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 73  | PC10          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 74  | PC11          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 75  | PC12          | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 76  | PD0           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 77  | PD1           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 78  | PD2           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 79  | PD3           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 80  | PD4           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 81  | PD5           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 82  | PD6           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 83  | PD7           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 84  | PB3           | NC/Reserved            | --        | Available for SWO trace output (J5 pin 6)  |
| 85  | PB4           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 86  | PB5           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 87  | PB6           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 88  | PB7           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 89  | BOOT0         | Boot Configuration     | Input     | 10K pull-down to GND (boot from Flash)     |
| 90  | PB8           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 91  | PB9           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 92  | PE0           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 93  | PE1           | NC/Reserved            | --        | Unused, configured as input with pull-down  |
| 94  | VSS           | Ground                 | PWR       | Connected to GND plane                     |
| 95  | VDD           | Digital VDD            | PWR       | +3.3V with 100nF decoupling               |
| 96  | VDD           | Digital VDD            | PWR       | +3.3V with 100nF decoupling               |
| 97  | VDD           | Digital VDD            | PWR       | +3.3V with 100nF decoupling (near VDDA)   |
| 98  | VSS           | Ground                 | PWR       | Connected to GND plane                     |
| 99  | VSS           | Ground                 | PWR       | Connected to GND plane                     |
| 100 | VDD           | Digital VDD            | PWR       | +3.3V with 100nF decoupling               |

---

## Pin Usage Statistics

| Category            | Count | Percentage |
|---------------------|-------|------------|
| Power (VDD/VSS)     | 14    | 14%        |
| Analog Power        | 2     | 2%         |
| Oscillator (HSE)    | 2     | 2%         |
| Oscillator (LSE)    | 2     | 2%         |
| Reset (NRST)        | 1     | 1%         |
| Boot (BOOT0)        | 1     | 1%         |
| SPI1 (ADS1258)      | 4     | 4%         |
| SPI2 (W5500)        | 4     | 4%         |
| USART2 (RS-485)     | 2     | 2%         |
| ADS1258 Control     | 3     | 3%         |
| W5500 Control       | 2     | 2%         |
| RS-485 DE/RE        | 1     | 1%         |
| LEDs                | 4     | 4%         |
| SWD Debug           | 2     | 2%         |
| **NC/Reserved**     | **56**| **56%**    |
| **Total**           |**100**| **100%**   |

---

## GPIO Configuration Summary

### Port A (PA0 - PA15)

| Pin   | GPIO  | AF/Mode              | Speed    | Pull     | Assignment              |
|-------|-------|----------------------|----------|----------|-------------------------|
| PA0   | PA0   | Input                | --       | Pull-down| NC/Reserved             |
| PA1   | PA1   | Input                | --       | Pull-down| NC/Reserved             |
| PA2   | PA2   | AF7 (USART2_TX)      | High     | None     | SP3485 DI               |
| PA3   | PA3   | AF7 (USART2_RX)      | --       | Pull-up  | SP3485 RO               |
| PA4   | PA4   | Output Push-Pull     | High     | None     | ADS1258 CS              |
| PA5   | PA5   | AF5 (SPI1_SCK)       | Very High| None     | ADS1258 SCLK            |
| PA6   | PA6   | AF5 (SPI1_MISO)      | Very High| None     | ADS1258 DOUT            |
| PA7   | PA7   | AF5 (SPI1_MOSI)      | Very High| None     | ADS1258 DIN             |
| PA8   | PA8   | Output Push-Pull     | High     | None     | SP3485 DE/RE            |
| PA9   | PA9   | Input                | --       | Pull-down| NC/Reserved             |
| PA10  | PA10  | Input                | --       | Pull-down| NC/Reserved             |
| PA11  | PA11  | Input                | --       | Pull-down| NC/Reserved             |
| PA12  | PA12  | Input                | --       | Pull-down| NC/Reserved             |
| PA13  | PA13  | AF0 (SWDIO)          | --       | Pull-up  | SWD debug data          |
| PA14  | PA14  | AF0 (SWCLK)          | --       | Pull-down| SWD debug clock         |
| PA15  | PA15  | Input                | --       | Pull-down| NC/Reserved             |

### Port B (PB0 - PB15)

| Pin   | GPIO  | AF/Mode              | Speed    | Pull     | Assignment              |
|-------|-------|----------------------|----------|----------|-------------------------|
| PB0   | PB0   | Input (EXTI0)        | --       | Pull-up  | ADS1258 DRDY (ext. 10K) |
| PB1   | PB1   | Output Push-Pull     | Low      | None     | ADS1258 START           |
| PB2   | PB2   | Output Push-Pull     | Low      | None     | ADS1258 RESET (ext. 10K)|
| PB3   | PB3   | Input                | --       | Pull-down| NC/Reserved (SWO avail) |
| PB4   | PB4   | Input                | --       | Pull-down| NC/Reserved             |
| PB5   | PB5   | Input                | --       | Pull-down| NC/Reserved             |
| PB6   | PB6   | Input                | --       | Pull-down| NC/Reserved             |
| PB7   | PB7   | Input                | --       | Pull-down| NC/Reserved             |
| PB8   | PB8   | Input                | --       | Pull-down| NC/Reserved             |
| PB9   | PB9   | Input                | --       | Pull-down| NC/Reserved             |
| PB10  | PB10  | Input                | --       | Pull-down| NC/Reserved             |
| PB11  | PB11  | Input                | --       | Pull-down| NC/Reserved             |
| PB12  | PB12  | Output Push-Pull     | High     | None     | W5500 SCSn              |
| PB13  | PB13  | AF5 (SPI2_SCK)       | Very High| None     | W5500 SCLK              |
| PB14  | PB14  | AF5 (SPI2_MISO)      | Very High| None     | W5500 MISO              |
| PB15  | PB15  | AF5 (SPI2_MOSI)      | Very High| None     | W5500 MOSI              |

### Port C (PC0 - PC15)

| Pin   | GPIO  | AF/Mode              | Speed    | Pull     | Assignment              |
|-------|-------|----------------------|----------|----------|-------------------------|
| PC0   | PC0   | Input                | --       | Pull-down| NC/Reserved             |
| PC1   | PC1   | Input                | --       | Pull-down| NC/Reserved             |
| PC2   | PC2   | Input                | --       | Pull-down| NC/Reserved             |
| PC3   | PC3   | Input                | --       | Pull-down| NC/Reserved             |
| PC4   | PC4   | Input                | --       | Pull-down| NC/Reserved             |
| PC5   | PC5   | Input                | --       | Pull-down| NC/Reserved             |
| PC6   | PC6   | Output Push-Pull     | Low      | None     | W5500 RSTn (ext. 10K)   |
| PC7   | PC7   | Input (EXTI7)        | --       | Pull-up  | W5500 INTn (ext. 10K)   |
| PC8   | PC8   | Input                | --       | Pull-down| NC/Reserved             |
| PC9   | PC9   | Input                | --       | Pull-down| NC/Reserved             |
| PC10  | PC10  | Input                | --       | Pull-down| NC/Reserved             |
| PC11  | PC11  | Input                | --       | Pull-down| NC/Reserved             |
| PC12  | PC12  | Input                | --       | Pull-down| NC/Reserved             |
| PC13  | PC13  | Input                | --       | Pull-down| NC/Reserved             |
| PC14  | PC14  | OSC32_IN             | --       | None     | 32.768 kHz crystal Y2   |
| PC15  | PC15  | OSC32_OUT            | --       | None     | 32.768 kHz crystal Y2   |

### Port D (PD0 - PD15)

| Pin   | GPIO  | AF/Mode              | Speed    | Pull     | Assignment              |
|-------|-------|----------------------|----------|----------|-------------------------|
| PD0   | PD0   | Input                | --       | Pull-down| NC/Reserved             |
| PD1   | PD1   | Input                | --       | Pull-down| NC/Reserved             |
| PD2   | PD2   | Input                | --       | Pull-down| NC/Reserved             |
| PD3   | PD3   | Input                | --       | Pull-down| NC/Reserved             |
| PD4   | PD4   | Input                | --       | Pull-down| NC/Reserved             |
| PD5   | PD5   | Input                | --       | Pull-down| NC/Reserved             |
| PD6   | PD6   | Input                | --       | Pull-down| NC/Reserved             |
| PD7   | PD7   | Input                | --       | Pull-down| NC/Reserved             |
| PD8   | PD8   | Input                | --       | Pull-down| NC/Reserved             |
| PD9   | PD9   | Input                | --       | Pull-down| NC/Reserved             |
| PD10  | PD10  | Input                | --       | Pull-down| NC/Reserved             |
| PD11  | PD11  | Input                | --       | Pull-down| NC/Reserved             |
| PD12  | PD12  | Output Push-Pull     | Low      | None     | Green LED (1K to LED)   |
| PD13  | PD13  | Output Push-Pull     | Low      | None     | Orange LED (1K to LED)  |
| PD14  | PD14  | Output Push-Pull     | Low      | None     | Red LED (1K to LED)     |
| PD15  | PD15  | Output Push-Pull     | Low      | None     | Blue LED (1K to LED)    |

### Port E (PE0 - PE15)

| Pin   | GPIO  | AF/Mode              | Speed    | Pull     | Assignment              |
|-------|-------|----------------------|----------|----------|-------------------------|
| PE0   | PE0   | Input                | --       | Pull-down| NC/Reserved             |
| PE1   | PE1   | Input                | --       | Pull-down| NC/Reserved             |
| PE2   | PE2   | Input                | --       | Pull-down| NC/Reserved             |
| PE3   | PE3   | Input                | --       | Pull-down| NC/Reserved             |
| PE4   | PE4   | Input                | --       | Pull-down| NC/Reserved             |
| PE5   | PE5   | Input                | --       | Pull-down| NC/Reserved             |
| PE6   | PE6   | Input                | --       | Pull-down| NC/Reserved             |
| PE7   | PE7   | Input                | --       | Pull-down| NC/Reserved             |
| PE8   | PE8   | Input                | --       | Pull-down| NC/Reserved             |
| PE9   | PE9   | Input                | --       | Pull-down| NC/Reserved             |
| PE10  | PE10  | Input                | --       | Pull-down| NC/Reserved             |
| PE11  | PE11  | Input                | --       | Pull-down| NC/Reserved             |
| PE12  | PE12  | Input                | --       | Pull-down| NC/Reserved             |
| PE13  | PE13  | Input                | --       | Pull-down| NC/Reserved             |
| PE14  | PE14  | Input                | --       | Pull-down| NC/Reserved             |
| PE15  | PE15  | Input                | --       | Pull-down| NC/Reserved             |

### Special Pins

| Pin # | Pin Name  | AF/Mode         | Notes                                      |
|-------|-----------|-----------------|---------------------------------------------|
| 6     | VBAT      | Power           | +3.3V (no backup battery)                  |
| 10    | PH0       | OSC_IN          | 8 MHz HSE crystal Y1                       |
| 11    | PH1       | OSC_OUT         | 8 MHz HSE crystal Y1                       |
| 12    | NRST      | Reset           | RC filter: 100nF to GND, 10K to +3.3V     |
| 17    | VSSA      | Analog GND      | Star ground to GND plane                   |
| 18    | VDDA      | Analog +3.3V    | 1uF + 100nF decoupling                    |
| 89    | BOOT0     | Boot Select     | 10K to GND (always boot from main Flash)   |

---

## Peripheral Conflict Check

The following table verifies there are no alternate function (AF) conflicts on assigned pins.

| Pin  | AF0       | AF5       | AF7        | Assigned AF | Conflict |
|------|-----------|-----------|------------|-------------|----------|
| PA2  | --        | --        | USART2_TX  | AF7         | None     |
| PA3  | --        | --        | USART2_RX  | AF7         | None     |
| PA4  | --        | SPI1_NSS  | --         | GPIO (CS)   | None     |
| PA5  | --        | SPI1_SCK  | --         | AF5         | None     |
| PA6  | --        | SPI1_MISO | --         | AF5         | None     |
| PA7  | --        | SPI1_MOSI | --         | AF5         | None     |
| PA13 | SWDIO     | --        | --         | AF0         | None     |
| PA14 | SWCLK     | --        | --         | AF0         | None     |
| PB12 | --        | SPI2_NSS  | --         | GPIO (CS)   | None     |
| PB13 | --        | SPI2_SCK  | --         | AF5         | None     |
| PB14 | --        | SPI2_MISO | --         | AF5         | None     |
| PB15 | --        | SPI2_MOSI | --         | AF5         | None     |

No alternate function conflicts detected. All peripheral assignments are valid.

---

## Interrupt Assignments

| IRQ Source         | Pin  | EXTI Line | Priority | Description                    |
|--------------------|------|-----------|----------|--------------------------------|
| ADS1258 DRDY       | PB0  | EXTI0     | High (1) | Data ready, triggers SPI read  |
| W5500 INTn         | PC7  | EXTI7     | Medium (5)| Socket event notification     |

---

## DMA Channel Assignments

| DMA    | Stream | Channel | Peripheral  | Direction | Purpose                     |
|--------|--------|---------|-------------|-----------|-----------------------------|
| DMA2   | Stream 0| Ch 3   | SPI1_RX     | P-to-M    | ADS1258 data receive        |
| DMA2   | Stream 3| Ch 3   | SPI1_TX     | M-to-P    | ADS1258 command transmit    |
| DMA1   | Stream 3| Ch 0   | SPI2_RX     | P-to-M    | W5500 data receive          |
| DMA1   | Stream 4| Ch 0   | SPI2_TX     | M-to-P    | W5500 data transmit         |
| DMA1   | Stream 6| Ch 4   | USART2_TX   | M-to-P    | Modbus RTU frame transmit   |
| DMA1   | Stream 5| Ch 4   | USART2_RX   | P-to-M    | Modbus RTU frame receive    |

---

## Design Notes

1. **Unused pins** are configured as GPIO inputs with internal pull-down resistors enabled to prevent floating inputs and reduce power consumption. They are marked as NC/Reserved and are available for future expansion.

2. **PA4 and PB12** (SPI chip selects) are driven as GPIO outputs rather than using the hardware NSS function. This provides software control of chip select timing, which is required for proper framing of ADS1258 and W5500 transactions.

3. **PB2 (BOOT1)** is shared with the ADS1258 RESET function. Since BOOT1 is only sampled during system reset and the ADS1258 RESET is driven high (inactive) during normal operation, there is no conflict. The MCU always boots from Flash (BOOT0=0, BOOT1=don't care).

4. **PA13 and PA14** are reserved for SWD debug access. These pins must not be repurposed in production without providing an alternative programming method.

5. **PB3** is left available as NC/Reserved but is routed to the SWD debug header (J5 pin 6) for optional SWO trace output during development.

6. **All 56 reserved pins** are physically routed to test pads on the PCB bottom layer for potential future use (additional sensors, display interface, SD card logging, etc.).

---

*End of WR-PCB-001 MCU Pin Assignment Table*
