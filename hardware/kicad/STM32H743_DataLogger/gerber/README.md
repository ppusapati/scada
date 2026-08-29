# Gerber output

This directory is empty on purpose. **No fabrication output has been generated, and
none should be until the items in `../docs/Outstanding_Issues.md` section 1 are
closed** — several IC pin maps are still placeholders that were never derived from
datasheets.

When the board is ready, generate from `../STM32H743_DataLogger_mfg.kicad_pcb`:

```
kicad-cli pcb export gerbers  --output gerber/ --layers F.Cu,In1.Cu,In2.Cu,B.Cu,F.Paste,B.Paste,F.SilkS,B.SilkS,F.Mask,B.Mask,Edge.Cuts  STM32H743_DataLogger_mfg.kicad_pcb
kicad-cli pcb export drill    --output gerber/ --format excellon --excellon-separate-th  STM32H743_DataLogger_mfg.kicad_pcb
```

Two things to check in the output before sending it out:

- **The four isolation slots under U40-U43** (1.2mm wide, y 21.4-22.6, at x 11.5-21.5,
  23.0-32.6, 37.5-47.5 and 49.0-58.6) must appear on Edge.Cuts as interior cutouts, not
  be silently dropped. They are what gives the RS485 and CAN barriers their creepage.
- **The isolated copper pours** must come out as separate islands on F.Cu and B.Cu with
  no connection to the main ground pour. If the plot shows them merged, the notch
  geometry did not fill as intended.

Run DRC in KiCad first. Nothing in this project has been through KiCad's own DRC —
the geometry checks here were done with scripts against the file, which catch courtyard
overlaps and barrier crossings but not clearance, annular ring, or track-width rules.
