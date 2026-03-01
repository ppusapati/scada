# Common Connector Specification

## Connector Definitions for SS-PCB-001 and WR-PCB-001

| Field            | Value                          |
|------------------|--------------------------------|
| Document ID      | COM-CON-001                    |
| Applicable Boards| SS-PCB-001 (Solar SMU), WR-PCB-001 (Water RTU) |
| Revision         | 1.0                            |
| Date             | 2026-02-28                     |
| Status           | Preliminary                    |

---

## 1. Connector Summary

| Ref | Function              | Type                | Pitch    | Board(s)         |
|-----|-----------------------|---------------------|----------|------------------|
| J1  | 24V DC Power Input    | 2-pin terminal block| 5.08 mm  | Both             |
| J2  | Sensor Input          | Terminal block pairs| 3.81 mm  | Both (size varies)|
| J3  | RS-485 Field Wiring   | 3-pin terminal block| 5.08 mm  | Both             |
| J4  | RS-485 Termination    | 2-pin header        | 2.54 mm  | Both             |
| J5  | SWD Debug             | 2x5 header          | 1.27 mm  | Both             |
| J6  | MicroSD Card Slot     | Push-push SMD       | --       | Solar SMU only   |
| J7  | Ethernet RJ45         | RJ45 + magnetics    | --       | Water RTU only   |

---

## 2. J1 -- 24V DC Power Input

### 2.1 Connector Specification

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Reference Designator | J1                             |
| Function             | 24V DC power input             |
| Type                 | 2-pin screw terminal block     |
| Pitch                | 5.08 mm (0.2 in)              |
| Wire Entry           | Top or side, 45-degree         |
| Current Rating       | 10A per contact (minimum)      |
| Voltage Rating       | 300V (minimum)                 |
| Wire Range           | 18-24 AWG (0.2-0.82 mm^2)     |
| Screw Size           | M2.5 or equivalent             |
| Manufacturer P/N     | Phoenix Contact 1757019        |
| Alternatives         | Weidmuller 1715720000, Wurth 691137710002 |

### 2.2 Pinout

| Pin | Signal | Description                    | Wire Color (recommended) |
|-----|--------|--------------------------------|--------------------------|
| 1   | +24V   | Positive 24V DC supply input   | Red                      |
| 2   | GND    | Ground / return                | Black or Blue            |

### 2.3 Mating Information

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Mating Type          | Screw clamp, captive           |
| Torque Specification | 0.5-0.6 Nm (4.4-5.3 in-lb)   |
| Recommended Wire     | 20 AWG stranded, UL 1015      |
| Ferrule              | Recommended: 20 AWG insulated bootlace ferrule |
| Maximum Cable Length  | 30 m (with 20 AWG, < 2V drop at 500 mA) |

---

## 3. J3 -- RS-485 Field Wiring

### 3.1 Connector Specification

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Reference Designator | J3                             |
| Function             | RS-485 bus connection          |
| Type                 | 3-pin screw terminal block     |
| Pitch                | 5.08 mm (0.2 in)              |
| Current Rating       | 10A per contact                |
| Voltage Rating       | 300V                           |
| Wire Range           | 18-24 AWG (0.2-0.82 mm^2)     |
| Manufacturer P/N     | Phoenix Contact 1757022        |
| Alternatives         | Weidmuller 1715730000, Wurth 691137710003 |

### 3.2 Pinout

| Pin | Signal   | Description                    | Wire Color (recommended) |
|-----|----------|--------------------------------|--------------------------|
| 1   | A (+)    | Non-inverting (Data+, D+)      | White or Yellow          |
| 2   | B (-)    | Inverting (Data-, D-)          | Orange or Brown          |
| 3   | GND      | Signal ground / reference      | Green or Shield          |

### 3.3 RS-485 Bus Wiring Notes

- Use shielded twisted-pair cable (e.g., Belden 3105A or equivalent).
- The A/B pair must be on the same twisted pair.
- The GND wire serves as a signal reference between devices. Without it, ground potential differences can exceed the SP3485 common-mode range (+/-7V).
- Maximum bus length: 1200 m at 9600 baud (as per EIA/TIA-485-A).
- Maximum number of unit loads: 32 (SP3485 = 1/8 unit load, allowing up to 256 devices).
- Maintain consistent A/B polarity across all devices on the bus.

### 3.4 Mating Information

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Torque Specification | 0.5-0.6 Nm (4.4-5.3 in-lb)   |
| Recommended Wire     | 22 AWG shielded twisted pair   |
| Ferrule              | Recommended: 22 AWG insulated bootlace ferrule |

---

## 4. J4 -- RS-485 Termination Jumper

### 4.1 Connector Specification

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Reference Designator | J4                             |
| Function             | RS-485 line termination enable |
| Type                 | 2-pin male header              |
| Pitch                | 2.54 mm (0.1 in)              |
| Manufacturer P/N     | Wurth 61300211121 (or equiv)   |
| Jumper Shunt         | 2.54mm shorting jumper block   |

### 4.2 Function

When the jumper shunt is installed on J4, a 120 ohm termination resistor (R_TERM) is connected across the RS-485 A and B lines. This resistor matches the characteristic impedance of standard RS-485 twisted-pair cable and must be installed at both physical ends of the RS-485 bus to prevent signal reflections.

```
        RS-485 A (+) ──────┬──────── J4 Pin 1
                           |
                          [R_TERM]
                          120 ohm
                           |
        RS-485 B (-) ──────┴──────── J4 Pin 2
```

| Pin | Connection           |
|-----|----------------------|
| 1   | RS-485 A (+) line    |
| 2   | RS-485 B (-) line    |

### 4.3 Installation Rules

- **Install jumper** only at the two devices located at the physical ends of the RS-485 bus.
- **Remove jumper** from all intermediate devices on the bus.
- With the jumper removed, the failsafe bias resistors (470 ohm pull-up on A, 470 ohm pull-down on B) remain active.
- The termination resistor component (R_TERM) is always populated on the PCB; only the jumper controls whether it is active.

---

## 5. J5 -- SWD Debug Header

### 5.1 Connector Specification

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Reference Designator | J5                             |
| Function             | ARM Serial Wire Debug (SWD)    |
| Type                 | 2x5 pin shrouded header        |
| Pitch                | 1.27 mm (0.05 in)             |
| Keying               | Polarized with key notch       |
| Manufacturer P/N     | Samtec FTSH-105-01-L-DV-K     |
| Alternatives         | Tag-Connect TC2050 (no-legs, pogo-pin) |

### 5.2 Pinout (ARM Cortex 10-pin SWD Standard)

```
                J5: SWD DEBUG HEADER
              (Top view, key notch at top)

            Pin 1                    Pin 2
              +--------+--------+
      VCC     |   1    |   2    |   SWDIO (PA13)
              +--------+--------+
      GND     |   3    |   4    |   SWCLK (PA14)
              +--------+--------+
      GND     |   5    |   6    |   SWO (PB3, optional)
              +--------+--------+
      N/C     |   7    |   8    |   N/C
              +--------+--------+
      NRST    |   9    |  10    |   GND
              +--------+--------+
```

| Pin | Signal   | Direction    | STM32 Pin | Description                |
|-----|----------|-------------|-----------|----------------------------|
| 1   | VCC      | Power Out   | +3.3V     | Target power, 3.3V         |
| 2   | SWDIO    | Bidirectional| PA13     | Serial Wire Data I/O       |
| 3   | GND      | Ground      | VSS       | Ground reference           |
| 4   | SWCLK    | Input       | PA14      | Serial Wire Clock          |
| 5   | GND      | Ground      | VSS       | Ground reference           |
| 6   | SWO      | Output      | PB3       | Serial Wire Output (trace) |
| 7   | N/C      | --          | --        | Not connected              |
| 8   | N/C      | --          | --        | Not connected              |
| 9   | NRST     | Bidirectional| NRST     | MCU reset (active low)     |
| 10  | GND      | Ground      | VSS       | Ground reference           |

### 5.3 Debug Probe Compatibility

| Debug Probe              | Compatible | Notes                     |
|--------------------------|------------|---------------------------|
| ST-Link V2               | Yes        | Standard SWD, 1.27mm adapter needed |
| ST-Link V3               | Yes        | Native 1.27mm STDC14 or adapter |
| J-Link (Segger)          | Yes        | Use SWD 10-pin adapter cable |
| CMSIS-DAP (DAPLink)      | Yes        | Standard ARM SWD pinout   |
| Black Magic Probe        | Yes        | Via SWD connector         |

### 5.4 SWD Signal Routing Notes

- SWDIO and SWCLK traces should be kept short (< 50 mm) and routed as a pair.
- A 10K pull-up resistor is on NRST (part of MCU reset circuit).
- Pin 6 (SWO) is shared with SPI3_SCK (PB3) on the Solar SMU. SWO trace output is available only when the SD card is not actively in use. On the Water RTU, PB3 is unused and SWO is always available.
- VCC (pin 1) can be used to detect target power presence by the debug probe.

---

## 6. J6 -- MicroSD Card Slot (Solar SMU Only)

### 6.1 Connector Specification

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Reference Designator | J6                             |
| Function             | MicroSD card data logging      |
| Type                 | MicroSD push-push socket, SMD  |
| Card Detect          | Yes, normally-open switch      |
| Board Applicability  | SS-PCB-001 (Solar SMU) ONLY   |
| Manufacturer P/N     | Molex 1040310811 (or equiv)    |
| Alternatives         | Wurth 693072010801, GCT MEM2075-00-115-01-A |

### 6.2 Pin Mapping (SPI Mode)

The MicroSD card is operated in SPI mode (not native SDIO mode) for firmware simplicity. The following table shows the MicroSD card pin mapping in SPI mode.

| MicroSD Pin | MicroSD Function (SPI) | STM32 Pin   | STM32 AF     | Pull-Up |
|-------------|------------------------|-------------|--------------|---------|
| 1           | CS (DAT3)              | PA15        | GPIO Output  | 10K to 3.3V |
| 2           | CMD (MOSI/DI)          | PB5         | SPI3_MOSI    | --      |
| 3           | VSS (GND)              | GND         | --           | --      |
| 4           | VDD (3.3V)             | +3.3V       | --           | 10uF + 100nF |
| 5           | CLK (SCLK)             | PB3         | SPI3_SCK     | --      |
| 6           | VSS (GND)              | GND         | --           | --      |
| 7           | DAT0 (MISO/DO)         | PB4         | SPI3_MISO    | 10K to 3.3V |
| 8           | DAT1 (NC in SPI)       | --          | --           | 10K to 3.3V |
| 9           | DAT2 (NC in SPI)       | --          | --           | 10K to 3.3V |
| CD          | Card Detect            | PD2         | GPIO Input   | 10K to 3.3V |

### 6.3 SPI3 Configuration for SD Card

| Parameter      | Value                          |
|----------------|--------------------------------|
| SPI Peripheral | SPI3                           |
| Clock Speed    | 400 kHz (init), up to 21 MHz (data) |
| SPI Mode       | CPOL=0, CPHA=0 (Mode 0)       |
| Data Width     | 8-bit, MSB first               |
| CS Control     | Software (PA15 GPIO)           |

### 6.4 Card Detect

The card detect switch in the MicroSD socket is a normally-open mechanical switch that closes when a card is inserted. PD2 is configured as a GPIO input with an internal or external 10K pull-up. When a card is inserted, the CD pin is pulled to GND.

| State         | PD2 Level | Meaning           |
|---------------|-----------|-------------------|
| Card absent   | HIGH      | No card in slot   |
| Card present  | LOW       | Card inserted     |

---

## 7. J7 -- RJ45 Ethernet Connector (Water RTU Only)

### 7.1 Connector Specification

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Reference Designator | J7                             |
| Function             | 10/100 Ethernet (Modbus TCP)   |
| Type                 | RJ45 with integrated magnetics |
| Board Applicability  | WR-PCB-001 (Water RTU) ONLY   |
| Part Number          | HR911105A                      |
| Manufacturer         | Hanrun Electronics             |
| Magnetics            | Integrated 1:1 transformer     |
| LEDs                 | Dual (green: link, yellow: activity) |
| Alternatives         | J1B1211CCD, Pulse JXD1-0007NL |

### 7.2 Ethernet Pinout (T568B Standard)

| RJ45 Pin | T568B Signal | Function        | W5500 Connection          |
|----------|-------------|-----------------|---------------------------|
| 1        | TX+         | Transmit Data + | TXP via 49.9 ohm series   |
| 2        | TX-         | Transmit Data - | TXN via 49.9 ohm series   |
| 3        | RX+         | Receive Data +  | RXP via magnetics         |
| 4        | --          | Unused (10/100) | --                        |
| 5        | --          | Unused (10/100) | --                        |
| 6        | RX-         | Receive Data -  | RXN via magnetics         |
| 7        | --          | Unused (10/100) | --                        |
| 8        | --          | Unused (10/100) | --                        |

### 7.3 Magnetics and Series Resistors

The HR911105A includes integrated magnetics (1:1 isolation transformer) that provide:
- 1500 Vrms isolation between PHY and cable
- Common-mode rejection
- Impedance matching to the Cat-5/5e cable (100 ohm differential)

Series resistors on the transmit pairs:

| Ref    | Value      | Purpose                     |
|--------|------------|-----------------------------|
| R_TX1  | 49.9 ohm   | TX+ impedance matching      |
| R_TX2  | 49.9 ohm   | TX- impedance matching      |

These resistors are placed between the W5500 TXP/TXN pins and the magnetics transformer primary. They match the W5500 output impedance to the transformer/cable impedance.

### 7.4 LED Connections

| LED    | Color  | Function        | Resistor | Drive Source     |
|--------|--------|-----------------|----------|------------------|
| LED1   | Green  | Link status     | 1K       | W5500 LINKLED    |
| LED2   | Yellow | Activity        | 1K       | W5500 ACTLED     |

### 7.5 Shield Connection

The RJ45 connector shield is connected to chassis ground through a parallel combination of 1M ohm resistor and 100 pF capacitor to DGND. This provides ESD discharge path while maintaining galvanic isolation at DC.

```
  RJ45 Shield ──┬──[1M ohm]──┬── DGND
                |             |
                +──[100pF]────+
```

### 7.6 W5500 External Components

| Ref    | Value         | Purpose                           |
|--------|---------------|-----------------------------------|
| Y3     | 25 MHz crystal| W5500 clock source (22pF load caps)|
| R_EX   | 12.4K, 1%     | EXRES1 bias resistor to GND       |
| C_TOC  | 10 uF         | TOCAP to GND                      |
| FB2    | 600R @ 100MHz | AVDD ferrite bead isolation        |

---

## 8. J2 -- Sensor Input Connectors

### 8.1 Water RTU (WR-PCB-001) -- 16-Pin Terminal Block

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Reference Designator | J2                             |
| Function             | 8x 4-20mA sensor loop inputs  |
| Type                 | 16-position screw terminal block (8 pairs) |
| Pitch                | 3.81 mm                        |
| Current Rating       | 6A per contact                 |
| Voltage Rating       | 300V                           |
| Wire Range           | 20-26 AWG (0.13-0.52 mm^2)    |
| Manufacturer P/N     | Phoenix Contact 1803277 (or equiv) |

**Pinout (Water RTU):**

| Pin Pair | Channel | Signal+ (from sensor) | Signal- (return)   | Measurement     |
|----------|---------|----------------------|---------------------|-----------------|
| 1, 2     | CH0     | Pressure +           | Pressure GND        | 0-10 bar        |
| 3, 4     | CH1     | pH +                 | pH GND              | 0-14 pH         |
| 5, 6     | CH2     | Turbidity +          | Turbidity GND       | 0-1000 NTU      |
| 7, 8     | CH3     | Chlorine +           | Chlorine GND        | 0-5 mg/L        |
| 9, 10    | CH4     | Flow +               | Flow GND            | 0-100% (L/min)  |
| 11, 12   | CH5     | Tank Level +         | Tank Level GND      | 0-100%          |
| 13, 14   | CH6     | Conductivity +       | Conductivity GND    | 0-2000 uS/cm    |
| 15, 16   | CH7     | Dissolved O2 +       | Dissolved O2 GND    | 0-20 mg/L       |

**4-20mA Wiring Diagram (per channel):**

```
  24V Loop PSU ──+── Sensor ──┬── J2 Pin (Signal+)
                 |            |
                 |         [250 ohm shunt]
                 |            |
                 +────────────┴── J2 Pin (Signal-)
```

### 8.2 Solar SMU (SS-PCB-001) -- 32-Pin Terminal Block

| Parameter            | Value                          |
|----------------------|--------------------------------|
| Reference Designator | J2                             |
| Function             | 16x string voltage + 16x string current inputs |
| Type                 | 32-position screw terminal block (16 pairs) |
| Pitch                | 3.81 mm                        |
| Current Rating       | 6A per contact                 |
| Voltage Rating       | 300V                           |
| Wire Range           | 18-26 AWG (0.13-0.82 mm^2)    |
| Manufacturer P/N     | Phoenix Contact 1803468 (or equiv) |

**Pinout (Solar SMU):**

| Pin Pair  | Channel | Signal                        |
|-----------|---------|-------------------------------|
| 1, 2      | Str 1   | V+ (voltage divider input), I_shunt (current sense) |
| 3, 4      | Str 2   | V+, I_shunt                   |
| 5, 6      | Str 3   | V+, I_shunt                   |
| 7, 8      | Str 4   | V+, I_shunt                   |
| 9, 10     | Str 5   | V+, I_shunt                   |
| 11, 12    | Str 6   | V+, I_shunt                   |
| 13, 14    | Str 7   | V+, I_shunt                   |
| 15, 16    | Str 8   | V+, I_shunt                   |
| 17, 18    | Str 9   | V+, I_shunt                   |
| 19, 20    | Str 10  | V+, I_shunt                   |
| 21, 22    | Str 11  | V+, I_shunt                   |
| 23, 24    | Str 12  | V+, I_shunt                   |
| 25, 26    | Str 13  | V+, I_shunt                   |
| 27, 28    | Str 14  | V+, I_shunt                   |
| 29, 30    | Str 15  | V+, I_shunt                   |
| 31, 32    | Str 16  | V+, I_shunt                   |

Odd pins carry the string high-side voltage (connected to a 1M/2.5K voltage divider network to scale 0-1000V down to 0-2.5V for the MUX/ADC). Even pins carry the current sense signal from external shunt resistors (50mV output scaled for the ADC).

### 8.3 Mating Connectors and Torque

| Parameter              | Water RTU                 | Solar SMU                 |
|------------------------|---------------------------|---------------------------|
| Recommended Wire Gauge | 22-24 AWG stranded        | 18-22 AWG stranded        |
| Ferrule               | Insulated bootlace, matched gauge | Insulated bootlace, matched gauge |
| Torque Specification   | 0.4-0.5 Nm (3.5-4.4 in-lb) | 0.4-0.5 Nm (3.5-4.4 in-lb) |
| Cable Type             | Shielded twisted pair (per channel) | Shielded, rated for PV voltage |
| Maximum Cable Length   | 100 m (4-20mA loop)      | 50 m (voltage sense)       |

---

## 9. General Torque Specifications

All screw terminal block connections on both boards follow these torque specifications:

| Connector Pitch | Screw Size | Torque (Nm)       | Torque (in-lb)     |
|-----------------|------------|-------------------|--------------------|
| 5.08 mm         | M2.5       | 0.5-0.6           | 4.4-5.3            |
| 3.81 mm         | M2         | 0.4-0.5           | 3.5-4.4            |
| 2.54 mm (header)| --         | N/A (friction fit) | N/A                |
| 1.27 mm (SWD)   | --         | N/A (friction fit) | N/A                |

**Important:**
- Always use a calibrated torque screwdriver for terminal block connections.
- Over-torquing can damage the terminal block housing or strip the screw threads.
- Under-torquing can result in loose connections, intermittent contact, and potential overheating at high currents.
- Use insulated bootlace ferrules on all stranded wire connections to prevent strand splaying and ensure reliable contact.

---

## 10. Connector Placement Guidelines

- J1 (power) should be placed at the board edge, adjacent to the fuse holder or inline fuse connection.
- J3 (RS-485) should be placed on the same board edge as J1 for consistent field wiring entry direction.
- J2 (sensor inputs) should be placed along the longest board edge to accommodate the large number of terminals.
- J5 (SWD) may be placed on the top surface near the MCU for development access; consider making it a no-stuff option for production units.
- J4 (termination jumper) should be placed adjacent to J3 and clearly labeled on the silkscreen.
- J6 (MicroSD, Solar SMU) should be accessible from the board edge for card insertion/removal.
- J7 (RJ45, Water RTU) should be at the board edge with the LED indicators visible externally.

---

*End of Common Connector Specification Document*
