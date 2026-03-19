#!/bin/bash
#
# Gerber Generation Script for STM32H743 DataLogger PCB
# Generates all manufacturing output files from the _mfg PCB layout
#
# Usage: ./generate_gerbers.sh
# Requires: KiCad 8.0 CLI (kicad-cli) installed
#
# Output Structure:
#   gerber/
#   ├── STM32H743_DataLogger-F_Cu.gtl       # Front Copper
#   ├── STM32H743_DataLogger-In1_Cu.g2l     # Inner Layer 1 (GND)
#   ├── STM32H743_DataLogger-In2_Cu.g3l     # Inner Layer 2 (Power)
#   ├── STM32H743_DataLogger-B_Cu.gbl       # Back Copper
#   ├── STM32H743_DataLogger-F_Mask.gts     # Front Solder Mask
#   ├── STM32H743_DataLogger-B_Mask.gbs     # Back Solder Mask
#   ├── STM32H743_DataLogger-F_SilkS.gto    # Front Silkscreen
#   ├── STM32H743_DataLogger-B_SilkS.gbo    # Back Silkscreen
#   ├── STM32H743_DataLogger-F_Paste.gtp    # Front Paste (stencil)
#   ├── STM32H743_DataLogger-B_Paste.gbp    # Back Paste (stencil)
#   ├── STM32H743_DataLogger-Edge_Cuts.gm1  # Board Outline
#   ├── STM32H743_DataLogger.drl            # Drill file (Excellon)
#   ├── STM32H743_DataLogger-NPTH.drl       # Non-plated holes
#   ├── STM32H743_DataLogger-job.gbrjob     # Gerber job file
#   ├── STM32H743_DataLogger-top.pos        # SMD pick-and-place (top)
#   ├── STM32H743_DataLogger-bottom.pos     # SMD pick-and-place (bottom)
#   └── STM32H743_DataLogger-bom.csv        # BOM for assembly house

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PCB_FILE="$PROJECT_DIR/STM32H743_DataLogger_mfg.kicad_pcb"
OUTPUT_DIR="$SCRIPT_DIR"

echo "========================================"
echo " STM32H743 DataLogger - Gerber Export"
echo "========================================"
echo "PCB File: $PCB_FILE"
echo "Output:   $OUTPUT_DIR"
echo ""

if [ ! -f "$PCB_FILE" ]; then
    echo "ERROR: PCB file not found: $PCB_FILE"
    exit 1
fi

# Check for KiCad CLI
if ! command -v kicad-cli &>/dev/null; then
    echo "WARNING: kicad-cli not found in PATH"
    echo "Install KiCad 8.0 or add to PATH"
    echo ""
    echo "Manual export instructions:"
    echo "  1. Open $PCB_FILE in KiCad PCB Editor"
    echo "  2. File -> Fabrication Outputs -> Gerbers (.gbr)"
    echo "  3. Select all copper + mask + silk + paste + Edge.Cuts"
    echo "  4. Generate Drill Files (Excellon format)"
    echo "  5. File -> Fabrication Outputs -> Component Placement (.pos)"
    echo ""
    exit 0
fi

echo "=== Generating Gerber files ==="
kicad-cli pcb export gerbers \
    --output "$OUTPUT_DIR/" \
    --layers "F.Cu,In1.Cu,In2.Cu,B.Cu,F.Mask,B.Mask,F.SilkS,B.SilkS,F.Paste,B.Paste,Edge.Cuts" \
    --subtract-soldermask \
    --use-drill-file-origin \
    "$PCB_FILE"

echo "=== Generating Drill files ==="
kicad-cli pcb export drill \
    --output "$OUTPUT_DIR/" \
    --format excellon \
    --drill-origin plot \
    --excellon-units mm \
    --excellon-zeros-format decimal \
    --generate-map \
    --map-format gerberx2 \
    "$PCB_FILE"

echo "=== Generating Pick-and-Place files ==="
kicad-cli pcb export pos \
    --output "$OUTPUT_DIR/STM32H743_DataLogger-top.pos" \
    --side front \
    --format csv \
    --units mm \
    "$PCB_FILE"

kicad-cli pcb export pos \
    --output "$OUTPUT_DIR/STM32H743_DataLogger-bottom.pos" \
    --side back \
    --format csv \
    --units mm \
    "$PCB_FILE"

echo ""
echo "=== Generated Files ==="
ls -la "$OUTPUT_DIR"/*.g* "$OUTPUT_DIR"/*.drl "$OUTPUT_DIR"/*.pos 2>/dev/null || true
echo ""
echo "=== Manufacturing Package Ready ==="
echo "Upload these files to your PCB fabricator (JLCPCB, PCBWay, etc.)"
echo ""
echo "Board Specifications for ordering:"
echo "  Dimensions:     160 x 100 mm"
echo "  Layers:         4"
echo "  Thickness:      1.6mm"
echo "  Material:       FR-4 TG170"
echo "  Copper Weight:  1oz (35um)"
echo "  Surface Finish: ENIG"
echo "  Solder Mask:    Green"
echo "  Silkscreen:     White"
echo "  Min Track/Space: 6/6 mil (0.15/0.15mm)"
echo "  Min Hole:       0.3mm"
echo "  Impedance:      Yes (50 ohm SE, 100 ohm diff)"
echo "  IPC Class:      2"
echo ""
