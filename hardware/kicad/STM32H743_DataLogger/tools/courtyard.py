"""Attach real courtyard geometry (F.CrtYd rect) to every footprint in the mfg PCB.

Sizes are the physical package body + IPC clearance, keyed by footprint library id.
Anchor offsets matter for the Phoenix terminal blocks, whose origin is pad 1 rather
than the body centre.
"""
import re

P = "../STM32H743_DataLogger_mfg.kicad_pcb"

# lib id -> (width, height, x offset from anchor, y offset from anchor)
CY = {
    "Capacitor_SMD:C_0402_1005Metric": (2.0, 1.1, 0, 0),
    "Resistor_SMD:R_0402_1005Metric": (2.0, 1.1, 0, 0),
    "Inductor_SMD:L_0402_1005Metric": (2.0, 1.1, 0, 0),
    "Capacitor_SMD:C_0805_2012Metric": (3.0, 1.9, 0, 0),
    "Resistor_SMD:R_0805_2012Metric": (3.0, 1.9, 0, 0),
    "Capacitor_SMD:C_1210_3225Metric": (4.4, 3.0, 0, 0),
    "Inductor_SMD:L_1210_3225Metric": (4.4, 3.0, 0, 0),
    "Inductor_SMD:L_0603_1608Metric": (2.4, 1.4, 0, 0),
    "LED_SMD:LED_0603_1608Metric": (2.4, 1.4, 0, 0),
    "Resistor_SMD:R_Array_Convex_4x0402": (4.2, 1.6, 0, 0),
    "Diode_SMD:D_SOD-323": (2.9, 1.6, 0, 0),
    "Diode_SMD:D_SOT-23_ANK": (3.6, 3.2, 0, 0),
    "Diode_SMD:D_SOT-363_SC-70-6": (2.6, 2.7, 0, 0),
    "Diode_SMD:D_SMA": (5.6, 3.0, 0, 0),
    "Diode_SMD:D_SMC": (8.4, 4.0, 0, 0),
    "Fuse:Fuse_1812_4532Metric": (5.4, 3.9, 0, 0),
    "Package_SO:SOP-4_3.8x4.1mm_P2.54mm": (7.4, 4.6, 0, 0),
    "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm": (6.6, 5.6, 0, 0),
    "Package_SO:SO-8_3.9x4.9mm_P1.27mm": (6.6, 5.6, 0, 0),
    "Package_SO:HSOP-8-1EP_3.9x4.9mm_P1.27mm_EP2.41x3.1mm": (6.6, 5.6, 0, 0),
    "Package_SO:SOIC-16_3.9x9.9mm_P1.27mm": (6.6, 10.6, 0, 0),
    "Package_SO:SOIC-16W_7.5x10.3mm_P1.27mm": (10.6, 11.0, 0, 0),
    "Package_SO:MSOP-8_3x3mm_P0.65mm": (5.2, 3.6, 0, 0),
    "Package_SO:MSOP-8-1EP_3x3mm_P0.65mm_EP1.68x1.88mm": (5.2, 3.6, 0, 0),
    "Package_TO_SOT_SMD:SOT-23-5": (3.6, 3.2, 0, 0),
    "Package_TO_SOT_SMD:SOT-223-3_TabPin2": (8.0, 7.4, 0, 0),
    "Package_QFP:LQFP-100_14x14mm_P0.5mm": (17.0, 17.0, 0, 0),
    "Package_DFN_QFN:QFN-48-1EP_7x7mm_P0.5mm_EP5.6x5.6mm": (8.0, 8.0, 0, 0),
    "Package_DFN_QFN:QFN-28-1EP_4x4mm_P0.4mm": (5.0, 5.0, 0, 0),
    "Crystal:Crystal_SMD_3215-4Pin_3.2x1.5mm": (4.0, 2.4, 0, 0),
    "Crystal:Crystal_SMD_2012-2Pin_2.0x1.2mm": (2.8, 2.0, 0, 0),
    "Crystal:Crystal_SMD_2016-4Pin_2.0x1.6mm": (2.8, 2.4, 0, 0),
    "Capacitor_SMD:CP_Elec_6.3x5.8": (7.0, 7.0, 0, 0),
    "MountingHole:MountingHole_3.2mm_M3_Pad_Via": (6.8, 6.8, 0, 0),
    "Fiducial:Fiducial_1mm_Dia_2.54mm_Outer": (2.8, 2.8, 0, 0),
    "TestPoint:TestPoint_Pad_1.0x1.0mm": (1.6, 1.6, 0, 0),
    "Relay_THT:Relay_SPST_HF46F-G_Form_A": (13.5, 8.5, 0, 0),
    # Phoenix MKDS 5.08mm: modules butt together, body = poles * 5.08 wide,
    # 9.6 deep; origin is pad 1 so the body centre sits (poles-1)*5.08/2 to +x.
    "TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-2-5.08_1x02_P5.08mm_Horizontal":
        (10.16, 9.6, 2.54, 0.8),
    "TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-3-5.08_1x03_P5.08mm_Horizontal":
        (15.24, 9.6, 5.08, 0.8),
    "Connector_Coaxial:SMA_Amphenol_132289_EdgeMount": (6.4, 10.0, 0, 0),
    "Connector_Coaxial:U.FL_Hirose_U.FL-R-SMT-1_Solder": (3.2, 2.6, 0, 0),
    "Connector_USB:USB_C_Receptacle_GCT_USB4085": (9.2, 7.6, 0, 0),
    "Connector_RJ:RJ45_Pulse_JXD0-0019NL": (16.0, 21.5, 0, 0),
    "Connector_Card:microSD_HC_Molex_104031-0811": (15.0, 14.5, 0, 0),
    "Connector_Card:SIM_Nano_Molex_503398-1892": (14.0, 12.5, 0, 0),
    "Connector:Tag-Connect_TC2050-IDC-FP_2x05_P1.27mm_Vertical": (12.0, 8.0, 0, 0),
    "RF_Module:Microchip_ATWINC15x0-MR210xB": (22.0, 15.5, 0, 0),
    "RF_GSM:SIMCom_SIM7600": (30.0, 30.0, 0, 0),
    "RF_Bluetooth:Microchip_RN4870": (12.0, 15.0, 0, 0),
}

FP = re.compile(r'\(footprint "([^"]+)"[^\n]*\(uuid "([^"]+)"\) \(at ([\d.-]+) ([\d.-]+)\)(.*?)\n  \)', re.S)


def pad_bbox(body):
    xs, ys = [], []
    for pm in re.finditer(r'\(at ([\d.-]+) ([\d.-]+)\) \(size ([\d.]+) ([\d.]+)\)', body):
        px, py, w, h = map(float, pm.groups())
        xs += [px - w / 2, px + w / 2]
        ys += [py - h / 2, py + h / 2]
    if not xs:
        return None
    return min(xs), min(ys), max(xs), max(ys)


def courtyard(lib, body):
    """Local-coordinate courtyard box: package body union pad extents + clearance."""
    box = None
    if lib in CY:
        w, h, ox, oy = CY[lib]
        box = (ox - w / 2, oy - h / 2, ox + w / 2, oy + h / 2)
    pb = pad_bbox(body)
    if pb:
        pb = (pb[0] - 0.25, pb[1] - 0.25, pb[2] + 0.25, pb[3] + 0.25)
        box = pb if box is None else (min(box[0], pb[0]), min(box[1], pb[1]),
                                      max(box[2], pb[2]), max(box[3], pb[3]))
    return box


def main():
    t = open(P).read()
    unknown = set()
    added = [0]

    def rep(m):
        lib, uuid, body = m.group(1), m.group(2), m.group(5)
        if lib not in CY:
            unknown.add(lib)
        if 'layer "F.CrtYd"' in body:
            return m.group(0)
        box = courtyard(lib, body)
        if box is None:
            return m.group(0)
        rect = ('    (fp_rect (start %.3f %.3f) (end %.3f %.3f) (stroke (width 0.05) (type solid))'
                ' (fill none) (layer "F.CrtYd") (uuid "%s-CY"))\n' % (box[0], box[1], box[2], box[3], uuid))
        added[0] += 1
        # insert right before the closing paren of the footprint
        txt = m.group(0)
        return txt[: txt.rindex("\n  )")] + "\n" + rect.rstrip("\n") + txt[txt.rindex("\n  )"):]

    t2 = FP.sub(rep, t)
    open(P, "w").write(t2)
    print("courtyards added:", added[0])
    if unknown:
        print("no body size defined (pad-derived only):")
        for u in sorted(unknown):
            print("  ", u)
    print("parens:", t2.count("("), t2.count(")"))


main()
