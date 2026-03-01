# STM32F407VGT6 Base Design Guide

## Common MCU Design for SS-PCB-001 and WR-PCB-001

| Field            | Value                          |
|------------------|--------------------------------|
| Document ID      | COM-MCU-001                    |
| Applicable Boards| SS-PCB-001 (Solar SMU), WR-PCB-001 (Water RTU) |
| Revision         | 1.0                            |
| Date             | 2026-02-28                     |
| Status           | Preliminary                    |

---

## 1. MCU Overview

Both the Solar SMU and Water RTU boards use the STM32F407VGT6 microcontroller. This document defines the common base design that is shared between both boards, covering the MCU package, power supply connections, oscillator circuits, boot configuration, reset circuit, debug interface, and unused pin handling.

| Parameter              | Value                          |
|------------------------|--------------------------------|
| Part Number            | STM32F407VGT6                  |
| Manufacturer           | STMicroelectronics             |
| Core                   | ARM Cortex-M4F (with FPU)     |
| Maximum Clock          | 168 MHz                        |
| Flash Memory           | 1 MB (2 banks, 12 sectors)    |
| SRAM                   | 192 KB (128 KB + 64 KB CCM)   |
| FPU                    | Single-precision floating point|
| DSP Instructions       | Yes (SIMD)                     |
| GPIO Count             | 82 (LQFP-100 package)         |
| Supply Voltage         | 1.8V to 3.6V (3.3V nominal)   |
| I/O Voltage            | 3.3V (5V tolerant on most pins)|
| Operating Temperature  | -40 to +85 degC (industrial)   |

---

## 2. Package Information

| Parameter              | Value                          |
|------------------------|--------------------------------|
| Package                | LQFP-100                       |
| Body Size              | 14 mm x 14 mm                 |
| Lead Pitch             | 0.5 mm                         |
| Lead Count             | 100                            |
| Height                 | 1.6 mm (max)                   |
| Exposed Pad            | None (LQFP has no thermal pad) |
| Moisture Sensitivity   | MSL 3 (per J-STD-020)         |
| Land Pattern           | Per IPC-7351B (LQFP-100_14x14mm_P0.5mm) |

### 2.1 PCB Footprint Notes

- Pad dimensions: 0.3 mm x 1.5 mm (nominal, per manufacturer recommendation).
- Pad-to-pad clearance: 0.2 mm minimum between adjacent pads.
- Courtyard: 16 mm x 16 mm (1 mm clearance beyond leads on each side).
- Solder paste: 80-90% of pad area (reduce for fine-pitch reflow yield).
- Surface finish: ENIG recommended for 0.5 mm pitch to ensure pad coplanarity.
- Pin 1 indicator: Silkscreen dot at pin 1 corner, plus a chamfer mark on the copper layer.

---

## 3. Power Supply Requirements

The STM32F407VGT6 in the LQFP-100 package has multiple VDD/VSS pin pairs that must each be individually decoupled. Proper decoupling is critical for stable MCU operation at 168 MHz.

### 3.1 VDD/VSS Pin Pairs

Each VDD pin requires its own 100nF ceramic decoupling capacitor placed as close to the pin as physically possible, with the capacitor ground pad connected via a via directly to the L2 ground plane.

| VDD Pin | VSS Pin(s) | Decoupling Capacitor | Placement Priority |
|---------|------------|---------------------|--------------------|
| Pin 11  | Pin 10     | 100nF ceramic (C11) | 4 (near oscillator)|
| Pin 19  | Pin 18     | 100nF ceramic (C12) | 3                  |
| Pin 28  | Pin 27     | 100nF ceramic (C13) | 3                  |
| Pin 50  | Pin 49     | 100nF ceramic (C14) | 3                  |
| Pin 75  | Pin 74     | 100nF ceramic (C15) | 3                  |
| Pin 100 | Pin 99     | 100nF ceramic (C16) | 3                  |

**All VSS pins must be connected to GND.** Do not leave any VSS pin unconnected.

### 3.2 Bulk Capacitor

| Reference | Value      | Package | Placement                    |
|-----------|-----------|---------|------------------------------|
| C18       | 4.7 uF    | 0805    | Near VDD pin 11 (closest to PH0/PH1 oscillator pins) |

The 4.7 uF bulk capacitor provides low-frequency charge storage for the MCU core, supplementing the 100nF capacitors that handle high-frequency transients. Use an X5R or X7R ceramic capacitor rated for 10V or higher.

### 3.3 VDDA / VSSA (Analog Supply)

The analog supply pin pair requires special attention because it powers the internal ADC, DAC, PLL analog section, and voltage reference. VDDA must always be connected, even if the internal ADC/DAC is not used, because the PLL analog section requires it.

| Pin       | Function       | Connection                    |
|-----------|----------------|-------------------------------|
| Pin 22    | VDDA           | +3.3V (via ferrite bead from digital 3.3V if possible) |
| Pin 23    | VSSA           | GND (shortest path to ground plane) |

**VDDA Decoupling:**

| Reference | Value      | Type        | Package | Purpose               |
|-----------|-----------|-------------|---------|------------------------|
| C19       | 1 uF      | Ceramic X5R | 0402    | VDDA low-freq bypass   |
| C20       | 100 nF    | Ceramic X7R | 0402    | VDDA high-freq bypass  |

- Place both C19 and C20 as close as possible to pin 22 (VDDA), within 3 mm.
- Connect the ground pads of C19 and C20 directly to pin 23 (VSSA) via the shortest possible trace, then to the L2 ground plane via a via at the capacitor ground pad.
- A ferrite bead (600 ohm @ 100 MHz) in series between the main 3.3V rail and VDDA is recommended to isolate digital switching noise from the analog supply.

```
                VDDA DECOUPLING
                ================

      +3.3V Rail
          |
     [FB (600R @ 100MHz)]
          |
    +─────+─────+
    |            |
  [C19]        [C20]
  1 uF         100nF
    |            |
    +─────+──────+
          |
          |  <-- Via to L2 GND here
          |
    Pin 22 (VDDA)
    Pin 23 (VSSA) -- GND
```

### 3.4 Power Connection Summary

```
                POWER CONNECTIONS
                =================

     +3.3V ──┬──┬──┬──┬──┬──┬──────────┬───────┬── VBAT (pin 6)
             |  |  |  |  |  |          |       |
           100nF each (x6)          [FB]     100nF
             |  |  |  |  |  |          |       |
             |  |  |  |  |  |          |      GND
             |  |  |  |  |  |          |
     VDD pins: 11, 19, 28,       VDDA (pin 22)
                50, 75, 100           |
             |  |  |  |  |  |       1uF + 100nF
             |  |  |  |  |  |          |
     GND ──┬─+──+──+──+──+──+──────── VSSA (pin 23)
           |
     VSS pins: 10, 18, 27, 49, 74, 99
           |
     4.7uF bulk (near pin 11)
           |
          GND
```

---

## 4. VBAT Connection

The VBAT pin powers the RTC (Real-Time Clock) domain and the backup registers. In these designs, no battery backup is used, so VBAT is connected directly to the 3.3V rail.

| Pin       | Connection           | Notes                        |
|-----------|---------------------|------------------------------|
| Pin 6     | +3.3V (direct)      | No battery, tied to VDD      |

**Decoupling:**

| Reference | Value      | Purpose                     |
|-----------|-----------|------------------------------|
| C21       | 100 nF    | VBAT decoupling capacitor    |

Place C21 close to pin 6 with a ground via at the capacitor ground pad.

**Note:** If battery backup is needed in the future (e.g., for RTC timekeeping during power loss), replace the direct 3.3V connection with a CR2032 coin cell holder and a Schottky diode OR circuit:

```
    +3.3V ──|>|──┬──> VBAT (pin 6)
                 |
    Battery ─|>|─┘
```

For now, direct 3.3V connection is used on both boards.

---

## 5. BOOT0 Configuration

The BOOT0 pin determines the boot source of the STM32F407 after reset.

| BOOT0 | BOOT1 (PB2) | Boot Source              |
|-------|-------------|--------------------------|
| 0     | X           | Main Flash memory        |
| 1     | 0           | System memory (bootloader)|
| 1     | 1           | Embedded SRAM            |

For normal operation, the MCU boots from internal flash memory. BOOT0 is tied to GND through a 10K resistor.

```
                BOOT CONFIGURATION
                ==================

     GND
       |
     [10K]  R_BOOT0
       |
       +──── BOOT0 (pin 94)
```

**Design Note:** A 10K resistor is used instead of a direct connection to GND so that BOOT0 can be temporarily overridden (e.g., by probing or by adding a jumper header) during development to enter the built-in serial bootloader (DFU mode via USART or USB) for firmware recovery.

On the Water RTU (WR-PCB-001), PB2 (BOOT1) is used as the ADS1258 RESET GPIO output. When BOOT0 = 0, the BOOT1/PB2 state is ignored for boot mode selection, so PB2 is free for use as a general-purpose GPIO. Similarly, on the Solar SMU, PB2 is available for other use.

---

## 6. NRST Filter Circuit

The NRST pin provides an external hardware reset input. It has an internal weak pull-up, but an external filter circuit is required for noise immunity and to provide a controlled reset delay at power-on.

### 6.1 Reset Filter Circuit

```
                RESET CIRCUIT
                =============

     +3.3V
       |
     [R_RST]
      10K
       |
       +──── NRST (pin 14) ──── to J5 SWD header (pin 9)
       |
     [C_RST]
      100nF
       |
      GND
```

| Component | Reference | Value  | Purpose                        |
|-----------|-----------|--------|--------------------------------|
| R_RST     | R_NRST    | 10K    | Pull-up to 3.3V               |
| C_RST     | C_NRST    | 100nF  | Noise filter capacitor         |

### 6.2 Reset Timing

The RC time constant of the NRST filter is:

```
    tau = R_RST * C_RST = 10K * 100nF = 1 ms
```

After VDD reaches a stable 3.3V, NRST charges through R_RST to the logic HIGH threshold (approximately 0.7 * VDD = 2.31V) in about 1.2 ms (1.2 * tau). This ensures the MCU does not exit reset until power supply rails are fully stable.

### 6.3 Reset Sources

| Source              | Mechanism                       | Notes                    |
|---------------------|---------------------------------|--------------------------|
| Power-on reset (POR)| Internal, VDD rising threshold  | ~1.8V threshold          |
| NRST pin            | External pull-low               | Via SWD probe or button  |
| Brownout reset (BOR)| Internal, configurable          | Option bytes             |
| Watchdog (IWDG/WWDG)| Internal, firmware-triggered   | Timeout reset            |
| Software reset      | NVIC_SystemReset()              | Firmware-initiated       |

### 6.4 Brownout Reset (BOR) Configuration

| BOR Level   | Rising Threshold | Falling Threshold | Recommended Use        |
|-------------|-----------------|-------------------|------------------------|
| BOR OFF     | -- (disabled)   | --                | Not recommended        |
| BOR Level 1 | 2.1V            | 2.0V              | Low-power applications |
| BOR Level 2 | 2.4V            | 2.3V              | General purpose        |
| BOR Level 3 | 2.7V            | 2.6V              | **Recommended (168 MHz)** |

**Recommendation:** Configure BOR Level 3 (2.7V) in firmware option bytes to ensure the MCU resets before VDD drops below the minimum reliable operating voltage for 168 MHz operation.

---

## 7. HSE Crystal Design (8 MHz)

### 7.1 Crystal Specification

| Parameter          | Value                          |
|--------------------|--------------------------------|
| Reference          | Y1                             |
| Frequency          | 8.000 MHz                      |
| Tolerance          | +/- 20 ppm                     |
| Stability          | +/- 30 ppm over temperature    |
| Load Capacitance   | 20 pF                          |
| ESR                | < 40 ohm (max)                 |
| Drive Level        | < 200 uW                       |
| Package            | HC49/SMD (4-pin with GND)      |
| Temperature Range  | -40 to +85 degC                |
| Manufacturer P/N   | Abracon ABM3B-8.000MHZ-B2-T (or equiv) |

### 7.2 Load Capacitor Calculation

The load capacitors (C_L1 and C_L2) must be selected to present the specified load capacitance to the crystal:

```
    C_L = (C_L1 * C_L2) / (C_L1 + C_L2) + C_stray

    Where:
    - C_L = crystal specified load capacitance = 20 pF
    - C_stray = PCB stray capacitance (typically 3-5 pF)
    - Assuming C_L1 = C_L2 = C_load_cap:
      C_L = C_load_cap / 2 + C_stray
      C_load_cap = 2 * (C_L - C_stray)
      C_load_cap = 2 * (20 - 5) = 30 pF

    Selected: 20 pF (standard value, accounting for ~2 pF pin
    capacitance and ~3 pF trace capacitance per pin)
```

### 7.3 Load Capacitor Components

| Reference | Value  | Type          | Package |
|-----------|--------|---------------|---------|
| C7        | 20 pF  | Ceramic NP0/C0G | 0402  |
| C8        | 20 pF  | Ceramic NP0/C0G | 0402  |

Use NP0/C0G dielectric ONLY for crystal load capacitors. Other dielectrics (X5R, X7R) have voltage and temperature-dependent capacitance that will shift the oscillator frequency.

### 7.4 HSE Connection Diagram

```
                8 MHz HSE CRYSTAL
                ==================

     PH0 (OSC_IN, pin 12) ──┬──[Y1: 8 MHz]──┬── PH1 (OSC_OUT, pin 13)
                             |                |
                           [C7]             [C8]
                           20pF             20pF
                             |                |
                            GND              GND
                          (case GND
                           if 4-pin)
```

### 7.5 HSE Layout Guidelines

1. **Placement:** Place Y1 and C7/C8 as close as possible to pins 12 and 13 (PH0/PH1). Maximum distance: 5 mm from MCU pads to crystal pads.
2. **Trace length:** Keep OSC_IN and OSC_OUT traces as short as possible (< 5 mm each).
3. **Trace width:** 0.15-0.2 mm (narrow, to minimize capacitive loading).
4. **No vias:** Route crystal traces on L1 only, directly from MCU pins to crystal pads.
5. **Guard ring:** Surround the crystal and load capacitors with a ground copper pour on L1, stitched to L2 ground plane with vias every 2-3 mm.
6. **No other traces:** Do not route any other signals within the crystal guard ring area.
7. **Ground plane:** Ensure continuous L2 ground plane under the entire crystal area.

```
    +──────────────────────────────────────+
    |  GND Guard Ring (L1, via-stitched)   |
    |  +────────────────────────────────+  |
    |  |                                |  |
    |  |  [C7]──── Y1 (8MHz) ────[C8]  |  |
    |  |   |                       |    |  |
    |  |  GND    to PH0   PH1    GND   |  |
    |  |                                |  |
    |  +────────────────────────────────+  |
    |  v  v  v  v  v  v  v  v  v  v  v    |  (v = via to L2 GND)
    +──────────────────────────────────────+
```

---

## 8. LSE Crystal Design (32.768 kHz)

### 8.1 Crystal Specification

| Parameter          | Value                          |
|--------------------|--------------------------------|
| Reference          | Y2                             |
| Frequency          | 32.768 kHz                     |
| Tolerance          | +/- 20 ppm                     |
| Load Capacitance   | 6.8 pF                         |
| ESR                | < 50K ohm (typical for 32 kHz) |
| Drive Level        | < 1 uW                         |
| Package            | 2.0 x 1.2 mm SMD               |
| Temperature Range  | -40 to +85 degC                |
| Manufacturer P/N   | Epson FC-12M 32.768K (or equiv)|

### 8.2 Load Capacitors

| Reference | Value    | Type          | Package |
|-----------|---------|---------------|---------|
| C9        | 6.8 pF  | Ceramic NP0/C0G | 0402  |
| C10       | 6.8 pF  | Ceramic NP0/C0G | 0402  |

### 8.3 LSE Connection Diagram

```
                32.768 kHz LSE CRYSTAL
                =======================

     PC14 (OSC32_IN, pin 8) ──┬──[Y2: 32.768 kHz]──┬── PC15 (OSC32_OUT, pin 9)
                              |                      |
                            [C9]                   [C10]
                            6.8pF                  6.8pF
                              |                      |
                             GND                    GND
```

### 8.4 LSE Layout Guidelines -- Guard Ring

The 32.768 kHz crystal is extremely sensitive to parasitic coupling and noise because of its very high impedance (ESR up to 50K ohm). A guard ring is mandatory.

1. **Placement:** Place Y2 and C9/C10 within 3 mm of pins 8 and 9 (PC14/PC15).
2. **Guard ring:** Create a copper guard ring on L1 that completely surrounds the crystal, load capacitors, and the traces to the MCU pins. Connect this guard ring to GND via vias spaced every 2 mm.
3. **No other traces:** Absolutely no other signals should be routed within or near the guard ring. This is the most critical layout rule for the LSE.
4. **Ground plane:** Continuous L2 ground plane under the entire area.
5. **No copper pour over crystal:** On L1, the area immediately under and around the crystal should be the guard ring only, with no other copper fills that could capacitively couple noise.
6. **Feedback resistor:** The STM32F407 has an internal feedback resistor for the LSE oscillator. No external resistor is needed.
7. **Series resistor (optional):** A 10M ohm resistor in series with OSC32_OUT can reduce drive level to protect fragile crystals. This is typically not needed for modern SMD crystals.

```
    +──────────────────────────────+
    |  GND Guard Ring              |
    |  +────────────────────────+  |
    |  |                        |  |
    |  |  [C9]── Y2 ────[C10]  |  |
    |  |   |    32kHz      |    |  |
    |  |  GND  PC14 PC15  GND  |  |
    |  |                        |  |
    |  +────────────────────────+  |
    |  v   v   v   v   v   v      |  (v = via to L2 GND)
    +──────────────────────────────+
```

---

## 9. Clock Tree Configuration

The STM32F407 clock tree derives all internal clocks from the 8 MHz HSE crystal through the main PLL.

### 9.1 PLL Configuration (168 MHz SYSCLK)

```
                CLOCK TREE
                ==========

     [8 MHz HSE] ──> PLL ──> SYSCLK = 168 MHz
                       |
                       +── PLL_M = 8   (HSE / 8 = 1 MHz input to PLL)
                       +── PLL_N = 336 (1 MHz x 336 = 336 MHz VCO)
                       +── PLL_P = 2   (336 / 2 = 168 MHz SYSCLK)

     SYSCLK (168 MHz)
         |
         +── AHB Prescaler = 1 ──> HCLK = 168 MHz
         |       |
         |       +── APB1 Prescaler = 4 ──> PCLK1 = 42 MHz
         |       |       |
         |       |       +── APB1 Timer Clk = 84 MHz (x2 auto-multiplier)
         |       |
         |       +── APB2 Prescaler = 2 ──> PCLK2 = 84 MHz
         |               |
         |               +── APB2 Timer Clk = 168 MHz (x2 auto-multiplier)
         |
         +── SysTick = 168 MHz (or /8 = 21 MHz)

     [32.768 kHz LSE] ──> RTC Clock
```

### 9.2 Bus Clock Summary

| Clock     | Source    | Prescaler | Frequency | Users                      |
|-----------|-----------|-----------|-----------|----------------------------|
| SYSCLK    | PLL_P    | --        | 168 MHz   | CPU core, DMA              |
| HCLK      | SYSCLK   | /1        | 168 MHz   | AHB bus, core, memory      |
| PCLK1     | HCLK     | /4        | 42 MHz    | APB1: SPI2/3, USART2, I2C1, TIM2-7 |
| PCLK2     | HCLK     | /2        | 84 MHz    | APB2: SPI1, USART1, TIM1/8-11 |
| APB1 Timer| PCLK1    | x2 (auto) | 84 MHz   | Timer 2-7, 12-14 clocks   |
| APB2 Timer| PCLK2    | x2 (auto) | 168 MHz  | Timer 1, 8-11 clocks       |
| RTC       | LSE      | --        | 32.768 kHz| Real-time clock             |

### 9.3 SPI Clock Derivation

| SPI Bus | APB Bus | APB Clock | Prescaler | SPI Clock | Usage                |
|---------|---------|-----------|-----------|-----------|----------------------|
| SPI1    | APB2    | 84 MHz    | /8        | 10.5 MHz  | ADC (ADS1258/1263)   |
| SPI1    | APB2    | 84 MHz    | /16       | 5.25 MHz  | ADC (conservative)   |
| SPI2    | APB1    | 42 MHz    | /2        | 21 MHz    | W5500 Ethernet (RTU) |
| SPI3    | APB1    | 42 MHz    | /2        | 21 MHz    | MicroSD card (data, SMU) |
| SPI3    | APB1    | 42 MHz    | /128      | 328 kHz   | MicroSD card (init, SMU) |

### 9.4 USART Baud Rate

| USART  | APB Bus | APB Clock | Baud Rates Available              |
|--------|---------|-----------|-----------------------------------|
| USART2 | APB1    | 42 MHz    | 9600, 19200, 38400, 57600, 115200 |

Default Modbus RTU baud rate: 9600 bps (8N1 or 8N2).

### 9.5 Clock Security System (CSS)

The STM32F407 includes a Clock Security System that monitors the HSE oscillator. If the HSE fails, the CSS automatically switches SYSCLK to the internal HSI (16 MHz) and generates an NMI interrupt.

**Recommendation:** Enable CSS in firmware to provide graceful degradation if the crystal oscillator fails. The NMI handler should:
1. Switch the PLL source to HSI.
2. Reconfigure PLL for a reduced SYSCLK (e.g., 128 MHz from 16 MHz HSI).
3. Set a fault flag visible on Modbus.
4. Illuminate the red LED (PD14).

---

## 10. SWD Debug Interface

### 10.1 Pin Connections

| STM32 Pin | AF       | Signal | J5 Pin | Direction     |
|-----------|----------|--------|--------|---------------|
| PA13      | AF0      | SWDIO  | 2      | Bidirectional |
| PA14      | AF0      | SWCLK  | 4      | Input (to MCU)|
| PB3       | AF0      | SWO    | 6      | Output (from MCU) |
| NRST      | --       | NRST   | 9      | Bidirectional |

### 10.2 SWD Header Pinout (J5)

```
     J5: SWD DEBUG HEADER (2x5, 1.27mm pitch)
     ==========================================
     Connector: Samtec FTSH-105-01-L-DV-K

          +-------+-------+
      VCC | 1     | 2     | SWDIO (PA13)
          +-------+-------+
      GND | 3     | 4     | SWCLK (PA14)
          +-------+-------+
      GND | 5     | 6     | SWO (PB3) -- optional
          +-------+-------+
      N/C | 7     | 8     | N/C
          +-------+-------+
     NRST | 9     | 10    | GND
          +-------+-------+
```

### 10.3 SWD Signal Considerations

- **SWDIO (PA13):** Has an internal pull-up. No external pull-up/pull-down required. Route as a controlled-impedance trace (50 ohm).
- **SWCLK (PA14):** Has an internal pull-down. No external pull-up/pull-down required. Route alongside SWDIO.
- **SWO (PB3):** Optional trace output pin for printf-style debugging via ITM (Instrumentation Trace Macrocell). On the Solar SMU, PB3 is shared with SPI3_SCK (SD card clock), so SWO is only available when the SD card is not actively in use. On the Water RTU, PB3 is unused by other peripherals and SWO is always available.
- **NRST:** Connected through the external 10K pull-up and 100nF filter capacitor (see Section 6). The debug probe drives NRST low through an open-drain output.

### 10.4 SWD Trace Routing

- Keep SWDIO and SWCLK traces under 50 mm total length.
- Route on L1 over continuous L2 ground plane.
- No series resistors on SWD signals (the debug probe handles termination).
- Place the J5 SWD header near the board edge for easy access during development.

### 10.5 Debug Probe Compatibility

| Debug Probe              | Compatible | Notes                     |
|--------------------------|------------|---------------------------|
| ST-Link V2               | Yes        | Standard SWD, 1.27mm adapter needed |
| ST-Link V3               | Yes        | Native 1.27mm STDC14 or adapter |
| J-Link (Segger)          | Yes        | Use SWD 10-pin adapter cable |
| CMSIS-DAP (DAPLink)      | Yes        | Standard ARM SWD pinout   |
| Black Magic Probe        | Yes        | Via SWD connector         |

---

## 11. Unused Pin Handling

All unused GPIO pins on the STM32F407 must be properly configured to prevent floating inputs, which can cause increased power consumption and potential EMI issues.

### 11.1 Recommended Configuration

| Configuration       | Setting                        |
|---------------------|--------------------------------|
| Mode                | Input (GPIO_MODE_INPUT)        |
| Pull                | Pull-down (GPIO_PULLDOWN)      |
| Speed               | Low                            |

### 11.2 Rationale

This configuration ensures:
- No floating inputs (which would oscillate between high and low, consuming switching power).
- Deterministic pin state (LOW).
- No contention with external circuits.
- Minimal EMI from toggling outputs.

### 11.3 Unused Pins (Common to Both Boards)

| Port  | Unused Pins                              |
|-------|------------------------------------------|
| PA    | PA0, PA1, PA9, PA10, PA11, PA12          |
| PD    | PD0, PD1, PD3-PD11                       |
| PE    | PE0-PE15 (all, if present on LQFP-100)   |

**Note:** Additional pins are unused on each specific board. The exact list differs between the Solar SMU (which uses PC0-PC9 for MUX control, PB3-PB7 for SPI3/I2C1) and the Water RTU (which uses PB12-PB15 for SPI2, PC6-PC7 for W5500 control). Each board's firmware initializes its own unused pins using the same principle: input mode with internal pull-down.

### 11.4 External Considerations

- Do NOT connect unused pins to VDD or GND on the PCB. Use internal pull-downs in firmware instead. This preserves flexibility for future board revisions.
- If EMC testing reveals issues with specific unused pins, consider adding a 10K external pull-down resistor on the PCB as a secondary measure.

---

## 12. Flash Programming and JTAG Considerations

### 12.1 Flash Programming Methods

| Method                  | Interface  | Tool                     | Notes                    |
|-------------------------|------------|--------------------------|--------------------------|
| SWD (primary)           | J5 header  | ST-Link, J-Link, DAPLink | Development and production |
| UART Bootloader         | USART2     | stm32flash, CubeProg     | Set BOOT0=1, uses PA2/PA3 |
| USB DFU                 | USB pins   | dfu-util, CubeProg       | Set BOOT0=1, if USB connected |
| SWD Mass Production     | J5 / pogo  | ST-Link + CubeProg batch | Production line flashing |

### 12.2 JTAG vs. SWD

The STM32F407 supports both JTAG (5-pin: TMS, TCK, TDI, TDO, TRST) and SWD (2-pin: SWDIO, SWCLK). These designs use SWD exclusively because:

1. SWD requires only 2 signal pins (PA13, PA14) versus 5 for full JTAG.
2. SWD provides full debug capability (breakpoints, stepping, register access, flash programming).
3. SWD frees up PA15, PB3, and PB4 for other uses (SD card SPI on Solar SMU).

**Important:** After reset, the STM32F407 defaults to JTAG mode (PA15, PB3, PB4 are allocated to JTAG). The firmware must release these pins for GPIO/alternate function use:

```
// In Embassy-STM32 (Rust):
// The HAL automatically configures SWD-only mode when PA15/PB3/PB4 are
// assigned to their alternate functions (SPI3, GPIO).

// In STM32 HAL (C):
// __HAL_AFIO_REMAP_SWJ_NOJTAG();  // Release PA15, PB3, PB4 from JTAG
```

### 12.3 Read-Out Protection (RDP)

For production firmware, enable Read-Out Protection Level 1 to prevent unauthorized flash readback via the debug interface.

| RDP Level | Flash Read | Flash Write                    | Debug      | Recovery              |
|-----------|-----------|--------------------------------|------------|------------------------|
| Level 0   | Allowed   | Allowed                        | Full access| N/A                   |
| Level 1   | Blocked   | Allowed (after mass erase)     | Limited    | Mass erase to Level 0 |
| Level 2   | Blocked   | Blocked                        | Disabled   | PERMANENT -- no recovery |

**Warning:** Never set RDP Level 2 during development. It permanently disables all debug access and cannot be reversed.

---

## 13. Decoupling Capacitor Placement Order

When laying out the PCB, place decoupling capacitors in the following priority order to ensure the most noise-sensitive pins are addressed first:

| Priority | Component | Location / Pin       | Value    | Notes                     |
|----------|-----------|---------------------|----------|---------------------------|
| 1        | C19       | VDDA (pin 22)       | 1 uF     | Analog supply, most sensitive |
| 2        | C20       | VDDA (pin 22)       | 100 nF   | Analog HF bypass          |
| 3        | C11       | VDD (pin 11)        | 100 nF   | Nearest to HSE oscillator |
| 4        | C12       | VDD (pin 19)        | 100 nF   |                           |
| 5        | C13       | VDD (pin 28)        | 100 nF   |                           |
| 6        | C14       | VDD (pin 50)        | 100 nF   |                           |
| 7        | C15       | VDD (pin 75)        | 100 nF   |                           |
| 8        | C16       | VDD (pin 100)       | 100 nF   |                           |
| 9        | C18       | VDD bulk (near pin 11)| 4.7 uF | Low-frequency bulk        |
| 10       | C21       | VBAT (pin 6)        | 100 nF   | Backup domain             |
| 11       | C7, C8    | HSE load caps       | 20 pF    | NP0/C0G only             |
| 12       | C9, C10   | LSE load caps       | 6.8 pF   | NP0/C0G only             |

**General Rule:** Each capacitor's ground pad must connect to the L2 ground plane through a via placed directly at (or within 0.5 mm of) the capacitor ground pad. Long ground traces to distant vias negate the effectiveness of decoupling.

All decoupling capacitors should be:
- Ceramic, X7R or C0G dielectric (100nF: X7R is acceptable; small pF values: C0G required)
- 0402 or 0603 package for minimum trace length
- Placed on the same layer as the IC, within 3 mm of the power pin
- Connected to power and ground planes via short, wide traces or direct via

---

## 14. MCU Pin Assignment Summary (Common to Both Boards)

The following pins have identical assignments on both the Solar SMU and Water RTU:

| STM32 Pin | Port.Pin | Common Function                | Both Boards |
|-----------|----------|-------------------------------|-------------|
| Pin 12    | PH0      | HSE OSC_IN (8 MHz crystal)    | Yes         |
| Pin 13    | PH1      | HSE OSC_OUT                   | Yes         |
| Pin 8     | PC14     | LSE OSC32_IN (32.768 kHz)     | Yes         |
| Pin 9     | PC15     | LSE OSC32_OUT                 | Yes         |
| Pin 94    | BOOT0    | 10K to GND (flash boot)       | Yes         |
| Pin 14    | NRST     | 100nF + 10K (reset filter)    | Yes         |
| Pin 6     | VBAT     | +3.3V (no battery)            | Yes         |
| Pin 22    | VDDA     | +3.3V (1uF + 100nF)          | Yes         |
| --        | PA2      | USART2_TX (RS-485 via SP3485) | Yes         |
| --        | PA3      | USART2_RX (RS-485 via SP3485) | Yes         |
| --        | PA4      | SPI1_CS (ADC chip select)     | Yes         |
| --        | PA5      | SPI1_SCK (ADC clock)          | Yes         |
| --        | PA6      | SPI1_MISO (ADC data out)      | Yes         |
| --        | PA7      | SPI1_MOSI (ADC data in)       | Yes         |
| --        | PA8      | RS-485 DE/RE direction ctrl   | Yes         |
| --        | PA13     | SWDIO (debug)                 | Yes         |
| --        | PA14     | SWCLK (debug)                 | Yes         |
| --        | PB0      | ADC DRDY (data ready input)   | Yes         |
| --        | PD12     | LED Green (heartbeat)         | Yes         |
| --        | PD13     | LED Orange (activity)         | Yes         |
| --        | PD14     | LED Red (fault)               | Yes         |
| --        | PD15     | LED Blue (board-specific)     | Yes         |

---

*End of STM32F407VGT6 Base Design Guide*
