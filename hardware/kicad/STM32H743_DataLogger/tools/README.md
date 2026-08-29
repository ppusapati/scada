# Verification and routing tools

Scripts used to check and route `../STM32H743_DataLogger_mfg.kicad_pcb`. They work
directly on the KiCad file, so they need no KiCad install — useful in CI, and useful
because several of them catch things KiCad's own checks do not.

Run them from this directory. Paths are relative to `..`.

## Checks

| Script | What it answers |
|--------|-----------------|
| `check.py` | Do any footprints overlap, or hang off the board? Uses F.CrtYd courtyards — real package bodies, not pad extents. |
| `icaudit.py` | Does each IC's pad-to-net map agree with its datasheet pinout? Compares against KiCad symbol pin names. **Follows `extends`** — symbols that derive from a parent return no pins otherwise and silently pass. |
| `netlist.py` | Writes `../docs/Netlist_from_PCB.md` and `.net`, and reports any net reaching fewer than two pads. |
| `postcheck.py` | After routing: per-net connectivity walked from the pad list, plus isolation-barrier integrity. |
| `courtyard.py` | One-off: attaches F.CrtYd geometry to footprints that lack it. |

`icaudit.py` needs the KiCad symbol libraries:

```
git clone --depth 1 https://gitlab.com/kicad/libraries/kicad-symbols
KICAD_SYMBOL_DIR=kicad-symbols python3 icaudit.py
```

It refuses to run without them rather than reporting every part "not checkable",
which would read as a clean audit.

## Routing

```
python3 dsn.py board.dsn                                    # export Specctra
java -jar freerouting.jar -de board.dsn -do board.ses       # route
python3 ses.py board.ses                                    # import tracks + vias
python3 striso.py                                           # strip anything crossing an isolation barrier
python3 postcheck.py                                        # verify what actually landed
```

### Things that will bite you

- **Do not trust freerouting's completion count.** It reported "3 unrouted" on a board
  where walking the netlist showed 49 connections missing and many MCU pins never
  reached. `postcheck.py` exists because of this.
- **Scale.** The DSN declares `(resolution um 10)`, i.e. 0.1um units, so millimetres are
  multiplied by 10000 on the way out. The session comes back at **100000 units/mm** —
  a different scale from the file it was generated from. Getting this wrong imports
  every track 10x oversized, and it looks plausible until you compare a track endpoint
  against its pad.
- **Integer coordinates.** `%g` formatting emits scientific notation past 10^6, which
  freerouting's parser rejects with a misleading "Keyword.FRONT expected" error.
- **Via size drives completion.** At 0.6mm the router cannot stagger a fanout into the
  0.5mm-pitch LQFP-100 and finishes under half the board. 0.45mm/0.25mm with 0.125mm
  clearance gets it past 90%.
- **The router has no idea what isolation means.** It will take logic tracks through the
  RS485/CAN field pocket. `striso.py` removes them; run it every time.
- **Getting freerouting to terminate is the hard part.** It only writes the session at
  job end, so a wall-clock kill loses everything. On this board it ran past every limit
  tried: `-mp`, `-oit`, and the config's `max_passes`, `job_timeout`,
  `optimizer.max_passes` and `optimizer.enabled` were all ignored, and it kept churning
  at 15-30 unrouted for 25+ minutes each time. It *did* terminate and write once, at
  pass 210 after ~35 minutes on a slightly smaller netlist, so it converges eventually
  rather than never — budget an hour and do not interrupt it. If you find the actual
  lever, put it here; I did not.

## Order of work

Route **after** the IC pin maps in `../docs/Outstanding_Issues.md` section 1 are
resolved, not before. Every fix to those moves all of that part's connections and
throws the route away — this happened three times during the work that produced these
scripts.
