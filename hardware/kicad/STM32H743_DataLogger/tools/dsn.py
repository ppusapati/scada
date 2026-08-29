"""Export the mfg PCB to Specctra DSN so freerouting can route it.

KiCad units are mm with y increasing downward; DSN here uses um with y increasing
upward, so every y is negated on the way out and on the way back in.
"""
import re
import sys
from collections import defaultdict

P = "../STM32H743_DataLogger_mfg.kicad_pcb"
OUT = sys.argv[1] if len(sys.argv) > 1 else "/tmp/board.dsn"
SC = 10000.0  # (resolution um 10) declares 0.1um steps, so mm -> 0.1um
W = int(0.15 * SC)        # 0.15mm track
CL = int(0.125 * SC)      # 0.125mm clearance
VIA_D = int(0.45 * SC)    # 0.45mm via pad / 0.25mm drill - fanout for 0.5mm pitch

FP = re.compile(r'\(footprint "([^"]+)"[^\n]*\(uuid "([^"]+)"\) \(at ([\d.-]+) ([\d.-]+)'
                r'( [\d.-]+)?\)(.*?)\n  \)', re.S)
PAD = re.compile(r'\(pad "([^"]+)" (\w+) (\w+) \(at ([\d.-]+) ([\d.-]+)(?: ([\d.-]+))?\) '
                 r'\(size ([\d.]+) ([\d.]+)\)(?: \(drill ([\d.]+)\))?[^\n]*?\(net (\d+) "([^"]*)"\)')

t = open(P).read()

nets_by_num = dict((int(a), b) for a, b in re.findall(r'^  \(net (\d+) "([^"]*)"\)', t, re.M))

CU = ["F.Cu", "In1.Cu", "In2.Cu", "B.Cu"]
LAYER_IDX = {n: i for i, n in enumerate(CU)}


def rot(px, py, ang):
    r = round(ang) % 360
    if r == 0:
        return px, py
    if r == 90:
        return py, -px
    if r == 180:
        return -px, -py
    if r == 270:
        return -py, px
    raise SystemExit("unhandled rotation %s" % ang)


images, placements, pins_by_net = {}, [], defaultdict(list)
padstacks = {}


def padstack(shape, w, h, drill):
    """One padstack per distinct geometry; through-hole pads span all copper."""
    key = ("TH" if drill else "SMD", shape, round(w, 3), round(h, 3))
    name = "PS_%s_%s_%sx%s" % (key[0], shape, str(key[2]).replace(".", "_"),
                               str(key[3]).replace(".", "_"))
    if name not in padstacks:
        layers = CU if drill else ["F.Cu"]
        body = []
        for ly in layers:
            if shape == "circle":
                body.append("      (shape (circle %s %d))" % (ly, round(w * SC)))
            else:
                body.append("      (shape (rect %s %d %d %d %d))"
                            % (ly, round(-w / 2 * SC), round(-h / 2 * SC),
                               round(w / 2 * SC), round(h / 2 * SC)))
        padstacks[name] = "    (padstack %s\n%s\n      (attach off)\n    )" % (name, "\n".join(body))
    return name


for m in FP.finditer(t):
    uuid, x, y, ang, body = m.group(2), float(m.group(3)), float(m.group(4)), float(m.group(5) or 0), m.group(6)
    ref = re.search(r'"Reference" "([^"]+)"', body).group(1)
    pins, seen = [], set()
    for pm in PAD.finditer(body):
        num, ptype, shape, px, py = pm.group(1), pm.group(2), pm.group(3), float(pm.group(4)), float(pm.group(5))
        w, h, drill = float(pm.group(7)), float(pm.group(8)), pm.group(9)
        net = int(pm.group(10))
        pid = num
        k = 1
        while pid in seen:                     # DSN pin ids must be unique per image
            k += 1
            pid = "%s_%d" % (num, k)
        seen.add(pid)
        ps = padstack("circle" if shape == "circle" else "rect", w, h, drill)
        pins.append((ps, pid, px, py))
        if net:
            pins_by_net[net].append("%s-%s" % (ref, pid))
    if not pins:
        continue
    img = "IMG_" + ref
    images[img] = pins
    placements.append((img, ref, x, y, ang))

# board outline: use the rectangular extent (the isolation slots stay as keepouts
# in KiCad and would confuse the router's boundary)
BOUND = "(rect pcb 0 %d %d 0)" % (int(-100 * SC), int(160 * SC))

lines = ['(pcb board',
         '  (parser',
         '    (string_quote ")',
         '    (space_in_quoted_tokens on)',
         '    (host_cad "kicad")',
         '    (host_version "8.0")',
         '  )',
         '  (resolution um 10)',
         '  (unit um)',
         '  (structure']
# In1/In2 carry the GND and +3V3 planes - only the outer layers may be routed
for i, ly in enumerate(CU):
    kind = "power" if ly == "In1.Cu" else "signal"   # In1 stays a solid GND reference
    lines.append('    (layer %s (type %s) (property (index %d)))' % (ly, kind, i))
lines.append('    (boundary %s)' % BOUND)
# No copper may cross the optocoupler barrier on any layer.
for x0, y0, x1, y1 in [(64, 19.3, 111, 20.7)]:
    for ly in CU:
        lines.append('    (keepout (rect %s %d %d %d %d))'
                     % (ly, round(x0 * SC), round(-y1 * SC), round(x1 * SC), round(-y0 * SC)))
lines.append('    (via "VIA_450_250")')
lines.append('    (rule (width %d) (clearance %d) (clearance %d (type smd_to_turn_gap)))'
             % (W, CL, CL))
lines.append('  )')

lines.append('  (placement')
for img, ref, x, y, ang in placements:
    lines.append('    (component %s (place %s %d %d front %d))'
                 % (img, ref, round(x * SC), round(-y * SC), round(ang)))
lines.append('  )')

lines.append('  (library')
for img, pins in images.items():
    lines.append('    (image %s' % img)
    for ps, pid, px, py in pins:
        lines.append('      (pin %s %s %d %d)' % (ps, pid, round(px * SC), round(-py * SC)))
    lines.append('    )')
for ps in padstacks.values():
    lines.append(ps)
lines.append('''    (padstack VIA_450_250
      (shape (circle F.Cu %d))
      (shape (circle In1.Cu %d))
      (shape (circle In2.Cu %d))
      (shape (circle B.Cu %d))
      (attach off)
    )''' % ((VIA_D,) * 4))
lines.append('  )')

lines.append('  (network')
routed = []
PLANE = {1, 2}          # GND and +3V3 are poured on In1/In2, not routed
for num, name in sorted(nets_by_num.items()):
    if num in PLANE or num == 0 or len(pins_by_net.get(num, [])) < 2:
        continue
    routed.append(name)
    lines.append('    (net "%s"' % name)
    lines.append('      (pins %s)' % " ".join(pins_by_net[num]))
    lines.append('    )')
lines.append('    (class kicad_default "" %s' % " ".join('"%s"' % n for n in routed))
lines.append('      (circuit (use_via VIA_450_250))')
lines.append('      (rule (width %d) (clearance %d))' % (W, CL))
lines.append('    )')
lines.append('  )')
lines.append('  (wiring')
lines.append('  )')
lines.append(')')

open(OUT, "w").write("\n".join(lines) + "\n")
single = [nets_by_num[n] for n in nets_by_num if n and len(pins_by_net.get(n, [])) == 1]
print("wrote %s: %d components, %d padstacks, %d routable nets" % (OUT, len(placements), len(padstacks), len(routed)))
if single:
    print("nets with a single pad (nothing to route, check these): %s" % ", ".join(sorted(single)))
