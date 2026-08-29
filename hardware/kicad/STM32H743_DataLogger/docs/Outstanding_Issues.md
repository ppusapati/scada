# Outstanding issues

Everything known to be incomplete or wrong, in the order it blocks release. Detail and
evidence for each is in `Design_Specification.md` sections 7.7-7.10 and
`Schematic_Symbol_Audit.md`.

## 1. Blocks fabrication

**IC pin maps that were never derived from datasheets.** Several components had their
pad-to-net map filled in sequentially. Three have been rebuilt (MCU, W5500, ISO3082) and
one partly (TPS54560), but these remain:

| Ref | Part | Problem | Needs |
|-----|------|---------|-------|
| U41 | ISO1042BQDWRQ1 | same off-by-one as the ISO3082 had — bus pair one pin low, GND2 on the NC pins, VCC2 on both 15 and 16 | datasheet pinout |
| U60 | SX1276IMLTRT | footprint is `QFN-28 4x4mm, 0.4mm pitch`; the part is **6x6mm on 0.65mm pitch**. Wrong package outline, and the wiring is sequential | datasheet: package **and** pinout |
| U3 | TPS7A4533DGNR | MSOP-8-EP. Library only has the KTT (TO-263-5) variant, a different pinout, so it cannot be checked or substituted | datasheet pinout |
| U61 | ATWINC1500-MR210PB | no symbol; sequential pattern | datasheet pinout |
| U62 | SIM7600G-H | no symbol; sequential pattern | datasheet pinout |
| U62B | RN4870-I/RM128 | no symbol; sequential pattern | datasheet pinout |
| U70 | W25Q64JVSSIQ | no symbol, but **verifies clean** by inspection | confirm only |
| U63 | TPS73641DCQR | shares the TLV1117-33 symbol, a different part; IN and GND look swapped against it but that symbol is the wrong reference | datasheet pinout |
| J60 | Pulse J0011D21BNL | footprint has 10 pads; the part is a 12-pin RJ45 with magnetics and LEDs | datasheet |

Not everything is affected — W25Q64, AT24C256 and the TLV1117 on U2 all check out, so
the placeholder wiring went into the complex parts rather than uniformly.

Note that several KiCad symbols derive from a parent through `extends`, so a naive pin
read returns nothing and the part silently passes. The audit script follows inheritance;
anyone repeating this check should make sure theirs does too — it was hiding the
op-amp fault below.

**Fixed since first raised, listed so it is not re-reported:** the OPA2376 analog
buffers (U20/U21) had supply and output swapped and each amplifier shorted
input-to-output, sitting in parallel with the ADC node instead of buffering it. Rewired
as unity-gain buffers on new AI_CHn_IN filter nodes.

**Buck converter is unfinished.** U1 (TPS54560) has nothing on EN, RT/CLK or COMP.
RT/CLK sets the switching frequency and COMP is the compensation network; the regulator
will not regulate without them. EN can be left open (internal pull-up), though a divider
is the usual choice for programmed UVLO.

## 2. Needs a decision before firmware is written

Four MCU pins moved when the intended assignment was converted to real pin numbers,
because the old notes double-booked two pins:

| Signal | Was | Now | Why |
|--------|-----|-----|-----|
| UART4_TX | PA0 | PB9 | PA0 is an ADC1 channel |
| UART4_RX | PC11 | PB8 | PC11 is SDMMC1_D3; all five UART4_RX pins were taken |
| WDT_KICK | PB8 | PD9 | displaced by UART4_RX |
| LORA_RST, GSM_PWR_KEY | PC2, PC3 | PB10, PB11 | those are the dual-pad PC2_C/PC3_C analog variants on this package |

`SD_CD` was never assigned at all and is now on PD15.

## 3. Schematic capture has not been done

The nine sheets hold symbols and text annotations only — **no wires, no net labels, no
junctions, no no-connects anywhere**. All connectivity lives in the PCB's pad
assignments.

- KiCad cannot netlist the schematic; ERC will report every pin unconnected.
- **Do not run Tools > Update PCB from Schematic** — it would erase the board's
  connectivity.
- `Netlist_from_PCB.md` and `netlist_from_pcb.net` carry the 160 nets to wire against.
- Nine symbols still need drawing before capture can start; see
  `Schematic_Symbol_Audit.md`.

## 4. Routing

Autorouted and verified independently against the pad list rather than trusting the
router's own count. See section 7.9 for the current figures and
`unrouted_nets.txt` for what is left.

Two things to watch on any re-route:

- Freerouting has no concept of which nets may enter the isolated corner; it took logic
  tracks through it on the previous pass and they had to be stripped. Re-check after
  every run.
- The isolator logic pins sit 0.7mm from the notch edge, which is what invites that. If
  it keeps happening, move the U40-U43 barrier row down about 2mm.

## 5. Verified and not outstanding

Recorded so it is not re-litigated:

- Placement: 231 footprints, zero courtyard collisions, nothing off-board, real package
  bodies (not pad extents).
- Isolation: notched pours, separate floating pours per field bus, milled slots under
  U40-U43, four-layer keepout under the optocoupler row, no logic-net pad inside the
  isolated pocket.
- Isolated supplies U42/U43 present, so the 5kV transceivers can actually power their
  bus sides.
- Digital inputs return on a floating DI_COM rather than logic ground.
- MCU pin map verified against every pin the package fixes.
- Every net reaches at least two pads.
