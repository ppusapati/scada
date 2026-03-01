# scada-dds — Cortex-A7 DDS Gateway Application
#
# This recipe builds the Rust-based SCADA DDS gateway application that runs
# on the Cortex-A7 Linux core of the STM32MP157. It provides:
#   - DDS data distribution (publish/subscribe via rustdds)
#   - XRCE-DDS agent for M4 co-processor communication
#   - Modbus TCP gateway for legacy SCADA master integration
#   - PTP (IEEE 1588) time synchronization management
#   - OTA firmware update for M4 co-processor
#   - RPMsg communication with M4 via /dev/ttyRPMSG0

SUMMARY = "SCADA DDS Gateway Application for STM32MP157"
DESCRIPTION = "Rust-based DDS gateway that bridges the M4 real-time co-processor \
    with the DDS network, providing Modbus TCP, PTP sync, and OTA updates."
HOMEPAGE = "https://github.com/scada-system/scada-dds"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=0000000000000000000000000000000"

# Source — fetch from local workspace or git
SRC_URI = " \
    file://src \
    file://Cargo.toml \
    file://Cargo.lock \
    file://config \
"

S = "${WORKDIR}"

# Rust toolchain dependencies
inherit cargo

# Cargo build configuration
CARGO_BUILD_FLAGS = "--release"

# Target architecture for Cortex-A7
CARGO_TARGET = "armv7-unknown-linux-gnueabihf"

# System dependencies
DEPENDS = " \
    openssl \
    openssl-native \
"

# Runtime dependencies
RDEPENDS:${PN} = " \
    linuxptp \
    openssl \
    kernel-module-rpmsg-char \
    kernel-module-virtio-rpmsg-bus \
"

# Recommended packages
RRECOMMENDS:${PN} = " \
    kernel-module-stm32-ipcc \
    m4-firmware \
"

# Build environment
export OPENSSL_DIR = "${STAGING_DIR_TARGET}${prefix}"
export OPENSSL_LIB_DIR = "${STAGING_DIR_TARGET}${libdir}"
export OPENSSL_INCLUDE_DIR = "${STAGING_DIR_TARGET}${includedir}"

do_compile() {
    # Set device variant feature
    if [ "${SCADA_DEVICE_TYPE}" = "water_rtu" ]; then
        FEATURES="--features water_rtu --no-default-features"
    else
        FEATURES="--features solar_smu --no-default-features"
    fi

    cd ${S}
    cargo build --release --target ${CARGO_TARGET} ${FEATURES}
}

do_install() {
    # Install binary
    install -d ${D}${bindir}
    install -m 0755 ${S}/target/${CARGO_TARGET}/release/scada-dds ${D}${bindir}/scada-dds

    # Install default configuration
    install -d ${D}${sysconfdir}/scada
    install -m 0644 ${S}/config/scada.toml ${D}${sysconfdir}/scada/scada.toml

    # Install systemd service
    install -d ${D}${systemd_system_unitdir}
    install -m 0644 ${WORKDIR}/scada-dds.service ${D}${systemd_system_unitdir}/scada-dds.service

    # Install tmpfiles.d for runtime directories
    install -d ${D}${libdir}/tmpfiles.d
    cat > ${D}${libdir}/tmpfiles.d/scada-dds.conf << 'TMPEOF'
d /var/log/scada 0755 root root -
d /var/lib/scada 0755 root root -
d /var/lib/scada/ota 0755 root root -
TMPEOF

    # Install logrotate configuration
    install -d ${D}${sysconfdir}/logrotate.d
    cat > ${D}${sysconfdir}/logrotate.d/scada-dds << 'LOGEOF'
/var/log/scada/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0644 root root
    postrotate
        systemctl reload scada-dds 2>/dev/null || true
    endscript
}
LOGEOF
}

# Systemd integration
inherit systemd
SYSTEMD_SERVICE:${PN} = "scada-dds.service"
SYSTEMD_AUTO_ENABLE = "enable"

# Package configuration files separately
CONFFILES:${PN} = "${sysconfdir}/scada/scada.toml"

# Security hardening — drop unnecessary capabilities
# The binary needs:
#   - CAP_NET_BIND_SERVICE: for Modbus TCP on port 502
#   - CAP_SYS_RAWIO: for /dev/mem access (shared memory with M4)
#   - CAP_NET_ADMIN: for PTP management
pkg_postinst:${PN}() {
    if [ -z "$D" ]; then
        setcap cap_net_bind_service,cap_sys_rawio,cap_net_admin+ep ${bindir}/scada-dds 2>/dev/null || true
    fi
}

# QA checks
FILES:${PN} = " \
    ${bindir}/scada-dds \
    ${sysconfdir}/scada/ \
    ${systemd_system_unitdir}/scada-dds.service \
    ${libdir}/tmpfiles.d/scada-dds.conf \
    ${sysconfdir}/logrotate.d/scada-dds \
"
