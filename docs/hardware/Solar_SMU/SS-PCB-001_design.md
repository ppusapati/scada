# SS-PCB-001 Hardware Design Specification

**Document:** SS-PCB-001 Design Specification
**Board Name:** Solar String Monitoring Unit (SMU)
**Revision:** A
**Date:** 2026-02-28
**Status:** Released

---

## Table of Contents

1. [Board Overview](#1-board-overview)
2. [Functional Description](#2-functional-description)
3. [Circuit Design](#3-circuit-design)
   - 3.1 [Power Supply](#31-power-supply)
   - 3.2 [MCU Section](#32-mcu-section)
   - 3.3 [ADS1263 ADC Section](#33-ads1263-adc-section)
   - 3.4 [Analog MUX Section](#34-analog-mux-section)
   - 3.5 [String Input Conditioning](#35-string-input-conditioning)
   - 3.6 [TMP117 Temperature Sensor](#36-tmp117-temperature-sensor)
   - 3.7 [SD Card Section](#37-sd-card-section)
   - 3.8 [RS-485 Communication](#38-rs-485-communication)
   - 3.9 [LED Indicators](#39-led-indicators)
4. [Design Notes](#4-design-notes)

---

## 1. Board Overview

| Parameter          | Value                                      |
|--------------------|--------------------------------------------|
| Board ID           | SS-PCB-001                                 |
| Board Name         | Solar String Monitoring Unit (SMU)         |
| MCU                | STM32F407VGT6 (ARM Cortex-M4, 168 MHz)    |
| Package            | LQFP-100                                   |
| Flash              | 1 MB                                       |
| RAM                | 192 KB                                     |
| ADC                | ADS1263 (32-bit, delta-sigma)              |
| Analog MUX         | 2x CD74HC4067 (16-channel)                |
| Communication      | RS-485 Modbus RTU (SP3485)                 |
| Data Logging       | MicroSD card (SPI mode)                    |
| Temperature        | TMP117 (I2C, +/- 0.1 C accuracy)          |
| Supply Voltage     | 24V DC nominal (18-30V range)              |
| Board Dimensions   | 120 mm x 80 mm (4-layer PCB)              |
| Operating Temp     | -40 C to +85 C                             |

---

## 2. Functional Description

The SS-PCB-001 Solar String Monitoring Unit is a field-deployable data acquisition board designed for solar photovoltaic plant monitoring. It provides the following measurement and communication capabilities:

### Measurement Channels

| Channel Type             | Count | Range                 | Resolution        |
|--------------------------|-------|-----------------------|-------------------|
| String Voltage           | 16    | 0-1000V DC            | 24-bit (0.06V)    |
| String Current           | 16    | 0-50A DC              | 24-bit (3 uA)     |
| Bus Voltage              | 1     | 0-1000V DC            | 24-bit            |
| Bus Current              | 1     | 0-200A DC             | 24-bit            |
| Irradiance (pyranometer) | 1     | 0-1500 W/m^2          | 24-bit            |
| Ambient Temperature      | 1     | -40 C to +125 C       | 16-bit (0.0078 C) |

### Functional Block Diagram

```
                        +------------------------------------------------------+
                        |                  SS-PCB-001 SMU                       |
                        |                                                      |
  24V DC IN ============|=> [Power Supply] => 5V / 3.3V                        |
                        |                                                      |
  16x String V  ========|=> [Dividers] => [MUX A (CD74HC4067)] =\              |
                        |                                        \             |
  16x String I  ========|=> [Shunts]  => [MUX B (CD74HC4067)] ===> [ADS1263]   |
                        |                                        /    |        |
  Bus V/I, Irrad =======|=> [Conditioning] => [Direct AIN] =====/    |        |
                        |                                          SPI1        |
                        |                                            |        |
  Ambient Temp          |   [TMP117] ---- I2C1 ---+                  |        |
                        |                         |                  |        |
  SD Card     =========<|=> [MicroSD] -- SPI3 -+  |                  |        |
                        |                      |  |                  |        |
  RS-485 A/B  =========<|=> [SP3485] - USART2 -+--+--[STM32F407VGT6]+        |
                        |                      |                              |
  SWD Header  =========<|=> [J5 Debug] --------+                              |
                        |                                                      |
  LEDs (4x)             |   [Green][Orange][Red][Blue]                         |
                        +------------------------------------------------------+
```

### Communication Protocol

- **Physical Layer:** RS-485 half-duplex, 9600/19200/38400/115200 baud (configurable)
- **Protocol:** Modbus RTU, slave mode
- **Address Range:** 1-247 (configurable via DIP switch or register)
- **Data Logging:** CSV format on MicroSD (FAT32), daily file rotation

---

## 3. Circuit Design

### 3.1 Power Supply

The power supply accepts 24V DC (nominal, 18-30V range) from the plant DIN-rail supply and generates two regulated rails: 5.0V for the ADC analog reference and 3.3V for the MCU and digital logic.

#### Protection and Regulation Chain

```
                          D1                Q1 (P-MOSFET)
                       SMAJ30A             IRF9540N
    24V DC IN ──────┬──[TVS]──┬────┤G  S├────┬──────────────────────────────────┐
         (+)        |         |    │  D │    |                                  |
                    |        GND   └────┘    |                                  |
                    |                        |                                  |
                    |               R_GATE   |                                  |
                    └──────[100K]────┘       |                                  |
                                             |                                  |
                                        C1   |   C2                             |
                                       100uF |  100nF                           |
                                        │    |    │                             |
                                       GND   |   GND                            |
                                             |                                  |
                                     ┌───────┴────────┐                         |
                                     │    LM2596-5V   │                         |
                                     │                │                         |
                                     │ VIN       VOUT │──┬──[L1 33uH]──┐       |
                                     │                │  |              |       |
                                     │ ON/OFF    FB   │──┼──────────────┤       |
                                     │                │  |              |       |
                                     │       GND      │  C3            C4      |
                                     └────────┬───────┘  100uF         100nF   |
                                              |          │              │       |
                                             GND        GND            GND     |
                                                         |                     |
                              5.0V Rail  <───────────────┤                     |
                             (ADC Vref)                  |                     |
                                                         |                     |
                                                    ┌────┴─────┐               |
                                                    │AMS1117   │               |
                                                    │  -3.3    │               |
                                                    │VIN  VOUT │──┬─────────>  3.3V Rail
                                                    │          │  |            (MCU, Digital)
                                                    │   GND    │  C5    C6
                                                    └────┬─────┘  10uF  100nF
                                                         |        │     │
                                                        GND      GND   GND
                                                                        |
    24V DC IN ──────────────────────────────────────────────────────────┘
         (-)                          POWER GND PLANE
```

#### Component Selection

| Ref   | Component      | Value/Part       | Description                            |
|-------|----------------|------------------|----------------------------------------|
| D1    | TVS Diode      | SMAJ30A          | 30V standoff, 48.4V clamping           |
| Q1    | P-MOSFET       | IRF9540N         | Reverse polarity protection             |
| R_GATE| Resistor       | 100K, 0603       | Gate pull-up for Q1                    |
| U1    | Switching Reg  | LM2596S-5.0      | 5V, 3A step-down converter             |
| L1    | Inductor       | 33 uH, 3A        | LM2596 output inductor                 |
| D2    | Schottky Diode | SS34              | LM2596 freewheeling diode              |
| U2    | LDO Regulator  | AMS1117-3.3      | 3.3V, 1A LDO                           |
| C1    | Electrolytic   | 100 uF / 50V     | Input bulk capacitor                   |
| C2    | MLCC           | 100 nF / 50V     | Input decoupling                       |
| C3    | Electrolytic   | 100 uF / 10V     | LM2596 output bulk                     |
| C4    | MLCC           | 100 nF / 10V     | LM2596 output decoupling               |
| C5    | MLCC           | 10 uF / 10V      | AMS1117 output bulk                    |
| C6    | MLCC           | 100 nF / 10V     | AMS1117 output decoupling              |

#### Power Budget

| Rail   | Consumer             | Typical Current | Max Current |
|--------|----------------------|-----------------|-------------|
| 3.3V   | STM32F407VGT6        | 80 mA           | 150 mA      |
| 3.3V   | ADS1263 (DVDD/IOVDD) | 5 mA            | 10 mA       |
| 3.3V   | TMP117               | 0.3 mA          | 1 mA        |
| 3.3V   | SD Card              | 50 mA           | 100 mA      |
| 3.3V   | SP3485               | 1 mA            | 60 mA       |
| 3.3V   | LEDs (4x)            | 4 mA            | 8 mA        |
| 5.0V   | ADS1263 (AVDD)       | 8 mA            | 15 mA       |
| 5.0V   | CD74HC4067 (2x)      | 2 mA            | 4 mA        |
| **Total** |                   | **~150 mA**     | **~348 mA** |

---

### 3.2 MCU Section

#### STM32F407VGT6 (U3) - LQFP-100

The MCU is the central controller, running at 168 MHz using an 8 MHz HSE crystal with the internal PLL.

```
                              100nF  100nF  100nF  100nF  100nF
                               │      │      │      │      │
                    VDD pins: ─┴──────┴──────┴──────┴──────┴──  (pins 11,19,28,50,75,100)
                                                                 Each pin gets 100nF to GND
                               4.7uF
                               │
                    VBAT:   ───┴── (pin 6) + 100nF
                               4.7uF  100nF
                               │      │
                    VDDA:   ───┴──────┴──  (pin 22) Analog supply from 3.3V via ferrite bead
                               1uF    100nF
                               │      │
                    VCAP1:  ───┴──────┘    (pin 49)
                    VCAP2:  ───┴──────┘    (pin 73)

                    VSS pins: all tied to GND plane (pins 10,27,74,99)
                    VSSA:     tied to AGND (pin 21)


        HSE Crystal (Y1) - 8 MHz                 LSE Crystal (Y2) - 32.768 kHz
        ┌─────────────────────┐                   ┌─────────────────────────┐
        │                     │                   │                         │
        │   PH0 (pin 12)     │                   │   PC14 (pin 8)         │
        │    │                │                   │    │                   │
        │    ├───[Y1 8MHz]────┤                   │    ├───[Y2 32.768k]───┤
        │    │    ┌─┐         │                   │    │    ┌─┐           │
        │    │    └─┘         │                   │    │    └─┘           │
        │    │                │                   │    │                  │
        │   PH1 (pin 13)     │                   │   PC15 (pin 9)       │
        │    │                │                   │    │                  │
        │   C7    C8         │                   │   C9     C10         │
        │  20pF  20pF        │                   │  6.8pF  6.8pF       │
        │   │     │          │                   │   │      │          │
        │  GND   GND         │                   │  GND    GND         │
        └─────────────────────┘                   └─────────────────────┘


        Reset Circuit                             BOOT0 Configuration
        ┌───────────────────┐                     ┌───────────────────┐
        │                   │                     │                   │
        │  3.3V             │                     │  BOOT0 (pin 94)   │
        │   │               │                     │   │               │
        │  [R1 10K]         │                     │  [R2 10K]         │
        │   │               │                     │   │               │
        │   ├─── NRST (pin 14)                    │  GND              │
        │   │               │                     │                   │
        │  [C11 100nF]      │                     │  (Boot from Flash)│
        │   │               │                     └───────────────────┘
        │  GND              │
        │                   │
        │   ├─── to J5 pin 5│
        └───────────────────┘


        SWD Debug Header (J5) - 2x5 1.27mm pitch
        ┌──────────────────────────────────────┐
        │  Pin 1: 3.3V    │  Pin 2: SWDIO (PA13) │
        │  Pin 3: GND     │  Pin 4: SWCLK (PA14) │
        │  Pin 5: NRST    │  Pin 6: SWO   (PB3*) │
        │  Pin 7: NC      │  Pin 8: NC            │
        │  Pin 9: GND     │  Pin 10: GND          │
        └──────────────────────────────────────┘
        * PB3 shared with SPI3_SCK; SWO available only when SD card is not active
```

#### Clock Configuration

| Clock Source | Frequency    | Usage                             |
|--------------|--------------|-----------------------------------|
| HSE (Y1)     | 8 MHz        | PLL input -> SYSCLK 168 MHz       |
| LSE (Y2)     | 32.768 kHz   | RTC, watchdog                      |
| PLL          | 168 MHz      | SYSCLK                             |
| APB1         | 42 MHz       | USART2, SPI3, I2C1, TIM2-7        |
| APB2         | 84 MHz       | SPI1, USART1, TIM1/8-11            |

---

### 3.3 ADS1263 ADC Section

The ADS1263 (U4) is a 32-bit, 38.4 kSPS delta-sigma ADC providing high-resolution measurements for all analog channels. It connects to the MCU via SPI1.

```
                                STM32F407                              ADS1263 (U4)
                            ┌──────────────┐                      ┌──────────────────┐
                            │              │                      │                  │
                            │  PA5 (SCK)  ─┼──────────────────────┼─ SCLK            │
                            │              │                      │                  │
                            │  PA6 (MISO) ─┼──────────────────────┼─ DOUT            │
                            │              │                      │                  │
                            │  PA7 (MOSI) ─┼──────────────────────┼─ DIN             │
                            │              │                      │                  │
                            │  PA4 (CS)   ─┼──────────────────────┼─ /CS             │
                            │              │                      │                  │
                            │              │        10K           │                  │
                            │  PB0 (DRDY) ─┼───┬──[R3]──3.3V     │                  │
                            │              │   │                  │                  │
                            │              │   └──────────────────┼─ /DRDY           │
                            │              │                      │                  │
                            │  PB1 (RESET)─┼──────────────────────┼─ /RESET          │
                            │              │                      │                  │
                            └──────────────┘                      │                  │
                                                                  │                  │
                                                                  │  AIN0  ──────────┼─── From MUX A (voltages)
                                                                  │  AIN1  ──────────┼─── From MUX B (currents)
                                                                  │  AIN2  ──────────┼─── Bus Voltage (conditioned)
                                                                  │  AIN3  ──────────┼─── Bus Current (conditioned)
                                                                  │  AIN4  ──────────┼─── Irradiance sensor
                                                                  │  AIN5  ──────────┼─── Spare / Calibration
                                                                  │                  │
                                                                  │  AVDD ───────────┼─── 5.0V (via ferrite bead)
                                                                  │  AVSS ───────────┼─── AGND
                                                                  │  DVDD ───────────┼─── 3.3V
                                                                  │  IOVDD ──────────┼─── 3.3V
                                                                  │                  │
                                                                  │  VREFP ──────────┼─── Internal 2.5V ref (enabled)
                                                                  │  VREFN ──────────┼─── AGND
                                                                  │                  │
                                                                  └──────────────────┘

        ADS1263 Decoupling:
        ┌───────────────────────────────────────────┐
        │                                           │
        │  AVDD  ──┬── [100nF] ──┬── [10uF] ── GND │
        │          │             │                  │
        │  DVDD  ──┬── [100nF] ──┬── [10uF] ── GND │
        │          │             │                  │
        │  IOVDD ──┬── [100nF] ──┬── [10uF] ── GND │
        │                                           │
        │  Place decoupling caps as close to pins   │
        │  as physically possible (< 3mm).          │
        └───────────────────────────────────────────┘
```

#### ADS1263 Configuration

| Register     | Value  | Description                                |
|--------------|--------|--------------------------------------------|
| MODE0        | 0x00   | Continuous conversion, no delay             |
| MODE1        | 0x60   | Sinc4 filter, 60 SPS                        |
| MODE2        | 0x04   | PGA bypass, internal ref                    |
| INPMUX       | var    | Multiplexed per channel scan                |
| REFMUX       | 0x00   | Internal 2.5V reference                     |
| IDACMUX      | 0xFF   | IDACs disconnected                          |
| PGA          | bypass | Direct input, 0 to AVDD range               |

---

### 3.4 Analog MUX Section

Two CD74HC4067 16-channel analog multiplexers are used to expand the ADS1263 input channels. MUX A handles string voltage measurements; MUX B handles string current measurements.

```
        STM32F407                       MUX A - CD74HC4067 (U5)
    ┌──────────────┐               ┌──────────────────────────────┐
    │              │               │                              │
    │  PC0 (S0)   ─┼───────────────┼─ S0                         │
    │  PC1 (S1)   ─┼───────────────┼─ S1          Y0  ───────────┼─── String 1 Voltage
    │  PC2 (S2)   ─┼───────────────┼─ S2          Y1  ───────────┼─── String 2 Voltage
    │  PC3 (S3)   ─┼───────────────┼─ S3          Y2  ───────────┼─── String 3 Voltage
    │  PC4 (EN)   ─┼───────────────┼─ /EN         Y3  ───────────┼─── String 4 Voltage
    │              │               │              Y4  ───────────┼─── String 5 Voltage
    └──────────────┘               │              Y5  ───────────┼─── String 6 Voltage
                                   │              Y6  ───────────┼─── String 7 Voltage
                                   │              Y7  ───────────┼─── String 8 Voltage
                                   │              Y8  ───────────┼─── String 9 Voltage
                                   │              Y9  ───────────┼─── String 10 Voltage
                                   │              Y10 ───────────┼─── String 11 Voltage
                                   │              Y11 ───────────┼─── String 12 Voltage
                                   │              Y12 ───────────┼─── String 13 Voltage
                                   │              Y13 ───────────┼─── String 14 Voltage
                                   │              Y14 ───────────┼─── String 15 Voltage
                                   │              Y15 ───────────┼─── String 16 Voltage
                                   │                              │
                                   │  COM (Z) ────────────────────┼─── ADS1263 AIN0
                                   │                              │
                                   │  VCC = 5.0V    GND = GND    │
                                   └──────────────────────────────┘

        STM32F407                       MUX B - CD74HC4067 (U6)
    ┌──────────────┐               ┌──────────────────────────────┐
    │              │               │                              │
    │  PC5 (S0)   ─┼───────────────┼─ S0                         │
    │  PC6 (S1)   ─┼───────────────┼─ S1          Y0  ───────────┼─── String 1 Current
    │  PC7 (S2)   ─┼───────────────┼─ S2          Y1  ───────────┼─── String 2 Current
    │  PC8 (S3)   ─┼───────────────┼─ S3          Y2  ───────────┼─── String 3 Current
    │  PC9 (EN)   ─┼───────────────┼─ /EN         Y3  ───────────┼─── String 4 Current
    │              │               │              Y4  ───────────┼─── String 5 Current
    └──────────────┘               │              Y5  ───────────┼─── String 6 Current
                                   │              Y6  ───────────┼─── String 7 Current
                                   │              Y7  ───────────┼─── String 8 Current
                                   │              Y8  ───────────┼─── String 9 Current
                                   │              Y9  ───────────┼─── String 10 Current
                                   │              Y10 ───────────┼─── String 11 Current
                                   │              Y11 ───────────┼─── String 12 Current
                                   │              Y12 ───────────┼─── String 13 Current
                                   │              Y13 ───────────┼─── String 14 Current
                                   │              Y14 ───────────┼─── String 15 Current
                                   │              Y15 ───────────┼─── String 16 Current
                                   │                              │
                                   │  COM (Z) ────────────────────┼─── ADS1263 AIN1
                                   │                              │
                                   │  VCC = 5.0V    GND = GND    │
                                   └──────────────────────────────┘
```

#### MUX Channel Mapping

| MUX Channel | S3 | S2 | S1 | S0 | MUX A Signal       | MUX B Signal       |
|-------------|----|----|----|----|---------------------|---------------------|
| 0           | 0  | 0  | 0  | 0  | String 1 Voltage   | String 1 Current   |
| 1           | 0  | 0  | 0  | 1  | String 2 Voltage   | String 2 Current   |
| 2           | 0  | 0  | 1  | 0  | String 3 Voltage   | String 3 Current   |
| 3           | 0  | 0  | 1  | 1  | String 4 Voltage   | String 4 Current   |
| 4           | 0  | 1  | 0  | 0  | String 5 Voltage   | String 5 Current   |
| 5           | 0  | 1  | 0  | 1  | String 6 Voltage   | String 6 Current   |
| 6           | 0  | 1  | 1  | 0  | String 7 Voltage   | String 7 Current   |
| 7           | 0  | 1  | 1  | 1  | String 8 Voltage   | String 8 Current   |
| 8           | 1  | 0  | 0  | 0  | String 9 Voltage   | String 9 Current   |
| 9           | 1  | 0  | 0  | 1  | String 10 Voltage  | String 10 Current  |
| 10          | 1  | 0  | 1  | 0  | String 11 Voltage  | String 11 Current  |
| 11          | 1  | 0  | 1  | 1  | String 12 Voltage  | String 12 Current  |
| 12          | 1  | 1  | 0  | 0  | String 13 Voltage  | String 13 Current  |
| 13          | 1  | 1  | 0  | 1  | String 14 Voltage  | String 14 Current  |
| 14          | 1  | 1  | 1  | 0  | String 15 Voltage  | String 15 Current  |
| 15          | 1  | 1  | 1  | 1  | String 16 Voltage  | String 16 Current  |

---

### 3.5 String Input Conditioning

Each of the 16 solar strings requires two conditioned analog signals: one for voltage measurement and one for current measurement.

#### Voltage Input Circuit (Per String)

The string voltage (0-1000V DC max) is divided down to 0-2.5V using a precision resistor divider with a ratio of 401:1.

```
    String (+) ──────────┐
      (0-1000V)          │
                         │
                    D_TVS (Bidirectional)
                    SMBJ16A
                         │
                         ├─────────────────────────────────────────┐
                         │                                         │
                        [R1a]  500K, 0.5W, 0.1%                   │
                         │                                         │
                         ├── (midpoint, safety redundancy)         │
                         │                                         │
                        [R1b]  500K, 0.5W, 0.1%                   │
                         │                                         │
                         ├───────────┬───────────── To MUX A (Yn)  │
                         │           │                             │
                        [R2]       [C_F]                           │
                       2.5K       100nF                            │
                       0.1%       (anti-alias)                     │
                         │           │                             │
                        GND        GND                             │
                                                                   │
    String (-) ─────────────────────────────────────────────────────┘
                         │
                        GND (via system ground)

    Calculation:
      R_top  = R1a + R1b = 1 MΩ
      R_bot  = R2 = 2.5 kΩ
      Ratio  = (R_top + R_bot) / R_bot = 1,002,500 / 2,500 = 401:1
      V_out  = V_string / 401
      At 1000V: V_out = 1000 / 401 = 2.494V (within 2.5V ADC range)
      At 600V:  V_out = 600 / 401  = 1.496V (typical operating point)

    Anti-alias filter:
      f_c = 1 / (2 * pi * R2 * C_F) = 1 / (2 * pi * 2500 * 100e-9) = 637 Hz
```

**Note:** R1 is split into two series 500K resistors (R1a, R1b) for voltage stress derating. Each resistor sees a maximum of 500V.

#### Current Input Circuit (Per String)

String current is measured using an external hall-effect current transformer (CT) or a low-side shunt resistor. The standard configuration uses a 50A/75mV shunt, yielding 1.5 mV/A.

```
    String Current Path
    ═══════════════════════╤═══════════════════════
                           │
                     ┌─────┴─────┐
                     │  SHUNT    │
                     │  RESISTOR │
                     │ 50A/75mV  │
                     │  (1.5mΩ)  │
                     └─────┬─────┘
                           │
    ═══════════════════════╧═══════════════════════

         Shunt (+) ────┐              Shunt (-) ────┐
                       │                             │
                  D_TVS1 (ESD)                  D_TVS2 (ESD)
                  PESD5V0S1BA                   PESD5V0S1BA
                       │                             │
                      [R_IN1]                       [R_IN2]
                      100R                          100R
                       │                             │
                       ├──────┬──── To MUX B (Yn)    │
                       │      │         (+)          │
                       │    [C_F2]                   │
                       │    100nF                    │
                       │      │                      │
                       │      ├──────────────────────┤
                       │                             │
                      GND                           GND
                      (AGND)

    Calculation:
      At 50A:  V_shunt = 50A * 1.5mΩ = 75 mV
      At 10A:  V_shunt = 10A * 1.5mΩ = 15 mV  (typical operating point)
      Sensitivity = 1.5 mV/A
      ADC resolution at 24-bit, 2.5V ref = 2.5 / 2^24 = 0.149 uV/count
      Current resolution = 0.149 uV / 1.5 mV/A = 0.099 mA per count

    Anti-alias filter:
      f_c = 1 / (2 * pi * R_IN * C_F2) = 1 / (2 * pi * 100 * 100e-9) = 15.9 kHz
```

#### Summary Per-String BOM

| Ref     | Component       | Value               | Qty per String | Qty Total (x16) |
|---------|-----------------|----------------------|----------------|------------------|
| R1a     | Resistor        | 500K, 0.5W, 0.1%    | 1              | 16               |
| R1b     | Resistor        | 500K, 0.5W, 0.1%    | 1              | 16               |
| R2      | Resistor        | 2.5K, 0.1%          | 1              | 16               |
| C_F     | MLCC            | 100 nF, C0G/NP0     | 1              | 16               |
| D_TVS   | TVS Diode       | SMBJ16A             | 1              | 16               |
| R_IN1   | Resistor        | 100R, 0.1%          | 1              | 16               |
| R_IN2   | Resistor        | 100R, 0.1%          | 1              | 16               |
| C_F2    | MLCC            | 100 nF, C0G/NP0     | 1              | 16               |
| D_TVS1  | ESD Diode       | PESD5V0S1BA         | 1              | 16               |
| D_TVS2  | ESD Diode       | PESD5V0S1BA         | 1              | 16               |

---

### 3.6 TMP117 Temperature Sensor

The TMP117 (U7) is a high-accuracy (+/- 0.1 C) digital temperature sensor connected via I2C1. It measures ambient PCB temperature and can be used for cold-junction compensation or enclosure monitoring.

```
                                    3.3V
                                     │
                              ┌──────┤
                              │      │
                             [R4]   [R5]
                             4.7K   4.7K
                              │      │
        STM32F407             │      │           TMP117 (U7)
    ┌──────────────┐          │      │       ┌──────────────┐
    │              │          │      │       │              │
    │  PB6 (SCL)  ─┼──────────┴──────┼───────┼─ SCL         │
    │              │                 │       │              │
    │  PB7 (SDA)  ─┼─────────────────┴───────┼─ SDA         │
    │              │                         │              │
    └──────────────┘                         │  V+ ─── 3.3V │
                                             │              │
                                             │  GND ── GND  │
                                             │              │
                                             │  ADD0 ── GND │  (I2C addr = 0x48)
                                             │              │
                                             │  ALERT ── NC │  (not used)
                                             │              │
                                             └──────────────┘

    Decoupling: 100nF MLCC on V+ to GND, placed within 2mm of pin.
```

#### TMP117 Configuration

| Parameter     | Setting        | Description                          |
|---------------|----------------|--------------------------------------|
| I2C Address   | 0x48           | ADD0 = GND                           |
| Resolution    | 16-bit         | 0.0078125 C per LSB                  |
| Conversion    | 1 Hz           | Continuous mode, 1s cycle time       |
| Averaging     | 8 samples      | Internal hardware averaging          |
| Alert         | Not connected  | Unused; polled via I2C register read |

---

### 3.7 SD Card Section

A MicroSD card socket (J3) provides local data logging capability. The SD card is connected in SPI mode via SPI3. A card detect switch (active low) is provided on PD2.

```
        STM32F407                          MicroSD Socket (J3)
    ┌──────────────┐                   ┌────────────────────────┐
    │              │                   │                        │
    │  PB3 (SCK)  ─┼──────────────────┼─ CLK   (pin 5)        │
    │              │                   │                        │
    │  PB4 (MISO) ─┼──────────────────┼─ DAT0  (pin 7) [MISO] │
    │              │                   │                        │
    │  PB5 (MOSI) ─┼──────────────────┼─ CMD   (pin 2) [MOSI] │
    │              │                   │                        │
    │  PA15 (CS)  ─┼──────────────────┼─ DAT3  (pin 1) [CS]   │
    │              │                   │                        │
    │              │          3.3V     │  VDD   (pin 4)        │
    │              │           │       │                        │
    │              │         [100nF]   │  VSS   (pin 3,6) ─ GND│
    │              │           │       │                        │
    │              │          GND      │                        │
    │              │                   │  CD (detect) ──────────┼───┐
    │              │                   │                        │   │
    │              │                   └────────────────────────┘   │
    │              │                                                │
    │              │              10K pull-up to 3.3V               │
    │              │                │                               │
    │  PD2 (DET)  ─┼────────────────┴──────────────────────────────┘
    │              │
    └──────────────┘

    SPI3 speed: 400 kHz during initialization, 12 MHz during normal operation.
    File system: FAT32 via FatFs library.

    Note: 10K pull-up resistors on MISO, MOSI, CLK, and CS lines are recommended
          for proper SPI initialization when the card is not inserted.
```

---

### 3.8 RS-485 Communication

The RS-485 interface uses an SP3485 (U8) half-duplex transceiver connected to USART2. Direction control (DE/RE) is managed by PA8.

```
        STM32F407                    SP3485 (U8)                    RS-485 Bus
    ┌──────────────┐            ┌───────────────┐              ┌──────────────────┐
    │              │            │               │              │                  │
    │  PA2 (TX)   ─┼────────────┼─ DI           │              │                  │
    │              │            │               │              │                  │
    │  PA3 (RX)   ─┼────────────┼─ RO           │   D_TVS_A   │                  │
    │              │            │            A  ─┼───[TVS]─────┼─ A (+)           │
    │  PA8 (DE)   ─┼─────┬──────┼─ DE           │   PESD5V0   │                  │
    │              │     │      │            B  ─┼───[TVS]─────┼─ B (-)           │
    │              │     └──────┼─ /RE          │   PESD5V0   │                  │
    │              │            │               │              │  GND             │
    └──────────────┘            │  VCC ── 3.3V  │              │                  │
                                │  GND ── GND   │              └──────────────────┘
                                └───────────────┘                    │
                                                                     │
                                                              ┌──────┴──────┐
                                                              │  TERM (J4)  │
                                                              │             │
                                                        A ────┤   [120R]    ├──── B
                                                              │  (jumper)   │
                                                              └─────────────┘

    DE/RE tied together: PA8 HIGH = transmit, PA8 LOW = receive.

    Termination: 120 Ohm resistor between A and B, selectable via 2-pin jumper (J4).
                 Install jumper only on the last device on the RS-485 bus.

    Bias resistors:
      A line: 560R pull-up to 3.3V (ensures defined idle state)
      B line: 560R pull-down to GND (ensures defined idle state)
```

#### RS-485 / Modbus Configuration

| Parameter       | Value              | Description                       |
|-----------------|--------------------|-----------------------------------|
| Baud Rate       | 9600 (default)     | Configurable: 9600-115200         |
| Data Bits       | 8                  | Standard Modbus RTU               |
| Parity          | None               | Configurable: None/Even/Odd       |
| Stop Bits       | 1                  | Standard                          |
| Protocol        | Modbus RTU         | Slave mode                        |
| Slave Address   | 1 (default)        | Configurable: 1-247               |
| Turnaround      | 3.5 char times     | Inter-frame delay per Modbus spec |

---

### 3.9 LED Indicators

Four LEDs provide visual status indication. Each LED is driven through a 1K series resistor from a dedicated GPIO pin (active high, push-pull output).

```
        STM32F407

    PD12 ──[R6  1K]──┤>|── GND      (LED1, GREEN  - System OK / Heartbeat)
    PD13 ──[R7  1K]──┤>|── GND      (LED2, ORANGE - Communication activity)
    PD14 ──[R8  1K]──┤>|── GND      (LED3, RED    - Fault / Error)
    PD15 ──[R9  1K]──┤>|── GND      (LED4, BLUE   - SD card write activity)

    LED forward voltage: ~2.0V (Green/Orange), ~2.1V (Red), ~3.0V (Blue)
    LED forward current: ~1.3 mA (Green/Orange/Red), ~0.3 mA (Blue)
    Calculated: I = (3.3V - Vf) / 1K
```

| LED  | Color  | GPIO | Function                                  |
|------|--------|------|-------------------------------------------|
| LED1 | Green  | PD12 | Heartbeat (1 Hz blink = normal operation) |
| LED2 | Orange | PD13 | RS-485 TX/RX activity                     |
| LED3 | Red    | PD14 | Fault indicator (overcurrent, comm error) |
| LED4 | Blue   | PD15 | SD card write in progress                 |

---

## 4. Design Notes

### 4.1 EMC Considerations

1. **Ground Plane Strategy:** 4-layer stackup recommended:
   - Layer 1: Signal (top) + component placement
   - Layer 2: Continuous GND plane (unbroken under MCU and ADC)
   - Layer 3: Power plane (3.3V / 5V split)
   - Layer 4: Signal (bottom) + connectors

2. **Analog/Digital Ground Split:**
   - Maintain a dedicated AGND region under the ADS1263, MUX ICs, and input conditioning circuits.
   - Connect AGND to DGND at a single star point, located directly beneath the ADS1263 AVSS pin.
   - Do NOT route digital signals across the AGND region.

   ```
   ┌─────────────────────────────────────────────────────────────────────┐
   │  PCB Ground Plane (Layer 2)                                        │
   │                                                                     │
   │  ┌──────────────────────┐         ┌──────────────────────────────┐ │
   │  │                      │         │                              │ │
   │  │      AGND Region     │         │       DGND Region           │ │
   │  │                      │         │                              │ │
   │  │  [ADS1263] [MUX A/B] │===STAR==│  [STM32]  [SP3485]  [SD]   │ │
   │  │  [Input Conditioning]│  POINT  │  [TMP117] [LEDs]            │ │
   │  │                      │         │                              │ │
   │  └──────────────────────┘         └──────────────────────────────┘ │
   │                                                                     │
   └─────────────────────────────────────────────────────────────────────┘
   ```

3. **Ferrite Bead Isolation:**
   - Place a ferrite bead (BLM18PG221SN1, 220R @ 100MHz) between the digital 3.3V rail and the VDDA pin of the STM32.
   - Place a ferrite bead between the 5V rail and the AVDD pin of the ADS1263.

4. **Decoupling Strategy:**
   - Every VDD pin on the STM32 gets a dedicated 100 nF MLCC (X7R, 0402) placed within 2 mm of the pin.
   - Bulk capacitors (4.7 uF or 10 uF) placed near the ferrite bead outputs.
   - For the ADS1263, use 100 nF + 10 uF on AVDD, DVDD, and IOVDD.
   - All decoupling caps connect to the local ground plane via the shortest possible vias.

### 4.2 Guard Traces and ADC Input Routing

1. **Guard Traces:** Route guard traces (driven at the same potential as the input signal or at the signal's DC bias point) around all high-impedance ADC input traces (AIN0-AIN5).

   ```
   Cross-section of guarded ADC trace:

        GND (pour)      Guard trace       ADC input       Guard trace       GND (pour)
   ──────────────── ─────────────────── ═══════════════ ─────────────────── ────────────────
                     (driven at Vcm)      (AINx)         (driven at Vcm)
   ```

2. **Trace Routing Rules for Analog Signals:**
   - Keep analog input traces shorter than 25 mm where possible.
   - Route analog traces on the top layer only, directly over the unbroken AGND plane.
   - Maintain a minimum clearance of 0.5 mm between analog and digital traces.
   - No vias in the analog signal path (single-layer routing).
   - Use matched trace lengths for differential pairs (shunt current sense).

3. **High-Voltage Clearance:**
   - String voltage input traces (up to 1000V): minimum 2 mm clearance between conductors.
   - Creepage distance on connector footprints: minimum 4 mm for 1000V DC per IEC 62109-1.
   - Reinforced insulation boundary between high-voltage input and low-voltage sections.

### 4.3 Component Placement Guidelines

1. Place the ADS1263 as close to the MUX outputs as possible (< 10 mm trace length from MUX COM to ADS1263 AIN).
2. Place input conditioning resistor dividers near the board-edge connectors to minimize high-voltage trace length on the PCB.
3. Place the LM2596 switching regulator and its inductor/diode away from the analog section (opposite corner or edge recommended).
4. Orient the SD card socket for easy insertion from the enclosure front panel.
5. Place the SWD header at the board edge for probe access during development.

### 4.4 Thermal Considerations

1. The LM2596 dissipates approximately (24V - 5V) * 0.15A * (1 - efficiency) ~ 0.5W at typical load. Provide a copper pour under the thermal pad.
2. The AMS1117 dissipates (5V - 3.3V) * 0.15A ~ 0.26W. Ensure adequate copper area for heat spreading.
3. Operating temperature range: -40 C to +85 C. All components are rated for industrial temperature range.

### 4.5 Connector Specifications

| Ref  | Connector      | Type                    | Function               |
|------|----------------|-------------------------|------------------------|
| J1   | Power Input    | Phoenix MSTB 2.5/2-ST  | 24V DC supply          |
| J2   | String Inputs  | 2x Weidmuller LSF 34-pin | 16x V+ / V- / I+ / I- |
| J3   | MicroSD        | Molex 104031-0811       | SD card socket         |
| J4   | RS-485 Term    | 2-pin jumper header     | 120R termination       |
| J5   | SWD Debug      | 2x5 1.27mm header      | Programming / debug    |
| J6   | RS-485 Bus     | Phoenix MSTB 2.5/3-ST  | A, B, GND              |
| J7   | Aux Analog     | Phoenix MSTB 2.5/4-ST  | Bus V, Bus I, Irrad, GND |

---

**End of Document - SS-PCB-001 Design Specification**
