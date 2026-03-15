# STM32H743 PLC-Grade Data Logger - Design Specification

## Project: SCADA Industrial Data Logger PCB
**Revision:** 1.0
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

## 2. Communication Interfaces

| Interface | IC/Module | Protocol | Speed | Connector |
|-----------|-----------|----------|-------|-----------|
| **Ethernet** | LAN8742A-CZ-TR | 10/100 RMII | 100Mbps | RJ45 (magnetics) |
| **LoRa** | RFM95W (SX1276) | LoRaWAN | 300bps-50kbps | SMA |
| **WiFi** | ESP32-C3-MINI-1 | 802.11 b/g/n | 150Mbps | u.FL |
| **BLE** | ESP32-C3-MINI-1 | BLE 5.0 | 2Mbps | u.FL (shared) |
| **GSM/LTE** | SIM7600G-H | LTE Cat-1 | 10Mbps DL | SMA x2 |
| **RS485** | ISO3082DWR | Modbus RTU | 20Mbps | 3-pin terminal |
| **CAN FD** | ISO1042BQDWRQ1 | CAN FD 2.0 | 5Mbps | 3-pin terminal |
| **USB** | Native USB OTG | USB 2.0 FS | 12Mbps | USB-C |

## 3. Power Architecture

```
24VDC Input (9-36V)
  │
  ├─ PTC Fuse (1.5A) ─► TVS (SMDJ36A) ─► Reverse Polarity MOSFET ─► EMI Filter
  │
  ├─ TPS54560 Buck ──────► +5V Rail (3A max)
  │     │
  │     ├─ TLV1117LV33 LDO ──► +3V3 Digital Rail (1A)
  │     │
  │     ├─ TPS7A4533 LDO ────► +3V3A Analog Rail (150mA, ultra-low noise)
  │     │
  │     └─ TPS73641 LDO ─────► +4V1 GSM Module Rail (400mA)
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

## 6. STM32H743 Pin Assignment Summary

| Peripheral | Pins | Function |
|-----------|------|----------|
| ETH RMII | PA1, PA2, PA7, PB11-13, PC1, PC4-5 | Ethernet PHY |
| FDCAN1 | PD0 (RX), PD1 (TX) | CAN FD Bus |
| USART2 | PD5 (TX), PD6 (RX), PD4 (DE) | RS485 |
| UART4 | PA0 (TX), PC11 (RX) | GSM Module |
| UART7 | PE8 (TX), PE7 (RX) | ESP32-C3 |
| SPI1 | PA5, PA6, PB5, PA4 | LoRa Module |
| SPI2 | PB13-15, PB12 | ESP32-C3 (alt) |
| SPI3 | PC10-12, PA15 | External Flash |
| SDMMC1 | PC8-12, PD2 | microSD Card |
| I2C1 | PB6 (SCL), PB7 (SDA) | EEPROM |
| ADC1 | PA0-PA3 | Analog Inputs (4ch) |
| USB OTG FS | PA11 (DM), PA12 (DP) | USB-C |
| SWD | PA13, PA14 | Debug |
| HSE | PH0, PH1 | 25MHz Crystal |
| LSE | PC14, PC15 | 32.768kHz RTC |

## 7. Design Files

```
STM32H743_DataLogger/
├── STM32H743_DataLogger.kicad_pro     # Project file
├── STM32H743_DataLogger.kicad_sch     # Top-level schematic (hierarchical)
├── STM32H743_DataLogger.kicad_pcb     # PCB layout
├── Power_Supply.kicad_sch             # 24V input, buck, LDOs
├── STM32H743_MCU.kicad_sch           # MCU, crystals, decoupling, debug
├── Analog_Inputs.kicad_sch           # 4ch AI (4-20mA/0-10V)
├── Digital_IO.kicad_sch              # 8x DI + 4x DO (relay)
├── RS485_CAN.kicad_sch               # Isolated RS485 + CAN FD
├── Ethernet.kicad_sch                # LAN8742A PHY + RJ45
├── Wireless.kicad_sch                # LoRa + BLE/WiFi + GSM/LTE
├── Storage.kicad_sch                 # microSD + Flash + EEPROM
├── docs/
│   ├── BOM_STM32H743_DataLogger.csv  # Complete Bill of Materials
│   └── Design_Specification.md       # This document
└── gerber/                           # Gerber output directory
```

## 8. Key Design Decisions

1. **25MHz HSE Crystal**: Required for Ethernet RMII (50MHz from 25MHz PLL)
2. **Separate Analog LDO (TPS7A4533)**: Ultra-low noise for ADC accuracy
3. **Isolated RS485/CAN**: ISO3082/ISO1042 provide 5kVrms galvanic isolation
4. **ESP32-C3 for BLE+WiFi**: Single module handles both wireless protocols
5. **SIM7600G-H**: Global LTE bands with 2G/3G fallback for remote sites
6. **4-Layer PCB**: Proper GND/Power planes for EMI and signal integrity
7. **DIN Rail Mounting**: Industrial form factor for PLC cabinets
8. **Phoenix Contact Terminals**: Industrial-grade connectors throughout

## 9. Estimated Cost (per unit)

| Item | 1 pc | 100 pc |
|------|------|--------|
| Components (BOM) | ~$135 | ~$85 |
| PCB Fabrication | ~$15 | ~$5 |
| Assembly (SMT+THT) | ~$35 | ~$12 |
| **Total** | **~$185** | **~$102** |
