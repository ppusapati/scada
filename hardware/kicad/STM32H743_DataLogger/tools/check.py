"""Honest placement report based on F.CrtYd courtyards."""
import re
import sys

P = "../STM32H743_DataLogger_mfg.kicad_pcb"
FP = re.compile(r'\(footprint "([^"]+)"[^\n]*\(uuid "([^"]+)"\) \(at ([\d.-]+) ([\d.-]+)( [\d.-]+)?\)(.*?)\n  \)', re.S)


def load(path=P):
    t = open(path).read()
    out = []
    for m in FP.finditer(t):
        body = m.group(6)
        ang = float(m.group(5) or 0)
        ref = re.search(r'"Reference" "([^"]+)"', body).group(1)
        cy = re.search(r'\(fp_rect \(start ([\d.-]+) ([\d.-]+)\) \(end ([\d.-]+) ([\d.-]+)\)'
                       r'[^)]*\)[^)]*\)[^)]*\) \(layer "F\.CrtYd"\)', body)
        if not cy:
            cy = re.search(r'\(fp_rect \(start ([\d.-]+) ([\d.-]+)\) \(end ([\d.-]+) ([\d.-]+)\).*?F\.CrtYd', body, re.S)
        if not cy:
            continue
        x, y = float(m.group(3)), float(m.group(4))
        a, b, c, d = map(float, cy.groups())
        # KiCad angle is CCW with y down: (x,y) -> (x cos + y sin, -x sin + y cos)
        r = round(ang) % 360
        if r == 90:                       # (x,y) -> (y,-x)
            a, b, c, d = b, -c, d, -a
        elif r == 180:
            a, b, c, d = -c, -d, -a, -b
        elif r == 270:                    # (x,y) -> (-y,x)
            a, b, c, d = -d, a, -b, c
        elif r:
            raise SystemExit("unhandled rotation %s on %s" % (ang, ref))
        out.append(dict(ref=ref, lib=m.group(1), uuid=m.group(2), x=x, y=y, rot=ang,
                        x0=x + a, y0=y + b, x1=x + c, y1=y + d))
    return t, out


def report(comps, board=(0, 0, 160, 100), margin=0.3):
    coll = []
    for i in range(len(comps)):
        for j in range(i + 1, len(comps)):
            a, b = comps[i], comps[j]
            ox = min(a["x1"], b["x1"]) - max(a["x0"], b["x0"])
            oy = min(a["y1"], b["y1"]) - max(a["y0"], b["y0"])
            if ox > margin and oy > margin:
                coll.append((round(min(ox, oy), 2), a["ref"], b["ref"]))
    oob = [c for c in comps
           if c["x0"] < board[0] - 0.2 or c["y0"] < board[1] - 0.2
           or c["x1"] > board[2] + 0.2 or c["y1"] > board[3] + 0.2]
    return sorted(coll, reverse=True), oob


if __name__ == "__main__":
    _, comps = load(sys.argv[1] if len(sys.argv) > 1 else P)
    coll, oob = report(comps)
    print("footprints with courtyards:", len(comps))
    print("\ncourtyard collisions: %d" % len(coll))
    for d, a, b in coll:
        print("  %-6s %-6s overlap %.2f mm" % (a, b, d))
    print("\noff-board: %d" % len(oob))
    for c in oob:
        print("  %-6s  x %.1f..%.1f  y %.1f..%.1f" % (c["ref"], c["x0"], c["x1"], c["y0"], c["y1"]))
