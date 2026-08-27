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
| Min Track Spacing | 0.125mm (5mil) |
| Min Via Pad/Drill | 0.45mm/0.25mm |
| Edge Clearance | 0.3mm min copper to board edge |
| Controlled Impedance | 50Ω single-ended (Ethernet, USB) |
| | 100Ω differential (Ethernet pairs) |
| Milled Slots | 4 isolation slots, 1.2mm wide, under U40-U43 (see 5.5) |
| V-Score/Tab | None (individual boards) |
| Panelization | As required by assembler |

## 2. Layer Stackup

```
Layer 1 (F.Cu)   : Signal + SMD Components    35μm Cu
Prepreg 1        : FR-4 2116                   0.200mm (εr=4.5)
Layer 2 (In1.Cu) : Solid GND Plane            35μm Cu
Core             : FR-4 1080x2                 0.800mm (εr=4.5)
Layer 3 (In2.Cu) : Signal + +3V3 power fill   35μm Cu
Prepreg 2        : FR-4 2116                   0.200mm (εr=4.5)
Layer 4 (B.Cu)   : Signal + SMD Components    35μm Cu
                                               ─────────
Total                                          ~1.600mm
```

Routing uses **F.Cu, In2.Cu and B.Cu**. In1.Cu is kept as an unbroken ground plane —
no tracks on it — so the outer layers always have a solid reference beneath them.
In2.Cu carries signal routing with the +3V3 pour filling the space around it, rather
than being a solid plane; the LQFP-100 escape needs the third routing layer. Note
that this makes +3V3 distribution a fill rather than a plane, so check the drop to
the heavier 3V3 loads if the load budget changes.

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

The isolation barrier runs horizontally at **y = 22mm** through U40-U43. Everything
above it is field side; everything below is logic side.

- **Milled slots** run under each package that crosses the barrier. Do not fill,
  tent, or bridge them:

| Package | Slot (x range, y 21.4-22.6) |
|---------|------------------------------|
| U40 ISO3082 (RS485) | 11.5 - 21.5 |
| U42 NXJ1S (RS485 isolated 5V) | 23.0 - 32.6 |
| U41 ISO1042 (CAN) | 37.5 - 47.5 |
| U43 NXJ1S (CAN isolated 5V) | 49.0 - 58.6 |

- **Copper pours.** GND and +3V3 are notched out of the whole corner x 0-63,
  y 0-26. Inside that notch the RS485 field side has its own floating
  RS485_ISO_GND pour (x 8-34, y 1-19) and CAN has CAN_ISO_GND (x 38-59, y 1-19),
  on F.Cu and B.Cu only - the inner layers carry no copper in the notch at all.
- **Clearances achieved:** 8mm from each field pour to main copper on the outer
  side, 7mm below, and 4mm between the RS485 and CAN domains. Verify these against
  the ISO3082/ISO1042 and NXJ1S datasheets for your target creepage class before
  release.
- **U42/U43 are not optional.** Without them RS485_ISO_VCC and CAN_ISO_VCC are
  undriven and neither transceiver's bus side will power up.
- **Digital inputs** are isolated by the TLP293 optocouplers only. Their field
  return is DI_COM, which is *not* logic ground - do not strap them together. A
  copper keepout on all four layers (x 64-111, y 19.3-20.7) stops any layer
  bridging the optocoupler row. Working voltage is 24V, so no slot is used here.

## 6. Fiducial Markers

| ID | Location | Purpose |
|----|----------|---------|
| FID1 | (4.5, 60.0) | Global fiducial - left edge |
| FID2 | (145.0, 97.0) | Global fiducial - bottom-right |
| FID3 | (10.5, 97.0) | Global fiducial - bottom-left |

- 1.0mm diameter copper circle, 2.54mm solder mask opening
- Used by pick-and-place machine for board alignment

## 7. Test Points

| ID | Net | Location |
|----|-----|----------|
| TP1 | +3V3 | (71.1, 58.2) |
| TP2 | +5V | (73.4, 58.2) |
| TP3 | +3V3A | (75.7, 58.2) |
| TP4 | GND | (78.0, 58.2) |
| TP5 | SWDIO | (58.2, 49.4) |
| TP6 | SWCLK | (31.1, 53.7) |
| TP7 | NRST | (28.2, 30.8) |
| TP8 | BOOT0 | (127.5, 75.3) |
| TP9 | GSM_RI | (99.5, 47.0) |
| TP10 | LORA_DIO1 | (129.0, 69.5) |

TP9 and TP10 exist because the MCU has no spare pin: GSM_RI and LORA_DIO1 have
nowhere else to go, so they are brought out as probe pads.

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
