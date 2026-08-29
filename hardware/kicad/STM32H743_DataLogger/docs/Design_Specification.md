# STM32H743 PLC-Grade Data Logger - Design Specification

## Project: SCADA Industrial Data Logger PCB
**Revision:** 1.2
**Date:** 2026-03-15
**PCB Tool:** KiCad 8.0

---

## 1. System Overview

| Parameter | Specification |
|-----------|--------------|
| MCU | STM32H743VIT6 (ARM Cortex-M7, 480MHz) |
| Flash | 2MB internal + 8MB external (W25Q64JV) |
| RAM | 1MB internal SRAM |
| ADC | 16-bit, 4 channels (4-20mA / 0-10V) |
| Digital Inputs | 8 channels, 24VDC, optocoupler isolated |
| Digital Outputs | 4 channels, relay (5A/250VAC) |
| Power Input | 24VDC (9-36V range) |
| Operating Temp | -40°C to +85°C (Industrial) |
| Watchdog | External TPS3823-33 hardware WDT |

## 2. Communication Interfaces

| Interface | IC/Module | Protocol | Speed | Temp Rating | Connector |
|-----------|-----------|----------|-------|-------------|-----------|
| **Ethernet** | W5500 (WIZnet) | TCP/IP HW Stack | 80Mbps SPI | -40 to +85°C | RJ45 |
| **LoRa** | SX1276IMLTRT (Semtech) | LoRaWAN | 300bps-50kbps | **-40 to +85°C** | SMA |
| **WiFi** | ATWINC1500-MR210PB | 802.11 b/g/n | 150Mbps | **-40 to +85°C** | u.FL |
| **BLE** | RN4870-I/RM128 | BLE 5.0 | 2Mbps | **-40 to +85°C** | Built-in |
| **GSM/LTE** | SIM7600G-H | LTE Cat-1 | 10Mbps DL | -35 to +75°C | SMA x2 |
| **RS485** | ISO3082DWR | Modbus RTU | 20Mbps | -40 to +125°C | 3-pin term |
| **CAN FD** | ISO1042BQDWRQ1 | CAN FD 2.0 | 5Mbps | -40 to +125°C | 3-pin term |
| **USB** | Native USB OTG | USB 2.0 FS | 12Mbps | — | USB-C |

### Architecture: Single MCU + SPI/UART Peripherals (No Dual MCU)

```
                        STM32H743VIT6
                     ┌──────────────────┐
          SPI1 ──────│ PA4-PA6,PB5      │────── SX1276 LoRa IC
          SPI2 ──────│ PB12-PB15       │────── ATWINC1500 WiFi
          SPI3 ──────│ PC10-PC12,PA15  │────── W25Q64JV Flash
          SPI4 ──────│ PE2-PE6         │────── W5500 Ethernet
        UART4 ──────│ PA0,PC11         │────── SIM7600G-H GSM
        UART7 ──────│ PE7,PE8          │────── RN4870 BLE
       USART2 ──────│ PD4-PD6         │────── ISO3082 RS485
       FDCAN1 ──────│ PD0,PD1         │────── ISO1042 CAN FD
       SDMMC1 ──────│ PC8-12,PD2      │────── microSD Card
         I2C1 ──────│ PB6,PB7         │────── AT24C256 EEPROM
         ADC1 ──────│ PA0-PA3         │────── 4x Analog Inputs
              ──────│ GPIO            │────── 8x DI + 4x DO
                     └──────────────────┘
```

## 3. Power Architecture

```
24VDC Input (9-36V)
  │
  ├─ PTC Fuse (1.5A) ─► TVS (SMDJ36A) ─► PMOS Reverse Polarity ─► EMI Filter
  │
  ├─ TPS54560 Buck ──────► +5V Rail (3A max)
  │     │
  │     ├─ TLV1117LV33 LDO ──► +3V3 Digital Rail (1A)
  │     │     └── MCU, W5500, ATWINC1500, RN4870, SX1276, Flash, EEPROM, SD
  │     │
  │     ├─ TPS7A4533 LDO ────► +3V3A Analog Rail (150mA, ultra-low noise)
  │     │     └── ADC VREF, op-amp buffers, analog front-end
  │     │
  │     └─ TPS73641 LDO ─────► +4V1 GSM Module Rail (400mA + 100uF bulk)
  │           └── SIM7600G-H (2A transient capable)
  │
  └─ Power LED (Green)
```

## 4. PCB Specifications

| Parameter | Value |
|-----------|-------|
| Dimensions | 160mm × 100mm (Eurocard) |
| Layers | 4 (Signal/GND/Power/Signal) |
| Thickness | 1.6mm |
| Material | FR-4 TG170 |
| Copper Weight | 1oz (35μm) all layers |
| Surface Finish | ENIG |
| Min Track Width | 0.15mm (6mil) |
| Min Clearance | 0.15mm (6mil) |
| Min Via | 0.5mm pad / 0.3mm drill |
| Solder Mask | Green, both sides |
| Silkscreen | White, both sides |
| Mounting | 6x M3 holes (DIN rail compatible) |
| Conformal Coating | HumiSeal 1B73 (MIL-I-46058C) |

## 5. Layer Stackup

```
Layer 1 (F.Cu)  : Signal + Components        35μm
Prepreg         : FR-4                        0.2mm (εr=4.5)
Layer 2 (In1.Cu): Solid GND Plane            35μm
Core            : FR-4                        0.8mm (εr=4.5)
Layer 3 (In2.Cu): Signal + +3V3 power fill   35μm
Prepreg         : FR-4                        0.2mm (εr=4.5)
Layer 4 (B.Cu)  : Signal + Components        35μm
──────────────────────────────────────────────
Total                                         ~1.6mm
```

Three routing layers (F.Cu, In2.Cu, B.Cu); In1.Cu stays an unbroken ground plane so
the outer layers keep a solid reference. The third routing layer is needed for the
0.5mm-pitch LQFP-100 escape — with only the two outer layers the autorouter could not
complete the fanout. Design rules: 0.15mm track, 0.125mm clearance, 0.45mm/0.25mm
vias.

## 6. STM32H743 Pin Assignment Summary (Rev 1.1)

| Peripheral | Pins | Function |
|-----------|------|----------|
| SPI1 (LoRa) | PA4(CS), PA5(SCK), PA6(MISO), PB5(MOSI) | SX1276 LoRa IC |
| SPI2 (WiFi) | PB12(CS), PB13(SCK), PB14(MISO), PB15(MOSI) | ATWINC1500 |
| SPI3 (Flash) | PA15(CS), PC10(SCK), PC11(MISO), PC12(MOSI) | W25Q64JV |
| SPI4 (Ethernet) | PE4(CS), PE2(SCK), PE5(MISO), PE6(MOSI) | W5500 |
| UART4 | PA0(TX), PC11(RX) | SIM7600G-H GSM |
| UART7 | PE8(TX), PE7(RX) | RN4870 BLE |
| FDCAN1 | PD0(RX), PD1(TX) | CAN FD Bus |
| USART2 | PD5(TX), PD6(RX), PD4(DE) | RS485 |
| SDMMC1 | PC8-12, PD2 | microSD Card |
| I2C1 | PB6(SCL), PB7(SDA) | EEPROM |
| ADC1 | PA0-PA3 | Analog Inputs (4ch) |
| USB OTG FS | PA11(DM), PA12(DP) | USB-C |
| SWD | PA13, PA14 | Debug |
| HSE | PH0, PH1 | 25MHz Crystal |
| LSE | PC14, PC15 | 32.768kHz RTC |
| WDT Kick | PB8 | TPS3823 WDI input |
| W5500 Control | PE3(RST), PB0(INT) | Ethernet control |
| WiFi Control | PB1(EN), PB2(RST), PD10(IRQ) | ATWINC1500 control |
| BLE Control | PD12(RST), PD13(STATUS) | RN4870 control |
| GSM Control | PC3(PWR_KEY), PD11(STATUS) | SIM7600 control |
| LoRa Control | PC13(DIO0), PC2(RST) | SX1276 control |

**Freed RMII pins** (available for future expansion): PA1, PA2, PA7, PB11, PC1, PC4, PC5

## 7. Industrial Compliance & Reliability

### 7.1 Temperature Ratings

| Component | Rating | Grade |
|-----------|--------|-------|
| STM32H743VIT6 | -40 to +85°C | Industrial |
| SX1276IMLTRT | -40 to +85°C | Industrial |
| ATWINC1500-MR210PB | -40 to +85°C | Industrial |
| RN4870-I/RM128 | -40 to +85°C | Industrial |
| W5500 | -40 to +85°C | Industrial |
| ISO3082DWR | -40 to +125°C | Automotive |
| ISO1042BQDWRQ1 | -40 to +125°C | Automotive |
| TPS3823-33 | -40 to +125°C | Industrial |
| SIM7600G-H | -35 to +75°C | Extended Commercial |
| Passive components | -55 to +125°C | AEC-Q200 rated |

### 7.2 Hardware Watchdog (TPS3823-33)
- External supervisory IC monitors 3.3V supply and MCU operation
- 1.6 second timeout — MCU must kick WDI pin periodically
- Active-low RESET output connected to STM32 NRST
- Manual reset input available
- Ensures system recovery from firmware hangs

### 7.3 Conformal Coating
- **Material:** HumiSeal 1B73 (Acrylic)
- **Standard:** MIL-I-46058C / IPC-CC-830B
- **Coverage:** Full board both sides (excluding connectors, test points)
- **Protection:** Moisture, dust, chemical splash, salt spray
- **Temperature:** -65°C to +125°C operating range

### 7.4 EMC Compliance Targets

| Standard | Test | Requirement |
|----------|------|-------------|
| IEC 61000-4-2 | ESD | ±8kV contact, ±15kV air |
| IEC 61000-4-4 | EFT/Burst | ±2kV on power, ±1kV on signal |
| IEC 61000-4-5 | Surge | ±2kV line-to-earth, ±1kV line-to-line |
| IEC 61000-4-6 | Conducted immunity | 10V, 150kHz-80MHz |
| IEC 61000-4-8 | Magnetic field | 30A/m |
| EN 55032 | Emissions | Class B |
| EN 55035 | Immunity | Industrial |

**EMC Design Measures:**
- TVS diodes on all external interfaces (SMDJ36A, PESD2CAN, PESD5V0S)
- PTC fuse + reverse polarity protection on power input
- Ferrite beads on Ethernet pairs (BLM18PG121SN1D)
- 4-layer PCB with solid GND plane (Layer 2)
- EMI filter inductor on power input
- Optocoupler isolation on digital inputs (3.75kVrms)
- Galvanic isolation on RS485/CAN (5kVrms)
- Common mode choke consideration for RS485/CAN bus lines

### 7.5 Isolation Summary

| Interface | Isolation Method | Rating | Field-side supply |
|-----------|-----------------|--------|-------------------|
| Digital Inputs | TLP293 Optocoupler | 3.75kVrms | none needed (loop powered from the field, return on DI_COM) |
| RS485 | ISO3082DWR | 5kVrms | U42 NXJ1S0505MC-R13, 5.2kVDC |
| CAN FD | ISO1042BQDWRQ1 | 5kVrms | U43 NXJ1S0505MC-R13, 5.2kVDC |
| Ethernet | RJ45 magnetics (J0011D21BNL) | 1.5kVrms | n/a |

The isolation is carried through into the copper, not just the parts: the GND and
+3V3 pours are notched out of the top-left corner, each field bus has its own
floating pour, milled slots run under the four packages that straddle the barrier
at y=22mm, and a four-layer keepout stops copper bridging the optocoupler row.
The digital-input field return is DI_COM, a floating node - it is deliberately not
tied to logic ground.

### 7.6 Board Floorplan

Field wiring is confined to the two long edges, logic sits in the middle band.

| Edge | Contents (left to right) |
|------|--------------------------|
| Top | RS485 (J50), CAN (J51), then digital inputs J30-J37 |
| Bottom | 24VDC in (J1), analog inputs J20-J23, relay outputs J40-J43 |
| Right | RJ45 (J60), USB-C (J11), LoRa/WiFi/LTE antennas (J73/J74/J70) |
| Left | microSD (J80) |

Relay contacts (mains-capable) are kept on the opposite edge from the low-voltage
analog inputs. Each circuit block is placed against its own connector: analog
conditioning over its terminals, the relay bank over J40-J43, the W5500 beside the
RJ45, the optocoupler row directly under the digital-input terminals.

Mounting holes are at the four corners plus (5, 32) and (155, 50); the two extra
holes were moved off the long-edge midpoints because the terminal rows now occupy
them, and H1 in particular had to leave the isolated corner so a chassis screw
cannot bridge the barrier.

### 7.7 Known Gap: Schematic Connectivity

The nine `.kicad_sch` sheets contain **symbols and text annotations only**. There are
no wires, net labels, junctions or no-connect flags anywhere in the project — checked
across every sheet. All connectivity lives in the pad `(net ...)` assignments inside
`STM32H743_DataLogger_mfg.kicad_pcb`.

Consequences to be aware of before opening the project in KiCad:

- The schematic cannot be netlisted, and ERC will report every pin unconnected.
- There is no schematic-to-board consistency check available.
- **Running *Tools > Update PCB from Schematic* would erase the board's
  connectivity.** Do not run it until the sheets are wired.

`docs/Netlist_from_PCB.md` lists all 157 nets with their pads, and
`docs/netlist_from_pcb.net` is the same data as a KiCad netlist. Wiring the sheets
against that reference is the outstanding task; the board itself is self-consistent
(every net reaches at least two pads).

### 7.8 MCU pin assignment (resolved)

An earlier revision of this board had U10's pad-to-net map filled in sequentially
rather than mapped to the LQFP-100 pinout — supply pins at evenly spaced pads,
peripherals in contiguous blocks, and **+24V on pin 1**, which would have destroyed the
MCU on power-up.

It has been rebuilt from the KiCad `MCU_ST_STM32H7` symbol for STM32H743VITx, which is
generated from the ST datasheet. The result is checked against the pins whose position
is fixed by the package, all of which now land correctly:

| Function | Pin(s) |
|----------|--------|
| NRST | 14 |
| BOOT0 | 94 |
| VBAT | 6 |
| VDDA / VREF+ / VSSA | 21 / 20 / 19 |
| VCAP | 48, 73 |
| VDD | 11, 27, 50, 75, 100 |
| VSS | 10, 26, 49, 74, 99 |

90 of 100 pins are assigned; 10 are left spare (PC1, PC2_C, PC3_C, PA7, PC4, PC5, PD8,
PC6, PC7, PA8).

**Four changes were forced during the conversion** — the old port-level notes could not
be transcribed as written, and these need firmware-side sign-off:

| Signal | Was | Now | Why |
|--------|-----|-----|-----|
| UART4_TX | PA0 | **PB9** | PA0 is an ADC1 channel. The four analog inputs must stay on PA0-PA3, and PB9 was the only free pin offering UART4_TX. |
| UART4_RX | PC11 | **PB8** | PC11 is SDMMC1_D3. All five UART4_RX-capable pins were occupied, so WDT_KICK gave up PB8. |
| WDT_KICK | PB8 | **PD9** | Displaced by UART4_RX. It is a plain GPIO output to the TPS3823 WDI, so any free pin serves. |
| LORA_RST, GSM_PWR_KEY | PC2, PC3 | **PB10, PB11** | On the 100-pin package those are the dual-pad PC2_C/PC3_C direct-analog variants. Both signals are plain GPIO resets, so ordinary GPIOs are used rather than relying on the analog switch. |

`SD_CD` was not assigned at all in the old notes and is now on PD15.

The full signal-to-pin table is in the MCU sheet's annotation.

### 7.9 IC pinout verification

With the KiCad symbol libraries available, every IC's footprint wiring was checked
against the pin names its datasheet-derived symbol gives. The test is simple and hard
to argue with: if a pin the datasheet calls VDD is not on a supply net, or one it calls
GND is not on a ground net, the pad-to-net map was not derived from the datasheet.

**19 of 21 checkable ICs are now consistent. Three were rebuilt from their symbols:**

| Part | What was wrong | Now |
|------|----------------|-----|
| U50 W5500 | SPI on the Ethernet differential pins, supplies and grounds scattered across 10 wrong pads — the whole map was sequential | Rebuilt from the 48-pin datasheet pinout. SPI on 32-37, TX/RX pairs on 1/2/5/6, PMODE2..0 grounded for all-capable auto-negotiation |
| U40 ISO3082DW | receiver and driver swapped (R/D on pins 3/6), bus A/B one pin out, GND2 on pin 15 carrying the isolated supply | Rebuilt from the 16-pin pinout |
| U1 TPS54560 | SW on both pins 7 and 8, so the switch node shorted to GND; FB not on pin 5 | BOOT/VIN/FB/SW/GND corrected |

**Two remain unverifiable** because the stock library has no symbol for the exact part:

- **U60 SX1276** — the footprint is `QFN-28-1EP_4x4mm_P0.4mm`, but SX1276IMLTRT is a
  **6x6mm QFN-28 on 0.65mm pitch**. The footprint is the wrong size as well as
  sequentially wired. This needs the datasheet before it can be corrected.
- **U3 TPS7A4533DGN** — MSOP-8-EP. The library only carries the KTT (TO-263-5)
  variant, which has a different pinout, so it cannot be used as a stand-in.

Also unverifiable for want of a symbol: U41 ISO1042, U42/U43 NXJ1S, U61 ATWINC1500,
U62 SIM7600, U62B RN4870, U70 W25Q64.

U41 is worth showing, because it carries the *same signature* the ISO3082 had before it
was rebuilt — both are TI isolated transceivers in SOIC-16W, so the two maps are
directly comparable:

| Pin | ISO3082 function | U40 after rebuild | U41 as it stands |
|-----|------------------|-------------------|------------------|
| 11 | NC | (open) | CAN_H |
| 12 | A | RS485_A | CAN_L |
| 13 | B | RS485_B | CAN_ISO_GND |
| 14 | NC | (open) | CAN_ISO_GND |
| 15 | GND2 | RS485_ISO_GND | CAN_ISO_VCC |
| 16 | Vcc2 | RS485_ISO_VCC | CAN_ISO_VCC |

The bus pair sits one pin low, the isolated ground is on the two NC pins, and the
isolated supply is on both 15 and 16 — the identical off-by-one that the RS485 side had.
TI places GND2 on 9/10/15 and VCC2 on 16 across this family, so VCC2 on both 15 and 16
is almost certainly wrong. It is left alone rather than guessed at: no ISO1042 symbol
exists in the library (ISO1044BD is SOIC-8 and ISO1050DUB is SOP-8, neither comparable),
so this needs the datasheet.

The rest should be assumed wrong until checked the same way. Note that not everything
is: U70 (W25Q64) and U71 (AT24C256) both verify clean, so the placeholder wiring was
applied to the complex parts rather than uniformly.

### 7.10 Missing support components found during verification

The W5500 datasheet requires three parts the design never included; all are now placed:

| Part | Pin | Purpose |
|------|-----|---------|
| R56 12.4k 1% | EXRES1 (10) | sets the transmit bias current; without it the PHY does not meet the 802.3 output spec |
| C87 4.7uF | TOCAP (20) | decouples the internal 1.2V transceiver regulator |
| C88 100nF | VBG (18) | bandgap reference decoupling |

Still outstanding on U1 (TPS54560): **EN, RT/CLK and COMP have no parts on them.**
RT/CLK needs a resistor to set the switching frequency and COMP needs a compensation
network — the buck will not regulate without them. EN can be left open (internal
pull-up) but a divider is the usual choice for programmed UVLO.

### 7.9 Routing status

Autorouted with freerouting on F.Cu / In2.Cu / B.Cu; In1.Cu stays an unbroken ground
plane. Verified independently against the pad list rather than trusting the router's
own report:

| | |
|---|---|
| Connections realised | 271 of 322 (84%) |
| Nets fully routed | 110 of 155 |
| Nets partially routed | 35 |
| Nets with no copper | 10 |
| Tracks / vias | 1520 / 303 (152 routing + 151 GND stitching) |
| Isolation violations | 0 |

`docs/unrouted_nets.txt` lists the 45 nets that still need work.

Note that freerouting reported "3 unrouted" for this session. That figure is wrong —
checking each net's pads against the imported copper shows many MCU pins never
reached, so the router's own count should not be relied on. The 84% above comes from
walking the connectivity graph per net.

Ten track segments and vias were removed after import because freerouting had routed
NRST, RS485_DE and FDCAN1_TX through the isolated corner. The router has no concept of
which nets may enter the field-side pocket, so this needs re-checking after every
routing pass. The isolator logic pins sit at y=26.7 with the notch edge at y=26, which
leaves almost no routing channel — if this recurs, move the U40-U43 barrier row down
about 2mm so the logic pins clear the notch.

**This routing will need redoing once the MCU pin assignment in 7.8 is fixed**, since
every MCU connection moves. It is kept because the placement, isolation geometry and
design rules it validates all stand.

## 8. Design Files

```
STM32H743_DataLogger/
├── STM32H743_DataLogger.kicad_pro     # Project file
├── STM32H743_DataLogger.kicad_sch     # Top-level schematic (hierarchical)
├── STM32H743_DataLogger.kicad_pcb     # PCB layout (4-layer, 160x100mm)
├── Power_Supply.kicad_sch             # 24V input, buck, LDOs
├── STM32H743_MCU.kicad_sch           # MCU, crystals, decoupling, debug
├── Analog_Inputs.kicad_sch           # 4ch AI (4-20mA/0-10V)
├── Digital_IO.kicad_sch              # 8x DI + 4x DO (relay)
├── RS485_CAN.kicad_sch               # Isolated RS485 + CAN FD
├── Ethernet.kicad_sch                # W5500 SPI Ethernet + RJ45
├── Wireless.kicad_sch                # SX1276 LoRa + ATWINC1500 WiFi
│                                       + RN4870 BLE + SIM7600G-H GSM
│                                       + TPS3823 Hardware Watchdog
├── Storage.kicad_sch                 # microSD + Flash + EEPROM
├── docs/
│   ├── BOM_STM32H743_DataLogger.csv  # Complete Bill of Materials
│   └── Design_Specification.md       # This document
└── gerber/                           # Gerber output directory
```

## 9. Key Design Decisions (Rev 1.2)

1. **W5500 over LAN8742A**: Hardwired TCP/IP offloads networking from MCU, uses only 4 SPI pins vs 11 RMII, simpler firmware
2. **SX1276 bare IC over RFM95W module**: Full industrial temp range (-40 to +85°C), custom RF matching for optimal performance
3. **ATWINC1500 over ESP32**: Eliminates dual-MCU architecture, industrial temp rated, SPI slave (no second processor)
4. **RN4870 for BLE**: Dedicated BLE 5.0, UART interface, built-in antenna, industrial temp, Microchip ecosystem
5. **TPS3823-33 Hardware Watchdog**: External WDT ensures system recovery, monitors supply voltage
6. **HumiSeal 1B73 Conformal Coating**: Military-grade moisture/dust protection
7. **4-Layer PCB with solid GND plane**: Essential for EMC and signal integrity
8. **Isolated RS485/CAN (5kVrms)**: Galvanic isolation for harsh industrial environments
9. **TPS7A4533 ultra-low-noise analog LDO**: Separate clean power for ADC accuracy
10. **DIN Rail Mounting**: Eurocard form factor for standard PLC cabinets

## 10. Estimated Cost (per unit)

| Item | 1 pc | 100 pc |
|------|------|--------|
| Components (BOM) | ~$142 | ~$90 |
| PCB Fabrication | ~$15 | ~$5 |
| Assembly (SMT+THT) | ~$35 | ~$12 |
| Conformal Coating | ~$5 | ~$2 |
| **Total** | **~$197** | **~$109** |
