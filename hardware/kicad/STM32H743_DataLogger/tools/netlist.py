"""Extract the netlist from the PCB pad assignments.

The schematics carry symbols only - no wires, labels or junctions - so the PCB is the
sole record of connectivity.  This writes it out as a reviewable table and as a KiCad
netlist so the connections can be checked, and so whoever wires the sheets has a
reference to work from.
"""
import re
from collections import defaultdict

BASE = "../"
t = open(BASE + "STM32H743_DataLogger_mfg.kicad_pcb").read()

nets = dict((int(a), b) for a, b in re.findall(r'^  \(net (\d+) "([^"]*)"\)', t, re.M))
FP = re.compile(r'\(footprint "([^"]+)"[^\n]*\(uuid "([^"]+)"\) \(at [^)]*\)(.*?)\n  \)', re.S)

pins = defaultdict(list)
val = {}
for m in FP.finditer(t):
    body = m.group(3)
    ref = re.search(r'"Reference" "([^"]+)"', body).group(1)
    v = re.search(r'"Value" "([^"]*)"', body)
    val[ref] = (v.group(1) if v else "", m.group(1))
    for pm in re.finditer(r'\(pad "([^"]+)"[^\n]*\(net (\d+) "([^"]*)"\)', body):
        n = int(pm.group(2))
        if n:
            pins[n].append((ref, pm.group(1)))

rows = []
for n, name in sorted(nets.items(), key=lambda kv: kv[1]):
    if n == 0:
        continue
    pl = pins.get(n, [])
    rows.append((name, pl))

with open(BASE + "docs/Netlist_from_PCB.md", "w") as f:
    f.write("# Netlist (extracted from the PCB)\n\n")
    f.write("**The schematic sheets contain symbols only — no wires, no net labels, no\n"
            "junctions.** All connectivity in this project lives in the pad assignments of\n"
            "`STM32H743_DataLogger_mfg.kicad_pcb`. That means KiCad cannot netlist the\n"
            "schematic, ERC will report everything unconnected, and running *Update PCB from\n"
            "Schematic* would erase the board's connectivity. Wiring the sheets is the\n"
            "outstanding task; this table is the reference to wire them against.\n\n")
    f.write("Generated from the board — %d nets, %d components.\n\n" % (len(rows), len(val)))
    single = [n for n, p in rows if len(p) < 2]
    if single:
        f.write("Nets with fewer than two pads: %s\n\n" % ", ".join(single))
    else:
        f.write("Every net reaches at least two pads.\n\n")
    f.write("| Net | Pads | Connections |\n|-----|------|-------------|\n")
    for name, pl in rows:
        f.write("| `%s` | %d | %s |\n" % (name, len(pl),
                                          " ".join("%s.%s" % p for p in sorted(pl))))

with open(BASE + "docs/netlist_from_pcb.net", "w") as f:
    f.write("(export (version D)\n  (components\n")
    for ref in sorted(val):
        f.write('    (comp (ref "%s") (value "%s") (footprint "%s"))\n'
                % (ref, val[ref][0], val[ref][1]))
    f.write("  )\n  (nets\n")
    for i, (name, pl) in enumerate(rows, 1):
        f.write('    (net (code "%d") (name "%s")\n' % (i, name))
        for r, p in sorted(pl):
            f.write('      (node (ref "%s") (pin "%s"))\n' % (r, p))
        f.write("    )\n")
    f.write("  )\n)\n")

print("wrote docs/Netlist_from_PCB.md and docs/netlist_from_pcb.net")
print("%d nets, %d components, %d nets with <2 pads" % (len(rows), len(val), len(single)))
