# SCADA System — Comprehensive Cost Estimation Report
## Solar String Monitoring Unit (SS-PCB-001) & Water Remote Terminal Unit (WR-PCB-001)

**Document No:** SCADA-COST-001
**Revision:** 1.0
**Date:** March 2026
**Currency:** Indian Rupees (INR) | 1 USD = ~85 INR
**Applicable Region:** India

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Bill of Materials — Solar SMU (SS-PCB-001)](#2-bill-of-materials--solar-smu-ss-pcb-001)
3. [Bill of Materials — Water RTU (WR-PCB-001)](#3-bill-of-materials--water-rtu-wr-pcb-001)
4. [PCB Design & Fabrication Costs](#4-pcb-design--fabrication-costs)
5. [SMT Assembly & Manufacturing Costs](#5-smt-assembly--manufacturing-costs)
6. [Testing & Calibration Costs](#6-testing--calibration-costs)
7. [Enclosures, Cabinets & Outdoor Station Fabrication](#7-enclosures-cabinets--outdoor-station-fabrication)
8. [Complete Unit Cost Summary](#8-complete-unit-cost-summary)
9. [Production Equipment & Infrastructure](#9-production-equipment--infrastructure)
10. [Indian Market Analysis](#10-indian-market-analysis)
11. [Regulatory & Certification Costs](#11-regulatory--certification-costs)
12. [Pricing Strategy & Margins](#12-pricing-strategy--margins)
13. [Appendix — Key Suppliers & Sources](#13-appendix--key-suppliers--sources)

---

## 1. Executive Summary

This report provides a comprehensive cost estimation for manufacturing the SCADA system hardware — comprising the **Solar String Monitoring Unit (SMU)** and the **Water Remote Terminal Unit (RTU)** — specifically for the Indian market. All costs are estimated in INR with USD equivalents where applicable.

### Quick Summary (Per Unit, at 100-unit Production Volume)

| Item | Solar SMU (INR) | Water RTU (INR) |
|------|-----------------|-----------------|
| Component BOM | ₹8,450 | ₹6,820 |
| PCB Fabrication (4-layer) | ₹650 | ₹480 |
| SMT Assembly | ₹1,200 | ₹1,000 |
| Testing & Calibration | ₹800 | ₹700 |
| Enclosure (DIN-rail IP20) | ₹450 | ₹350 |
| Packaging & Miscellaneous | ₹200 | ₹200 |
| **Per-Unit Manufacturing Cost** | **₹11,750** | **₹9,550** |
| **Per-Unit Cost (USD)** | **~$138** | **~$112** |

### Cost for Complete Outdoor SCADA Station (with cabinet, wiring, accessories)

| Configuration | Cost (INR) | Cost (USD) |
|---------------|------------|------------|
| Solar SMU Station (16-string) | ₹45,000 – ₹65,000 | $530 – $765 |
| Water RTU Station (8-channel) | ₹55,000 – ₹80,000 | $647 – $941 |

---

## 2. Bill of Materials — Solar SMU (SS-PCB-001)

### 2.1 Active Components (ICs)

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) | Notes |
|-----|-----------|-----|-------------------|----------------|-------|
| U3 | STM32F407VGT6 (LQFP-100) | 1 | ₹1,080 | ₹1,080 | DigiKey India price |
| U4 | ADS1263IPW (32-bit ADC) | 1 | ₹2,500 | ₹2,500 | TI precision ADC, Mouser/Semikart |
| U5 | CD74HC4067SM96 (MUX A) | 1 | ₹85 | ₹85 | 16-ch analog MUX |
| U6 | CD74HC4067SM96 (MUX B) | 1 | ₹85 | ₹85 | 16-ch analog MUX |
| U1 | LM2596S-5.0 (DC-DC) | 1 | ₹120 | ₹120 | 5V/3A buck regulator |
| U2 | AMS1117-3.3 (LDO) | 1 | ₹15 | ₹15 | 3.3V/1A LDO |
| U7 | SP3485EN (RS-485) | 1 | ₹28 | ₹28 | IndiaMART price |
| U8 | TMP117AIDRVR (Temp sensor) | 1 | ₹450 | ₹450 | Precision ±0.1°C |
| Q1 | SI2301CDS (P-MOSFET) | 1 | ₹12 | ₹12 | Reverse polarity |
| | **Subtotal — Active ICs** | | | **₹4,375** | |

### 2.2 Discrete Semiconductors

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) |
|-----|-----------|-----|-------------------|----------------|
| D1 | SMBJ30A TVS diode | 1 | ₹18 | ₹18 |
| D2 | SS34 Schottky diode | 1 | ₹12 | ₹12 |
| D3, D4 | SMBJ6.0A TVS (RS-485) | 2 | ₹15 | ₹30 |
| D5–D36 | SMBJ5.0A TVS (string inputs) | 32 | ₹12 | ₹384 |
| LED1–4 | 0805 LEDs (G, O, R, B) | 4 | ₹5 | ₹20 |
| | **Subtotal — Discretes** | | | **₹464** |

### 2.3 Passive Components

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) |
|-----|-----------|-----|-------------------|----------------|
| R_DIV_HI | 1M 1% 0805 | 16 | ₹2 | ₹32 |
| R_DIV_LO | 2.5K 0.1% precision 0805 | 16 | ₹15 | ₹240 |
| R_SHUNT | 10mΩ current shunt 2512 | 16 | ₹35 | ₹560 |
| R_FILT | 100Ω 1% 0805 | 32 | ₹1.5 | ₹48 |
| R_LED | 1K 0805 | 4 | ₹1.5 | ₹6 |
| R_PU (various) | 10K, 4.7K pull-ups 0805 | 8 | ₹1.5 | ₹12 |
| R_BIAS | 470Ω 0805 | 2 | ₹1.5 | ₹3 |
| R_TERM | 120Ω 0805 | 1 | ₹1.5 | ₹1.5 |
| R_BOOT0, R_NRST | 10K 0805 | 2 | ₹1.5 | ₹3 |
| R_SPI | 33Ω 0805 | 6 | ₹1.5 | ₹9 |
| C_IN_BULK | 680µF/35V electrolytic | 1 | ₹35 | ₹35 |
| C_OUT_BULK | 220µF/10V electrolytic | 1 | ₹18 | ₹18 |
| C_BYPASS | 100nF 0805 ceramic | 15 | ₹2 | ₹30 |
| C (10µF) | 10µF 0805 ceramic | 8 | ₹5 | ₹40 |
| C (1µF) | 1µF 0805 ceramic | 2 | ₹3 | ₹6 |
| C_HSE | 20pF NP0 0805 | 2 | ₹3 | ₹6 |
| C_LSE | 6.8pF NP0 0805 | 2 | ₹3 | ₹6 |
| C_FILT | 100nF C0G 0805 (input filters) | 32 | ₹3 | ₹96 |
| C_LDO | 10µF + 22µF ceramic | 2 | ₹5 | ₹10 |
| C (4.7µF) | 4.7µF bulk VDD 0805 | 1 | ₹4 | ₹4 |
| | **Subtotal — Passives** | | | **₹1,166** |

### 2.4 Inductors & Ferrites

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) |
|-----|-----------|-----|-------------------|----------------|
| L1 | 33µH 3A power inductor (Bourns) | 1 | ₹85 | ₹85 |
| FB1 | 600Ω ferrite bead 0805 | 1 | ₹8 | ₹8 |
| | **Subtotal — Magnetics** | | | **₹93** |

### 2.5 Crystals & Oscillators

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) |
|-----|-----------|-----|-------------------|----------------|
| Y1 | 8 MHz HC49/SMD crystal | 1 | ₹25 | ₹25 |
| Y2 | 32.768 kHz SMD crystal | 1 | ₹20 | ₹20 |
| | **Subtotal — Crystals** | | | **₹45** |

### 2.6 Connectors

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) |
|-----|-----------|-----|-------------------|----------------|
| J1 | 2-pin terminal block 5.08mm (Phoenix) | 1 | ₹45 | ₹45 |
| J2 | 32-pin terminal block 3.81mm (Phoenix) | 1 | ₹850 | ₹850 |
| J3 | 3-pin terminal block 5.08mm (Phoenix) | 1 | ₹60 | ₹60 |
| J4 | 2-pin header 2.54mm + jumper | 1 | ₹8 | ₹8 |
| J5 | 2x5 SWD header 1.27mm (Samtec) | 1 | ₹180 | ₹180 |
| J6 | MicroSD socket (Molex) | 1 | ₹65 | ₹65 |
| | **Subtotal — Connectors** | | | **₹1,208** |

### 2.7 Solar SMU — Total BOM Cost

| Category | Cost (INR) |
|----------|------------|
| Active ICs | ₹4,375 |
| Discrete Semiconductors | ₹464 |
| Passive Components | ₹1,166 |
| Magnetics | ₹93 |
| Crystals | ₹45 |
| Connectors | ₹1,208 |
| **Component BOM Total** | **₹7,351** |
| Procurement overhead (+15%) | ₹1,103 |
| **Landed BOM Cost** | **₹8,454** |

> The 15% procurement overhead accounts for shipping, customs/BCD duty on imported ICs, GST input credits, MOQ surcharges, and component wastage/attrition during assembly.

---

## 3. Bill of Materials — Water RTU (WR-PCB-001)

### 3.1 Active Components (ICs)

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) | Notes |
|-----|-----------|-----|-------------------|----------------|-------|
| U3 | STM32F407VGT6 (LQFP-100) | 1 | ₹1,080 | ₹1,080 | DigiKey India price |
| U4 | ADS1258IPHPR (24-bit ADC) | 1 | ₹1,200 | ₹1,200 | TI 16-ch delta-sigma |
| U5 | W5500 (Ethernet controller) | 1 | ₹250 | ₹250 | WIZnet QFN-48 |
| U1 | LM2596S-5.0 (DC-DC) | 1 | ₹120 | ₹120 | 5V/3A buck regulator |
| U2 | AMS1117-3.3 (LDO) | 1 | ₹15 | ₹15 | 3.3V/1A LDO |
| U6 | SP3485EN (RS-485) | 1 | ₹28 | ₹28 | Half-duplex transceiver |
| Q1 | SI2301CDS (P-MOSFET) | 1 | ₹12 | ₹12 | Reverse polarity |
| | **Subtotal — Active ICs** | | | **₹2,705** | |

### 3.2 Discrete Semiconductors

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) |
|-----|-----------|-----|-------------------|----------------|
| D1 | SMBJ30A TVS diode | 1 | ₹18 | ₹18 |
| D2 | SS34 Schottky diode | 1 | ₹12 | ₹12 |
| D3, D4 | SMBJ6.5A TVS (RS-485) | 2 | ₹15 | ₹30 |
| D5–D12 | SM6T6V8A TVS (4-20mA inputs) | 8 | ₹18 | ₹144 |
| D13–D28 | BAT54 Schottky (clamp) | 16 | ₹5 | ₹80 |
| LED1–4 | 0805 LEDs (G, O, R, B) | 4 | ₹5 | ₹20 |
| | **Subtotal — Discretes** | | | **₹304** |

### 3.3 Passive Components

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) |
|-----|-----------|-----|-------------------|----------------|
| R_SHUNT | 250Ω 0.1% precision 0805 | 8 | ₹15 | ₹120 |
| R_FILT | 100Ω 1% 0805 | 8 | ₹1.5 | ₹12 |
| R_LED | 1K 0805 | 4 | ₹1.5 | ₹6 |
| R_PU (various) | 10K pull-ups 0805 | 7 | ₹1.5 | ₹10.5 |
| R_BIAS | 470Ω 0805 | 2 | ₹1.5 | ₹3 |
| R_TERM | 120Ω 0805 | 1 | ₹1.5 | ₹1.5 |
| R_BOOT0, R_NRST | 10K 0805 | 2 | ₹1.5 | ₹3 |
| R_SPI1 | 33Ω 0805 (ADS1258 SPI) | 6 | ₹1.5 | ₹9 |
| R_SPI2 | 33Ω 0805 (W5500 SPI) | 6 | ₹1.5 | ₹9 |
| R_TX | 49.9Ω 0805 (Ethernet) | 2 | ₹2 | ₹4 |
| R_EXRES | 12.4K 1% 0805 (W5500) | 1 | ₹2 | ₹2 |
| C_IN_BULK | 100µF/50V electrolytic | 1 | ₹25 | ₹25 |
| C_OUT_BULK | 220µF/10V electrolytic | 1 | ₹18 | ₹18 |
| C_BYPASS | 100nF 0805 ceramic | 18 | ₹2 | ₹36 |
| C (10µF) | 10µF 0805 ceramic | 12 | ₹5 | ₹60 |
| C (1µF) | 1µF 0805 ceramic | 1 | ₹3 | ₹3 |
| C_HSE | 22pF NP0 0805 | 2 | ₹3 | ₹6 |
| C_LSE | 6.8pF NP0 0805 | 2 | ₹3 | ₹6 |
| C_W5500_XTAL | 22pF NP0 0805 | 2 | ₹3 | ₹6 |
| C_FILT | 100nF C0G 0805 (input filters) | 8 | ₹3 | ₹24 |
| C (4.7µF) | 4.7µF bulk VDD 0805 | 1 | ₹4 | ₹4 |
| C_LDO | 10µF ceramic | 2 | ₹5 | ₹10 |
| | **Subtotal — Passives** | | | **₹378** |

### 3.4 Inductors, Ferrites, Crystals & Connectors

| Ref | Component | Qty | Unit Price (INR) | Extended (INR) |
|-----|-----------|-----|-------------------|----------------|
| L1 | 33µH 3A power inductor | 1 | ₹85 | ₹85 |
| FB1 | 600Ω ferrite bead 0805 | 1 | ₹8 | ₹8 |
| FB2 | 600Ω ferrite bead 0805 (W5500) | 1 | ₹8 | ₹8 |
| Y1 | 8 MHz HC49/SMD crystal | 1 | ₹25 | ₹25 |
| Y2 | 32.768 kHz SMD crystal | 1 | ₹20 | ₹20 |
| Y3 | 25 MHz HC49/SMD crystal (W5500) | 1 | ₹25 | ₹25 |
| J1 | 2-pin terminal block 5.08mm | 1 | ₹45 | ₹45 |
| J2 | 16-pin terminal block 3.81mm | 1 | ₹420 | ₹420 |
| J3 | 3-pin terminal block 5.08mm | 1 | ₹60 | ₹60 |
| J4 | 2-pin header 2.54mm + jumper | 1 | ₹8 | ₹8 |
| J5 | 2x5 SWD header 1.27mm | 1 | ₹180 | ₹180 |
| J7 | HR911105A RJ45 w/ magnetics | 1 | ₹120 | ₹120 |
| | **Subtotal — Others** | | | **₹1,004** |

### 3.5 Water RTU — Total BOM Cost

| Category | Cost (INR) |
|----------|------------|
| Active ICs | ₹2,705 |
| Discrete Semiconductors | ₹304 |
| Passive Components | ₹378 |
| Magnetics, Crystals, Connectors | ₹1,004 |
| **Component BOM Total** | **₹4,391** |
| Procurement overhead (+15%) | ₹659 |
| **Landed BOM Cost** | **₹5,050** |

### 3.6 BOM Cost Comparison

| Board | Components | BOM (INR) | Landed BOM (INR) | Landed BOM (USD) |
|-------|------------|-----------|-------------------|-------------------|
| Solar SMU (SS-PCB-001) | ~120 | ₹7,351 | ₹8,454 | ~$99 |
| Water RTU (WR-PCB-001) | ~95 | ₹4,391 | ₹5,050 | ~$59 |

> **Volume discounts:** At 500+ units, expect 10-15% BOM reduction. At 1000+ units, expect 20-25% reduction through direct manufacturer negotiations and consolidated purchase orders.

---

## 4. PCB Design & Fabrication Costs

### 4.1 PCB Design (NRE — One-Time)

| Activity | Cost (INR) | Cost (USD) | Notes |
|----------|------------|------------|-------|
| Schematic capture (both boards) | ₹80,000 | $941 | KiCad/Altium, 2-3 weeks |
| PCB layout — Solar SMU (160x100mm, 4L) | ₹1,20,000 | $1,412 | 4-layer, impedance-controlled |
| PCB layout — Water RTU (120x80mm, 4L) | ₹1,00,000 | $1,176 | 4-layer, impedance-controlled |
| Design rule check & review | ₹20,000 | $235 | Signal integrity, DFM review |
| Gerber generation & fab-ready files | ₹15,000 | $176 | Includes BOM, placement files |
| **Total PCB Design (one-time)** | **₹3,35,000** | **$3,941** | |

> Indian PCB design houses (e.g., Mistral Solutions, Tessolve, Ennetix) typically charge ₹800-₹1,500 per pin for complex mixed-signal designs. Freelancers on platforms charge ₹500-₹800 per pin.

### 4.2 PCB Fabrication (Per Board — Recurring)

**Solar SMU — 4-layer, 160x100mm, 1.6mm, FR-4, HASL**

| Quantity | Per Board (INR) | Per Board (USD) | Total Lot (INR) | Source |
|----------|-----------------|-----------------|-----------------|--------|
| 10 pcs (prototype) | ₹1,200 | $14.1 | ₹12,000 | Indian fab (PCB Power, LionCircuits) |
| 50 pcs | ₹750 | $8.8 | ₹37,500 | |
| 100 pcs | ₹550 | $6.5 | ₹55,000 | |
| 500 pcs | ₹320 | $3.8 | ₹1,60,000 | |
| 1000 pcs | ₹250 | $2.9 | ₹2,50,000 | JLCPCB/Chinese fab |

**Water RTU — 4-layer, 120x80mm, 1.6mm, FR-4, HASL**

| Quantity | Per Board (INR) | Per Board (USD) | Total Lot (INR) |
|----------|-----------------|-----------------|-----------------|
| 10 pcs (prototype) | ₹900 | $10.6 | ₹9,000 |
| 50 pcs | ₹550 | $6.5 | ₹27,500 |
| 100 pcs | ₹400 | $4.7 | ₹40,000 |
| 500 pcs | ₹240 | $2.8 | ₹1,20,000 |
| 1000 pcs | ₹190 | $2.2 | ₹1,90,000 |

**ENIG Finish Surcharge:** +₹100-₹200 per board (recommended for LQFP-100 & QFN-48 fine-pitch)

### 4.3 Stencil Costs (One-time per board revision)

| Item | Cost (INR) | Notes |
|------|------------|-------|
| SMT stencil — Solar SMU (top) | ₹3,500 | Laser-cut stainless steel, 0.12mm |
| SMT stencil — Solar SMU (bottom) | ₹2,500 | If double-sided assembly |
| SMT stencil — Water RTU (top) | ₹3,000 | Laser-cut stainless steel, 0.12mm |
| SMT stencil — Water RTU (bottom) | ₹2,000 | If double-sided assembly |
| **Total stencils** | **₹11,000** | One-time per revision |

---

## 5. SMT Assembly & Manufacturing Costs

### 5.1 Assembly NRE (One-Time Setup)

| Item | Cost (INR) | Notes |
|------|------------|-------|
| SMT programming & setup — Solar SMU | ₹15,000 | Pick-and-place programming |
| SMT programming & setup — Water RTU | ₹12,000 | Pick-and-place programming |
| Reflow profile development (per board) | ₹5,000 | Lead-free profile optimization |
| AOI (Automated Optical Inspection) setup | ₹8,000 | Program creation, golden board |
| Test fixture development (per board) | ₹25,000 | ICT/functional test jig |
| **Total Assembly NRE** | **₹65,000** | One-time per board variant |

### 5.2 Per-Board Assembly Cost

| Activity | Solar SMU (INR) | Water RTU (INR) | Notes |
|----------|-----------------|-----------------|-------|
| Solder paste printing | ₹50 | ₹50 | Stencil printing |
| Pick-and-place (SMT) | ₹400 | ₹350 | ~120 placements (SMU), ~95 (RTU) |
| Reflow soldering | ₹80 | ₹80 | Lead-free profile |
| Through-hole assembly | ₹150 | ₹120 | Connectors, electrolytics (manual) |
| Wave/selective soldering | ₹60 | ₹60 | TH components |
| AOI inspection | ₹50 | ₹50 | Automated optical inspection |
| Touch-up & rework | ₹80 | ₹60 | Manual inspection/rework |
| Board wash & cleaning | ₹30 | ₹30 | IPA wash, ultrasonic |
| **Per-board assembly total** | **₹900** | **₹800** | At 100-unit volume |

> **Volume pricing:** At 500+ units, assembly cost drops to ₹600-₹700 (SMU) and ₹500-₹600 (RTU).

### 5.3 Indian EMS Providers (Recommended)

| Company | Location | Capability | Min Order | Typical Rate |
|---------|----------|------------|-----------|-------------|
| Roots EMS | Bengaluru | SMT, THT, Box-build | 50 pcs | ₹3-5/pad |
| SFO Technologies | Kochi | Full EMS, Class 3 | 100 pcs | ₹2-4/pad |
| Kaynes Technology | Mysuru | SMT, conformal coating | 50 pcs | ₹3-5/pad |
| Syrma SGS | Chennai | SMT, automated testing | 100 pcs | ₹2-4/pad |
| Elin Electronics | Ghaziabad | Budget EMS | 25 pcs | ₹2-3/pad |
| Karkhana.io | Bengaluru | Prototype-friendly | 5 pcs | ₹5-8/pad |

---

## 6. Testing & Calibration Costs

### 6.1 Production Testing (Per Board)

| Test | Solar SMU (INR) | Water RTU (INR) | Notes |
|------|-----------------|-----------------|-------|
| Visual inspection (manual) | ₹30 | ₹30 | IPC-A-610 Class 2 |
| In-Circuit Test (ICT) | ₹80 | ₹80 | Power rail, continuity |
| Firmware flashing (SWD) | ₹30 | ₹30 | ST-Link V2, production script |
| Power-on self-test (POST) | ₹50 | ₹50 | Automated via test jig |
| ADC calibration | ₹200 | ₹150 | Multi-point cal, Keithley source |
| Communication test (RS-485) | ₹40 | ₹40 | Modbus RTU loopback |
| Communication test (Ethernet) | — | ₹60 | Modbus TCP ping/register test |
| SD card test | ₹30 | — | Write/read/verify |
| I2C temperature sensor test | ₹20 | — | TMP117 read check |
| LED & indicator test | ₹15 | ₹15 | Visual verification |
| Burn-in (2-hour soak @ 50°C) | ₹100 | ₹100 | Elevated temperature aging |
| Final QC & labeling | ₹30 | ₹30 | Serial number, QC sticker |
| **Per-board testing total** | **₹625** | **₹585** | |

### 6.2 Calibration Equipment (One-Time Investment)

| Equipment | Cost (INR) | Notes |
|-----------|------------|-------|
| Keithley 2400 SourceMeter (or equiv.) | ₹8,00,000 | For ADC calibration reference |
| Fluke 8846A 6.5-digit DMM | ₹3,50,000 | Voltage/current verification |
| 4-20mA loop calibrator | ₹45,000 | Yokogawa CA310 or equivalent |
| Current shunt calibrator (DC) | ₹1,50,000 | For Solar SMU string inputs |
| RS-485 protocol analyzer | ₹35,000 | Bus monitor + Modbus master |
| Ethernet tester/analyzer | ₹25,000 | Cable test + Modbus TCP |
| ST-Link V3 programmer (x5) | ₹25,000 | Production programming stations |
| Temperature chamber (small, -20 to 85°C) | ₹3,50,000 | Environmental screening |
| Custom test jig — Solar SMU | ₹1,50,000 | Bed-of-nails, pogo pins |
| Custom test jig — Water RTU | ₹1,20,000 | Bed-of-nails, pogo pins |
| PC + test software development | ₹2,00,000 | LabVIEW/Python test automation |
| **Total calibration equipment** | **₹22,50,000** | ~$26,470 |

### 6.3 Calibration Traceability

| Item | Cost (INR/year) | Notes |
|------|-----------------|-------|
| NABL calibration of reference instruments | ₹80,000 | Annual recalibration |
| Calibration certificates (per batch) | ₹5,000 | Per production lot |
| **Annual calibration maintenance** | **₹85,000** | |

---

## 7. Enclosures, Cabinets & Outdoor Station Fabrication

### 7.1 Board-Level Enclosures (DIN-Rail Mount, IP20)

| Enclosure Type | Solar SMU (INR) | Water RTU (INR) | Notes |
|----------------|-----------------|-----------------|-------|
| ABS/PC DIN-rail enclosure | ₹350 | ₹280 | Standard sizes, snap-on |
| Polycarbonate panel mount box | ₹450 | ₹350 | Clear lid option |
| CNC-cut ventilation slots | ₹80 | ₹60 | Thermal management |
| Silkscreen printing/labeling | ₹30 | ₹30 | Company branding |
| DIN-rail clip bracket | ₹25 | ₹25 | 35mm standard |
| **Board enclosure total** | **₹435 – ₹585** | **₹345 – ₹465** | |

### 7.2 Outdoor Field Station Enclosure (IP65)

**Small IP65 Station (Wall/Pole Mount — for single RTU/SMU)**

| Item | Cost (INR) | Notes |
|------|------------|-------|
| IP65 GI/CRCA enclosure (400x300x200mm) | ₹4,500 | Powder-coated, RAL 7035 |
| Cable glands (PG9/PG11, x6) | ₹300 | IP68 nylon glands |
| DIN rail (35mm, 30cm length) | ₹80 | Mounting the PCB |
| Terminal blocks (power + signal) | ₹250 | Weidmuller/Phoenix |
| MCB/fuse holder (24V DC, 2A) | ₹180 | DIN-rail miniature circuit breaker |
| Surge protector (24V DC, DIN) | ₹1,200 | Type 2+3 SPD |
| Grounding bar & earth stud | ₹150 | Copper, M6 |
| Internal wiring (18 AWG, 5m) | ₹100 | Color-coded |
| Cable ties, channels, labels | ₹120 | Cable management |
| Desiccant/breather plug | ₹80 | Anti-condensation |
| Gasket + SS hardware | ₹150 | Stainless steel bolts, EPDM gasket |
| **Subtotal — small station** | **₹7,110** | |
| Assembly labor (2 hours) | ₹500 | Panel wiring |
| **Total small field station** | **₹7,610** | ~$90 |

**Medium IP65 Station (with solar power, pole mount)**

| Item | Cost (INR) | Notes |
|------|------------|-------|
| IP65 SS304 enclosure (500x400x250mm) | ₹12,000 | Stainless steel, outdoor-grade |
| Solar panel (20W, 12V) | ₹1,800 | Polycrystalline |
| MPPT charge controller (12V/24V, 10A) | ₹2,500 | DIN-rail mount |
| Battery (12V 7Ah SLA or LiFePO4) | ₹2,200 | Sealed lead-acid or lithium |
| DC-DC converter (12V to 24V, 1A) | ₹800 | Boost to 24V system rail |
| Pole mounting bracket (GI, 80mm dia.) | ₹1,500 | Hot-dip galvanized |
| All internal components (as above) | ₹3,000 | Glands, DIN, wiring, etc. |
| Assembly labor (4 hours) | ₹1,000 | Full panel build |
| **Total medium station** | **₹24,800** | ~$292 |

### 7.3 Painting & Surface Treatment

| Treatment | Cost (INR/unit) | Notes |
|-----------|-----------------|-------|
| Powder coating (epoxy-polyester) | ₹200 – ₹500 | Per enclosure, RAL color |
| Hot-dip galvanizing (mounting bracket) | ₹150 – ₹300 | Anti-corrosion for outdoor |
| Conformal coating on PCB (acrylic) | ₹200 – ₹400 | Spray/dip, IPC-CC-830C |
| UV-resistant paint for outdoor label | ₹50 | Nameplate protection |
| Anti-tamper seal | ₹15 | Tamper-evident sticker |
| **Typical per-unit surface treatment** | **₹615 – ₹1,265** | |

### 7.4 Outdoor Station Cost Summary

| Configuration | Cost (INR) | Cost (USD) |
|---------------|------------|------------|
| Board only (no enclosure) | N/A | N/A |
| DIN-rail enclosure (IP20, indoor) | ₹350 – ₹585 | $4 – $7 |
| Wall-mount IP65 field station | ₹7,610 | $90 |
| Pole-mount IP65 + solar power | ₹24,800 | $292 |
| Large ground-mount cabinet (600x800x300) | ₹35,000 – ₹45,000 | $412 – $530 |

---

## 8. Complete Unit Cost Summary

### 8.1 Solar SMU — Complete Unit Costs

| Cost Element | Prototype (10 pcs) | Small Batch (100 pcs) | Volume (500 pcs) | Mass (1000 pcs) |
|-------------|--------------------|-----------------------|-------------------|------------------|
| Component BOM | ₹8,800 | ₹8,450 | ₹7,200 | ₹6,400 |
| PCB fabrication | ₹1,200 | ₹550 | ₹320 | ₹250 |
| SMT assembly | ₹2,200 | ₹900 | ₹650 | ₹550 |
| Testing & calibration | ₹1,000 | ₹625 | ₹500 | ₹450 |
| DIN-rail enclosure | ₹585 | ₹450 | ₹380 | ₹350 |
| Packaging | ₹300 | ₹200 | ₹150 | ₹120 |
| **Per-unit cost** | **₹14,085** | **₹11,175** | **₹9,200** | **₹8,120** |
| **Per-unit cost (USD)** | **$166** | **$131** | **$108** | **$96** |
| **Total lot cost** | **₹1,40,850** | **₹11,17,500** | **₹46,00,000** | **₹81,20,000** |

### 8.2 Water RTU — Complete Unit Costs

| Cost Element | Prototype (10 pcs) | Small Batch (100 pcs) | Volume (500 pcs) | Mass (1000 pcs) |
|-------------|--------------------|-----------------------|-------------------|------------------|
| Component BOM | ₹5,500 | ₹5,050 | ₹4,300 | ₹3,800 |
| PCB fabrication | ₹900 | ₹400 | ₹240 | ₹190 |
| SMT assembly | ₹1,800 | ₹800 | ₹550 | ₹450 |
| Testing & calibration | ₹900 | ₹585 | ₹470 | ₹420 |
| DIN-rail enclosure | ₹465 | ₹350 | ₹300 | ₹280 |
| Packaging | ₹300 | ₹200 | ₹150 | ₹120 |
| **Per-unit cost** | **₹9,865** | **₹7,385** | **₹6,010** | **₹5,260** |
| **Per-unit cost (USD)** | **$116** | **$87** | **$71** | **$62** |
| **Total lot cost** | **₹98,650** | **₹7,38,500** | **₹30,05,000** | **₹52,60,000** |

### 8.3 NRE (One-Time) Cost Summary

| Item | Cost (INR) | Cost (USD) |
|------|------------|------------|
| PCB design (both boards) | ₹3,35,000 | $3,941 |
| Stencils (both boards) | ₹11,000 | $129 |
| Assembly NRE (both boards) | ₹65,000 | $765 |
| Test jigs (both boards) | ₹2,70,000 | $3,176 |
| Test software development | ₹2,00,000 | $2,353 |
| Calibration equipment | ₹22,50,000 | $26,471 |
| **Total NRE** | **₹33,31,000** | **$39,188** |

> The calibration equipment is a capital investment that serves ongoing production. Excluding it, the NRE is ₹10,81,000 (~$12,718).

---

## 9. Production Equipment & Infrastructure

### 9.1 In-House Manufacturing Setup (If Applicable)

| Equipment | Cost (INR) | Notes |
|-----------|------------|-------|
| **SMT Line (Basic)** | | |
| Semi-auto stencil printer | ₹8,00,000 | Manual alignment, pneumatic |
| Pick-and-place (4-head, 8000 CPH) | ₹35,00,000 | Chinese-origin (Neoden/CHMT) |
| Reflow oven (6-zone, N2 optional) | ₹12,00,000 | Lead-free capable |
| AOI machine (basic 2D) | ₹15,00,000 | Automated optical inspection |
| Soldering station (5 units) | ₹1,50,000 | Hakko/JBC for rework |
| Hot air rework station (2 units) | ₹60,000 | BGA/QFP rework |
| ESD workbenches (5 units) | ₹3,00,000 | Anti-static, with mats |
| Fume extraction system | ₹1,50,000 | HEPA + carbon filter |
| Ultrasonic cleaner | ₹80,000 | Board cleaning post-solder |
| **Subtotal — SMT Line** | **₹77,40,000** | ~$91,059 |
| | | |
| **Test & Measurement** | | |
| Oscilloscope (4-ch, 200 MHz) | ₹2,50,000 | Rigol/Siglent |
| Digital multimeters (5 units) | ₹75,000 | Fluke 87V or equivalent |
| DC power supplies (5 units) | ₹1,25,000 | 0-30V/5A programmable |
| Logic analyzer | ₹50,000 | SPI/I2C/UART debugging |
| Spectrum analyzer (basic) | ₹3,00,000 | EMC pre-compliance |
| LCR meter | ₹60,000 | Component verification |
| **Subtotal — Test Equipment** | **₹8,60,000** | ~$10,118 |
| | | |
| **Facility** | | |
| Cleanroom/ESD-safe workspace (500 sq ft) | ₹5,00,000 | Setup cost, Class 10000 |
| Compressed air system | ₹2,00,000 | For SMT equipment |
| UPS (5 kVA) | ₹1,50,000 | Power protection |
| **Subtotal — Facility** | **₹8,50,000** | ~$10,000 |
| | | |
| **Total In-House Setup** | **₹94,50,000** | **~$1,11,176** |

> **Recommendation:** For volumes under 1,000 units/year, outsource to an Indian EMS provider. In-house manufacturing is justified only above 2,000-3,000 units/year.

### 9.2 Outsourced Manufacturing (Recommended for <1000 units/year)

| Service | Cost Structure | Notes |
|---------|---------------|-------|
| PCB fabrication | Per-board pricing (see Section 4) | Indian or Chinese fab |
| Component procurement | BOM cost + 5-10% handling fee | EMS procures from authorized distributors |
| SMT assembly | Per-board (see Section 5) | Includes paste, place, reflow |
| Testing | Per-board (see Section 6) | Using customer-supplied test jigs |
| **Typical EMS turnkey cost** | BOM + 25-35% markup | Includes procurement, assembly, testing |

---

## 10. Indian Market Analysis

### 10.1 India SCADA Market Overview

| Parameter | Value | Source |
|-----------|-------|--------|
| India SCADA market CAGR (2025-2033) | 5.51% | IMARC Group |
| India's global SCADA market share | ~4.4% | Cognitive Market Research |
| India process automation market (2025) | ~$4.75 billion (₹40,375 crore) | Expert Market Research |
| Asia-Pacific SCADA market share | 32.9% of global | Multiple sources |

### 10.2 Government Drivers

| Initiative | Investment | Impact on SCADA |
|-----------|------------|-----------------|
| Revamped Distribution Sector Scheme (RDSS) | ₹3.03 lakh crore | Smart meters, feeder automation, SCADA upgrades |
| Green Energy Corridor Phase-II | ₹12,031 crore | 20 GW RE integration, substation SCADA |
| Jal Jeevan Mission (JJM) | ₹3.60 lakh crore | Water quality monitoring RTUs |
| National Solar Mission (Phase-III) | ₹19,500 crore target | Solar plant monitoring SCADA |
| Smart Cities Mission | ₹48,000 crore | Urban infrastructure monitoring |
| SAMARTH Udyog Bharat 4.0 | Government-backed | Industry 4.0, manufacturing SCADA |

### 10.3 Competitive Landscape — RTU Pricing in India

| Product Category | Price Range (INR) | Key Players |
|-----------------|-------------------|-------------|
| Basic IoT/Telemetry RTU | ₹10,000 – ₹14,000 | Indian startups, KLEON |
| Mid-range SCADA RTU (4G/RS-485) | ₹40,000 – ₹60,000 | System Level Solutions, Masibus |
| Industrial RTU (multi-protocol) | ₹1,00,000 – ₹1,80,000 | ABB, Schneider, L&T |
| Solar monitoring system | ₹1,500 – ₹5,00,000 | Meteocontrol, Logics PowerAMR |
| Water telemetry RTU (JJM-compliant) | ₹14,000 – ₹45,000 | KLEON, domestic startups |
| Full SCADA + RTU panel | ₹1,50,000 – ₹2,50,000 | L&T, Siemens India |

### 10.4 Competitive Positioning of Our Product

| Parameter | Our Solar SMU | Competing Products |
|-----------|--------------|-------------------|
| Unit cost (100 qty) | ₹11,175 | ₹40,000 – ₹1,80,000 |
| Channels | 16 strings (V+I) | 4-16 typical |
| ADC resolution | 32-bit | 12-16 bit typical |
| Communication | RS-485 Modbus | RS-485 / 4G |
| Data logging | MicroSD | Often cloud-only |
| **Price advantage** | **3-10x cheaper** | Imported/branded |

| Parameter | Our Water RTU | Competing Products |
|-----------|--------------|-------------------|
| Unit cost (100 qty) | ₹7,385 | ₹14,000 – ₹1,80,000 |
| Channels | 8 x 4-20mA | 4-8 typical |
| ADC resolution | 24-bit | 12-16 bit typical |
| Communication | RS-485 + Ethernet | RS-485 or 4G |
| Protocols | Modbus RTU + TCP | Modbus/DNP3 |
| **Price advantage** | **2-15x cheaper** | Imported/branded |

### 10.5 Target Market Segments

| Segment | TAM in India | Our Target SAM | Potential Units/Year |
|---------|-------------|----------------|---------------------|
| Solar plants (>1 MW) | 8,000+ plants | 500 plants x 10 SMU | 5,000 units |
| Water utilities (JJM) | 5 lakh+ habitations | 10,000 pump houses | 10,000 units |
| Industrial wastewater | 15,000+ CETPs | 2,000 installations | 4,000 units |
| Smart Cities (water/power) | 100 cities | 50 cities x 20 RTUs | 1,000 units |
| **Total addressable** | | | **20,000 units/year** |

### 10.6 Make in India Benefits & Import Duties

| Item | Rate/Benefit | Notes |
|------|-------------|-------|
| BCD (Basic Customs Duty) on ICs | 0-7.5% | Most semiconductor ICs at 0% |
| BCD on PCB (bare board, imported) | 10-15% | Incentive to fabricate in India |
| BCD on passive components | 7.5-10% | Resistors, capacitors |
| GST on SCADA equipment | 18% | HSN 9032 (measuring/controlling) |
| GST input credit | Full credit | On component purchases |
| PLI Scheme (IT Hardware) | 4-6% incentive | On incremental production |
| SPECS Scheme | 25% capex subsidy | On capital equipment for electronics mfg |
| M-SIPS Scheme | 20-25% subsidy | Modified Special Incentive Package |

---

## 11. Regulatory & Certification Costs

### 11.1 Type Testing & Certification

| Certification | Cost (INR) | Timeline | Mandatory? |
|--------------|------------|----------|------------|
| **EMC Testing (STQC/ERTL)** | | | |
| — Conducted emissions (EN 55032) | ₹80,000 | 2-3 weeks | Yes (for sale) |
| — Radiated emissions (EN 55032) | ₹80,000 | 2-3 weeks | Yes |
| — ESD immunity (IEC 61000-4-2) | ₹40,000 | 1-2 weeks | Recommended |
| — Surge immunity (IEC 61000-4-5) | ₹50,000 | 1-2 weeks | Recommended |
| — Conducted immunity (IEC 61000-4-6) | ₹40,000 | 1-2 weeks | Recommended |
| **Subtotal — EMC** | **₹2,90,000** | | |
| | | | |
| **Environmental Testing** | | | |
| — Temperature cycling (-20 to 85°C) | ₹60,000 | 2 weeks | Recommended |
| — Humidity test (85°C/85% RH) | ₹40,000 | 2 weeks | Recommended |
| — Vibration test (IEC 60068-2-6) | ₹50,000 | 1 week | For utility-grade |
| — IP65 ingress protection test | ₹35,000 | 1 week | If claimed |
| — Salt spray test (outdoor/coastal) | ₹30,000 | 1 week | Coastal deployment |
| **Subtotal — Environmental** | **₹2,15,000** | | |
| | | | |
| **Safety & Compliance** | | | |
| — BIS certification (CRS registration) | ₹2,50,000 | 3-6 months | Yes (electronics) |
| — CE marking (if exporting) | ₹3,00,000 | 2-3 months | For EU export |
| — IEC 61131-2 compliance (PLC) | ₹2,00,000 | 3 months | If claiming PLC |
| — UL listing (for US market) | ₹5,00,000+ | 4-6 months | For US export |
| | | | |
| **Total Type Testing & Certification** | | | |
| — Domestic only (BIS + EMC + Env) | **₹7,55,000** | | ~$8,882 |
| — Domestic + CE (export) | **₹10,55,000** | | ~$12,412 |

### 11.2 MSME Benefits

| Benefit | Savings | Notes |
|---------|---------|-------|
| STQC MSME rebate | 50% on application fees | Startup India registered |
| NSIC single-point registration | Free bid participation | Govt. tender eligibility |
| GeM registration | Market access | Government e-Marketplace |
| ZED certification | Quality recognition | MSME quality benchmark |

---

## 12. Pricing Strategy & Margins

### 12.1 Recommended Selling Price (RSP)

**Solar SMU (SS-PCB-001) — Board + DIN Enclosure:**

| Volume | Mfg Cost (INR) | Margin | RSP (INR) | RSP (USD) |
|--------|----------------|--------|-----------|-----------|
| 1-10 pcs | ₹14,085 | 60% | ₹22,500 | $265 |
| 11-100 pcs | ₹11,175 | 50% | ₹16,750 | $197 |
| 101-500 pcs | ₹9,200 | 45% | ₹13,340 | $157 |
| 500+ pcs | ₹8,120 | 40% | ₹11,370 | $134 |

**Water RTU (WR-PCB-001) — Board + DIN Enclosure:**

| Volume | Mfg Cost (INR) | Margin | RSP (INR) | RSP (USD) |
|--------|----------------|--------|-----------|-----------|
| 1-10 pcs | ₹9,865 | 60% | ₹15,800 | $186 |
| 11-100 pcs | ₹7,385 | 50% | ₹11,080 | $130 |
| 101-500 pcs | ₹6,010 | 45% | ₹8,715 | $103 |
| 500+ pcs | ₹5,260 | 40% | ₹7,365 | $87 |

**Complete Outdoor Station (Board + IP65 Enclosure + Accessories):**

| Product | RSP (INR) | RSP (USD) | Competing Price |
|---------|-----------|-----------|-----------------|
| Solar SMU Station (16-string) | ₹35,000 – ₹45,000 | $412 – $530 | ₹60,000 – ₹2,00,000 |
| Water RTU Station (8-channel) | ₹25,000 – ₹35,000 | $294 – $412 | ₹40,000 – ₹1,80,000 |

### 12.2 Annual Revenue Projections (Year 1-3)

| Year | SMU Units | RTU Units | Revenue (INR) | Revenue (USD) |
|------|-----------|-----------|---------------|---------------|
| Year 1 | 200 | 300 | ₹62 lakh | $72,940 |
| Year 2 | 500 | 800 | ₹1.5 crore | $1,76,470 |
| Year 3 | 1,500 | 2,500 | ₹4.2 crore | $4,94,118 |

### 12.3 Break-Even Analysis

| Item | Value |
|------|-------|
| Total NRE investment | ₹33.31 lakh |
| Certification costs | ₹7.55 lakh |
| **Total initial investment** | **₹40.86 lakh** (~$48,071) |
| Average margin per unit (blended) | ₹4,500 |
| **Break-even volume** | **~908 units** |
| At Year 1 production (500 units) | 91% of break-even |
| **Expected break-even** | **Month 14-16** |

---

## 13. Appendix — Key Suppliers & Sources

### 13.1 Component Distributors (India)

| Distributor | Website | Speciality | Payment |
|-------------|---------|------------|---------|
| DigiKey India | digikey.in | Full catalog, INR pricing | UPI/NEFT/Card |
| Mouser India | mouser.in | DDP shipping, INR | Card/Wire |
| Element14 India | element14.com | Fast delivery, INR | Card/Wire |
| Semikart | semikart.com | Indian distributor, INR | NEFT/Card |
| Robu.in | robu.in | Maker components | UPI/Card |
| IndiaMART | indiamart.com | Bulk/wholesale ICs | Negotiated |

### 13.2 PCB Fabricators (India)

| Company | Location | Layers | Lead Time |
|---------|----------|--------|-----------|
| PCB Power | Ahmedabad | 1-16L | 3-7 days |
| LionCircuits | Bengaluru | 1-8L | 5-7 days |
| CSIL | Gandhinagar | 1-12L | 5-10 days |
| Shogini | Pune | 1-8L | 7-14 days |
| Epitome Components | Delhi | 1-6L | 5-10 days |
| JLCPCB (import) | China | 1-20L | 7-15 days (with shipping) |

### 13.3 EMS / Assembly Partners

| Company | Location | Capability |
|---------|----------|------------|
| Roots EMS | Bengaluru | Full SMT, box-build |
| Kaynes Technology | Mysuru | SMT, conformal coat, certified |
| SFO Technologies | Kochi | Class 3, defense-grade |
| Syrma SGS | Chennai | High-volume SMT |
| Elin Electronics | Ghaziabad | Cost-effective assembly |
| Karkhana.io | Bengaluru | Prototype-friendly, small batch |

### 13.4 Enclosure & Cabinet Vendors

| Company | Type | Location |
|---------|------|----------|
| RackOm | IP65 outdoor cabinets | Faridabad |
| DNA Technology | ABS IP65 enclosures | Nashik |
| Kew Electricals | Polycarbonate IP65 | Mumbai |
| Cabinet India | Custom CRCA/SS panels | Multiple |
| Rittal India | Premium industrial enclosures | Bengaluru |

### 13.5 Testing Laboratories (NABL-Accredited)

| Lab | Location | Services |
|-----|----------|----------|
| STQC ERTL (North) | New Delhi | EMC, Safety, Environmental |
| STQC ETDC | Bengaluru | EMC, Telecom, Safety |
| STQC ERTL (South) | Thiruvananthapuram | EMC, Safety |
| EMTAC | Bengaluru | EMC testing |
| TUV SUD | Multiple | CE, UL, Safety |
| Bureau Veritas | Multiple | Product certification |

---

## Cost Summary Dashboard

```
┌──────────────────────────────────────────────────────────────────┐
│              SCADA System — Cost at a Glance (100 units)         │
├─────────────────────────┬──────────────────┬─────────────────────┤
│                         │   Solar SMU      │   Water RTU         │
├─────────────────────────┼──────────────────┼─────────────────────┤
│ Component BOM           │   ₹8,450         │   ₹5,050            │
│ PCB Fabrication         │   ₹550           │   ₹400              │
│ SMT Assembly            │   ₹900           │   ₹800              │
│ Testing & Calibration   │   ₹625           │   ₹585              │
│ Enclosure (DIN, IP20)   │   ₹450           │   ₹350              │
│ Packaging               │   ₹200           │   ₹200              │
├─────────────────────────┼──────────────────┼─────────────────────┤
│ PER-UNIT COST           │   ₹11,175        │   ₹7,385            │
│ PER-UNIT COST (USD)     │   $131           │   $87               │
├─────────────────────────┼──────────────────┼─────────────────────┤
│ RSP (50% margin)        │   ₹16,750        │   ₹11,080           │
│ RSP (USD)               │   $197           │   $130              │
├─────────────────────────┼──────────────────┼─────────────────────┤
│ COMPETING PRODUCTS      │   ₹60K – ₹2L     │   ₹40K – ₹1.8L     │
│ PRICE ADVANTAGE         │   3-10x cheaper  │   2-15x cheaper     │
└─────────────────────────┴──────────────────┴─────────────────────┘

One-Time NRE: ₹33.31 lakh ($39,188)
Certification: ₹7.55 lakh ($8,882)
Break-Even:    ~908 units (Month 14-16)
India SCADA Market CAGR: 5.51% (2025-2033)
```

---

*Report prepared based on market research from DigiKey India, Mouser India, IndiaMART, TradeIndia, IMARC Group, STQC, and industry sources. All prices are estimates as of March 2026 and subject to change based on market conditions, exchange rates, and volume negotiations.*

*Sources: [DigiKey India](https://www.digikey.in), [Mouser India](https://www.mouser.in), [IndiaMART](https://www.indiamart.com), [PCB Power](https://www.pcbpower.com), [STQC](https://www.stqc.gov.in), [IMARC Group](https://www.imarcgroup.com/india-scada-market), [Fortune Business Insights](https://www.fortunebusinessinsights.com/scada-market-102433)*
