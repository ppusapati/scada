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
Layer 3 (In2.Cu): Split Power Plane          35μm
Prepreg         : FR-4                        0.2mm (εr=4.5)
Layer 4 (B.Cu)  : Signal + Components        35μm
──────────────────────────────────────────────
Total                                         ~1.6mm
```

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

| Interface | Isolation Method | Rating |
|-----------|-----------------|--------|
| Digital Inputs | TLP293 Optocoupler | 3.75kVrms |
| RS485 | ISO3082DWR | 5kVrms |
| CAN FD | ISO1042BQDWRQ1 | 5kVrms |
| Ethernet | RJ45 magnetics (J0011D21BNL) | 1.5kVrms |

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
