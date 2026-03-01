# m4-firmware — Cortex-M4 Real-Time SCADA Firmware
#
# This recipe cross-compiles the Rust-based Cortex-M4 firmware for the
# STM32MP157 co-processor. The firmware provides:
#   - High-speed sensor acquisition (ADS1263/ADS1258 ADC)
#   - XRCE-DDS client over RPMsg/OpenAMP
#   - Modbus RTU slave on RS-485
#   - IEEE 1588 PTP timestamping
#   - Real-time task scheduling via Embassy async runtime
#
# The compiled ELF is installed to /lib/firmware/ where the Linux remoteproc
# subsystem loads it onto the M4 core during boot (or via OTA update).

SUMMARY = "SCADA M4 Co-Processor Real-Time Firmware"
DESCRIPTION = "Bare-metal Rust firmware for the STM32MP157 Cortex-M4 co-processor, \
    providing real-time sensor acquisition and DDS communication."
HOMEPAGE = "https://github.com/scada-system/scada-m4"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=0000000000000000000000000000000"

# Source — fetch from local workspace or git
SRC_URI = " \
    file://src \
    file://Cargo.toml \
    file://Cargo.lock \
    file://.cargo/config.toml \
    file://memory.x \
"

S = "${WORKDIR}"

# We need the Rust cross-compilation toolchain for thumbv7em-none-eabihf
# This is a bare-metal target (no_std), not a Linux target
DEPENDS = " \
    rust-native \
    rust-cross-${TARGET_ARCH} \
"

# M4 firmware target triple (Cortex-M4 with hardware FPU)
M4_TARGET = "thumbv7em-none-eabihf"

# Firmware output filename
M4_FW_NAME = "${SCADA_M4_FIRMWARE}"

do_configure() {
    # Ensure the correct Rust target is installed
    rustup target add ${M4_TARGET} 2>/dev/null || true
}

do_compile() {
    cd ${S}

    # Select device variant features
    if [ "${SCADA_DEVICE_TYPE}" = "water_rtu" ]; then
        FEATURES="--features water_rtu --no-default-features"
    else
        FEATURES="--features solar_smu --no-default-features"
    fi

    # Build the firmware in release mode
    # Note: This is a no_std bare-metal build targeting the M4 core
    cargo build \
        --release \
        --target ${M4_TARGET} \
        ${FEATURES}

    # The output ELF is at target/thumbv7em-none-eabihf/release/scada-m4-realtime
    # Verify it's a valid ARM ELF
    if [ ! -f ${S}/target/${M4_TARGET}/release/scada-m4-realtime ]; then
        bbfatal "M4 firmware ELF not found after build"
    fi

    # Check ELF format
    file ${S}/target/${M4_TARGET}/release/scada-m4-realtime | grep -q "ELF 32-bit" || \
        bbwarn "M4 firmware may not be a valid 32-bit ELF"
}

do_install() {
    # Install firmware ELF to /lib/firmware/ for remoteproc
    install -d ${D}${nonarch_base_libdir}/firmware
    install -m 0644 \
        ${S}/target/${M4_TARGET}/release/scada-m4-realtime \
        ${D}${nonarch_base_libdir}/firmware/${M4_FW_NAME}

    # Install the resource table info for debugging
    install -d ${D}${nonarch_base_libdir}/firmware/scada

    # Create a version info file
    cat > ${D}${nonarch_base_libdir}/firmware/scada/version.txt << VEREOF
SCADA M4 Firmware
Build Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Device Type: ${SCADA_DEVICE_TYPE}
Target: ${M4_TARGET}
VEREOF

    # Install remoteproc helper script
    install -d ${D}${sbindir}
    cat > ${D}${sbindir}/scada-m4-ctl << 'CTLEOF'
#!/bin/sh
# SCADA M4 Co-Processor Control Script
#
# Usage:
#   scada-m4-ctl start    — Load and start M4 firmware
#   scada-m4-ctl stop     — Stop M4 co-processor
#   scada-m4-ctl restart  — Stop and restart M4
#   scada-m4-ctl status   — Show M4 state
#   scada-m4-ctl log      — Show M4 trace output

REMOTEPROC="/sys/class/remoteproc/remoteproc0"
FIRMWARE_DIR="/lib/firmware"

case "$1" in
    start)
        echo "Starting M4 co-processor..."
        if [ -f "${REMOTEPROC}/state" ]; then
            STATE=$(cat "${REMOTEPROC}/state")
            if [ "$STATE" = "running" ]; then
                echo "M4 is already running"
                exit 0
            fi
        fi
        echo "start" > "${REMOTEPROC}/state" 2>/dev/null
        sleep 2
        STATE=$(cat "${REMOTEPROC}/state" 2>/dev/null)
        echo "M4 state: ${STATE}"
        ;;
    stop)
        echo "Stopping M4 co-processor..."
        echo "stop" > "${REMOTEPROC}/state" 2>/dev/null
        sleep 1
        echo "M4 stopped"
        ;;
    restart)
        $0 stop
        sleep 1
        $0 start
        ;;
    status)
        if [ -f "${REMOTEPROC}/state" ]; then
            STATE=$(cat "${REMOTEPROC}/state")
            FW=$(cat "${REMOTEPROC}/firmware" 2>/dev/null)
            echo "M4 State:    ${STATE}"
            echo "M4 Firmware: ${FW}"
        else
            echo "Remoteproc not available"
            exit 1
        fi
        ;;
    log)
        # Read M4 trace buffer via debugfs
        if [ -f "/sys/kernel/debug/remoteproc/remoteproc0/trace0" ]; then
            cat "/sys/kernel/debug/remoteproc/remoteproc0/trace0"
        else
            echo "M4 trace not available (debugfs not mounted or no trace buffer)"
        fi
        ;;
    firmware)
        if [ -n "$2" ]; then
            echo "Setting M4 firmware to: $2"
            echo "$2" > "${REMOTEPROC}/firmware" 2>/dev/null
        else
            FW=$(cat "${REMOTEPROC}/firmware" 2>/dev/null)
            echo "Current firmware: ${FW}"
            echo "Available firmware:"
            ls -la ${FIRMWARE_DIR}/stm32mp157-m4* 2>/dev/null
        fi
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status|log|firmware [name]}"
        exit 1
        ;;
esac
CTLEOF
    chmod 0755 ${D}${sbindir}/scada-m4-ctl
}

# No runtime dependencies (bare-metal firmware)
RDEPENDS:${PN} = ""

# This is firmware, not a regular executable
INSANE_SKIP:${PN} = "arch ldflags"

# Package files
FILES:${PN} = " \
    ${nonarch_base_libdir}/firmware/${M4_FW_NAME} \
    ${nonarch_base_libdir}/firmware/scada/ \
    ${sbindir}/scada-m4-ctl \
"

# Ensure firmware is loaded at boot via remoteproc
# This is typically configured in the device tree (remoteproc node)
# and the systemd service for scada-dds handles the startup sequence
