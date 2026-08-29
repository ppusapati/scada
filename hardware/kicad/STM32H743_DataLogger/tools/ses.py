"""Import a freerouting .ses session into the mfg PCB as tracks and vias.

The session is parsed as real s-expressions rather than by regex - freerouting wraps
and nests paths unpredictably.  DSN y is up and in 0.1um units; KiCad y is down and in
mm, so every coordinate is scaled and the y negated.
"""
import re
import sys

PCB = "../STM32H743_DataLogger_mfg.kicad_pcb"
SES = sys.argv[1] if len(sys.argv) > 1 else "board.ses"
# freerouting writes the session at 100000 units/mm even though it declares
# (resolution um 10) - verified against a known placement (H1 at 5mm -> 500000).
SC = 100000.0


def parse(text):
    """Tokenise and build nested lists; quoted strings stay as single tokens."""
    toks = re.findall(r'\(|\)|"[^"]*"|[^\s()]+', text)
    stack, cur = [], []
    for tk in toks:
        if tk == "(":
            stack.append(cur)
            cur = []
        elif tk == ")":
            done = cur
            cur = stack.pop() if stack else []
            cur.append(done)
        else:
            cur.append(tk.strip('"') if tk.startswith('"') else tk)
    return cur


def walk(node, tag):
    """Yield every sub-list whose head is `tag`."""
    if isinstance(node, list):
        if node and node[0] == tag:
            yield node
        for c in node:
            yield from walk(c, tag)


tree = parse(open(SES).read())
pcb = open(PCB).read()
nets = dict((b, int(a)) for a, b in re.findall(r'^  \(net (\d+) "([^"]*)"\)', pcb, re.M))

segs, vias, unknown = [], [], set()
for net in walk(tree, "net"):
    if len(net) < 2:
        continue
    name = net[1]
    num = nets.get(name)
    if num is None:
        unknown.add(name)
        continue
    for wire in walk(net, "wire"):
        paths = list(walk(wire, "path")) + list(walk(wire, "polyline_path"))
        for path in paths:
            layer, width = path[1], float(path[2])
            nums = [float(v) for v in path[3:] if re.match(r'^-?[\d.]+$', str(v))]
            pts = [(nums[i] / SC, -nums[i + 1] / SC) for i in range(0, len(nums) - 1, 2)]
            for a, b in zip(pts, pts[1:]):
                if a != b:
                    segs.append((a, b, width / SC, layer, num))
    for via in walk(net, "via"):
        vals = [v for v in via[2:] if re.match(r'^-?[\d.]+$', str(v))]
        for i in range(0, len(vals) - 1, 2):
            vias.append((float(vals[i]) / SC, -float(vals[i + 1]) / SC, num))

if unknown:
    print("session references nets not in the PCB (ignored): %s" % sorted(unknown)[:8])

# --- replace routing ------------------------------------------------------------
keep = [ln for ln in pcb.split("\n")
        if not ln.strip().startswith("(segment ") and not ln.strip().startswith("(via ")]
pcb = "\n".join(keep)
pcb = re.sub(r'\n  ;; =+\n  ;; (GND STITCHING VIAS|ROUTING).*?\n  ;; =+\n', "\n", pcb, flags=re.S)

out = ["\n  ;; ============================================================\n",
       "  ;; ROUTING - autorouted (freerouting), F.Cu / In2.Cu / B.Cu\n",
       "  ;; In1.Cu is left as a solid GND reference and carries no tracks.\n",
       "  ;; ============================================================\n"]
for i, (a, b, w, layer, n) in enumerate(segs, 1):
    out.append('  (segment (start %.4f %.4f) (end %.4f %.4f) (width %.3f) (layer "%s") (net %d) '
               '(uuid "MFG-TRK-%05d"))\n' % (a[0], a[1], b[0], b[1], w, layer, n, i))
for i, (x, y, n) in enumerate(vias, 1):
    out.append('  (via (at %.4f %.4f) (size 0.45) (drill 0.25) (layers "F.Cu" "B.Cu") (net %d) '
               '(uuid "MFG-RVIA-%05d"))\n' % (x, y, n, i))

# --- GND stitching, regenerated clear of the new routing -------------------------
# The old stitching grid predates the routes and would short them, so it is dropped
# above and rebuilt here avoiding every track, via and courtyard.
import math
sys.path.insert(0, "/tmp/claude-0/-home-user-scada/82617dd3-4401-5ec5-b5ef-6ee880824847/scratchpad")
from check import load
_, comps = load()
BOXES = [(c["x0"] - 0.6, c["y0"] - 0.6, c["x1"] + 0.6, c["y1"] + 0.6) for c in comps]
NOTCH = (-1, -1, 64, 27)   # notch plus clearance, so no via lands on its edge
KEEP = (64, 19.3, 111, 20.7)
VIA_R = 0.225 + 0.2          # via radius + clearance


def near_seg(px, py, a, b, w):
    ax, ay = a
    bx, by = b
    dx, dy = bx - ax, by - ay
    L2 = dx * dx + dy * dy
    tpar = 0.0 if L2 == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / L2))
    cx, cy = ax + tpar * dx, ay + tpar * dy
    return math.hypot(px - cx, py - cy) < VIA_R + w / 2


def freept(px, py):
    if not (3 < px < 157 and 3 < py < 97):
        return False
    for x0, y0, x1, y1 in BOXES + [NOTCH, KEEP]:
        if x0 < px < x1 and y0 < py < y1:
            return False
    for a, b, w, ly, n in segs:
        if n != 1 and near_seg(px, py, a, b, w):
            return False
    for vx, vy, n in vias:
        if n != 1 and math.hypot(px - vx, py - vy) < 2 * VIA_R:
            return False
    return True


stitch = []
for gy in range(6, 97, 5):
    for gx in range(6, 157, 5):
        if freept(gx, gy):
            stitch.append('  (via (at %d %d) (size 0.45) (drill 0.25) (layers "F.Cu" "B.Cu") '
                          '(net 1) (uuid "MFG-VIA-%03d"))\n' % (gx, gy, len(stitch) + 1))
out.append("\n  ;; ============================================================\n")
out.append("  ;; GND STITCHING VIAS - 5mm grid, clear of routing and courtyards\n")
out.append("  ;; ============================================================\n")
out.extend(stitch)

anchor = "\n  ;; ============================================================\n  ;; ISOLATED FIELD-SIDE POURS"
assert anchor in pcb
pcb = pcb.replace(anchor, "".join(out) + anchor, 1)
open(PCB, "w").write(pcb)
print("imported %d track segments and %d routing vias" % (len(segs), len(vias)))
print("placed %d GND stitching vias clear of the routing" % len(stitch))
print("layers used:", sorted(set(s[3] for s in segs)))
print("parens:", pcb.count("("), pcb.count(")"))
