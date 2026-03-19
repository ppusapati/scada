# STM32H743 DataLogger - PCB Assembly Notes (Manufacturing)

**PCB File:** `STM32H743_DataLogger_mfg.kicad_pcb`
**Revision:** 1.0 MFG
**Date:** 2026-03-19
**IPC Class:** 2 (Standard Electronic Product)

---

## 1. Board Fabrication Specifications

| Parameter | Value |
|-----------|-------|
| Dimensions | 160.0 x 100.0 mm (±0.1mm) |
| Layers | 4 |
| Total Thickness | 1.6mm (±0.13mm) |
| Material | FR-4 TG170 (Tg ≥ 170°C) |
| Copper Weight | 1 oz (35μm) all layers |
| Surface Finish | ENIG (3-5μin Au / 120-240μin Ni) |
| Solder Mask | Green LPI, both sides |
| Silkscreen | White, both sides |
| Min Track Width | 0.15mm (6mil) |
| Min Track Spacing | 0.15mm (6mil) |
| Min Via Pad/Drill | 0.5mm/0.3mm |
| Edge Clearance | 0.3mm min copper to board edge |
| Controlled Impedance | 50Ω single-ended (Ethernet, USB) |
| | 100Ω differential (Ethernet pairs) |
| V-Score/Tab | None (individual boards) |
| Panelization | As required by assembler |

## 2. Layer Stackup

```
Layer 1 (F.Cu)   : Signal + SMD Components    35μm Cu
Prepreg 1        : FR-4 2116                   0.200mm (εr=4.5)
Layer 2 (In1.Cu) : Solid GND Plane            35μm Cu
Core             : FR-4 1080x2                 0.800mm (εr=4.5)
Layer 3 (In2.Cu) : Split Power (+3V3/+5V)     35μm Cu
Prepreg 2        : FR-4 2116                   0.200mm (εr=4.5)
Layer 4 (B.Cu)   : Signal + SMD Components    35μm Cu
                                               ─────────
Total                                          ~1.600mm
```

## 3. Impedance Control Requirements

| Signal Type | Target Z | Tolerance | Trace Width | Reference Layer |
|-------------|----------|-----------|-------------|-----------------|
| Ethernet TX/RX (single) | 50Ω | ±10% | 0.28mm | In1.Cu (GND) |
| Ethernet TX/RX (diff pair) | 100Ω | ±10% | 0.20mm/0.15mm gap | In1.Cu (GND) |
| USB D+/D- (diff pair) | 90Ω | ±10% | 0.20mm/0.12mm gap | In1.Cu (GND) |
| General SPI signals | — | — | 0.20mm | — |

## 4. Assembly Process

### 4.1 Solder Paste
- **Alloy:** SAC305 (Sn96.5/Ag3.0/Cu0.5) - Lead-free RoHS
- **Stencil:** 0.12mm thickness (100μm for fine-pitch QFP/QFN)
- **Aperture reduction:** 10% on QFN-48 (U50 W5500), QFN-28 (U60 SX1276)
- **Aperture modifications:** Window pane on LQFP-100 (U10 STM32H743) EP if any

### 4.2 Reflow Profile (Lead-free)
| Zone | Temperature | Time |
|------|-------------|------|
| Preheat | 150-200°C | 60-120s |
| Soak | 200-217°C | 60-120s |
| Reflow | 235-250°C peak | 30-60s (>217°C) |
| Peak | 245°C max | 10s |
| Cooldown | <6°C/sec | — |

### 4.3 Assembly Sequence
1. **SMD Top Side (F.Cu):** Stencil print → Place SMD → Reflow solder
2. **SMD Bottom Side (B.Cu):** Flip → Stencil print → Place SMD → Reflow solder
3. **Through-Hole:** Wave solder or hand solder remaining THT components
4. **Manual Assembly:** Relay K1, through-hole terminal blocks, antenna connectors
5. **Inspection:** AOI (Automated Optical Inspection) both sides
6. **Testing:** ICT/functional test per test plan

### 4.4 Through-Hole Components (Manual/Wave)
| Reference | Description | Notes |
|-----------|-------------|-------|
| K1 | HF46F-G relay (SPDT) | Check orientation, 5V coil |
| J1 | 24VDC power terminal (2-pin) | Phoenix Contact 5.08mm |
| J20-J23 | Analog input terminals (2-pin x4) | Phoenix Contact 5.08mm |
| J30-J37 | Digital input terminals (2-pin x8) | Phoenix Contact 5.08mm |
| J40-J43 | Relay output terminals (3-pin x4) | Phoenix Contact 5.08mm |
| J50 | RS485 terminal (3-pin) | Phoenix Contact 5.08mm |
| J51 | CAN terminal (3-pin) | Phoenix Contact 5.08mm |
| J12 | UART debug header (1x4 2.54mm) | Optional, for development |
| J13 | BOOT0 jumper (1x3 2.54mm) | Optional, for development |
| J60 | RJ45 Ethernet jack | Pulse J0011D21BNL with magnetics |
| J70, J73, J74 | SMA antenna connectors (x3) | Edge-mount, LoRa + LTE |
| H1-H6 | M3 mounting hardware | Install after conformal coating |

## 5. Critical Component Notes

### 5.1 MCU - U10 (STM32H743VIT6)
- LQFP-100, 14x14mm, 0.5mm pitch
- **Pin 1 orientation:** Top-left corner (check silkscreen dot)
- Thermal pad: Connect to GND plane via thermal vias
- All VDD/VSS pins must be connected per ST AN4938

### 5.2 QFN Components
- **U50 (W5500):** QFN-48, 7x7mm, 0.5mm pitch, exposed pad 5.6x5.6mm
  - Exposed pad = GND, use 9-via thermal array
  - Paste aperture: windowed (4x4 pattern, 50% coverage)
- **U60 (SX1276):** QFN-28, 4x4mm, 0.4mm pitch, exposed pad 2.4x2.4mm
  - Exposed pad = GND, 4 thermal vias minimum
  - RF section: maintain 50Ω impedance to SMA connector
  - Keep ground plane solid under RF matching network

### 5.3 Wireless Modules
- **U61 (ATWINC1500):** Module, reflow compatible
  - Keep GND plane solid under module footprint
  - No copper on any layer within 5mm of antenna area
- **U62B (RN4870):** BLE module with built-in antenna
  - Keep GND plane solid under module
  - No copper within 3mm of antenna side
- **U62 (SIM7600G-H):** GSM/LTE module (bottom side)
  - Large thermal pad - ensure good solder connection
  - 100μF bulk caps (C94, C95) must be within 5mm

### 5.4 Crystals
- **Y1 (25MHz HSE):** Place within 10mm of MCU PH0/PH1 pins
  - Guard ring GND on all layers around crystal
  - No routing under crystal footprint
- **Y2 (32.768kHz LSE):** Place within 5mm of MCU PC14/PC15 pins
  - Minimize trace length, guard with GND
- **Y3 (25MHz W5500):** Place within 10mm of W5500 XTLI/XTLO
- **Y4 (32MHz TCXO SX1276):** Place within 5mm of SX1276

### 5.5 Isolated Interfaces
- **U40 (ISO3082 RS485):** Maintain isolation gap >0.4mm on PCB
  - Separate GND zones: digital GND (pin side) vs bus GND (bus side)
- **U41 (ISO1042 CAN):** Same isolation requirements as RS485
  - Route bus-side traces away from digital traces

## 6. Fiducial Markers

| ID | Location | Purpose |
|----|----------|---------|
| FID1 | (3, 3) | Global fiducial - top-left |
| FID2 | (157, 3) | Global fiducial - top-right |
| FID3 | (3, 97) | Global fiducial - bottom-left |

- 1.0mm diameter copper circle, 2.54mm solder mask opening
- Used by pick-and-place machine for board alignment

## 7. Test Points

| ID | Net | Location | Purpose |
|----|-----|----------|---------|
| TP1 | +3V3 | (45, 30) | Digital 3.3V rail |
| TP2 | +5V | (45, 25) | 5V buck output |
| TP3 | +3V3A | (45, 40) | Analog 3.3V rail |
| TP4 | GND | (48, 30) | Ground reference |
| TP5 | SWDIO | (108, 10) | SWD debug data |
| TP6 | SWCLK | (110, 10) | SWD debug clock |
| TP7 | NRST | (112, 10) | MCU reset |
| TP8 | BOOT0 | (114, 10) | Boot mode select |

## 8. Conformal Coating

- **Material:** HumiSeal 1B73 (Acrylic)
- **Standard:** MIL-I-46058C / IPC-CC-830B
- **Application:** Spray or selective coating
- **Thickness:** 25-75μm (1-3 mil)
- **Cure:** Air dry 30 min @ 25°C, or 10 min @ 60°C

### Do NOT coat:
- Connectors (J1, J10-J14, J20-J74, J80)
- Test points (TP1-TP8)
- Mounting holes (H1-H6)
- SMA/U.FL antenna connectors
- SIM card holder (J72)
- microSD slot (J80)
- Relay contacts (K1)
- LED lenses (LED1-LED4)

## 9. Electrical Test Requirements

### 9.1 Power-On Sequence Test
1. Apply 24VDC (verify 9-36V range)
2. Measure +5V rail: 5.0V ±2%
3. Measure +3V3 rail: 3.3V ±2%
4. Measure +3V3A rail: 3.3V ±2%
5. Verify current draw: <200mA idle (no comms)

### 9.2 Functional Test
1. SWD connection (via J10 or J14)
2. Flash firmware via SWD
3. Verify UART debug console (J12) at 115200 baud
4. Verify Ethernet link (J60)
5. Verify SD card detect
6. Verify watchdog kick (PB8 toggle)
7. Verify all SPI CS lines toggle

## 10. Packaging & Handling

- ESD-sensitive components present (STM32, W5500, SX1276, ATWINC1500)
- Store and ship in ESD-safe packaging
- Moisture sensitivity: MSL-3 for QFN packages (bake if exposed >168hrs)
- Handle by board edges only
