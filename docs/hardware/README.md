# SCADA System - Hardware Documentation

## Overview

This directory contains complete hardware documentation for the SCADA (Supervisory Control and Data Acquisition) system, covering both the Solar String Monitoring Unit (SMU) and Water Remote Terminal Unit (RTU).

## Directory Structure

```
docs/hardware/
├── system_architecture.md          # Full system architecture & block diagrams
├── Solar_SMU/
│   ├── SS-PCB-001_design.md        # Solar SMU board design specification
│   └── SS-PCB-001_pinout.md        # Complete MCU pin assignments (100-pin)
├── Water_RTU/
│   ├── WR-PCB-001_design.md        # Water RTU board design specification
│   └── WR-PCB-001_pinout.md        # Complete MCU pin assignments (100-pin)
├── bom/
│   ├── SS-PCB-001_BOM.csv          # Solar SMU Bill of Materials (full)
│   └── WR-PCB-001_BOM.csv          # Water RTU Bill of Materials (full)
├── common/
│   ├── power_supply.md             # Power supply design & requirements
│   ├── connectors.md               # Connector specifications & wiring
│   ├── pcb_guidelines.md           # PCB layout & manufacturing guidelines
│   └── stm32f407_base.md           # Common MCU configuration
├── dds_upgrade/
│   └── dds_implementation.md      # DDS upgrade: STM32MP157 architecture, cost, topics
├── kicad/
│   ├── Solar_SMU/
│   │   ├── SS-PCB-001.kicad_pro    # KiCad 8 project file
│   │   └── SS-PCB-001.kicad_sch    # KiCad 8 schematic (4 sheets)
│   ├── Water_RTU/
│   │   ├── WR-PCB-001.kicad_pro    # KiCad 8 project file
│   │   └── WR-PCB-001.kicad_sch    # KiCad 8 schematic (5 sheets)
│   └── libraries/
│       └── scada_custom.kicad_sym  # Custom symbol library (6 symbols)
└── README.md                       # This file
```

## Board Summary

| Board | Reference | MCU | Key Peripherals | Communication |
|-------|-----------|-----|-----------------|---------------|
| Solar SMU | SS-PCB-001 | STM32F407VGT6 | ADS1263 ADC, 2x CD74HC4067 MUX, TMP117, SD Card | RS-485 Modbus RTU |
| Water RTU | WR-PCB-001 | STM32F407VGT6 | ADS1258 ADC, W5500 Ethernet, SP3485 | RS-485 Modbus RTU + Modbus TCP |

## Quick Reference

- **MCU**: STM32F407VGT6 (Cortex-M4F, 168 MHz, 1 MB Flash, 192 KB SRAM)
- **Power**: 24V DC field power (18-30V), regulated to 5V and 3.3V on-board
- **Operating Temperature**: -20 to +70 deg C (Water RTU), -40 to +85 deg C (Solar SMU)
- **Firmware**: Embassy async Rust (no_std, bare-metal)
- **KiCad Version**: 8.0 (S-expression schematic format)

## Document Index

| Document | Description |
|----------|-------------|
| [System Architecture](system_architecture.md) | System block diagrams, communication protocols, Modbus register maps, data flow |
| [Solar SMU Design](Solar_SMU/SS-PCB-001_design.md) | Circuit design, ASCII schematics, component details for SS-PCB-001 |
| [Solar SMU Pinout](Solar_SMU/SS-PCB-001_pinout.md) | STM32F407 100-pin LQFP assignment, GPIO config, DMA, interrupts |
| [Water RTU Design](Water_RTU/WR-PCB-001_design.md) | Circuit design, ASCII schematics, component details for WR-PCB-001 |
| [Water RTU Pinout](Water_RTU/WR-PCB-001_pinout.md) | STM32F407 100-pin LQFP assignment, GPIO config, DMA, interrupts |
| [Solar SMU BOM](bom/SS-PCB-001_BOM.csv) | Full bill of materials with manufacturer part numbers |
| [Water RTU BOM](bom/WR-PCB-001_BOM.csv) | Full bill of materials with manufacturer part numbers |
| [Power Supply](common/power_supply.md) | Shared 24V/5V/3.3V power supply design |
| [Connectors](common/connectors.md) | Connector specifications, pinouts, wire gauges |
| [PCB Guidelines](common/pcb_guidelines.md) | 4-layer stackup, routing rules, DFM notes |
| [STM32F407 Base](common/stm32f407_base.md) | Common MCU configuration, clock tree, decoupling |
| [Cost Estimation Report](../cost_estimation_report.md) | Complete cost analysis: BOM, PCB, assembly, testing, enclosures, market analysis (INR) |
| [DDS Upgrade — STM32MP157](dds_upgrade/dds_implementation.md) | Full DDS architecture with STM32MP157, dual-core design, revised BOM, cost impact |
