"""Compare every IC's footprint wiring against its datasheet pinout.

For each component whose KiCad symbol resolves, check the nets on its pads against the
pin names the symbol gives. Power and ground pins are the tell: if a pin the datasheet
calls VDD is not on a supply net, or a pin it calls GND is not on a ground net, the
footprint's pad-to-net map was not derived from the datasheet.
"""
import glob
import os
import re

SCR = "./"
# KiCad symbol libraries. Override with KICAD_SYMBOL_DIR, or clone
# https://gitlab.com/kicad/libraries/kicad-symbols and point at it.
LIB = os.environ.get("KICAD_SYMBOL_DIR", SCR + "ksym/").rstrip("/") + "/"
SCH = "../"
PCB = SCH + "STM32H743_DataLogger_mfg.kicad_pcb"

SUPPLY = {"+3V3", "+5V", "+24V", "+3V3A", "+4V1", "RS485_ISO_VCC", "CAN_ISO_VCC",
          "W5500_1V8OUT", "VBUCK_SW", "VBUCK_BST"}
GNDS = {"GND", "RS485_ISO_GND", "CAN_ISO_GND", "DI_COM"}


def sym_pins(path, _depth=0):
    """Pins of a symbol, following `extends` to the parent it derives from."""
    t = open(path).read()
    ext = re.search(r'\(extends "([^"]+)"', t)
    if ext and _depth < 4:
        parent = os.path.join(os.path.dirname(path), ext.group(1) + ".kicad_sym")
        if os.path.exists(parent):
            return sym_pins(parent, _depth + 1)
    out = {}
    for m in re.finditer(r'\(pin\s+\w+\s+\w+\s*\n', t):
        i = m.start()
        d = 0
        j = i
        while j < len(t):
            if t[j] == '(':
                d += 1
            elif t[j] == ')':
                d -= 1
                if d == 0:
                    break
            j += 1
        b = t[i:j + 1]
        nm = re.search(r'\(name "([^"]*)"', b)
        nu = re.search(r'\(number "([^"]*)"', b)
        if nm and nu:
            out[nu.group(1)] = nm.group(1)
    return out


# ref -> lib_id, from the schematics
libid = {}
for f in glob.glob(SCH + "*.kicad_sch"):
    t = open(f).read()
    for m in re.finditer(r'\(symbol \(lib_id "([^"]+)"\).*?"Reference" "([^"]+)"', t, re.S):
        libid[m.group(2)] = m.group(1)
    for m in re.finditer(r'\(lib_id "([^"]+)"\)(.{0,400}?)\(property "Reference" "([^"]+)"', t, re.S):
        libid.setdefault(m.group(3), m.group(1))

t = open(PCB).read()
FP = re.compile(r'\(footprint "([^"]+)"[^\n]*\(uuid "([^"]+)"\) \(at [^)]*\)(.*?)\n  \)', re.S)

if not glob.glob(LIB + "*.kicad_symdir") and not glob.glob(LIB + "*.kicad_sym"):
    raise SystemExit(
        "No KiCad symbol libraries at %s.\n"
        "Every part would report 'not checkable' and the audit would look clean.\n"
        "Set KICAD_SYMBOL_DIR, or clone "
        "https://gitlab.com/kicad/libraries/kicad-symbols" % LIB)

rows = []
for m in FP.finditer(t):
    body = m.group(3)
    ref = re.search(r'"Reference" "([^"]+)"', body).group(1)
    if not re.match(r'^U\d', ref):
        continue
    lid = libid.get(ref)
    if not lid or ":" not in lid:
        rows.append((ref, lid or "?", None, "no symbol reference"))
        continue
    lib, sym = lid.split(":", 1)
    p = LIB + lib + ".kicad_symdir/" + sym + ".kicad_sym"
    if not os.path.exists(p):
        rows.append((ref, lid, None, "symbol not in library"))
        continue
    pins = sym_pins(p)
    pads = dict(re.findall(r'\(pad "(\w+)".*?\(net \d+ "([^"]*)"\)', body))
    bad = []
    for num, name in pins.items():
        net = pads.get(num)
        if net is None:
            continue
        u = name.upper()
        if (u.startswith(("VDD", "VCC", "AVDD", "VBAT", "VIN", "V+"))
                and not u.startswith(("VSS",))) and net not in SUPPLY:
            bad.append("%s(%s)=%s" % (num, name, net or "unset"))
        elif (u.startswith(("GND", "AGND", "VSS", "DGND", "PAD")) or u in ("EP", "TAB")) and net not in GNDS:
            bad.append("%s(%s)=%s" % (num, name, net or "unset"))
    rows.append((ref, lid, len(pins), bad))

print("%-6s %-42s %-6s %s" % ("ref", "symbol", "pins", "power/ground pins on the wrong net"))
ok = wrong = unchecked = 0
for ref, lid, n, bad in sorted(rows):
    if n is None:
        unchecked += 1
        print("  %-6s %-42s %-6s %s" % (ref, lid, "-", bad))
    elif bad:
        wrong += 1
        print("  %-6s %-42s %-6d %d wrong: %s" % (ref, lid, n, len(bad), ", ".join(bad[:6])))
    else:
        ok += 1
        print("  %-6s %-42s %-6d clean" % (ref, lid, n))
print("\n%d ICs consistent with the datasheet, %d contradict it, %d not checkable"
      % (ok, wrong, unchecked))
