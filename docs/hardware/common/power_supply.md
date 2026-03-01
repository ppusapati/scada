# Common Power Supply Design

## Shared Power Supply Topology for SS-PCB-001 and WR-PCB-001

| Field            | Value                          |
|------------------|--------------------------------|
| Document ID      | COM-PWR-001                    |
| Applicable Boards| SS-PCB-001 (Solar SMU), WR-PCB-001 (Water RTU) |
| Revision         | 1.0                            |
| Date             | 2026-02-28                     |
| Status           | Preliminary                    |

---

## 1. Overview

Both the Solar SMU (SS-PCB-001) and the Water RTU (WR-PCB-001) share an identical power supply front-end and regulation topology. The power supply converts a 24V DC field supply down to regulated 5V and 3.3V rails, with comprehensive input protection against reverse polarity, transient voltage spikes, and electromagnetic interference.

This document defines the common power supply design that is replicated on both boards without modification.

---

## 2. 24V DC Input Specification

| Parameter                | Value                          |
|--------------------------|--------------------------------|
| Nominal Input Voltage    | 24V DC                         |
| Operating Input Range    | 18V to 30V DC                  |
| Maximum Input Current    | 500 mA (fused at panel)        |
| Input Fuse Rating        | 500 mA slow-blow, panel-mount  |
| Connector                | J1, 2-pin terminal block, 5.08mm pitch |
| Wire Gauge               | 18-22 AWG stranded             |
| Polarity                 | Pin 1 = +24V, Pin 2 = GND     |

The 24V DC supply is typically sourced from an industrial DIN-rail power supply (e.g., Mean Well HDR-30-24 or equivalent) located in the control panel. The operating range of 18-30V accommodates typical industrial supply tolerances and cable voltage drop over long field wiring runs.

---

## 3. Input Protection

### 3.1 EMI Input Filter -- Ferrite Bead (FB1)

A ferrite bead is placed in series with the +24V input line as the first component after the terminal block. This attenuates high-frequency conducted EMI from the field wiring before it reaches the downstream electronics.

| Parameter        | Value                          |
|------------------|--------------------------------|
| Reference        | FB1                            |
| Type             | Ferrite bead, SMD              |
| Impedance        | 600 ohm @ 100 MHz             |
| DC Resistance    | < 0.5 ohm                     |
| Rated Current    | >= 1A                          |
| Package          | 0805 or 1206                   |
| Manufacturer P/N | Murata BLM21PG601SN1D (or equiv) |

### 3.2 TVS Transient Suppression (D1)

A unidirectional TVS diode clamps voltage transients on the 24V rail. It is placed after the ferrite bead, between the +24V node and GND.

| Parameter             | Value                          |
|-----------------------|--------------------------------|
| Reference             | D1                             |
| Part Number           | SMBJ30A                        |
| Standoff Voltage      | 30V (does not conduct below 30V) |
| Breakdown Voltage     | 33.3V (min)                    |
| Clamping Voltage      | 48.4V @ 5.8A (peak pulse)     |
| Peak Pulse Current    | 5.8A (10/1000 us waveform)    |
| Power Rating          | 600W (10/1000 us)             |
| Package               | SMB (DO-214AA)                 |
| Direction             | Unidirectional (cathode to +24V) |
| Manufacturer          | Littelfuse / Vishay            |

The SMBJ30A is selected so that its 30V standoff voltage is above the maximum operating input (30V) while still providing effective clamping for transients. During normal operation at 24V, the TVS draws negligible leakage current.

### 3.3 Reverse Polarity Protection (Q1)

A P-channel MOSFET provides low-loss reverse polarity protection. When the input polarity is correct, the MOSFET conducts with very low R_DS(on), resulting in minimal voltage drop compared to a series diode approach.

| Parameter             | Value                          |
|-----------------------|--------------------------------|
| Reference             | Q1                             |
| Part Number           | SI2301CDS                      |
| Type                  | P-Channel Enhancement MOSFET  |
| V_DS (max)            | -20V                           |
| I_D (continuous)      | -2.3A                          |
| R_DS(on)              | 0.14 ohm @ V_GS = -4.5V      |
| Package               | SOT-23-3                       |
| Gate Pull-Up          | R1 = 10K to source (+24V side) |
| Gate Bypass Cap       | C2 = 100nF ceramic             |
| Manufacturer          | Vishay Siliconix               |

**Operating Principle:**

- **Correct polarity (+24V applied):** The source pin is at +24V, the gate is pulled toward GND through R1, creating a negative V_GS that turns on the P-FET. Current flows from source to drain with approximately 0.14 ohm drop. At 500 mA, this is only 70 mV.
- **Reverse polarity (-24V applied):** The gate-source voltage becomes positive (or zero), keeping the MOSFET off. No current flows, protecting the downstream circuit. The body diode is reverse-biased and does not conduct.

**Circuit Configuration:**

```
                 Q1 (SI2301CDS)
                S ───── G ───── D
                |       |       |
  +24V (after   |      R1      |   To LM2596
   FB1 + TVS) ──┘     10K      └── VIN
                       |
                      GND
                       |
                      C2
                     100nF
                       |
                      GND
```

---

## 4. LM2596S-5.0 Step-Down Converter

### 4.1 Circuit Description

The LM2596S-5.0 is a fixed 5V output, 3A step-down (buck) switching regulator operating at 150 kHz. It converts the protected 24V input down to a regulated 5V rail that powers the ADC analog sections, communication transceivers, and serves as the input to the 3.3V LDO.

| Parameter             | Value                          |
|-----------------------|--------------------------------|
| Reference             | U1                             |
| Part Number           | LM2596S-5.0                   |
| Manufacturer          | Texas Instruments              |
| Topology              | Buck (step-down)               |
| Input Voltage Range   | 4.5V to 40V                   |
| Output Voltage        | 5.0V fixed (+/- 4%)           |
| Output Current        | 3A maximum                     |
| Switching Frequency   | 150 kHz                        |
| Efficiency            | ~80% at 24V in, 500mA load   |
| Package               | TO-263-5 (D2PAK)              |
| Thermal Pad           | Tab (pin 3) = GND             |

### 4.2 Component Selection

#### Input Capacitor (C_in)

| Parameter        | Value                          |
|------------------|--------------------------------|
| Reference        | C1                             |
| Type             | Aluminum electrolytic          |
| Capacitance      | 680 uF                         |
| Voltage Rating   | 35V                            |
| ESR              | < 0.2 ohm (low-ESR type)      |
| Ripple Current   | >= 1A RMS @ 150 kHz           |
| Temperature      | 105 degC rated                 |
| Manufacturer P/N | Panasonic EEU-FC1V681 (or equiv) |

The 680 uF input capacitor exceeds the LM2596 datasheet minimum of 470 uF to provide additional margin for input ripple current handling and transient response. A low-ESR type is essential to minimize input voltage ripple. An additional 100nF ceramic bypass capacitor (C3) is placed directly at the VIN pin for high-frequency decoupling.

#### Output Inductor (L1)

| Parameter        | Value                          |
|------------------|--------------------------------|
| Reference        | L1                             |
| Type             | Shielded power inductor        |
| Inductance       | 33 uH                         |
| Saturation Current | >= 3A                        |
| DC Resistance    | < 0.1 ohm                     |
| Core Material    | Ferrite (shielded)             |
| Package          | 12.5 x 12.5 mm (CDRH127 style)|
| Manufacturer P/N | Sumida CDRH127/LDNP-330MC (or equiv) |

The 33 uH value is selected per the LM2596 datasheet recommendation for a 24V-to-5V conversion at 150 kHz. The inductor must handle the full 3A output current without saturating. A shielded type is selected to minimize radiated EMI.

#### Freewheeling Diode (D2)

| Parameter        | Value                          |
|------------------|--------------------------------|
| Reference        | D2                             |
| Part Number      | SS34                           |
| Type             | Schottky barrier diode         |
| Reverse Voltage  | 40V                            |
| Forward Current  | 3A                             |
| Forward Voltage  | 0.5V @ 3A                     |
| Package          | SMA (DO-214AC)                 |
| Manufacturer     | ON Semiconductor / Vishay      |

The SS34 Schottky diode serves as the freewheeling (catch) diode in the buck converter. Its low forward voltage drop (0.5V) minimizes power loss compared to a standard rectifier. The 40V reverse voltage rating provides adequate margin for the 30V maximum input.

#### Output Capacitor (C_out)

| Parameter        | Value                          |
|------------------|--------------------------------|
| Reference        | C3                             |
| Type             | Aluminum electrolytic          |
| Capacitance      | 220 uF                         |
| Voltage Rating   | 10V                            |
| ESR              | < 0.1 ohm (low-ESR type)      |
| Ripple Current   | >= 0.5A RMS @ 150 kHz         |
| Temperature      | 105 degC rated                 |
| Manufacturer P/N | Panasonic EEU-FC1A221 (or equiv) |

The 220 uF output capacitor meets the LM2596 datasheet requirement for output filtering at the selected inductor value. Low ESR is required to minimize output voltage ripple.

### 4.3 LM2596 Application Circuit

```
                                  LM2596S-5.0
                                 +-----------+
                                 |           |
  +24V (protected) ──┬──[C_in]──| VIN    SW |──┬──[L1 33uH]──┬──> +5V OUT
                     |  680uF   |           |  |              |
                    [C3]  35V   | ON/OFF    | [D2]           [C_out]
                   100nF        |  (to GND) |  SS34          220uF
                     |          |       GND |  |              10V
                     |          |    FB     |  |              |
                    GND         +--+-+------+  |              |
                                   | |         |              |
                                   | +─────────+              |
                                   |                          |
                                  GND ────────────────────── GND
```

**Notes:**
- The ON/OFF pin is tied to GND (always on). For remote shutdown capability, connect to an open-drain GPIO via a pull-up resistor.
- The FB pin is internally connected to the output via the internal voltage divider for the fixed 5.0V version. No external feedback resistors are required.
- Place C_in and C3 as close as possible to the VIN and GND pins.
- Route the SW-to-L1-to-C_out-to-GND loop with short, wide traces to minimize switching noise radiation.

---

## 5. AMS1117-3.3 LDO Regulator

The AMS1117-3.3 is a fixed 3.3V, 1A low-dropout linear regulator that derives the 3.3V digital supply from the 5V rail. It powers the STM32F407 MCU, digital sections of the ADC, and 3.3V I/O on communication interfaces.

| Parameter             | Value                          |
|-----------------------|--------------------------------|
| Reference             | U2                             |
| Part Number           | AMS1117-3.3                   |
| Manufacturer          | Advanced Monolithic Systems    |
| Output Voltage        | 3.3V fixed (+/- 1%)           |
| Output Current        | 1A maximum                     |
| Dropout Voltage       | 1.1V @ 1A (typ 1.0V)         |
| Input Voltage Range   | 4.4V to 15V (5V typical)      |
| Quiescent Current     | 5 mA typical                   |
| Package               | SOT-223-3                      |
| Thermal Pad           | Tab (pin 2) = VOUT             |

### 5.1 Input and Output Decoupling

| Reference | Type            | Capacitance | Voltage | Purpose              | Manufacturer P/N       |
|-----------|-----------------|-------------|---------|----------------------|------------------------|
| C4        | Ceramic (X5R)   | 10 uF       | 10V     | LDO input capacitor  | Murata GRM21BR61A106ME (or equiv) |
| C5        | Ceramic (X5R)   | 10 uF       | 10V     | LDO output capacitor | Murata GRM21BR61A106ME (or equiv) |

**Notes:**
- The AMS1117 requires a minimum of 10 uF on both input and output for stability. Ceramic capacitors with X5R or X7R dielectric are acceptable.
- Place both capacitors as close as possible to the AMS1117 input and output pins.
- The output capacitor ESR must be in the range of 0.1 to 0.5 ohm for stability (ceramic capacitors with very low ESR may require adding a small series resistor in some designs, but for the AMS1117-3.3 with 10 uF X5R, stability is maintained).

### 5.2 AMS1117 Application Circuit

```
                      AMS1117-3.3
                     +-----------+
                     |           |
  +5V ────┬─────────| VIN  VOUT |─────────┬────> +3.3V OUT
          |         |           |         |
         [C4]       |    GND    |        [C5]
         10uF       +-----+----+        10uF
         10V               |             10V
          |                |              |
         GND              GND            GND
```

---

## 6. Power Tree Summary

```
                         POWER TREE BLOCK DIAGRAM
  ═══════════════════════════════════════════════════════════════════

  24V DC IN (18-30V)
       |
       +── J1 Terminal Block (2-pin, 5.08mm)
       |
       +── FB1 (Ferrite Bead, 600R @ 100MHz)
       |
       +── D1 (SMBJ30A TVS, clamp to GND)
       |
       +── Q1 (SI2301CDS P-MOSFET, reverse polarity)
       |
       +── C1 (680uF/35V, input bulk)
       |   C3_bypass (100nF ceramic)
       |
       +── U1: LM2596S-5.0 (Buck Converter, 150 kHz)
       |       |
       |       +── L1 (33uH shielded inductor)
       |       +── D2 (SS34 Schottky freewheeling)
       |       +── C_out (220uF/10V output bulk)
       |
       +───────> +5V RAIL (3A max)
                    |
                    +── ADC analog supply (ADS1258/ADS1263 AVDD)
                    +── ADC reference voltage (VREFP, Water RTU)
                    +── W5500 Ethernet (Water RTU only)
                    +── SP3485 RS-485 transceiver (VCC)
                    |
                    +── C4 (10uF/10V, LDO input)
                    |
                    +── U2: AMS1117-3.3 (LDO, 1A)
                    |       |
                    |       +── C5 (10uF/10V, LDO output)
                    |
                    +───> +3.3V RAIL (1A max)
                             |
                             +── STM32F407VGT6 (VDD, VDDA, VBAT)
                             +── ADC digital supply (DVDD, IOVDD)
                             +── W5500 digital I/O (Water RTU)
                             +── TMP117 sensor (Solar SMU)
                             +── MicroSD card (Solar SMU)
                             +── MUX VCC (CD74HC4067, Solar SMU)
                             +── Pull-up resistors, LEDs
```

---

## 7. Rail Loading Summary

| Rail   | Regulator   | Max Capacity | Typical Load (Solar SMU) | Typical Load (Water RTU) |
|--------|-------------|-------------|--------------------------|--------------------------|
| 24V    | Field PSU   | 500 mA fused| ~200 mA                  | ~250 mA                  |
| 5V     | LM2596      | 3A          | ~150 mA                  | ~300 mA                  |
| 3.3V   | AMS1117     | 1A          | ~250 mA                  | ~350 mA                  |

**5V Rail Consumers:**

| Consumer               | Board(s)       | Typical Current |
|------------------------|----------------|-----------------|
| ADS1263 AVDD           | Solar SMU      | 15 mA           |
| ADS1258 AVDD + VREFP   | Water RTU      | 25 mA           |
| W5500                  | Water RTU      | 180 mA          |
| SP3485                 | Both           | 5 mA (idle)     |
| AMS1117 input (3.3V load) | Both       | varies          |

**3.3V Rail Consumers:**

| Consumer               | Board(s)       | Typical Current |
|------------------------|----------------|-----------------|
| STM32F407VGT6          | Both           | 80-120 mA       |
| ADC digital (DVDD/IOVDD)| Both          | 10 mA           |
| W5500 digital I/O      | Water RTU      | 30 mA           |
| CD74HC4067 x2          | Solar SMU      | 2 mA            |
| TMP117                 | Solar SMU      | 0.3 mA          |
| MicroSD card           | Solar SMU      | 50-100 mA       |
| Pull-ups, LEDs, misc   | Both           | 20 mA           |

---

## 8. Power Sequencing Considerations

The power supply topology inherently provides correct sequencing:

1. **24V applied** -- Input protection engages immediately (TVS ready, Q1 turns on within microseconds).
2. **5V rail stabilizes** -- The LM2596 soft-start ramps the output over approximately 1-2 ms. The 5V rail is stable before the 3.3V regulator can produce a valid output.
3. **3.3V rail stabilizes** -- The AMS1117 output follows the 5V rail with minimal delay (< 1 ms). The STM32F407 internal power-on reset (POR) holds the MCU in reset until VDD exceeds the POR threshold (~1.8V rising).
4. **MCU exits reset** -- After the 3.3V rail is stable and the NRST RC filter charges (approximately 1 ms time constant with 100nF + 10K), the MCU begins executing from flash.

**No additional sequencing circuitry is required.** The natural ramp-up order (24V -> 5V -> 3.3V) ensures that analog supplies are established before digital logic begins operating, which is the preferred order for mixed-signal systems.

**Power-Down Sequence:**

When 24V is removed, the 5V rail decays first (governed by the 220 uF output capacitor and load current), followed by the 3.3V rail. The STM32F407 enters POR when VDD drops below the POR falling threshold (~1.7V). The MCU should be designed to save critical state to flash or SRAM backup (VBAT) before power is lost. Brownout detection (BOR) in the STM32F407 can be configured to generate an interrupt or reset at a programmable threshold (e.g., 2.7V) to allow graceful shutdown.

---

## 9. Thermal Calculations and Derating

### 9.1 LM2596 Power Dissipation

The LM2596 power dissipation can be estimated as:

```
P_in  = V_in * I_out / efficiency
P_out = V_out * I_out
P_loss = P_in - P_out

Example (worst case, 30V input, 500mA total 5V load):
  P_in  = 30V * 0.5A / 0.78 = 19.2W (input power)
  P_out = 5V * 0.5A = 2.5W
  P_loss = 19.2 - 2.5 = 16.7W (total switching losses + inductor + diode)

LM2596 IC dissipation (approx 30% of total loss): ~5W
Inductor loss: ~0.5W (I^2 * R_DCR = 0.25 * 0.1)
Diode loss: ~2.5W (V_F * I_D * D = 0.5 * 0.5 * 0.79)
```

**Note:** At typical operating conditions (24V in, 300 mA load), the IC dissipation is approximately 1.5W, which is within the TO-263 package capability with a PCB copper pour heatsink of at least 4 cm^2.

**Thermal Derating:**
- T_junction (max) = 125 degC
- Theta_JA (TO-263, with copper pour) = 25-35 degC/W
- At 1.5W: T_J = T_A + (1.5 * 30) = T_A + 45 degC
- At T_A = 70 degC (max operating): T_J = 115 degC (within limits)
- At worst case 5W: requires enhanced heatsinking or reduced ambient

### 9.2 AMS1117 Power Dissipation

```
P_dissipation = (V_in - V_out) * I_out
              = (5.0V - 3.3V) * I_out
              = 1.7V * I_out

At 350 mA (Water RTU typical): P = 1.7 * 0.35 = 0.595W
At 250 mA (Solar SMU typical):  P = 1.7 * 0.25 = 0.425W
At 1A (absolute max):           P = 1.7 * 1.0  = 1.7W
```

**Thermal Derating:**
- Theta_JA (SOT-223, with copper pour) = 50-70 degC/W
- At 0.6W: T_J = T_A + (0.6 * 60) = T_A + 36 degC
- At T_A = 70 degC: T_J = 106 degC (within 125 degC limit)
- At 1A: T_J = 70 + (1.7 * 60) = 172 degC -- EXCEEDS LIMIT
  - **Maximum continuous current at 70 degC ambient: ~0.9A** (derate accordingly)

### 9.3 Capacitor Derating

All electrolytic capacitors are derated as follows:
- Voltage: operate at no more than 80% of rated voltage
  - C1 (680uF/35V): max operating voltage = 30V (within 24V nominal + transients)
  - C_out (220uF/10V): max operating voltage = 5V (50% -- well within limits)
- Temperature: 105 degC rated capacitors selected for 70 degC max ambient with adequate margin

---

## 10. EMC Considerations

### 10.1 Conducted EMI Mitigation

| Technique                    | Implementation                         |
|------------------------------|----------------------------------------|
| Input ferrite bead (FB1)     | 600R @ 100 MHz, first element in chain |
| Input bulk capacitor (C1)    | 680 uF, absorbs current pulses         |
| Ceramic bypass at VIN        | 100nF, close to LM2596 VIN pin         |
| TVS clamping (D1)            | Limits transient voltage excursions    |

### 10.2 Radiated EMI Mitigation

| Technique                    | Implementation                         |
|------------------------------|----------------------------------------|
| Shielded inductor (L1)       | Ferrite-shielded type, minimizes flux leakage |
| Short SW trace               | Keep LM2596 SW pin to inductor trace < 10mm |
| GND copper pour              | Continuous ground plane under converter |
| Component placement          | Keep switching components on one side of board |

### 10.3 Common-Mode Filtering

For field cable connections (24V input, RS-485, sensor inputs), common-mode noise is mitigated by:

- Ferrite bead on +24V input (FB1)
- Separate analog ground (AGND) tied to digital ground (GND) at a single point near the ADC
- Ground plane partitioning on PCB (see PCB Guidelines document)
- Cable shield connections to chassis ground via high-impedance path (1M + 100pF) where applicable

### 10.4 Layout Recommendations for Power Supply

1. Place the LM2596, L1, D2, C1, and C_out in a tight cluster with minimal trace lengths.
2. The current loop formed by SW-L1-C_out-GND-D2 should be as small as possible.
3. Use a dedicated ground copper pour under the switching converter, connected to the main ground plane.
4. Keep the AMS1117 LDO and its capacitors away from the switching converter (minimum 10mm separation) to avoid injecting switching noise into the 3.3V rail.
5. Route the 5V trace from the LM2596 output to the AMS1117 input as a wide trace (>= 0.5mm) or copper pour.
6. Place a ferrite bead between the 5V rail and the ADC analog supply (AVDD) to isolate switching noise from the precision analog section.

---

## 11. Component Summary Table

| Ref  | Part Number           | Manufacturer        | Description                        | Package       | Qty |
|------|-----------------------|---------------------|------------------------------------|---------------|-----|
| FB1  | BLM21PG601SN1D       | Murata              | Ferrite bead 600R @ 100 MHz        | 0805          | 1   |
| D1   | SMBJ30A              | Littelfuse          | TVS diode 30V unidirectional       | SMB           | 1   |
| Q1   | SI2301CDS            | Vishay Siliconix    | P-MOSFET -20V -2.3A               | SOT-23        | 1   |
| R1   | CRCW060310K0FKEA     | Vishay              | 10K 1% resistor (Q1 gate pull-up) | 0603          | 1   |
| C2   | GRM155R71H104KE14    | Murata              | 100nF 50V X7R (Q1 gate bypass)    | 0402          | 1   |
| U1   | LM2596S-5.0/NOPB     | Texas Instruments   | 5V 3A step-down regulator          | TO-263-5      | 1   |
| C1   | EEU-FC1V681          | Panasonic           | 680uF 35V electrolytic (input)    | Radial 10mm   | 1   |
| C3   | GRM21BR71H104KA01    | Murata              | 100nF 50V X7R (VIN bypass)        | 0805          | 1   |
| L1   | CDRH127/LDNP-330MC   | Sumida              | 33uH 3A shielded inductor         | 12.5x12.5mm   | 1   |
| D2   | SS34                 | ON Semiconductor    | Schottky 40V 3A (freewheeling)    | SMA           | 1   |
| C_out| EEU-FC1A221          | Panasonic           | 220uF 10V electrolytic (output)   | Radial 6.3mm  | 1   |
| U2   | AMS1117-3.3          | AMS                 | 3.3V 1A LDO regulator             | SOT-223       | 1   |
| C4   | GRM21BR61A106ME19    | Murata              | 10uF 10V X5R (LDO input)         | 0805          | 1   |
| C5   | GRM21BR61A106ME19    | Murata              | 10uF 10V X5R (LDO output)        | 0805          | 1   |

---

## 12. Test Points

The following test points should be provided on the PCB for power supply validation:

| Test Point | Net     | Purpose                           |
|------------|---------|-----------------------------------|
| TP1        | +24V    | Input voltage measurement         |
| TP2        | +5V     | 5V rail measurement               |
| TP3        | +3.3V   | 3.3V rail measurement             |
| TP4        | GND     | Ground reference for probing      |

---

*End of Common Power Supply Design Document*
