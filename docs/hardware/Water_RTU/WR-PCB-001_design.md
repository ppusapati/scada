# WR-PCB-001 Hardware Design Specification

## Water Remote Terminal Unit (RTU) - Board Design Document

| Field            | Value                          |
|------------------|--------------------------------|
| Document ID      | WR-PCB-001-DS-REV1             |
| Board            | WR-PCB-001                     |
| Revision         | 1.0                            |
| Date             | 2026-02-28                     |
| Status           | Preliminary                    |

---

## 1. Board Overview

The **WR-PCB-001** is a Water Remote Terminal Unit (RTU) designed for deployment in municipal and industrial water treatment and distribution monitoring systems. The board is built around the **STM32F407VGT6** ARM Cortex-M4 microcontroller and provides high-resolution analog acquisition of 8 industrial 4-20mA sensor loops with dual communication interfaces (RS-485 Modbus RTU and Ethernet Modbus TCP).

**Key Specifications:**

| Parameter              | Value                                      |
|------------------------|--------------------------------------------|
| MCU                    | STM32F407VGT6 (LQFP-100)                  |
| Core                   | ARM Cortex-M4F, 168 MHz                   |
| Flash / RAM            | 1 MB / 192 KB                              |
| ADC                    | ADS1258, 24-bit, 16-channel SAR            |
| Analog Inputs          | 8x 4-20mA single-ended                    |
| Communication          | RS-485 Modbus RTU, Ethernet Modbus TCP     |
| RS-485 Transceiver     | SP3485                                     |
| Ethernet Controller    | W5500 (hardwired TCP/IP)                   |
| Supply Voltage         | 24V DC nominal (18-30V operating)          |
| Power Consumption      | < 3W typical                               |
| Operating Temperature  | -20 to +70 degC                            |
| PCB Dimensions         | 120mm x 80mm (4-layer)                     |
| Mounting               | DIN rail (35mm) via clip bracket            |
| Enclosure Rating       | IP20 (panel mount) or IP65 (field housing) |

---

## 2. Functional Description

The WR-PCB-001 reads **8 industrial 4-20mA sensor loops** covering the following water quality and process parameters:

- **Pressure** (0-10 bar)
- **pH** (0-14 pH)
- **Turbidity** (0-1000 NTU)
- **Chlorine** (0-5 mg/L)
- **Flow** (0-100% scaled to L/min)
- **Tank Level** (0-100%)
- **Conductivity** (0-2000 uS/cm)
- **Dissolved Oxygen** (0-20 mg/L)

Each 4-20mA loop is conditioned through a precision 250 ohm shunt resistor to produce a 1.000V to 5.000V signal, which is digitized by the **ADS1258** 24-bit ADC. The MCU processes the raw ADC readings, applies calibration and scaling, performs fault detection, and makes data available over two independent communication interfaces:

1. **RS-485 Modbus RTU** -- for integration with existing SCADA infrastructure using serial fieldbus wiring. Half-duplex via SP3485 transceiver, supporting baud rates from 9600 to 115200.

2. **Ethernet Modbus TCP** -- for modern IP-based SCADA systems and direct HMI connectivity. Implemented via the W5500 hardwired TCP/IP Ethernet controller with an integrated TCP/IP stack, eliminating MCU stack overhead.

**Data Flow:**

```
Sensor (4-20mA) --> Input Conditioning --> ADS1258 (SPI1) --> STM32F407 --> Modbus RTU (RS-485)
                                                                       \-> Modbus TCP (Ethernet)
```

The MCU firmware operates a cyclic scan loop:
1. Trigger ADS1258 auto-scan conversion across CH0-CH7.
2. Read 24-bit results via SPI on DRDY interrupt.
3. Convert raw counts to engineering units using per-channel calibration coefficients.
4. Check fault thresholds (open circuit, under-range, over-range, short circuit).
5. Update Modbus holding registers (both RTU and TCP share the same register map).
6. Service Modbus requests from master/client.

---

## 3. Circuit Design

### 3.1 Power Supply

The power supply converts the 24V DC field supply down to 5V and 3.3V rails using the same topology as the Solar SMU board. Protection against reverse polarity, transients, and EMI is provided at the input stage.

**Power Tree:**

```
                          POWER SUPPLY BLOCK DIAGRAM
                          ==========================

  24V DC IN                                        5V RAIL             3.3V RAIL
  (18-30V)                                     (ADS1258 Vref,       (MCU, ADS1258
     |                                          W5500, SP3485)      digital, W5500)
     |                                              |                     |
     v                                              v                     v
  +--+--+    +------+    +------+    +--------+  +--+--+    +---------+  +--+--+
  | TB1 |--->| FB1  |--->| TVS  |--->| Q1     |->|LM2596|-->|AMS1117  |->| 3.3V|
  | 24V |    |Ferrite|   |SMBJ  |    |P-FET   |  |Step  |   |LDO 3.3V|  | OUT |
  | GND |    |Bead  |    |30A   |    |Reverse |  |Down  |   |         |  |     |
  +--+--+    +------+    +------+    |Polarity|  |to 5V |   +---------+  +-----+
                                     +--------+  +--+--+
                                                    |
                                                 +--+--+
                                                 |100uF|
                                                 |+10uF|
                                                 |decap|
                                                 +-----+
```

**Detailed Power Supply Schematic:**

```
                                    DETAILED POWER SUPPLY
                                    =====================

     24V IN        FB1         D1            Q1              LM2596-5.0
    +terminal   (Ferrite    (SMBJ30A      (Si2301DS                      L1
  o---+---[FB1]---+---+    TVS Clamp)    P-MOSFET)         +--------+  (33uH)
      |           |   |       |         S     G     D      |        |   ___
      |           |  ===     ===        |     |     |      | VIN  SW|--|___|--+
      |           |  |_|    |_| |       +--+--+     +------| EN     |        |
      |           |   |       |         |  |        |      | FB  BST|--||--+ |
      |           |   +-------+---------+  |        |      |     GND|  ||  | |
      |           |   |                    |        |      +---+----+  ||  | |
      |           |  C1                   R1       C2          |       D2  | |
      |           | 100uF                10K      100nF        |    (SS34) | |
      |           | 50V                   |        |           |       |   | |
      |           |  |                    |        |           |       +---+ |
  o---+-----------+--+--------------------+--------+-----------+-------+----+---> 5V OUT
   -terminal      |                                            |       |
      GND         +--------------------------------------------+-------+------- GND
                                                               |
                                                              C3
                                                            220uF
                                                             10V
                                                              |
                                                             GND

                      LDO: AMS1117-3.3
                      +-----------+
     5V OUT ----+-----|VIN    VOUT|-----+------> 3.3V OUT
                |     |     GND   |     |
               C4     +-----+-----+    C5
              10uF          |          10uF
              10V           |          10V
                |           |           |
               GND         GND        GND
```

**Component Details:**

| Ref  | Part           | Value/Rating        | Purpose                              |
|------|----------------|---------------------|--------------------------------------|
| FB1  | Ferrite Bead   | 600 ohm @ 100 MHz  | Input EMI filter                     |
| C1   | Electrolytic   | 100uF / 50V        | Input bulk capacitor                 |
| D1   | TVS Diode      | SMBJ30A (30V)      | Input transient suppression          |
| Q1   | P-MOSFET       | Si2301DS            | Reverse polarity protection          |
| R1   | Resistor       | 10K                 | Gate pull-up for Q1                  |
| C2   | Ceramic        | 100nF               | Q1 gate bypass                       |
| U1   | LM2596-5.0     | 5V / 3A step-down   | Primary DC-DC converter              |
| L1   | Inductor       | 33uH / 3A           | LM2596 output inductor               |
| D2   | Schottky       | SS34 (3A/40V)       | LM2596 freewheeling diode            |
| C3   | Electrolytic   | 220uF / 10V         | 5V output bulk capacitor             |
| U2   | AMS1117-3.3    | 3.3V / 1A LDO       | 3.3V linear regulator                |
| C4   | Ceramic        | 10uF / 10V          | LDO input capacitor                  |
| C5   | Ceramic        | 10uF / 10V          | LDO output capacitor                 |

**Rail Loading Summary:**

| Rail   | Source       | Max Current | Loads                                |
|--------|-------------|-------------|--------------------------------------|
| 24V    | Field PSU   | Fused 500mA | Input to DC-DC only                  |
| 5V     | LM2596      | 3A max      | ADS1258 AVDD/Vref, W5500, SP3485    |
| 3.3V   | AMS1117     | 1A max      | STM32F407, ADS1258 DVDD, W5500 I/O  |

---

### 3.2 MCU Section -- STM32F407VGT6 (LQFP-100)

The microcontroller is the STM32F407VGT6 in a 100-pin LQFP package, running at 168 MHz from the internal PLL clocked by an external 8 MHz crystal.

**MCU Core Connections:**

```
                        STM32F407VGT6 MCU CONNECTIONS
                        ==============================

                          +3.3V
                           |
                      +----+----+
                      |  4.7uF  |  (bulk, ceramic, placed near pin 11)
                      +----+----+
                           |
     +3.3V ---+---+---+---+---+---+---+---+---+---+---+
              |   |   |   |   |   |   |   |   |   |   |
            100nF each (one per VDD/VDDA pin)          |
              |   |   |   |   |   |   |   |   |   |   |
              +---+---+---+---+---+---+---+---+---+---+
              |                                         |
         VDD pins: 11, 19, 28, 50,               VDDA (pin 22)
                    75, 100                        |
              |                                   1uF + 100nF
              |                                    to VSSA
         +----+----+
         |         |
     +---+---------+---+
     |  STM32F407VGT6  |
     |   (LQFP-100)    |
     |                  |
     | HSE IN  (pin 12) |<---[8 MHz XTAL Y1]---+
     | HSE OUT (pin 13) |<---------------------+|
     |                  |      22pF   22pF      ||
     |                  |       |       |       ||
     |                  |      GND    GND       ||
     |                  |                       ||
     | LSE IN  (pin 8)  |<---[32.768kHz Y2]---+ |
     | LSE OUT (pin 9)  |<--------------------+||
     |                  |      6.8pF  6.8pF    |||
     |                  |       |       |      |||
     |                  |      GND    GND      |||
     |                  |                      |||
     | BOOT0   (pin 94) |---[10K]---GND        |||
     |                  |                      |||
     | NRST    (pin 14) |---[100nF]---GND      |||
     |                  | \---[10K]---+3.3V    |||
     |                  |                      |||
     | VBAT    (pin 6)  |---+3.3V              |||
     |                  |                      |||
     | VSS     (pins)   |---GND (all VSS pins) |||
     | VSSA    (pin 23) |---GND                |||
     +------------------+                      |||
```

**SWD Debug Header (J5):**

```
        J5: SWD DEBUG (2x5 1.27mm)
        +---+---+
    VCC | 1 | 2 | SWDIO (PA13)
        +---+---+
    GND | 3 | 4 | SWCLK (PA14)
        +---+---+
    GND | 5 | 6 | SWO   (PB3) -- optional trace output
        +---+---+
    N/C | 7 | 8 | N/C
        +---+---+
   NRST | 9 |10 | GND
        +---+---+
```

**Crystal Specifications:**

| Parameter       | HSE (Y1)          | LSE (Y2)              |
|-----------------|--------------------|------------------------|
| Frequency       | 8.000 MHz          | 32.768 kHz             |
| Tolerance       | +/- 20 ppm         | +/- 20 ppm            |
| Load Caps       | 22 pF each         | 6.8 pF each            |
| Package         | HC49/S             | 2x1.2mm SMD            |
| Purpose         | System clock (PLL) | RTC timebase            |

**MCU Clock Configuration:**

- HSE = 8 MHz
- PLL_M = 8, PLL_N = 336, PLL_P = 2 --> SYSCLK = 168 MHz
- AHB prescaler = 1 --> HCLK = 168 MHz
- APB1 prescaler = 4 --> PCLK1 = 42 MHz
- APB2 prescaler = 2 --> PCLK2 = 84 MHz

---

### 3.3 ADS1258 ADC Section

The ADS1258 is a 24-bit, 16-channel delta-sigma ADC from Texas Instruments operating in auto-scan mode. It is connected to the MCU via SPI1 and reads the 8 conditioned 4-20mA input channels.

**ADS1258 Configuration:**

| Parameter          | Setting                          |
|--------------------|----------------------------------|
| Mode               | Auto-scan (channels CH0-CH7)     |
| STATUS byte        | Enabled                          |
| CHOP               | Enabled (offset correction)       |
| Data rate           | ~23.7 kSPS per channel (fixed)   |
| Reference           | External, VREFP = 5.0V, VREFN = GND |
| Input range         | 0 to +VREF (unipolar)            |
| Active channels     | CH0-CH7 (single-ended)           |
| Disabled channels   | CH8-CH15                         |
| Digital interface   | SPI, CPOL=0, CPHA=1 (Mode 1)    |

**ADS1258 Pin Connections:**

```
                        ADS1258 ADC CONNECTION DIAGRAM
                        ================================

                                  +5V (AVDD)            +3.3V (DVDD)
                                    |                      |
                                +---+---+              +---+---+
                                |100nF  | 10uF         |100nF  | 10uF
                                |  ||   | _|_          |  ||   | _|_
                                |  ||   | | |          |  ||   | | |
                                +---+---+ +-+          +---+---+ +-+
                                    |       |              |       |
                                    +---+---+              +---+---+
                                        |                      |
         STM32F407                 +----+----------------------+----+
         =========                 |   AVDD                  DVDD   |
                                   |                                |
         PA5 (SCK)  ----[33R]----->| SCLK              AIN0  (CH0) |<--- Pressure
         PA6 (MISO) <---[33R]------| DOUT              AIN1  (CH1) |<--- pH
         PA7 (MOSI) ----[33R]----->| DIN               AIN2  (CH2) |<--- Turbidity
         PA4 (CS)   ----[33R]----->| CS                 AIN3  (CH3) |<--- Chlorine
                                   |                    AIN4  (CH4) |<--- Flow
         PB0 (DRDY) <--+--[10K]-->| DRDY               AIN5  (CH5) |<--- Tank Level
                        |   +3.3V  |                    AIN6  (CH6) |<--- Conductivity
         PB1 (START) ------------>| START               AIN7  (CH7) |<--- Dissolved O2
                                   |                                |
         PB2 (RESET) ---[10K]--+->| RESET              AIN8-AIN15  |--- (NC, tied to AINCOM)
                                |  |                                |
                              +3.3V|    VREFP   VREFN    AINCOM    |
                                   +----+---------+--------+-------+
                                        |         |        |
                                      +5.0V      GND      GND
                                        |
                                    +---+---+
                                    |100nF  | 10uF
                                    |  ||   | _|_
                                    |  ||   | | |
                                    +---+---+ +-+
                                        |       |
                                        +---+---+
                                            |
                                           GND
```

**ADS1258 Register Configuration:**

| Register     | Address | Value  | Description                                      |
|--------------|---------|--------|--------------------------------------------------|
| CONFIG0      | 0x00    | 0x0A   | CHOP enabled, STATUS enabled, auto-scan mode     |
| CONFIG1      | 0x01    | 0x20   | IDLMOD=0, DLY=010 (8 DRDY periods delay)        |
| MUXSCH       | 0x02    | 0x00   | Not used in auto-scan                            |
| MUXDIF       | 0x03    | 0x00   | No differential pairs                            |
| MUXSG0       | 0x04    | 0xFF   | CH0-CH7 enabled (single-ended)                   |
| MUXSG1       | 0x05    | 0x00   | CH8-CH15 disabled                                |
| SYSRED       | 0x06    | 0x00   | System monitors disabled                         |
| GPIOC        | 0x07    | 0x00   | GPIO not used                                    |
| GPIOD        | 0x08    | 0x00   | GPIO data default                                |

**SPI1 Timing:**

- SPI clock: 10.5 MHz (APB2/8 = 84/8)
- Mode: CPOL=0, CPHA=1 (SPI Mode 1)
- Data: 8-bit, MSB first
- CS: software controlled (PA4 GPIO)

---

### 3.4 4-20mA Input Conditioning

Each of the 8 analog input channels uses an identical conditioning circuit to convert the 4-20mA current loop signal into a voltage suitable for the ADS1258 ADC input.

**Design Equations:**

- Shunt resistor: R_shunt = 250 ohm (0.1% tolerance, 15 ppm/degC)
- V_min = 4 mA x 250 ohm = 1.000 V
- V_max = 20 mA x 250 ohm = 5.000 V
- ADC full scale = 5.000 V (external Vref)
- Resolution per bit = 5.0 V / 2^24 = 298 nV
- Current resolution = 298 nV / 250 ohm = 1.19 nA (theoretical)

**Per-Channel Input Conditioning Circuit:**

```
            PER-CHANNEL 4-20mA INPUT CONDITIONING
            =======================================

                                      +5V (Vref)
                                        |
                                       D2
                                    (Schottky
                                     BAT54)
                                        |
  4-20mA LOOP    D1        R_F        _|_
  FROM SENSOR   (TVS)    (100 ohm)    | |
  TERMINAL    bi-direct    EMI       R_SHUNT
  ----+--------->|<--------[===]---+--| |--- (250 ohm, 0.1%)
      |       (SM6T6V8A)           |  |_|
      |          |                 |    |
      |         GND               |   GND
      |                           |
      |                          C_F
      |                        (100nF)
      |                        ceramic
      |                           |
      |                          GND
      |
      |                    +------+------> To ADS1258 CHx input
      |                    |
      |                   D3
      |                (Schottky
      |                 BAT54)
      |                    |
      |                   GND
      |
    RETURN
    (GND)


    SIGNAL PATH DETAIL:
    ====================

    4-20mA IN ---[D1: TVS 6.8V]---[R_F: 100R]---+---[R_SHUNT: 250R]--- GND
                       |                          |         |
                      GND                       [C_F]       |
                                                [100nF]     +----------> ADS1258 CHx
                                                  |         |
                                                 GND      [D2] to +5V (Schottky clamp)
                                                          [D3] to GND  (Schottky clamp)
```

**Voltage at ADC Input vs. Loop Current:**

```
    Voltage (V)
    5.000 |..................................*  (20 mA)
          |                              .*
          |                           .*
          |                        .*
          |                     .*
          |                  .*
          |               .*
          |            .*
          |         .*
    1.000 |......*                            (4 mA)
          |   .*
    0.000 |*______________________________________
          0     4     8    12    16    20    24
                    Loop Current (mA)
```

**Component Values Per Channel (x8 identical):**

| Ref       | Part               | Value             | Purpose                         |
|-----------|--------------------|-------------------|---------------------------------|
| D1        | TVS Diode          | SM6T6V8A (6.8V)  | ESD/surge protection, bidir.    |
| R_F       | Resistor           | 100 ohm, 1%      | EMI filter series element        |
| C_F       | Ceramic Cap        | 100nF, 50V, C0G  | EMI filter shunt element         |
| R_SHUNT   | Precision Resistor | 250 ohm, 0.1%    | Current-to-voltage conversion    |
| D2        | Schottky Diode     | BAT54             | Overvoltage clamp to +5V Vref   |
| D3        | Schottky Diode     | BAT54             | Undervoltage clamp to GND       |

**EMI Filter Characteristics:**

- Filter type: single-pole RC low-pass
- R = 100 ohm, C = 100 nF
- Cutoff frequency: f_c = 1 / (2 * pi * R * C) = 15.9 kHz
- Adequate for rejecting industrial noise while preserving the 4-20mA signal bandwidth (typically < 10 Hz update rate from sensors)

---

### 3.5 W5500 Ethernet Section

The W5500 is a hardwired TCP/IP embedded Ethernet controller from WIZnet providing 8 simultaneous socket connections. It offloads the entire TCP/IP stack from the MCU, simplifying Modbus TCP implementation.

**W5500 Connection Diagram:**

```
                         W5500 ETHERNET CONNECTION DIAGRAM
                         ==================================

                                    +3.3V
                                      |
                                    [FB2]  (ferrite bead, isolated 3.3V)
                                      |
                                    +3.3VA
                                      |
                                +-----+-----+
                                |100nF|100nF |100nF  (one per VDD pin)
                                |     |      |
                                +-----+-----+
                                      |
         STM32F407               +----+----+              HR911105A
         =========               |  W5500  |              RJ45 JACK
                                 |         |          (w/ magnetics)
     PB13 (SCK)  ---[33R]------>| SCLK    |             +--------+
     PB14 (MISO) <--[33R]------| MISO    |             |        |
     PB15 (MOSI) ---[33R]------>| MOSI    |  TX+       | 1 TX+  |===\
     PB12 (CS)   ---[33R]------>| SCSn    |--[49.9R]-->| 2 TX-  |====\
                                 |         |  TX-       |        |=====}==> RJ45
     PC7 (INT)   <--[10K]--+---| INTn    |--[49.9R]-->| 3 RX+  |====/ Ethernet
                    +3.3V   |   |         |  RX+       | 6 RX-  |===/  Cable
                            |   |         |<-----------| (mag.)  |
     PC6 (RST)   ----------+-->| RSTn    |  RX-       |        |
                   [10K]    |   |         |<-----------| LEDs   |
                    |       |   |    RSVD |--[12.4K]-->|  GND   |
                  +3.3V     |   |     GND |--GND       +--------+
                            |   |         |
                            |   | EXRES1  |---[12.4K]---GND
                            |   |         |
                            |   |  XTAL   |
                            |   |  XI  XO |
                            |   +--+-+--+-+
                            |      | |  |
                            |     [25 MHz]
                            |      Y3
                            |      | |
                            |    [22pF][22pF]
                            |      |     |
                            |     GND   GND
                            |
                          [100nF]
                            |
                           GND
```

**W5500 Network Configuration (Default):**

| Parameter      | Default Value     |
|----------------|-------------------|
| IP Address     | 192.168.1.100     |
| Subnet Mask    | 255.255.255.0     |
| Gateway        | 192.168.1.1       |
| MAC Address    | 02:00:00:XX:XX:XX |
| Modbus TCP Port| 502               |

**SPI2 Timing:**

- SPI clock: 21 MHz (APB1/2 = 42/2)
- Mode: CPOL=0, CPHA=0 (SPI Mode 0)
- Data: 8-bit, MSB first
- CS: software controlled (PB12 GPIO)

**Component Details:**

| Ref  | Part               | Value           | Purpose                          |
|------|--------------------|-----------------|----------------------------------|
| U4   | W5500              | QFN-48          | Hardwired TCP/IP Ethernet        |
| Y3   | Crystal            | 25 MHz          | W5500 clock source               |
| J2   | HR911105A          | RJ45+magnetics  | Ethernet connector with xfmr     |
| FB2  | Ferrite Bead       | 600R @ 100MHz   | Analog supply isolation           |
| R_TX | Resistor (x2)      | 49.9 ohm        | TX line impedance matching        |
| R_PU | Pull-up (x2)       | 10K             | INTn, RSTn pull-ups               |
| R_EX | Resistor           | 12.4K, 1%       | EXRES1 bias resistor              |

---

### 3.6 RS-485 Section -- SP3485 Transceiver

The RS-485 interface uses the SP3485 half-duplex transceiver for Modbus RTU communication with the SCADA master station.

**RS-485 Connection Diagram:**

```
                          RS-485 INTERFACE CIRCUIT
                          =========================

                              +3.3V            +3.3V
                                |                |
                              [R_A]            [R_B]
                              470 ohm          470 ohm
                              (Failsafe         (Failsafe
                               Bias)             Bias)
                                |                |
         STM32F407           +--+---+            |        J4: RS-485
         =========           |SP3485|            |       TERMINAL BLOCK
                             |      |            |       +---------+
     PA2 (TX)  ------------>| DI  A |---+--------+--+--->| A (+)   |
                             |      |   |           |    |         |
     PA3 (RX)  <------------| RO  B |---+-----------+--->| B (-)   |
                             |      |   |           |    |         |
     PA8 (DE)  -----+------>| DE    |   |           |    | GND     |
                    |   +--->| RE    |  [D4]       [D5]  +---------+
                    |   |    |      |  (TVS)      (TVS)      |
                    +---+    | GND  |  SMBJ6.5   SMBJ6.5    |
                             +--+---+   |           |        |
                                |       |           |        |
                               GND     GND         GND      |
                                                             |
                                                          +--+--+
                                                   J3:    |120 R|  Termination
                                                  JUMPER  |     |  (install J3
                                                          +-----+  for end-of-line)

    DE/RE ACCENT ACCENT ACCENT ACCENT TIMING:
    ==================
    PA8 HIGH = Transmit mode (DE=1, RE=1 --> driver enabled, receiver disabled)
    PA8 LOW  = Receive mode  (DE=0, RE=0 --> driver disabled, receiver enabled)
```

**Modbus RTU Configuration (Default):**

| Parameter         | Default Value      |
|-------------------|--------------------|
| Baud Rate         | 9600 bps           |
| Data Bits         | 8                  |
| Parity            | None               |
| Stop Bits         | 2                  |
| Slave Address     | 1                  |
| Response Timeout  | 1000 ms            |

**Failsafe Bias Explanation:**

When the RS-485 bus is open (no driver active), the failsafe bias resistors ensure the receiver sees a defined logic HIGH (idle/mark) state, preventing false start-bit detection:
- R_A = 470 ohm to VCC on the A line (pulls A high)
- R_B = 470 ohm to GND on the B line (pulls B low)
- This ensures V_A > V_B when bus is idle, which is the Modbus idle state

**Component Details:**

| Ref  | Part            | Value          | Purpose                            |
|------|-----------------|----------------|------------------------------------|
| U3   | SP3485          | SOIC-8         | RS-485 half-duplex transceiver     |
| R_A  | Resistor        | 470 ohm        | A-line failsafe pull-up to VCC     |
| R_B  | Resistor        | 470 ohm        | B-line failsafe pull-down to GND   |
| J3   | 2-pin jumper    | --             | 120 ohm termination enable         |
| R_T  | Resistor        | 120 ohm        | Line termination (end-of-line)     |
| D4   | TVS Diode       | SMBJ6.5        | A-line surge protection            |
| D5   | TVS Diode       | SMBJ6.5        | B-line surge protection            |
| J4   | Terminal Block  | 3-pin, 5.08mm  | Field wiring (A, B, GND)          |

---

### 3.7 LED Section

Four indicator LEDs are provided for system status feedback, connected to Port D GPIO pins with current-limiting resistors.

**LED Circuit:**

```
                           LED INDICATOR CIRCUIT
                           ======================

     STM32F407
     GPIO Pin          R (1K)        LED          Color      Function
     =========        ========      =====        =======    ==========

     PD12 ----+------[1K ohm]----->|---- GND     GREEN      System OK / Heartbeat
              |
     PD13 ----+------[1K ohm]----->|---- GND     ORANGE     Modbus TX activity
              |
     PD14 ----+------[1K ohm]----->|---- GND     RED        Fault / Sensor alarm
              |
     PD15 ----+------[1K ohm]----->|---- GND     BLUE       Ethernet link active
```

**LED Behavior:**

| LED    | Color  | Pin  | Steady ON              | Blinking (1 Hz)           | OFF                  |
|--------|--------|------|------------------------|---------------------------|----------------------|
| LED1   | Green  | PD12 | System running OK      | Initializing              | System halted        |
| LED2   | Orange | PD13 | --                     | Modbus RTU/TCP activity   | No comm activity     |
| LED3   | Red    | PD14 | Critical fault         | Sensor warning            | All sensors OK       |
| LED4   | Blue   | PD15 | Ethernet link up       | Ethernet activity         | No Ethernet link     |

---

## 4. Channel Assignment Table

The following table defines the mapping between ADS1258 input channels and water quality/process sensors. Each channel corresponds to one 4-20mA input conditioning circuit.

| Channel | ADS1258 Input | Sensor Type      | Measurement Range | Engineering Unit | 4mA Value | 20mA Value | Modbus Register |
|---------|---------------|------------------|--------------------|-----------------|-----------|------------|-----------------|
| CH0     | AIN0          | Pressure         | 0 - 10 bar         | bar             | 0.000     | 10.000     | 40001-40002     |
| CH1     | AIN1          | pH               | 0 - 14             | pH              | 0.000     | 14.000     | 40003-40004     |
| CH2     | AIN2          | Turbidity        | 0 - 1000 NTU       | NTU             | 0.000     | 1000.000   | 40005-40006     |
| CH3     | AIN3          | Chlorine         | 0 - 5 mg/L         | mg/L            | 0.000     | 5.000      | 40007-40008     |
| CH4     | AIN4          | Flow             | 0 - 100%           | L/min           | 0.000     | 100.000    | 40009-40010     |
| CH5     | AIN5          | Tank Level       | 0 - 100%           | %               | 0.000     | 100.000    | 40011-40012     |
| CH6     | AIN6          | Conductivity     | 0 - 2000 uS/cm     | uS/cm           | 0.000     | 2000.000   | 40013-40014     |
| CH7     | AIN7          | Dissolved Oxygen | 0 - 20 mg/L        | mg/L            | 0.000     | 20.000     | 40015-40016     |

**Scaling Formula:**

```
Engineering_Value = ((ADC_Voltage - 1.000) / (5.000 - 1.000)) * (Range_Max - Range_Min) + Range_Min
```

Where:
- ADC_Voltage = (ADC_Raw_Code / 2^24) * 5.000 V
- 1.000 V corresponds to 4 mA (range minimum)
- 5.000 V corresponds to 20 mA (range maximum)

**Modbus Register Map Notes:**
- Each channel occupies 2 consecutive 16-bit holding registers (IEEE 754 32-bit float, big-endian)
- Function code 03 (Read Holding Registers) is used to read sensor values
- Function code 04 (Read Input Registers) at addresses 30001-30008 returns raw ADC codes (32-bit unsigned)
- Status registers at 40101-40108 contain per-channel fault flags

---

## 5. 4-20mA Fault Detection

The firmware continuously monitors each 4-20mA channel for fault conditions based on the measured loop current. Fault detection thresholds are defined to distinguish between normal operation, degraded conditions, and hard faults.

**Fault Detection Thresholds:**

```
    Current (mA)
    ============

    >24.0  +---------+  SHORT CIRCUIT
           |         |  Sensor or wiring shorted. Immediate alarm.
    24.0   +---------+
           |         |
    20.5   +---------+  OVER-RANGE
           |         |  Sensor reading above calibrated maximum.
           |         |  Warning flag set.
    20.0   |.........|  ---- Nominal 20 mA (Full Scale) ----
           |         |
           | NORMAL  |  Normal operating range.
           | RANGE   |  Readings are valid. No faults.
           |         |
     4.0   |.........|  ---- Nominal 4 mA (Zero Scale) ----
           |         |
     3.8   +---------+  UNDER-RANGE (Broken Wire Suspect)
           |         |  Current slightly below minimum. Possible
           |         |  degraded wiring or sensor drift. Warning flag.
     1.0   +---------+
           |         |
           |  OPEN   |  OPEN CIRCUIT
           | CIRCUIT |  No current flowing. Broken wire or
           |         |  disconnected sensor. Immediate alarm.
     0.0   +---------+
```

**Threshold Table:**

| Condition     | Current Range   | Voltage Range     | Fault Code | Severity | Action                         |
|---------------|-----------------|-------------------|------------|----------|--------------------------------|
| Open Circuit  | < 1.0 mA        | < 0.250 V         | 0x01       | ALARM    | Set fault flag, hold last value|
| Under-Range   | 1.0 - 3.8 mA   | 0.250 - 0.950 V   | 0x02       | WARNING  | Set warning, use reading       |
| Normal        | 3.8 - 20.5 mA  | 0.950 - 5.125 V   | 0x00       | OK       | Normal operation               |
| Over-Range    | 20.5 - 24.0 mA | 5.125 - 6.000 V   | 0x04       | WARNING  | Set warning, clamp to max      |
| Short Circuit | > 24.0 mA       | > 6.000 V         | 0x08       | ALARM    | Set fault flag, hold last value|

**Conversion from ADC Voltage to Current:**

```
Loop_Current_mA = ADC_Voltage / 0.250 (i.e., V / R_shunt)
```

Where R_shunt = 250 ohm, so:
- 0.250 V --> 1.0 mA
- 0.950 V --> 3.8 mA
- 5.125 V --> 20.5 mA
- 6.000 V --> 24.0 mA

**Note:** Voltages above 5.0V (the ADC Vref) will be clamped by the Schottky diodes on the input conditioning circuit. The short circuit condition (>24 mA) will result in the ADC reading full-scale (0xFFFFFF), which the firmware interprets as a short circuit fault.

**Fault Response Behavior:**

1. **Open Circuit (< 1 mA):** The channel engineering value is frozen at the last known good reading. The fault code 0x01 is written to the channel status register. The red LED (PD14) blinks. A Modbus exception status bit is set.

2. **Under-Range (1-3.8 mA):** The channel continues to report the measured value (even though it is below the nominal 4 mA zero). The warning code 0x02 is written. This condition often indicates a partially broken wire or corroded terminal, and serves as an early maintenance alert.

3. **Normal (3.8-20.5 mA):** Normal operation. Fault code 0x00. Green LED (PD12) steady.

4. **Over-Range (20.5-24 mA):** The channel value is clamped to the maximum of the engineering range. Warning code 0x04 is set. This can indicate a sensor calibration issue or process excursion beyond sensor range.

5. **Short Circuit (> 24 mA):** Similar to open circuit -- the channel value is frozen, fault code 0x08 is set, and an alarm is raised. This typically indicates wiring damage or a failed sensor.

---

## Appendix A: Connector Pinouts

**J1: 24V DC Power Input (2-pin terminal block, 5.08mm)**

| Pin | Signal | Description        |
|-----|--------|--------------------|
| 1   | +24V   | Positive supply    |
| 2   | GND    | Ground / return    |

**J2: Ethernet RJ45 (HR911105A)**

Standard Ethernet RJ45 pinout with integrated magnetics.

**J3: RS-485 Termination Jumper (2-pin header)**

Install jumper to enable 120 ohm termination on the RS-485 bus. Required only at the two physical ends of the bus.

**J4: RS-485 Field Wiring (3-pin terminal block, 5.08mm)**

| Pin | Signal | Description          |
|-----|--------|----------------------|
| 1   | A (+)  | Non-inverting (D+)   |
| 2   | B (-)  | Inverting (D-)       |
| 3   | GND    | Signal ground / ref  |

**J5: SWD Debug Header (2x5 pin, 1.27mm)**

See Section 3.2 for pinout.

**J6: 4-20mA Input Terminal Block (2x8 = 16-pin, 3.81mm)**

| Pin Pair | Channel | Signal+ (from sensor) | Signal- (return/GND) |
|----------|---------|----------------------|-----------------------|
| 1, 2     | CH0     | Pressure +           | Pressure -            |
| 3, 4     | CH1     | pH +                 | pH -                  |
| 5, 6     | CH2     | Turbidity +          | Turbidity -           |
| 7, 8     | CH3     | Chlorine +           | Chlorine -            |
| 9, 10    | CH4     | Flow +               | Flow -                |
| 11, 12   | CH5     | Tank Level +         | Tank Level -          |
| 13, 14   | CH6     | Conductivity +       | Conductivity -        |
| 15, 16   | CH7     | Dissolved O2 +       | Dissolved O2 -        |

---

## Appendix B: Bill of Materials (Key Components)

| Ref  | Manufacturer Part     | Description                    | Qty |
|------|-----------------------|--------------------------------|-----|
| U1   | LM2596S-5.0          | 5V 3A step-down regulator      | 1   |
| U2   | AMS1117-3.3           | 3.3V 1A LDO regulator          | 1   |
| U3   | SP3485EN              | RS-485 transceiver             | 1   |
| U4   | W5500                 | Hardwired TCP/IP Ethernet IC   | 1   |
| U5   | ADS1258IPHPR          | 24-bit 16-ch ADC               | 1   |
| U6   | STM32F407VGT6         | ARM Cortex-M4 MCU              | 1   |
| Q1   | Si2301DS              | P-channel MOSFET               | 1   |
| Y1   | 8 MHz crystal         | HSE for MCU                    | 1   |
| Y2   | 32.768 kHz crystal    | LSE for RTC                    | 1   |
| Y3   | 25 MHz crystal        | W5500 clock                    | 1   |
| J2   | HR911105A             | RJ45 w/ integrated magnetics   | 1   |
| R_SH | 250 ohm 0.1%         | Precision shunt resistor       | 8   |

---

*End of WR-PCB-001 Hardware Design Specification*
