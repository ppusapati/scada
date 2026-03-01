# DDS Implementation — STM32MP157 Architecture Upgrade
## SCADA System Deterministic Real-Time Design

**Document No:** SCADA-DDS-001
**Revision:** 1.0
**Date:** March 2026
**Target Platform:** STM32MP157CAC3 (TFBGA-361)
**Determinism:** Hard real-time (<1 ms)

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Why DDS for SCADA](#2-why-dds-for-scada)
3. [STM32MP157 Platform Details](#3-stm32mp157-platform-details)
4. [Dual-Core Architecture — DDS + Real-Time I/O](#4-dual-core-architecture--dds--real-time-io)
5. [DDS Middleware Selection](#5-dds-middleware-selection)
6. [DDS Topic Architecture for SCADA](#6-dds-topic-architecture-for-scada)
7. [Hardware Design Changes](#7-hardware-design-changes)
8. [Revised BOM — Solar SMU (DDS Variant)](#8-revised-bom--solar-smu-dds-variant)
9. [Revised BOM — Water RTU (DDS Variant)](#9-revised-bom--water-rtu-dds-variant)
10. [Revised Cost Estimation](#10-revised-cost-estimation)
11. [Software Architecture](#11-software-architecture)
12. [Determinism Analysis](#12-determinism-analysis)
13. [Migration Path from Modbus to DDS](#13-migration-path-from-modbus-to-dds)
14. [Alternative Platforms Considered](#14-alternative-platforms-considered)

---

## 1. Architecture Overview

### Current Architecture (STM32F407 — Modbus Only)

```
┌──────────────────────────┐
│   STM32F407VGT6          │
│   Cortex-M4 @ 168 MHz   │
│   192 KB SRAM            │     RS-485
│   Bare-metal Embassy     │────────────── SCADA Master
│   Modbus RTU/TCP         │     Ethernet
│   No DDS capability      │    (RTU only)
└──────────────────────────┘
```

### New Architecture (STM32MP157 — Full DDS + Hard RT)

```
┌──────────────────────────────────────────────────────────┐
│                    STM32MP157CAC3                         │
│                                                          │
│  ┌─────────────────────────┐  ┌────────────────────────┐ │
│  │  Cortex-A7 (Dual)       │  │  Cortex-M4 @ 209 MHz  │ │
│  │  @ 650/800 MHz          │  │                        │ │
│  │                         │  │  FreeRTOS / Bare-metal │ │
│  │  OpenSTLinux (Yocto)    │  │  Hard RT I/O Control   │ │
│  │  ┌───────────────────┐  │  │  ┌──────────────────┐  │ │
│  │  │  Fast DDS / Cyclo │  │  │  │ ADC Driver       │  │ │
│  │  │  Full DDS Stack   │  │  │  │ (ADS1263/1258)   │  │ │
│  │  │  RTPS Discovery   │  │  │  │ SPI Master       │  │ │
│  │  │  QoS Engine       │  │  │  │ GPIO Control     │  │ │
│  │  │  DDS Topics       │  │  │  │ RS-485 Driver    │  │ │
│  │  └───────────────────┘  │  │  │ Sensor Sampling  │  │ │
│  │                         │  │  └──────────────────┘  │ │
│  │  ┌───────────────────┐  │  │                        │ │
│  │  │ XRCE-DDS Agent    │  │  │  ┌──────────────────┐  │ │
│  │  │ (bridge M4 data)  │◄─┼──┤  │ XRCE-DDS Client  │  │ │
│  │  └───────────────────┘  │  │  │ (pub sensor data)│  │ │
│  │         OpenAMP/RPMsg   │  │  └──────────────────┘  │ │
│  │                         │  │                        │ │
│  │  ┌───────────────────┐  │  │  ┌──────────────────┐  │ │
│  │  │ Modbus Gateway    │  │  │  │ PTP IEEE 1588    │  │ │
│  │  │ (backward compat) │  │  │  │ HW Timestamping  │  │ │
│  │  └───────────────────┘  │  │  └──────────────────┘  │ │
│  └─────────────────────────┘  └────────────────────────┘ │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │              Shared Peripherals                     │  │
│  │  Gigabit Ethernet (IEEE 1588 PTP)                  │  │
│  │  SPI1 (ADC)  |  SPI2 (W5500/SD)  |  USART (485)   │  │
│  │  I2C (sensors)  |  CAN FD  |  USB OTG              │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────┐  ┌──────────────────────────────┐   │
│  │ DDR3L 256MB    │  │ eMMC 4GB (Linux rootfs)      │   │
│  │ (External)     │  │ or microSD                    │   │
│  └────────────────┘  └──────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

### Key Benefits of This Architecture

| Feature | STM32F407 (Current) | STM32MP157 (DDS) |
|---------|--------------------|--------------------|
| Communication | Modbus RTU/TCP | Full DDS + Modbus (backward compat) |
| Discovery | Manual configuration | Automatic DDS discovery |
| QoS | None | Full DDS QoS (reliability, deadline, lifespan) |
| Determinism | Sub-ms (bare-metal) | Sub-ms on M4 core (isolated from Linux) |
| Time Sync | None | IEEE 1588 PTP (<1 µs accuracy) |
| Data Model | Register-based (Modbus) | Topic-based (pub-sub) |
| Scalability | 247 devices per bus | Unlimited DDS domain |
| Edge Computing | None | Linux on A7 (analytics, logging, OPC UA) |
| Security | None | DDS Security, TLS, Linux firewall |
| Firmware Update | Manual SWD | OTA via Linux (A7) + RPMsg to M4 |

---

## 2. Why DDS for SCADA

### 2.1 Limitations of Modbus

- **No discovery** — Every device must be manually configured with addresses
- **No QoS** — No reliability guarantees, no deadline enforcement, no lifespan
- **No pub-sub** — Master-slave polling only; high latency at scale
- **No security** — No authentication, encryption, or access control
- **No timestamps** — Data has no inherent time context
- **Bandwidth limited** — RS-485 maxes at 115200 baud; TCP adds overhead
- **Single master** — Only one master can poll at a time

### 2.2 DDS Advantages for Industrial SCADA

```
DDS Data-Centric Publish-Subscribe (DCPS)

  ┌──────────┐    DDS Topic:       ┌──────────────┐
  │ Solar SMU │    "StringVoltage"  │ SCADA Master │
  │ Publisher │───────────────────► │ Subscriber   │
  └──────────┘         │            └──────────────┘
                       │
  ┌──────────┐         │            ┌──────────────┐
  │ Solar SMU │         └──────────►│ Data Logger  │
  │ Publisher │                     │ Subscriber   │
  └──────────┘                     └──────────────┘

  ┌──────────┐    DDS Topic:       ┌──────────────┐
  │ Water RTU│    "WaterLevel"     │ Alarm Server │
  │ Publisher │───────────────────►│ Subscriber   │
  └──────────┘                     └──────────────┘

  - Automatic discovery (no manual address config)
  - Multiple subscribers per topic (multicast)
  - QoS-driven delivery (reliability, deadline, history)
  - Peer-to-peer (no single point of failure)
  - IEEE 1588 PTP time synchronization
```

### 2.3 DDS QoS Policies for SCADA

| QoS Policy | Setting | Purpose |
|------------|---------|---------|
| Reliability | RELIABLE | Guaranteed delivery of sensor readings |
| Deadline | 100 ms | Alert if data not received within deadline |
| Lifespan | 5 seconds | Discard stale sensor data |
| History | KEEP_LAST (10) | Retain last 10 samples per topic |
| Durability | TRANSIENT_LOCAL | Late subscribers get last known value |
| Ownership | EXCLUSIVE | Only highest-priority publisher wins |
| Partition | "Solar/Plant_01" | Logical grouping of DDS entities |

---

## 3. STM32MP157 Platform Details

### 3.1 STM32MP157CAC3 Specifications

| Parameter | Value |
|-----------|-------|
| Part Number | STM32MP157CAC3 |
| Package | TFBGA-361 (12x12 mm, 0.5mm pitch) |
| Application Core | 2x Cortex-A7 @ 650 MHz (800 MHz boost) |
| Real-Time Core | 1x Cortex-M4 @ 209 MHz (FPU, DSP) |
| L1 Cache | 32 KB I-cache + 32 KB D-cache per A7 |
| L2 Cache | 256 KB shared |
| Cortex-M4 SRAM | 128 KB (64 KB TCM + 64 KB SRAM) |
| MCU SRAM | 384 KB total (various banks) |
| DDR Support | DDR3/DDR3L/LPDDR2/LPDDR3, 16/32-bit, up to 1 GB |
| Ethernet | 1x Gigabit MAC (RGMII/RMII), IEEE 1588 PTP |
| CAN | 2x CAN FD (ISO 11898-1) |
| SPI | 6x SPI/I2S |
| I2C | 6x I2C |
| USART | 4x USART + 4x UART |
| ADC | 2x 12-bit ADC (16 channels each) |
| USB | 1x USB 2.0 OTG HS |
| Display | LTDC + MIPI-DSI |
| Crypto | AES-256, SHA-256, TRNG |
| Security | Secure Boot (HAB), TrustZone, ECDSA |
| GPIO | Up to 176 GPIOs |
| Temperature | -40 to +85°C (industrial grade) |
| Price (India) | ~₹850 / ~$10 (qty 100) |

### 3.2 Companion Components Required

| Component | Part Number | Purpose | Price (INR) |
|-----------|-------------|---------|-------------|
| STPMIC1A | STPMIC1APQR | Power Management IC | ₹350 |
| DDR3L 256MB | MT41K128M16JT-125 (Micron) | System memory (16-bit, 4 Gbit) | ₹450 |
| eMMC 4GB | MTFC4GACAJCN (Micron) | Linux rootfs + data storage | ₹550 |
| Ethernet PHY | KSZ9031RNXIA (Microchip) | Gigabit RGMII PHY, 1588 PTP | ₹250 |
| 24 MHz Crystal | ABM3B-24.000MHZ | HSE for MPU | ₹25 |
| 32.768 kHz Crystal | ABS07-32.768KHZ-T | LSE for RTC | ₹20 |

### 3.3 STPMIC1A Power Tree

```
                    24V DC Input
                         │
                    ┌────┴─────┐
                    │ LM2596S  │  5.0V / 3A
                    │ 5.0 Buck │──────────── Analog (ADC, sensors)
                    └────┬─────┘
                         │ 5.0V
                    ┌────┴──────────┐
                    │   STPMIC1A    │
                    │  (QFN-56)     │
                    ├───────────────┤
                    │ BUCK1: VDDCORE│── 1.2V (A7 + M4 core logic)
                    │ BUCK2: VDD_DDR│── 1.35V (DDR3L memory)
                    │ BUCK3: VDD    │── 3.3V (I/O supply)
                    │ BUCK4: VDD_3V3│── 3.3V (peripherals)
                    │ LDO1: VDD_1V8│── 1.8V (USB PHY, PLL)
                    │ LDO2: VDDA_1V8── 1.8V (ADC, DAC analog)
                    │ LDO3: VTT_DDR│── 0.675V (DDR termination)
                    │ LDO4: VREF_DDR── 0.675V (DDR reference)
                    │ LDO5: VDD_USB│── 3.3V (USB supply)
                    │ LDO6: VDD_SD │── 3.3V (SD card / eMMC)
                    │ VBUS_OTG     │── 5.0V (USB OTG VBUS)
                    │ SW_OUT       │── Switched output
                    └──────────────┘

Power Sequencing (controlled by STPMIC1A):
  VDDCORE (1.2V) → VDD_DDR (1.35V) → VDD (3.3V) → VTT_DDR → Boot
```

---

## 4. Dual-Core Architecture — DDS + Real-Time I/O

### 4.1 Core Assignment

```
┌────────────────────────────────────────────────────────────────┐
│                CORTEX-A7 (Linux Domain)                        │
│                                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │ Fast DDS     │  │ Modbus       │  │ System Services    │  │
│  │              │  │ Gateway      │  │                    │  │
│  │ - Publisher  │  │              │  │ - OTA Updates      │  │
│  │ - Subscriber │  │ - TCP Server │  │ - SSH/Debug        │  │
│  │ - Discovery  │  │ - RTU Master │  │ - NTP/PTP          │  │
│  │ - QoS Engine │  │ - DDS Bridge │  │ - Syslog           │  │
│  │ - Security   │  │              │  │ - Edge Analytics   │  │
│  └──────────────┘  └──────────────┘  └────────────────────┘  │
│                                                                │
│  Ethernet (GbE)  |  USB  |  eMMC  |  HDMI (optional HMI)     │
│  CAN FD          |  WiFi (optional USB dongle)                │
└────────────────────────────┬───────────────────────────────────┘
                             │ OpenAMP / RPMsg
                             │ (inter-processor mailbox)
┌────────────────────────────┴───────────────────────────────────┐
│                CORTEX-M4 (Real-Time Domain)                    │
│                                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │ ADC Driver   │  │ XRCE-DDS    │  │ Sensor Processing  │  │
│  │              │  │ Client       │  │                    │  │
│  │ - ADS1263    │  │              │  │ - Oversampling     │  │
│  │ - ADS1258    │  │ - Serialize  │  │ - Filtering (IIR)  │  │
│  │ - SPI Master │  │ - Publish    │  │ - Scaling          │  │
│  │ - DMA xfer   │  │ - Subscribe  │  │ - Alarm Detection  │  │
│  │ - Trigger    │  │ - RPMsg Tx   │  │ - Timestamp (PTP)  │  │
│  └──────────────┘  └──────────────┘  └────────────────────┘  │
│                                                                │
│  SPI1 (ADC)  |  USART2 (RS-485)  |  I2C (TMP117)             │
│  SPI3 (SD card, SMU only)  |  GPIO (LEDs, MUX select)         │
│  Timer (precise sampling trigger)                              │
└────────────────────────────────────────────────────────────────┘

Data Flow (Hard RT ADC → DDS Topic):
  1. M4 Timer triggers ADC sample (deterministic, jitter < 10 µs)
  2. M4 reads ADS1263/1258 via SPI DMA
  3. M4 applies calibration + filtering (< 100 µs)
  4. M4 serializes to XRCE-DDS topic + PTP timestamp
  5. M4 sends via RPMsg mailbox to A7 (~50 µs transfer)
  6. A7 XRCE Agent publishes to DDS domain via Ethernet
  Total end-to-end: < 1 ms (M4 path) + ~2 ms (network)
```

### 4.2 Inter-Processor Communication (OpenAMP)

The Cortex-M4 and Cortex-A7 communicate via **OpenAMP/RPMsg** — a shared-memory mailbox mechanism built into the STM32MP1 hardware.

```
Cortex-A7 (Linux)                  Cortex-M4 (FreeRTOS)
     │                                    │
     │         Shared SRAM (MCUSRAM)      │
     │    ┌────────────────────────┐      │
     ├───►│  RPMsg VirtIO Ring     │◄─────┤
     │    │  (Tx buffer: 512B x32)│      │
     │    │  (Rx buffer: 512B x32)│      │
     │    └────────────────────────┘      │
     │                                    │
     │    ┌────────────────────────┐      │
     ├───►│  Resource Table        │◄─────┤
     │    │  (M4 firmware desc)   │      │
     │    └────────────────────────┘      │
     │                                    │
  rpmsg_char              STM32 OpenAMP
  /dev/ttyRPMSG0          HAL driver
```

**RPMsg Performance:**
- Message latency: ~50 µs (M4 → A7 via shared SRAM)
- Throughput: ~10 MB/s
- Message size: Up to 512 bytes per RPMsg frame
- No data copy on M4 side (zero-copy to shared SRAM)

### 4.3 Peripheral Assignment

| Peripheral | Assigned To | Purpose |
|------------|-------------|---------|
| SPI1 | Cortex-M4 | ADC (ADS1263/ADS1258) — deterministic |
| SPI3 | Cortex-M4 | SD card (Solar SMU) — data logging |
| USART2 | Cortex-M4 | RS-485 transceiver — legacy Modbus |
| I2C1 | Cortex-M4 | TMP117 temperature sensor |
| TIM2 | Cortex-M4 | ADC sampling trigger (1 kHz) |
| GPIO (PC0-PC9) | Cortex-M4 | MUX select lines (Solar SMU) |
| GPIO (PD12-PD15) | Cortex-M4 | Status LEDs |
| ETH1 (RGMII) | Cortex-A7 | Gigabit Ethernet + DDS + PTP |
| USB OTG | Cortex-A7 | Debug, OTA updates |
| FDCAN1 | Cortex-A7 | CAN FD (optional industrial bus) |
| SDMMC2 | Cortex-A7 | eMMC boot storage |
| UART4 | Cortex-A7 | Linux console / debug |

---

## 5. DDS Middleware Selection

### 5.1 Comparison for STM32MP157

| Feature | eProsima Fast DDS | Eclipse CycloneDDS | RTI Connext Micro |
|---------|-------------------|--------------------|--------------------|
| License | Apache 2.0 (free) | EPL 2.0 (free) | Commercial (~$5K/yr) |
| Language | C++ | C | C |
| Min RAM (Linux) | ~32 MB | ~20 MB | ~100 KB (RTOS) |
| Discovery | Full RTPS | Full RTPS | Full RTPS |
| QoS Policies | All standard | All standard | Subset (deterministic) |
| Security | DDS Security plugin | DDS Security plugin | Built-in |
| ROS 2 Compatible | Yes (default RMW) | Yes (alternate RMW) | Yes (RMW available) |
| Indian Support | Community | Community | RTI India office |
| **Recommendation** | **Primary choice** | Backup option | If budget permits |

**Selected: eProsima Fast DDS** (Cortex-A7) + **Micro XRCE-DDS** (Cortex-M4)

### 5.2 Cortex-M4: Micro XRCE-DDS Client

The M4 core runs a lightweight **Micro XRCE-DDS client** (~75 KB Flash, ~2.5 KB RAM) that:
- Serializes sensor data into CDR format
- Publishes via RPMsg transport to the XRCE Agent on A7
- Subscribes to control/configuration topics from A7
- Operates with zero dynamic memory allocation (deterministic)

```
M4 XRCE Client                         A7 XRCE Agent + Fast DDS
┌─────────────┐                        ┌─────────────────────────┐
│ create_topic │   RPMsg Transport      │ Agent receives CDR data │
│ create_pub   │──────────────────────►│ Publishes to DDS domain │
│ write_data   │                        │ via Ethernet/UDP        │
│ (CDR encoded)│                        │                         │
│              │◄──────────────────────│ Subscribes & forwards   │
│ read_data    │   RPMsg Transport      │ control commands to M4  │
└─────────────┘                        └─────────────────────────┘
```

### 5.3 Resource Usage on STM32MP157

| Resource | Cortex-A7 (Linux) | Cortex-M4 (FreeRTOS) |
|----------|--------------------|-----------------------|
| RAM | ~64 MB (Linux + DDS) | ~40 KB (FreeRTOS + XRCE + ADC) |
| Flash/Storage | ~200 MB (eMMC) | ~256 KB (M4 firmware in DDR) |
| CPU Load | ~15% (DDS idle), ~40% (peak) | ~30% (sampling + filtering) |
| Available | 192 MB free (of 256 MB DDR) | 88 KB free (of 128 KB SRAM) |

---

## 6. DDS Topic Architecture for SCADA

### 6.1 Topic Definitions

**Solar SMU Topics:**

| Topic Name | Type | Direction | QoS | Rate |
|------------|------|-----------|-----|------|
| `solar/string_voltage` | StringVoltageData | Pub | RELIABLE, Deadline=100ms | 1 Hz |
| `solar/string_current` | StringCurrentData | Pub | RELIABLE, Deadline=100ms | 1 Hz |
| `solar/bus_power` | BusPowerData | Pub | RELIABLE, Deadline=200ms | 1 Hz |
| `solar/irradiance` | IrradianceData | Pub | RELIABLE, Deadline=1s | 0.5 Hz |
| `solar/module_temp` | TemperatureData | Pub | RELIABLE, Deadline=5s | 0.2 Hz |
| `solar/alarm` | AlarmEvent | Pub | RELIABLE, Durability=TRANSIENT | Event |
| `solar/config` | ConfigCommand | Sub | RELIABLE | On-demand |
| `solar/heartbeat` | HeartbeatData | Pub | BEST_EFFORT, Lifespan=5s | 1 Hz |

**Water RTU Topics:**

| Topic Name | Type | Direction | QoS | Rate |
|------------|------|-----------|-----|------|
| `water/level` | WaterLevelData | Pub | RELIABLE, Deadline=200ms | 1 Hz |
| `water/flow` | FlowRateData | Pub | RELIABLE, Deadline=200ms | 1 Hz |
| `water/pressure` | PressureData | Pub | RELIABLE, Deadline=500ms | 0.5 Hz |
| `water/quality` | WaterQualityData | Pub | RELIABLE, Deadline=1s | 0.2 Hz |
| `water/pump_status` | PumpStatusData | Pub | RELIABLE | Event |
| `water/alarm` | AlarmEvent | Pub | RELIABLE, Durability=TRANSIENT | Event |
| `water/config` | ConfigCommand | Sub | RELIABLE | On-demand |
| `water/heartbeat` | HeartbeatData | Pub | BEST_EFFORT, Lifespan=5s | 1 Hz |

### 6.2 IDL Data Type Definitions

```idl
// Common types
struct Timestamp {
    int32 sec;         // Unix epoch seconds
    uint32 nanosec;    // Nanosecond fraction (PTP synced)
};

struct DeviceIdentity {
    string<32> device_id;     // e.g., "SMU-PLANT01-STR04"
    string<16> device_type;   // "SOLAR_SMU" or "WATER_RTU"
    string<16> firmware_ver;  // e.g., "2.1.0"
    string<32> location;      // GPS or site ID
};

// Solar SMU Data Types
struct StringVoltageData {
    DeviceIdentity device;
    Timestamp      timestamp;
    uint8          string_count;       // 1-16
    float          voltages[16];       // Volts DC (0-1000V)
    float          voltage_accuracy;   // ±V
    uint16         status_flags;       // Bit-field: OV, UV, fault
};

struct StringCurrentData {
    DeviceIdentity device;
    Timestamp      timestamp;
    uint8          string_count;
    float          currents[16];       // Amps DC (0-15A)
    float          current_accuracy;   // ±A
    uint16         status_flags;
};

struct BusPowerData {
    DeviceIdentity device;
    Timestamp      timestamp;
    float          bus_voltage;        // V DC (0-1000V)
    float          bus_current;        // A DC (0-200A)
    float          total_power_kw;     // kW
    float          energy_kwh;         // kWh cumulative
};

// Water RTU Data Types
struct WaterLevelData {
    DeviceIdentity device;
    Timestamp      timestamp;
    uint8          channel_count;      // 1-8
    float          levels_m[8];        // Meters
    float          raw_ma[8];          // 4-20 mA raw
    uint16         status_flags;
};

struct AlarmEvent {
    DeviceIdentity device;
    Timestamp      timestamp;
    uint16         alarm_code;         // Alarm ID
    uint8          severity;           // 1=Info, 2=Warning, 3=Critical
    string<128>    description;        // Human-readable
    float          value;              // Measured value
    float          threshold;          // Alarm threshold
};
```

### 6.3 DDS Domain Architecture

```
DDS Domain ID: 0 (default)

  Partition: "Solar/Plant_Alpha"
  ┌──────────────────────────────────────────────┐
  │  SMU-01 (Pub)   SMU-02 (Pub)   SMU-N (Pub)  │
  │  16 strings     16 strings     16 strings    │
  │  Topics: solar/string_voltage                │
  │          solar/string_current                │
  │          solar/bus_power                     │
  │          solar/irradiance                    │
  │          solar/alarm                         │
  └──────────────────────┬───────────────────────┘
                         │ Gigabit Ethernet
                         │ (UDP Multicast or Unicast)
  ┌──────────────────────┴───────────────────────┐
  │            SCADA Gateway / HMI               │
  │            (Fast DDS Subscriber)             │
  │                                              │
  │  ┌──────────────┐  ┌──────────────────────┐ │
  │  │ DDS→OPC UA   │  │ DDS→Historian       │ │
  │  │ Gateway      │  │ (InfluxDB/TimescaleDB│ │
  │  └──────────────┘  └──────────────────────┘ │
  │  ┌──────────────┐  ┌──────────────────────┐ │
  │  │ DDS→Modbus   │  │ Web Dashboard       │ │
  │  │ Gateway      │  │ (Grafana)           │ │
  │  │ (backward)   │  │                     │ │
  │  └──────────────┘  └──────────────────────┘ │
  └──────────────────────────────────────────────┘

  Partition: "Water/District_07"
  ┌──────────────────────────────────────────────┐
  │  RTU-01 (Pub)   RTU-02 (Pub)   RTU-N (Pub)  │
  │  8 channels     8 channels     8 channels   │
  │  Topics: water/level                        │
  │          water/flow                         │
  │          water/pressure                     │
  │          water/alarm                        │
  └──────────────────────────────────────────────┘
```

---

## 7. Hardware Design Changes

### 7.1 Components Removed (vs. STM32F407 Design)

| Component | Reason |
|-----------|--------|
| STM32F407VGT6 | Replaced by STM32MP157CAC3 |
| W5500 Ethernet controller | Not needed — MP157 has built-in Gigabit MAC |
| Y3 (25 MHz crystal for W5500) | Not needed |
| FB2, C_W5500_* (W5500 decoupling) | Not needed |
| R_SPI2 (W5500 SPI damping) | Not needed |
| R_TX, R_EXRES (W5500 bias) | Not needed |

### 7.2 Components Added

| Component | Part Number | Purpose | Price (INR) |
|-----------|-------------|---------|-------------|
| STM32MP157CAC3 | STM32MP157CAC3 | Main MPU (replaces F407) | ₹850 |
| STPMIC1A | STPMIC1APQR | Power management IC | ₹350 |
| DDR3L 256MB (16-bit) | MT41K128M16JT-125 | System memory for Linux | ₹450 |
| eMMC 4GB | MTFC4GACAJCN-4M IT | Linux rootfs + data | ₹550 |
| KSZ9031RNXIA | KSZ9031RNXIA-TR | Gigabit Ethernet PHY | ₹250 |
| RJ45 + Magnetics | HR911105A | Ethernet connector (both boards) | ₹120 |
| 24 MHz crystal | ABM3B-24.000MHZ | HSE for MP157 (not 8 MHz) | ₹25 |
| STPMIC1 inductors (x4) | Various (2.2µH, 4.7µH) | Buck regulator inductors | ₹120 |
| STPMIC1 decoupling | Various MLCC | Per datasheet | ₹80 |
| DDR3L decoupling | 100nF x8, 10µF x2 | DDR power filtering | ₹30 |
| DDR3L termination | VTT resistors (22Ω x4) | Signal integrity | ₹10 |
| eMMC decoupling | 100nF + 10µF | eMMC supply | ₹10 |
| Ethernet PHY decoupling | 100nF x4, 10µF x2 | PHY supply | ₹20 |
| USB Type-C connector | USB4085-GF-A | OTA / debug | ₹35 |
| 25 MHz crystal (PHY) | ABM3B-25.000MHZ | Ethernet PHY clock | ₹25 |

### 7.3 PCB Impact

| Parameter | STM32F407 (Current) | STM32MP157 (DDS) |
|-----------|--------------------|--------------------|
| Layer count | 4 | **6 (minimum)** |
| Board size — Solar SMU | 160 x 100 mm | **170 x 110 mm** |
| Board size — Water RTU | 120 x 80 mm | **140 x 100 mm** |
| DDR routing | None | **16-bit DDR3L, matched-length** |
| BGA escape | None (LQFP-100) | **361-ball TFBGA, via-in-pad** |
| Impedance control | 50Ω single, 100Ω diff | **+ 100Ω DDR3L differential** |
| Stackup | Sig-GND-PWR-Sig | **Sig-GND-Sig-PWR-GND-Sig** |
| Minimum trace/space | 0.15/0.15 mm | **0.1/0.1 mm (BGA escape)** |
| Via type | Standard (0.3mm) | **+ micro-via (0.1mm laser)** |

---

## 8. Revised BOM — Solar SMU (DDS Variant)

### 8.1 Major Component Changes

| Component | Old (F407) | New (MP157) | Old Price (INR) | New Price (INR) |
|-----------|-----------|-------------|-----------------|-----------------|
| MCU/MPU | STM32F407VGT6 | STM32MP157CAC3 | ₹1,080 | ₹850 |
| PMIC | AMS1117-3.3 only | STPMIC1A + AMS1117 | ₹15 | ₹365 |
| Memory | None (internal) | DDR3L 256MB + eMMC 4GB | ₹0 | ₹1,000 |
| Ethernet PHY | None (W5500) | KSZ9031RNXIA | ₹250 | ₹250 |
| HSE Crystal | 8 MHz | 24 MHz | ₹25 | ₹25 |
| ETH Crystal | 25 MHz (W5500) | 25 MHz (PHY) | ₹25 | ₹25 |
| USB Connector | None | USB Type-C | ₹0 | ₹35 |
| PMIC Inductors | None | 4x (2.2µH, 4.7µH) | ₹0 | ₹120 |
| PMIC Capacitors | None | ~20 extra MLCC | ₹0 | ₹80 |
| DDR Passives | None | Termination + decoupling | ₹0 | ₹40 |

### 8.2 Solar SMU DDS BOM Summary

| Category | Old (F407) INR | New (MP157) INR | Delta |
|----------|----------------|-----------------|-------|
| Active ICs | ₹4,375 | ₹4,820 | +₹445 |
| Discrete Semiconductors | ₹464 | ₹464 | ₹0 |
| Passive Components | ₹1,166 | ₹1,436 | +₹270 |
| Memory (DDR3L + eMMC) | ₹0 | ₹1,000 | +₹1,000 |
| Magnetics (+ PMIC inductors) | ₹93 | ₹213 | +₹120 |
| Crystals | ₹45 | ₹70 | +₹25 |
| Connectors (+ USB-C + RJ45) | ₹1,208 | ₹1,363 | +₹155 |
| **Component BOM Total** | **₹7,351** | **₹9,366** | **+₹2,015** |
| Procurement overhead (+15%) | ₹1,103 | ₹1,405 | +₹302 |
| **Landed BOM Cost** | **₹8,454** | **₹10,771** | **+₹2,317** |

---

## 9. Revised BOM — Water RTU (DDS Variant)

### 9.1 Water RTU DDS BOM Summary

| Category | Old (F407) INR | New (MP157) INR | Delta |
|----------|----------------|-----------------|-------|
| Active ICs | ₹2,705 | ₹2,843 | +₹138 |
| Discrete Semiconductors | ₹304 | ₹304 | ₹0 |
| Passive Components | ₹378 | ₹598 | +₹220 |
| Memory (DDR3L + eMMC) | ₹0 | ₹1,000 | +₹1,000 |
| Magnetics, Crystals, Connectors | ₹1,004 | ₹1,239 | +₹235 |
| **Component BOM Total** | **₹4,391** | **₹5,984** | **+₹1,593** |
| Procurement overhead (+15%) | ₹659 | ₹898 | +₹239 |
| **Landed BOM Cost** | **₹5,050** | **₹6,882** | **+₹1,832** |

> **Note:** The Water RTU DDS variant *removes* the W5500 Ethernet controller (~₹250) + its decoupling and bias components, since the MP157 has a built-in Gigabit Ethernet MAC. The Gigabit PHY (KSZ9031) replaces it at similar cost but with 10x bandwidth.

---

## 10. Revised Cost Estimation

### 10.1 Per-Unit Cost Comparison (at 100 units)

| Cost Element | Solar SMU (F407) | Solar SMU (MP157 DDS) | Delta |
|-------------|------------------|-----------------------|-------|
| Component BOM | ₹8,454 | ₹10,771 | +₹2,317 |
| PCB fabrication (6-layer) | ₹550 | ₹950 | +₹400 |
| SMT assembly (BGA) | ₹900 | ₹1,800 | +₹900 |
| Testing & calibration | ₹625 | ₹900 | +₹275 |
| Enclosure | ₹450 | ₹450 | ₹0 |
| Packaging | ₹200 | ₹200 | ₹0 |
| **Per-unit total** | **₹11,175** | **₹15,071** | **+₹3,896** |
| **Per-unit (USD)** | **$131** | **$177** | **+$46** |

| Cost Element | Water RTU (F407) | Water RTU (MP157 DDS) | Delta |
|-------------|------------------|-----------------------|-------|
| Component BOM | ₹5,050 | ₹6,882 | +₹1,832 |
| PCB fabrication (6-layer) | ₹400 | ₹750 | +₹350 |
| SMT assembly (BGA) | ₹800 | ₹1,500 | +₹700 |
| Testing & calibration | ₹585 | ₹850 | +₹265 |
| Enclosure | ₹350 | ₹350 | ₹0 |
| Packaging | ₹200 | ₹200 | ₹0 |
| **Per-unit total** | **₹7,385** | **₹10,532** | **+₹3,147** |
| **Per-unit (USD)** | **$87** | **$124** | **+$37** |

### 10.2 Additional NRE for DDS Variant

| Item | Cost (INR) | Notes |
|------|------------|-------|
| PCB re-design (6-layer, DDR3L routing) | ₹2,50,000 | Per board variant |
| DDR3L signal integrity simulation | ₹1,00,000 | HyperLynx / SIwave |
| Linux BSP bring-up (OpenSTLinux/Yocto) | ₹3,00,000 | Kernel, device tree, drivers |
| Fast DDS integration & tuning | ₹2,00,000 | Topic design, QoS tuning |
| XRCE-DDS M4 firmware development | ₹1,50,000 | FreeRTOS + OpenAMP + XRCE |
| Modbus-DDS gateway development | ₹1,00,000 | Backward compatibility |
| PTP/IEEE 1588 integration | ₹80,000 | Time synchronization |
| DDS security configuration | ₹80,000 | TLS, authentication, access control |
| System integration testing | ₹1,50,000 | End-to-end, latency verification |
| **Total additional NRE** | **₹13,10,000** | ~$15,412 |

### 10.3 Combined NRE (F407 base + DDS upgrade)

| Item | Cost (INR) |
|------|------------|
| Original NRE (Section 8.3 of cost report) | ₹33,31,000 |
| DDS upgrade NRE | ₹13,10,000 |
| **Total NRE** | **₹46,41,000** (~$54,600) |

### 10.4 PCB Fabrication Cost (6-Layer, DDS Variant)

**Solar SMU — 6-layer, 170x110mm, 1.6mm, FR-4, ENIG**

| Quantity | Per Board (INR) | Notes |
|----------|-----------------|-------|
| 10 pcs (prototype) | ₹2,200 | Via-in-pad, micro-via |
| 50 pcs | ₹1,400 | |
| 100 pcs | ₹950 | |
| 500 pcs | ₹600 | |
| 1000 pcs | ₹450 | |

**Water RTU — 6-layer, 140x100mm, 1.6mm, FR-4, ENIG**

| Quantity | Per Board (INR) | Notes |
|----------|-----------------|-------|
| 10 pcs (prototype) | ₹1,800 | |
| 50 pcs | ₹1,100 | |
| 100 pcs | ₹750 | |
| 500 pcs | ₹480 | |
| 1000 pcs | ₹380 | |

> **ENIG finish is mandatory** for 0.5mm pitch BGA (TFBGA-361) and DDR3L BGA. HASL is not suitable.

### 10.5 Assembly Cost Impact

| Factor | STM32F407 Board | STM32MP157 Board | Impact |
|--------|----------------|-------------------|--------|
| BGA placement | None (LQFP only) | 361-ball TFBGA + DDR3L BGA | +₹300 (BGA reflow profile) |
| X-ray inspection | Not needed | Required (BGA solder joints) | +₹150 per board |
| DDR3L memory | Not present | BGA memory, critical placement | +₹100 |
| Component count | ~120 (SMU), ~95 (RTU) | ~150 (SMU), ~125 (RTU) | +₹200 |
| Reflow complexity | Single profile | Dual-zone (BGA + standard) | +₹100 |
| **Assembly premium** | — | **+₹850 to ₹900 per board** | |

---

## 11. Software Architecture

### 11.1 Cortex-A7 Software Stack

```
┌──────────────────────────────────────────┐
│              User Space                  │
│                                          │
│  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ Fast DDS │  │ Modbus   │  │ OPC UA │ │
│  │ daemon   │  │ Gateway  │  │ Server │ │
│  │ (C++)    │  │ (Python) │  │(opt.)  │ │
│  └──────────┘  └──────────┘  └────────┘ │
│  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ XRCE-DDS │  │ MQTT     │  │ Web UI │ │
│  │ Agent    │  │ Bridge   │  │ (opt.) │ │
│  │          │  │ (opt.)   │  │        │ │
│  └──────────┘  └──────────┘  └────────┘ │
│                                          │
│  ┌───────────────────────────────────┐   │
│  │  OpenAMP/RPMsg User-space driver  │   │
│  │  /dev/ttyRPMSG0                   │   │
│  └───────────────────────────────────┘   │
├──────────────────────────────────────────┤
│              Kernel Space                │
│                                          │
│  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ stmmac   │  │ rpmsg    │  │ PTP    │ │
│  │ Ethernet │  │ driver   │  │ (ptp4l)│ │
│  │ driver   │  │          │  │        │ │
│  └──────────┘  └──────────┘  └────────┘ │
│                                          │
│  Linux 5.15+ (RT_PREEMPT patch)          │
│  OpenSTLinux / Yocto (Kirkstone+)        │
├──────────────────────────────────────────┤
│  U-Boot → TF-A (Trusted Firmware-A)      │
│  Boot: eMMC → DDR3L init → Linux         │
└──────────────────────────────────────────┘
```

### 11.2 Cortex-M4 Software Stack

```
┌──────────────────────────────────────────┐
│           Application Layer              │
│                                          │
│  ┌──────────────────────────────────┐    │
│  │  Sensor Acquisition Task         │    │
│  │  - ADC read (SPI DMA)           │    │
│  │  - MUX channel cycling          │    │
│  │  - Calibration + filtering      │    │
│  │  - Alarm threshold check        │    │
│  │  Priority: configTIMER (highest)│    │
│  └──────────────────────────────────┘    │
│  ┌──────────────────────────────────┐    │
│  │  XRCE-DDS Publisher Task         │    │
│  │  - Serialize sensor data (CDR)  │    │
│  │  - Publish via RPMsg transport  │    │
│  │  - Subscribe to config commands │    │
│  │  Priority: configTIMER - 1      │    │
│  └──────────────────────────────────┘    │
│  ┌──────────────────────────────────┐    │
│  │  Legacy Modbus Task (optional)   │    │
│  │  - RS-485 Modbus RTU slave      │    │
│  │  - Register map compatibility   │    │
│  │  Priority: configTIMER - 2      │    │
│  └──────────────────────────────────┘    │
├──────────────────────────────────────────┤
│  FreeRTOS v10.5+ (or bare-metal HAL)     │
│  STM32Cube HAL + OpenAMP middleware      │
│  Micro XRCE-DDS Client Library           │
└──────────────────────────────────────────┘
```

### 11.3 Boot Sequence

```
Power ON
  │
  ├─ STPMIC1A sequences power rails
  │   VDDCORE(1.2V) → VDD_DDR(1.35V) → VDD(3.3V)
  │
  ├─ ROM Bootloader (A7)
  │   Reads boot pins → selects eMMC
  │
  ├─ TF-A (Trusted Firmware-A) — BL2
  │   DDR3L controller init (timing calibration)
  │   STPMIC1A runtime configuration
  │
  ├─ U-Boot — BL33
  │   Load Linux kernel + device tree
  │   Load M4 firmware to DDR/SRAM
  │
  ├─ Linux Kernel Boot (A7)
  │   stmmac Ethernet driver
  │   PTP clock init
  │   RPMsg/OpenAMP subsystem
  │   Start M4 coprocessor (remoteproc)
  │
  ├─ M4 Firmware Start
  │   FreeRTOS init
  │   OpenAMP/RPMsg endpoint create
  │   ADC init (SPI, DMA)
  │   XRCE-DDS client create
  │   Start sensor acquisition loop
  │
  └─ User-space Services (A7)
      Fast DDS daemon start
      XRCE-DDS Agent start
      Modbus gateway (optional)
      PTP daemon (ptp4l + phc2sys)
```

---

## 12. Determinism Analysis

### 12.1 Hard Real-Time Guarantees (<1 ms)

The Cortex-M4 core provides hard real-time determinism because:

| Factor | Guarantee |
|--------|-----------|
| Dedicated core | M4 runs independently from Linux on A7 |
| TCM memory | 64 KB Tightly Coupled Memory (0 wait-state) |
| No OS jitter | FreeRTOS tick < 1 µs jitter |
| DMA transfers | SPI ADC reads via DMA (zero CPU involvement) |
| Priority inversion | FreeRTOS mutex with priority inheritance |
| Interrupt latency | < 12 cycles (~57 ns @ 209 MHz) |
| No cache effects | TCM is non-cacheable (deterministic access) |

### 12.2 Timing Budget (ADC Sample → DDS Publish)

```
Event                          Time         Cumulative
─────────────────────────────────────────────────────────
TIM2 interrupt fires           0 µs         0 µs
MUX channel select (GPIO)      2 µs         2 µs
ADC start conversion (SPI cmd) 5 µs         7 µs
ADC conversion time (ADS1263)  ~260 µs      267 µs
SPI DMA read result            10 µs        277 µs
Calibration + IIR filter       15 µs        292 µs
PTP timestamp capture          2 µs         294 µs
CDR serialization (XRCE)       20 µs        314 µs
RPMsg write to shared SRAM     5 µs         319 µs
RPMsg interrupt to A7          1 µs         320 µs
────────────────── M4 path complete ──────────────────
A7 XRCE Agent wakeup           50 µs        370 µs
DDS publish (UDP sendto)       100 µs       470 µs
Ethernet PHY transmit          10 µs        480 µs
────────────────── End-to-end ───────────────────────
TOTAL                                       ~480 µs
                                            (< 1 ms ✓)
```

### 12.3 Worst-Case Latency Analysis

| Path | Typical | Worst Case | Meets <1ms? |
|------|---------|------------|-------------|
| M4 ADC sample → RPMsg | 320 µs | 450 µs | Yes (M4 isolated) |
| RPMsg → DDS publish | 160 µs | 500 µs | — |
| **End-to-end (M4 → wire)** | **480 µs** | **950 µs** | **Yes** |
| Network propagation (1 hop) | 50 µs | 200 µs | — |
| **Full round-trip** | **530 µs** | **1,150 µs** | Marginal |

> **Note:** The <1 ms guarantee applies to the M4 sensing path. The DDS publish (A7 side) adds variable latency due to Linux scheduling. With RT_PREEMPT kernel, the A7 path is typically <500 µs but not hard-guaranteed. For guaranteed <1 ms wire-to-wire, use PTP timestamps on the M4 side.

### 12.4 IEEE 1588 PTP Time Synchronization

```
        ┌─────────────┐          ┌─────────────┐
        │  PTP Master  │          │  PTP Slave   │
        │  (Gateway)   │          │  (RTU/SMU)   │
        │              │  Sync    │              │
        │  ptp4l       │─────────►│  ptp4l       │
        │  phc2sys     │          │  phc2sys     │
        │              │◄─────────│              │
        │              │ Delay_Req│              │
        └─────────────┘          └─────────────┘

Synchronization Accuracy:
  - PTP over Gigabit Ethernet: < 1 µs
  - STM32MP157 HW timestamping: < 100 ns
  - phc2sys (PHC → system clock): < 500 ns
  - M4 timestamp via shared PTP counter: < 1 µs
```

---

## 13. Migration Path from Modbus to DDS

### 13.1 Backward Compatibility Strategy

The DDS variant maintains **full Modbus backward compatibility** via a gateway service:

```
                    STM32MP157
  ┌─────────────────────────────────────────┐
  │                                         │
  │  ┌─────────────────┐                   │
  │  │ Fast DDS        │──── DDS Topics ──►  New SCADA systems
  │  │ (Primary)       │     (Ethernet)       (DDS subscribers)
  │  └────────┬────────┘                   │
  │           │                            │
  │  ┌────────┴────────┐                   │
  │  │ Modbus-DDS      │                   │
  │  │ Gateway         │                   │
  │  │ (maps DDS ↔     │                   │
  │  │  Modbus regs)   │                   │
  │  └────────┬────────┘                   │
  │           │                            │
  │  ┌────────┴────────┐  ┌────────────┐  │
  │  │ Modbus TCP      │  │ Modbus RTU │  │
  │  │ Server          │  │ Slave (M4) │  │
  │  │ (Port 502)      │  │ (RS-485)   │  │
  │  └─────────────────┘  └────────────┘  │
  │                                        │
  └────────────────────────────────────────┘
         │                      │
    Modbus TCP              Modbus RTU
    (Ethernet)              (RS-485)
         │                      │
    Legacy SCADA            Legacy Master
    systems                 stations
```

### 13.2 Phased Migration Plan

| Phase | Duration | Scope | DDS Feature |
|-------|----------|-------|-------------|
| Phase 0 | Current | STM32F407 + Modbus only | None |
| Phase 1 | 3 months | MP157 hardware, Modbus + DDS dual-stack | DDS topics + Modbus gateway |
| Phase 2 | 6 months | DDS primary, Modbus secondary | Full QoS, PTP sync, discovery |
| Phase 3 | 12 months | DDS-only new installations | DDS Security, OPC UA bridge |
| Phase 4 | 18 months | Deprecate Modbus on new units | Edge analytics, AI/ML |

---

## 14. Alternative Platforms Considered

### 14.1 Platform Comparison Summary

| Feature | STM32MP157 (Selected) | TI AM6442 | TI AM243x | NXP i.MX 8M Plus | STM32H745 |
|---------|----------------------|-----------|-----------|-------------------|-----------|
| DDS Support | Full (A7) + XRCE (M4) | Full (A53) + Micro (R5F) | XRCE only (R5F) | Full (A53) + XRCE (M7) | XRCE only (M7) |
| Hard RT Core | M4 @ 209 MHz | 4x R5F @ 800 MHz | 4x R5F @ 800 MHz | M7 @ 800 MHz | M7 @ 480 MHz |
| TSN | PTP + SW TSN | Native HW TSN | Native HW TSN | 1x HW TSN port | PTP only |
| Industrial Eth | CAN FD | EtherCAT, PROFINET | EtherCAT, PROFINET | CAN FD | CAN FD |
| DDR Required | Yes (DDR3L) | Yes (DDR4) | No (2MB SRAM) | Yes (LPDDR4) | No (1MB SRAM) |
| PCB Layers | 6 | 8+ | 4-6 | 8+ | 4 |
| IC Cost (INR) | **₹850** | ₹2,000 | ₹1,500 | ₹3,000 | ₹1,200 |
| BOM Cost (INR) | **₹10,771** | ₹14,000 | ₹7,500 | ₹16,000 | ₹6,500 |
| Linux | Yes | Yes | No | Yes | No |
| Ecosystem | Excellent (ST) | Good (TI) | Good (TI) | Good (NXP) | Excellent (ST) |

### 14.2 Why STM32MP157 Was Selected

1. **Best cost/feature ratio** — Full DDS capability at ₹850/chip (vs ₹2,000+ for TI AM64x)
2. **ST ecosystem continuity** — Team already familiar with STM32 tools (CubeMX, CubeIDE)
3. **Sufficient for SCADA** — Dual A7 + M4 provides adequate compute for DDS + hard RT I/O
4. **Lower PCB complexity** — 6-layer PCB (vs 8+ for AM64x or i.MX 8M Plus)
5. **DDR3L (not DDR4)** — Simpler routing, lower cost memory
6. **Gigabit Ethernet + PTP** — Adequate for SCADA networking (TSN scheduling not critical at 1 Hz data rates)
7. **CAN FD** — Additional industrial bus option
8. **OpenAMP/RPMsg** — Mature inter-processor communication
9. **OpenSTLinux** — ST-maintained Yocto BSP with long-term support
10. **India availability** — In stock at DigiKey India, Mouser India, and Semikart

### 14.3 When to Choose a Different Platform

| If you need... | Choose | Reason |
|----------------|--------|--------|
| Native EtherCAT/PROFINET | TI AM6442 | PRU-ICSSG with certified stacks |
| Hardware TSN (802.1Qbv) | TI AM6442 or AM243x | Only platforms with HW TSN |
| Edge AI/ML inference | NXP i.MX 8M Plus | Built-in 2.3 TOPS NPU |
| Lowest BOM cost (no DDS) | STM32H745 | No DDR, LQFP, 1MB SRAM, DDS-XRCE only |
| FPGA determinism | Xilinx Zynq-7000 | Hardware-level protocol implementation |
| No DDR + full RT + industrial | TI AM243x | 2MB SRAM, R5F cores, no Linux |

---

## Cost Impact Summary Dashboard

```
┌────────────────────────────────────────────────────────────────────┐
│         DDS Upgrade Cost Impact (100 units, INR)                   │
├───────────────────────┬──────────────┬──────────────┬──────────────┤
│                       │ F407 (Modbus)│ MP157 (DDS)  │   Delta      │
├───────────────────────┼──────────────┼──────────────┼──────────────┤
│ Solar SMU per-unit    │   ₹11,175    │   ₹15,071    │  +₹3,896     │
│ Water RTU per-unit    │   ₹7,385     │   ₹10,532    │  +₹3,147     │
│ Per-unit avg (USD)    │   $109       │   $151       │  +$42        │
├───────────────────────┼──────────────┼──────────────┼──────────────┤
│ Total NRE             │   ₹33.3L     │   ₹46.4L     │  +₹13.1L     │
│ Total NRE (USD)       │   $39,188    │   $54,600    │  +$15,412    │
├───────────────────────┼──────────────┼──────────────┼──────────────┤
│ VALUE ADDED:                                                       │
│  + Full DDS pub-sub (automatic discovery, QoS, security)          │
│  + Hard real-time <1ms (isolated Cortex-M4)                       │
│  + IEEE 1588 PTP time sync (<1 µs)                                │
│  + Gigabit Ethernet (10x bandwidth vs W5500)                      │
│  + Linux edge computing (OTA, analytics, OPC UA)                  │
│  + Modbus backward compatibility (gateway)                        │
│  + CAN FD industrial bus                                          │
│  + DDS Security (authentication, encryption)                      │
│  + Scalable to thousands of nodes                                 │
├───────────────────────┴──────────────┴──────────────┴──────────────┤
│ COMPETING DDS-CAPABLE RTUs: ₹1,00,000 - ₹3,00,000               │
│ OUR PRICE ADVANTAGE: 7-20x cheaper                                │
│ BREAK-EVEN (additional NRE): ~370 units                           │
└────────────────────────────────────────────────────────────────────┘
```

---

*Document based on STM32MP157 reference design (AN5031), eProsima Fast DDS documentation, DDS-XRCE specification (OMG), and Indian market pricing as of March 2026.*

*Key References:*
- *[ST AN5031 — STM32MP1 Hardware Development](https://www.st.com/resource/en/application_note/an5031)*
- *[ST AN5122 — DDR Memory Routing Guidelines](https://www.st.com/resource/en/application_note/an5122)*
- *[eProsima Fast DDS](https://www.eprosima.com/middleware/fast-dds)*
- *[eProsima Micro XRCE-DDS](https://micro-xrce-dds.docs.eprosima.com)*
- *[STM32MP157 Product Page](https://www.st.com/en/microcontrollers-microprocessors/stm32mp157c.html)*
- *[DigiKey India — STM32MP157CAC3](https://www.digikey.in/en/products/detail/stmicroelectronics/STM32MP157CAC3/10326844)*
