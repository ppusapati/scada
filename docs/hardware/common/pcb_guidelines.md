# PCB Design and Manufacturing Guidelines

## Common PCB Design Rules for SS-PCB-001 and WR-PCB-001

| Field            | Value                          |
|------------------|--------------------------------|
| Document ID      | COM-PCB-001                    |
| Applicable Boards| SS-PCB-001 (Solar SMU), WR-PCB-001 (Water RTU) |
| Revision         | 1.0                            |
| Date             | 2026-02-28                     |
| Status           | Preliminary                    |

---

## 1. Board Dimensions

| Parameter        | Solar SMU (SS-PCB-001) | Water RTU (WR-PCB-001) |
|------------------|------------------------|------------------------|
| Width            | 160 mm                 | 120 mm                 |
| Height           | 100 mm                 | 80 mm                  |
| Board Area       | 160 cm^2               | 96 cm^2                |
| Shape            | Rectangular            | Rectangular            |
| Corner Radius    | 2 mm                   | 2 mm                   |
| Mounting         | DIN rail (35mm clip)   | DIN rail (35mm clip)   |
| Board Thickness  | 1.6 mm (+/- 10%)       | 1.6 mm (+/- 10%)       |

---

## 2. Layer Stackup

Both boards use a 4-layer stackup. The layer order and copper weights are defined below.

### 2.1 Layer Definitions

```
   Layer    Name     Function                  Copper Weight
   ═════    ═════    ═══════════════════════    ═════════════
   L1       TOP      Signal + component pads   1 oz (35 um)
   L2       GND      Ground plane (continuous) 0.5 oz (17.5 um)
   L3       PWR      Power plane (split)       0.5 oz (17.5 um)
   L4       BOT      Signal + component pads   1 oz (35 um)
```

### 2.2 Stackup Cross-Section

```
         1 oz Cu ─── L1: TOP (Signal) ───────────────── 35 um
                     Prepreg (FR-4)                      ~0.2 mm
       0.5 oz Cu ─── L2: GND (Ground plane) ──────────── 17.5 um
                     Core (FR-4)                         ~0.8 mm
       0.5 oz Cu ─── L3: PWR (Power plane) ──────────── 17.5 um
                     Prepreg (FR-4)                      ~0.2 mm
         1 oz Cu ─── L4: BOT (Signal) ───────────────── 35 um

         Total nominal thickness: 1.6 mm
```

### 2.3 Layer Assignments

| Layer | Primary Use                                        | Routing Guidelines                |
|-------|----------------------------------------------------|-----------------------------------|
| L1    | Component placement, high-speed signal routing     | SPI traces, crystal traces, MCU breakout |
| L2    | Solid ground plane                                 | NO routing on this layer (keep continuous) |
| L3    | Power distribution (+5V, +3.3V, +24V zones)       | Minimal routing, wide fills only  |
| L4    | Secondary component placement, low-speed routing   | RS-485 traces, LED traces, mechanical |

**Critical Rule:** Layer 2 (GND) must remain as continuous and unbroken as possible. This layer serves as the return current path for all high-speed signals on L1 and L4. Any breaks or slots in the ground plane will force return currents to detour, creating loop antennas and increasing EMI.

---

## 3. Impedance Control

### 3.1 Single-Ended Impedance (50 ohm)

For SPI bus traces (SCK, MOSI, MISO, CS) between the MCU and ADC, and between the MCU and W5500/SD card.

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Target Impedance     | 50 ohm (+/- 10%)              |
| Trace Width (L1/L4)  | ~0.28 mm (calculate per fab stackup) |
| Reference Layer      | L2 (GND)                      |
| Dielectric Thickness | ~0.2 mm (prepreg L1-L2)       |
| Dielectric Constant  | Er = 4.2-4.6 (FR-4)           |

### 3.2 Differential Impedance (100 ohm)

For RS-485 differential pairs (A/B lines from SP3485 to J3) and Ethernet differential pairs (TX+/TX-, RX+/RX- from W5500 to RJ45).

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Target Impedance     | 100 ohm differential (+/- 10%)|
| Trace Width          | ~0.2 mm per trace              |
| Pair Spacing (edge)  | ~0.2 mm between traces         |
| Reference Layer      | L2 (GND)                      |
| Coupling             | Edge-coupled microstrip        |

**Note:** Request impedance-controlled fabrication from the PCB manufacturer. Provide the target impedances and allow the fab to adjust trace widths based on their actual prepreg and core thicknesses. Request an impedance test coupon on the panel.

---

## 4. Power Plane Design

### 4.1 L3 Power Plane Zones

The L3 (PWR) layer is divided into zones for different power rails:

```
    ┌────────────────────────────────────────────────────┐
    │                                                    │
    │   ┌──────────────────┐  ┌────────────────────────┐│
    │   │                  │  │                        ││
    │   │   +5V Zone       │  │   +3.3V Zone           ││
    │   │                  │  │                        ││
    │   │  (ADC analog,    │  │  (MCU, digital logic,  ││
    │   │   transceivers)  │  │   IOVDD, pull-ups)     ││
    │   │                  │  │                        ││
    │   └──────────────────┘  └────────────────────────┘│
    │                                                    │
    │   ┌──────────────────┐                             │
    │   │   +24V (trace)   │   (no fill -- just wide     │
    │   │   Input only     │    traces for power input)  │
    │   └──────────────────┘                             │
    │                                                    │
    └────────────────────────────────────────────────────┘
```

### 4.2 Power Plane Separation: Analog vs. Digital

The +5V zone must be further divided between analog 5V (AVDD for the ADC) and digital 5V (transceivers, other logic):

- Use a ferrite bead (FB2) between the main +5V rail and the ADC AVDD supply.
- On L3, create a separate copper pour for AVDD that is connected to the +5V zone only through the ferrite bead footprint.
- The ADC AVDD pour should be directly beneath the ADC footprint on L1.

---

## 5. Ground Plane Strategy

### 5.1 Analog Ground (AGND)

The analog ground (AGND) is used for all precision analog circuitry: the ADC (ADS1258/ADS1263), input conditioning circuits, shunt resistors, multiplexers (Solar SMU), and reference voltage decoupling.

**Star Connection Strategy:**

AGND and DGND are NOT separate copper pours on L2. Instead, L2 is a single continuous ground plane. The separation is achieved through careful component placement and routing:

1. Group all analog components (ADC, input conditioning, reference circuits) in one area of the PCB.
2. Group all digital components (MCU, transceivers, Ethernet, LEDs) in a separate area.
3. Route return currents so that analog currents flow through the analog section of the ground plane and digital currents flow through the digital section.
4. The AGND and DGND symbols in the schematic are connected at a single point on the PCB, located directly beneath the ADC. This is the "star point."

```
                        GROUND PLANE TOPOLOGY (L2)
    ┌────────────────────────────────────────────────────┐
    │                                                    │
    │   ANALOG SECTION              DIGITAL SECTION      │
    │   ┌─────────────┐            ┌──────────────────┐ │
    │   │ ADC         │            │ MCU              │ │
    │   │ Input cond. │            │ Transceivers     │ │
    │   │ Mux (SMU)   │            │ W5500 (RTU)      │ │
    │   │ Shunts      │            │ LEDs             │ │
    │   │        ★ ←──┼── Star ────┼─→               │ │
    │   │  (AGND)     │   Point    │    (DGND)        │ │
    │   └─────────────┘            └──────────────────┘ │
    │                                                    │
    │          CONTINUOUS GROUND PLANE (L2)               │
    │          (no slots, no splits)                      │
    └────────────────────────────────────────────────────┘
```

### 5.2 Ground Plane Rules

1. Do NOT split the ground plane with a physical slot or gap.
2. Minimize vias that cross from L1 to L4 in the analog section (each via creates a small discontinuity in the ground plane).
3. Place ground vias near every decoupling capacitor.
4. Surround the crystal oscillator with a ground via fence (see Section 8).
5. The switching power supply section should have a dedicated ground copper pour on L1 that connects to L2 via multiple vias.

---

## 6. Decoupling Capacitor Placement Rules

### 6.1 General Rules

1. Place 100nF ceramic capacitors as close as possible to each VDD pin of every IC, with the capacitor ground pad connected to the nearest ground via.
2. The trace from the VDD pin to the capacitor pad should be less than 3 mm.
3. Use 0402 or 0603 package size for 100nF decoupling capacitors to minimize trace length.
4. Bulk capacitors (4.7uF, 10uF) may be placed slightly farther away (within 10 mm) as they handle lower-frequency transients.
5. Connect the capacitor ground pad to L2 (GND) via a via placed directly at the capacitor ground pad (within 0.5 mm).

### 6.2 Placement Priority Order

For the STM32F407 and other multi-pin ICs, place decoupling capacitors in this priority order:

1. **VDDA / VSSA** -- most noise-sensitive, place first and closest
2. **VDD pin nearest to the oscillator** -- clock domain sensitive
3. **Remaining VDD pins** -- one 100nF per pin
4. **Bulk capacitor (4.7uF)** -- placed near pin 11 or any convenient VDD pin
5. **VBAT** -- 100nF, close to VBAT pin

### 6.3 Capacitor Via Strategy

```
     IC VDD Pin
         |
     [short trace < 3mm]
         |
    ┌────+────┐
    │  100nF  │  Capacitor
    └────+────┘
         |
     [via to L2] ← Place via directly at capacitor GND pad
         |
    ═══ L2: GND Plane ═══
```

---

## 7. SPI Trace Routing

### 7.1 SPI1 (MCU to ADC)

SPI1 connects the STM32F407 to the ADC (ADS1258 on Water RTU, ADS1263 on Solar SMU) and carries precision analog data.

| Signal    | STM32 Pin | Series Resistor | Notes                        |
|-----------|-----------|-----------------|------------------------------|
| SPI1_SCK  | PA5       | 33 ohm          | Clock, route first           |
| SPI1_MISO | PA6       | 33 ohm          | Data from ADC                |
| SPI1_MOSI | PA7       | 33 ohm          | Data to ADC                  |
| SPI1_CS   | PA4       | 33 ohm          | Chip select (active low)     |

**Routing Rules:**

1. **Series resistors (33 ohm):** Place as close as possible to the MCU side of each SPI trace (within 5 mm of the STM32 pin). These resistors reduce overshoot and ringing caused by impedance mismatches and trace stubs.
2. **Matched length:** Match all SPI1 traces (SCK, MISO, MOSI, CS) to within +/- 5 mm of each other. The clock frequency is 4-10 MHz (period 100-250 ns), so length matching prevents clock-data skew.
3. **Route as a group:** Keep all four SPI1 traces together in a parallel bundle with 0.2-0.3 mm spacing between traces.
4. **Reference plane:** Route on L1 over continuous L2 ground plane. Do not route across ground plane splits.
5. **Maximum trace length:** 80 mm (MCU to ADC).
6. **No vias** between MCU and ADC if possible. If a via is necessary, add a ground via adjacent to it.

### 7.2 SPI2 (MCU to W5500, Water RTU Only)

| Signal    | STM32 Pin | Series Resistor | Notes                        |
|-----------|-----------|-----------------|------------------------------|
| SPI2_SCK  | PB13      | 33 ohm          | Clock                        |
| SPI2_MISO | PB14      | 33 ohm          | Data from W5500              |
| SPI2_MOSI | PB15      | 33 ohm          | Data to W5500                |
| SPI2_CS   | PB12      | 33 ohm          | Chip select                  |

Same routing rules as SPI1. SPI2 runs at up to 21 MHz, making series resistors and impedance control more critical.

### 7.3 SPI3 (MCU to MicroSD, Solar SMU Only)

| Signal    | STM32 Pin | Series Resistor | Notes                        |
|-----------|-----------|-----------------|------------------------------|
| SPI3_SCK  | PB3       | 33 ohm          | Clock                        |
| SPI3_MISO | PB4       | 33 ohm (opt.)   | Data from SD card            |
| SPI3_MOSI | PB5       | 33 ohm          | Data to SD card              |
| SD_CS     | PA15      | --              | Chip select (GPIO)           |

SD card SPI operates up to 21 MHz in data transfer mode. Follow the same routing rules as SPI1/SPI2.

---

## 8. Ethernet Routing Rules (Water RTU Only)

### 8.1 Differential Pair Routing (TX and RX)

The Ethernet differential pairs (TXP/TXN and RXP/RXN) between the W5500 and the HR911105A RJ45 connector must be routed as controlled-impedance differential pairs.

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Differential Impedance| 100 ohm (+/- 10%)             |
| Trace Width          | Per fab stackup calculation    |
| Pair Spacing          | Per fab stackup calculation   |
| Series Resistors     | 49.9 ohm on each TX trace      |
| Coupling Style       | Edge-coupled microstrip (L1)   |

**Routing Rules:**

1. **Pair symmetry:** Route TXP and TXN as a tightly coupled differential pair with equal lengths. Same for RXP/RXN.
2. **Length matching:** Within each pair, match trace lengths to within 0.5 mm.
3. **Series resistors:** Place 49.9 ohm resistors as close as possible to the W5500 TXP/TXN pins (within 5 mm).
4. **Keep short:** Total differential pair length should be less than 30 mm from W5500 to RJ45 connector.
5. **No vias:** Route entirely on L1 if possible.
6. **Clearance:** Maintain at least 0.5 mm clearance between TX and RX differential pairs.
7. **Ground plane:** Ensure continuous L2 ground plane underneath the entire Ethernet routing path.
8. **No crossing:** TX and RX pairs should not cross each other.

### 8.2 W5500 Crystal (Y3, 25 MHz)

- Place Y3 within 5 mm of the W5500 XI/XO pins.
- Route crystal traces as short as possible (< 5 mm each).
- Surround the crystal with a ground guard ring on L1 connected to L2 via ground vias.
- Place load capacitors (22 pF) between the crystal and ground, as close to the crystal pads as possible.

---

## 9. Thermal Relief on Power Pads

All pads connected to copper fills or planes must use thermal relief connections to allow proper soldering (both hand and reflow).

### 9.1 Thermal Relief Pattern

```
          ┌─────────────────────┐
          │    Copper fill      │
          │         │           │
          │    ─────+─────     │    4-spoke thermal relief
          │         │           │    Spoke width: 0.3 mm
          │     ┌───+───┐      │    Gap width: 0.25 mm
          │     │  PAD  │      │    Anti-pad: pad + 0.5mm each side
          │     └───+───┘      │
          │         │           │
          │    ─────+─────     │
          │         │           │
          └─────────────────────┘
```

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Spoke count          | 4                              |
| Spoke width          | 0.3 mm (minimum)               |
| Gap width            | 0.25 mm                        |

### 9.2 Exceptions (Direct Connect)

The following pads should use direct connection (NO thermal relief) for maximum thermal and electrical conductivity:

- LM2596 GND tab (TO-263 tab, pin 3) -- requires maximum heat sinking
- AMS1117 VOUT tab (SOT-223 tab, pin 2) -- thermal path for dissipation
- Power inductor L1 pads -- carry high switching current
- Schottky diode D2 pads -- carry high switching current

---

## 10. Via Specifications

### 10.1 Standard Signal Vias

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Drill Diameter       | 0.3 mm                         |
| Pad Diameter         | 0.6 mm                         |
| Annular Ring         | 0.15 mm (minimum)              |
| Plating              | Electroplated copper, >= 20 um |
| Aspect Ratio         | 5.3:1 (1.6mm / 0.3mm)         |
| Type                 | Through-hole, plated           |

### 10.2 Power Vias

For connections between power planes (L3) and component pads (L1/L4), use multiple vias in parallel to reduce impedance:

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Drill Diameter       | 0.3 mm (same as signal)        |
| Pad Diameter         | 0.6 mm                         |
| Count                | 2-4 vias per power pad         |
| Spacing              | 1.0 mm center-to-center        |

### 10.3 Ground Stitching Vias

Place ground stitching vias around the board perimeter and throughout the ground plane to connect L2 (GND) to L4 ground copper:

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Spacing              | 5-10 mm along board edge       |
| Spacing (general)    | 10-15 mm grid across board     |
| Purpose              | Reduce ground plane inductance, improve shielding |

---

## 11. Solder Mask and Silkscreen

### 11.1 Solder Mask

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Color                | Green (standard)               |
| Type                 | LPI (Liquid Photo-Imageable)   |
| Finish               | Matte or glossy                |
| Solder Mask Expansion| 0.05 mm per side (pad exposure)|
| Minimum Dam Width    | 0.1 mm between pads            |
| Solder Mask over Vias| Tented (both sides) for signal vias |
|                      | Open for power and thermal vias|

### 11.2 Silkscreen

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Color                | White                          |
| Minimum Line Width   | 0.15 mm                        |
| Minimum Text Height  | 0.8 mm                         |
| Font                 | Sans-serif, proportional        |
| Content Required     | Reference designators, connector labels, pin 1 markers, polarity marks, board name, revision, date |

**Silkscreen Content Checklist:**

- [ ] Board name (e.g., "WR-PCB-001 Rev 1.0")
- [ ] Date code location
- [ ] All connector labels with pin numbers (J1: +24V/GND, J3: A/B/GND, etc.)
- [ ] Polarity marks on all polarized components (electrolytic caps, diodes, LEDs)
- [ ] Pin 1 dot on all ICs
- [ ] Reference designators for all components (do not overlap pads)
- [ ] Test point labels
- [ ] LED function labels (POWER, FAULT, COMM, etc.)
- [ ] Company logo (if applicable)
- [ ] Regulatory marks area

---

## 12. DFM (Design for Manufacturability) Notes

### 12.1 Minimum Feature Sizes

| Feature              | Minimum Value   | Preferred Value |
|----------------------|-----------------|-----------------|
| Trace Width          | 0.15 mm         | 0.2 mm          |
| Trace Spacing        | 0.15 mm         | 0.2 mm          |
| Pad-to-Pad Clearance | 0.15 mm        | 0.2 mm          |
| Pad-to-Trace Clearance| 0.15 mm       | 0.2 mm          |
| Drill-to-Drill       | 0.2 mm edge     | 0.3 mm edge     |
| Drill-to-Board Edge  | 0.3 mm edge     | 0.5 mm edge     |
| Copper-to-Board Edge | 0.25 mm         | 0.5 mm          |

### 12.2 Surface Finish

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Preferred Finish     | HASL (Hot Air Solder Leveling) lead-free |
| Alternative          | ENIG (Electroless Nickel Immersion Gold) for fine-pitch |
| Shelf Life           | 12 months (HASL), 12 months (ENIG) |

For the LQFP-100 (0.5mm pitch) STM32F407 and TSSOP packages, ENIG is recommended for improved pad coplanarity. HASL is acceptable if the fab can guarantee coplanarity within IPC-A-610 Class 2 limits.

### 12.3 Assembly Notes

- All SMD components on the top side (L1) for single-pass reflow unless board density requires bottom-side placement.
- Through-hole components (terminal blocks J1-J3, electrolytic capacitors) are wave-soldered or hand-soldered after reflow.
- Connector terminal blocks require selective soldering or hand soldering due to thermal mass.

---

## 13. Fiducial Marks

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Quantity             | 3 minimum (per board side with SMD) |
| Diameter             | 1.0 mm (copper pad)            |
| Solder Mask Opening  | 2.0 mm (clear area around fiducial) |
| Shape                | Circle (solid copper, no hole) |
| Placement            | 2 in opposite corners, 1 on adjacent edge |
| Clearance            | No traces, vias, or silk within 2mm radius |
| Layer                | Top (L1) at minimum; bottom (L4) if bottom-side components exist |

**Purpose:** Fiducial marks provide alignment reference points for automated pick-and-place machines. They must be visible to the machine's vision system and placed asymmetrically to establish board orientation.

---

## 14. Mounting Holes

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Quantity             | 4 (one per corner)             |
| Screw Size           | M3                             |
| Drill Diameter       | 3.2 mm                         |
| Pad Diameter         | 6.0 mm                         |
| Pad Plating          | Plated (connected to GND)      |
| Clearance to Traces  | 1.0 mm from pad edge           |
| Corner Inset         | 5 mm from board edges (X and Y)|

**Notes:**
- Mounting holes are plated and connected to the L2 ground plane for chassis grounding and ESD discharge.
- The mounting screws should be M3 stainless steel pan-head with nylon washers between the screw head and the PCB to avoid cracking the board.
- For DIN rail mounting, the clip bracket attaches to 2 of the 4 mounting holes (typically the top pair).

---

## 15. Environmental Rating and Conformal Coating

### 15.1 Operating Environment

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Operating Temp Range | -20 to +70 degC                |
| Storage Temp Range   | -40 to +85 degC                |
| Humidity             | 5% to 95% RH, non-condensing  |
| Altitude             | Up to 2000 m                   |
| Pollution Degree     | 2 (per IEC 61010-1)            |

### 15.2 Conformal Coating

For field-deployed units (IP65 field housing), conformal coating is recommended:

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Coating Type         | Acrylic (AR) or Silicone (SR)  |
| Thickness            | 25-75 um                       |
| Coverage             | All components and traces      |
| Exclusion Zones      | Connectors (J1-J7), SWD header (J5), test points, mounting holes |
| Standard             | IPC-CC-830C                    |
| Application Method   | Selective spray or dip          |

**Notes:**
- Conformal coating protects against moisture, dust, and chemical contamination in harsh field environments.
- Mark exclusion zones clearly in the PCB design files (keepout areas for coating).
- Connectors must NOT be coated to maintain electrical contact integrity.

### 15.3 PCB Material

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Base Material        | FR-4 (IPC-4101, Class B/L)     |
| Glass Transition Temp| Tg >= 150 degC (high-Tg FR-4)  |
| Flammability Rating  | UL 94 V-0                      |
| CTI                  | >= 175V (IEC 60112)             |
| CAF Resistance       | Required for 0.3mm drill spacing|

High-Tg FR-4 material is specified to ensure the board maintains dimensional stability at the upper operating temperature of 70 degC and during lead-free reflow soldering (peak 260 degC).

---

*End of PCB Design and Manufacturing Guidelines Document*
