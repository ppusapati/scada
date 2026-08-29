"""Verify the imported routing: per-net connectivity plus isolation-barrier integrity."""
import re
import math
from collections import defaultdict

P = "../STM32H743_DataLogger_mfg.kicad_pcb"
t = open(P).read()

nets = dict((int(a), b) for a, b in re.findall(r'^  \(net (\d+) "([^"]*)"\)', t, re.M))
PLANE = {1, 2}                              # poured, not routed
ISO = (0, 0, 63, 26)                        # notch: no main copper allowed
KEEP = (64, 19.3, 111, 20.7)                # optocoupler barrier
ISO_NETS = {"RS485_ISO_GND", "RS485_ISO_VCC", "RS485_A", "RS485_B",
            "CAN_ISO_GND", "CAN_ISO_VCC", "CAN_H", "CAN_L"}

FP = re.compile(r'\(footprint "[^"]+"[^\n]*\(uuid "([^"]+)"\) \(at ([\d.-]+) ([\d.-]+)'
                r'( [\d.-]+)?\)(.*?)\n  \)', re.S)


def rot(px, py, ang):
    r = round(ang) % 360
    return {0: (px, py), 90: (py, -px), 180: (-px, -py), 270: (-py, px)}[r]


pads = defaultdict(list)          # net -> [(x, y, ref.pad, is_through)]
for m in FP.finditer(t):
    x, y, ang, body = float(m.group(2)), float(m.group(3)), float(m.group(4) or 0), m.group(5)
    ref = re.search(r'"Reference" "([^"]+)"', body).group(1)
    for line in re.findall(r'\(pad "[^"]+".*', body):
        at = re.search(r'\(at ([\d.-]+) ([\d.-]+)', line)
        nt = re.search(r'\(net (\d+) "([^"]*)"\)', line)
        num = re.search(r'\(pad "([^"]+)"', line).group(1)
        if not (at and nt):
            continue
        px, py = rot(float(at.group(1)), float(at.group(2)), ang)
        net = int(nt.group(1))
        if net:
            pads[net].append((x + px, y + py, "%s.%s" % (ref, num), "thru_hole" in line))

segs = [(float(a), float(b), float(c), float(d), ly, int(n)) for a, b, c, d, w, ly, n in
        re.findall(r'\(segment \(start ([\d.-]+) ([\d.-]+)\) \(end ([\d.-]+) ([\d.-]+)\) '
                   r'\(width ([\d.]+)\) \(layer "([^"]+)"\) \(net (\d+)\)', t)]
vias = [(float(a), float(b), int(n)) for a, b, s, d, n in
        re.findall(r'\(via \(at ([\d.-]+) ([\d.-]+)\) \(size ([\d.]+)\) \(drill ([\d.]+)\)'
                   r' \(layers "[^"]+" "[^"]+"\) \(net (\d+)\)', t)]

print("tracks: %d   vias: %d" % (len(segs), len(vias)))

# --- connectivity ---------------------------------------------------------------
seg_by_net = defaultdict(list)
for a, b, c, d, ly, n in segs:
    seg_by_net[n].append(((a, b), (c, d)))
via_by_net = defaultdict(list)
for x, y, n in vias:
    via_by_net[n].append((x, y))

TOL = 0.12
unrouted, partial = [], []
for net, pl in sorted(pads.items()):
    if net in PLANE or len(pl) < 2:
        continue
    pts = [(p[0], p[1]) for p in pl]
    edges = seg_by_net.get(net, [])
    if not edges:
        unrouted.append(nets.get(net, str(net)))
        continue
    nodes = {}

    def nid(p):
        for q in nodes:
            if abs(q[0] - p[0]) < TOL and abs(q[1] - p[1]) < TOL:
                return nodes[q]
        nodes[p] = len(nodes)
        return nodes[p]

    parent = {}

    def find(a):
        while parent.get(a, a) != a:
            a = parent[a]
        return a

    def uni(a, b):
        ra, rb = find(a), find(b)
        parent.setdefault(a, a)
        parent.setdefault(b, b)
        if ra != rb:
            parent[ra] = rb
    for a, b in edges:
        uni(nid(a), nid(b))
    roots = set()
    missing = 0
    for p in pts:
        hit = None
        for q in nodes:
            if abs(q[0] - p[0]) < TOL and abs(q[1] - p[1]) < TOL:
                hit = nodes[q]
                break
        if hit is None:
            missing += 1
        else:
            roots.add(find(hit))
    if missing or len(roots) > 1:
        partial.append((nets.get(net, str(net)), len(pts), missing, len(roots)))

print("\nnets with no copper at all: %d %s" % (len(unrouted), unrouted[:12]))
print("nets not fully joined: %d" % len(partial))
for n, np_, ms, gr in partial[:15]:
    print("   %-18s pads=%-3d unterminated=%-3d islands=%d" % (n, np_, ms, gr))

# --- isolation integrity --------------------------------------------------------
def inbox(x, y, b):
    return b[0] <= x <= b[2] and b[1] <= y <= b[3]


viol = []
for a, b, c, d, ly, n in segs:
    name = nets.get(n, "")
    for box, tag in ((ISO, "isolated corner"), (KEEP, "opto barrier")):
        if inbox(a, b, box) or inbox(c, d, box):
            if tag == "isolated corner" and name in ISO_NETS:
                continue
            viol.append((name, tag, round(a, 1), round(b, 1)))
for x, y, n in vias:
    for box, tag in ((ISO, "isolated corner"), (KEEP, "opto barrier")):
        if inbox(x, y, box) and not (tag == "isolated corner" and nets.get(n, "") in ISO_NETS):
            viol.append((nets.get(n, ""), tag + " (via)", round(x, 1), round(y, 1)))
print("\nisolation violations: %d" % len(viol))
for v in viol[:15]:
    print("   ", v)
