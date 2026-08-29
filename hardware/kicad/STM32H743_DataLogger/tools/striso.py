"""Remove any routing that crosses into the isolated corner or the opto barrier.

Freerouting has no concept of which nets are allowed in the field-side pocket, so a
few logic tracks clipped its edge. Those are deleted outright - an incomplete net is
recoverable, a bridged 5kV barrier is not.
"""
import re

P = "../STM32H743_DataLogger_mfg.kicad_pcb"
t = open(P).read()
nets = dict((int(a), b) for a, b in re.findall(r'^  \(net (\d+) "([^"]*)"\)', t, re.M))
ISO_NETS = {"RS485_ISO_GND", "RS485_ISO_VCC", "RS485_A", "RS485_B",
            "CAN_ISO_GND", "CAN_ISO_VCC", "CAN_H", "CAN_L"}
ISO = (0, 0, 63, 26)
KEEP = (64, 19.3, 111, 20.7)


def inbox(x, y, b):
    return b[0] <= x <= b[2] and b[1] <= y <= b[3]


def crosses(x1, y1, x2, y2, name):
    for box, allow_iso in ((ISO, True), (KEEP, False)):
        if allow_iso and name in ISO_NETS:
            continue
        # sample along the segment so a track passing through is caught too
        for i in range(21):
            f = i / 20.0
            if inbox(x1 + (x2 - x1) * f, y1 + (y2 - y1) * f, box):
                return True
    return False


out, dropped = [], {}
for line in t.split("\n"):
    s = line.strip()
    m = re.match(r'\(segment \(start ([\d.-]+) ([\d.-]+)\) \(end ([\d.-]+) ([\d.-]+)\).*\(net (\d+)\)', s)
    if m:
        x1, y1, x2, y2 = (float(m.group(i)) for i in range(1, 5))
        name = nets.get(int(m.group(5)), "")
        if crosses(x1, y1, x2, y2, name):
            dropped[name] = dropped.get(name, 0) + 1
            continue
    v = re.match(r'\(via \(at ([\d.-]+) ([\d.-]+)\).*\(net (\d+)\)', s)
    if v:
        x, y = float(v.group(1)), float(v.group(2))
        name = nets.get(int(v.group(3)), "")
        if (inbox(x, y, ISO) and name not in ISO_NETS) or inbox(x, y, KEEP):
            dropped[name] = dropped.get(name, 0) + 1
            continue
    out.append(line)

t = "\n".join(out)
open(P, "w").write(t)
print("removed %d items crossing an isolation barrier:" % sum(dropped.values()))
for k, v in sorted(dropped.items(), key=lambda kv: -kv[1]):
    print("   %-16s %d" % (k or "(no net)", v))
print("parens:", t.count("("), t.count(")"))
