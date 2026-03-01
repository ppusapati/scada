# SCADA System Architecture

## 1. System Block Diagram

```
                              ┌──────────────────────────────────────────────────────────────┐
                              │                    SCADA Control Room                         │
                              │                                                              │
                              │  ┌─────────────┐   ┌────────────────┐   ┌───────────────┐   │
                              │  │   SvelteKit  │   │   Go Backend   │   │  PostgreSQL   │   │
                              │  │   HMI        │◄─►│   Server       │◄─►│  TimescaleDB  │   │
                              │  │   Dashboard  │WS │   REST+WS+gRPC │   │               │   │
                              │  └─────────────┘   └───────┬────────┘   └───────────────┘   │
                              │                            │                                 │
                              │                    ┌───────┴────────┐                        │
                              │                    │   Mosquitto     │                        │
                              │                    │   MQTT Broker   │                        │
                              │                    │   (port 1883)   │                        │
                              │                    └───────┬────────┘                        │
                              └────────────────────────────┼────────────────────────────────┘
                                                           │
                              ─────────────────────────────┼──────────────────────────────────
                                     Ethernet / RS-485 Field Network
                              ─────────────────────────────┼──────────────────────────────────
                                                           │
                  ┌────────────────────────────┬───────────┴──────────┬────────────────────┐
                  │                            │                      │                    │
     ┌────────────┴──────────┐   ┌─────────────┴────────┐  ┌─────────┴────────┐          │
     │   Solar SMU            │   │   Solar SMU           │  │   Water RTU       │         ...
     │   SS-PCB-001           │   │   SS-PCB-001          │  │   WR-PCB-001      │
     │   (String Monitor)     │   │   (String Monitor)    │  │   (Sensor Hub)    │
     │                        │   │                       │  │                   │
     │   16 String Voltages   │   │   16 String Voltages  │  │   8x 4-20mA      │
     │   16 String Currents   │   │   16 String Currents  │  │   Pressure        │
     │   Bus V/I              │   │   Bus V/I             │  │   pH / Turbidity  │
     │   Irradiance           │   │   Irradiance          │  │   Chlorine / DO   │
     │   Temperature          │   │   Temperature         │  │   Flow / Level    │
     │                        │   │                       │  │                   │
     │   RS-485 Modbus RTU    │   │   RS-485 Modbus RTU   │  │   RS-485 + TCP    │
     └────────────────────────┘   └───────────────────────┘  └───────────────────┘
```

## 2. Solar SMU (SS-PCB-001) Block Diagram

```
     String 1-16 Voltage  ──────────►┌─────────────┐
     (0-1000V via dividers)           │ CD74HC4067  │───────► AIN0
                                      │  MUX A      │         ┌──────────────────────┐
     String 1-16 Current  ──────────►│ CD74HC4067  │───────► │                      │
     (50mV shunt resistors)           │  MUX B      │  AIN1  │     ADS1263          │
                                      └──────┬──────┘   ──►  │     32-bit ADC       │
                                      S0-S3, EN (GPIO)       │                      │
                                             ▲               │  AIN2 ◄── Bus V      │
                                             │               │  AIN3 ◄── Bus I      │
     ┌───────────────────────────────────────┐│               │  AIN4 ◄── Irradiance │
     │                                       ││               │  AIN5 ◄── Mod Temp   │
     │         STM32F407VGT6                 ││               │                      │
     │         168 MHz Cortex-M4F            ││               │  SPI1 (4 MHz)        │
     │                                       ││               └──────────┬───────────┘
     │   PC0-PC4 ► MUX A select + enable ───┘│                          │
     │   PC5-PC9 ► MUX B select + enable ────┘                          │
     │                                                                   │
     │   PA4      ► ADS1263_CS   ◄───────────────────────────────────────┘
     │   PA5      ► SPI1_SCK     ◄───────────────────────────────────────┘
     │   PA6      ► SPI1_MISO   ◄────────────────────────────────────────┘
     │   PA7      ► SPI1_MOSI   ◄────────────────────────────────────────┘
     │   PB0      ► ADS1263_DRDY ◄────────────────────────────────────────┘
     │   PB1      ► ADS1263_RST  ◄────────────────────────────────────────┘
     │                                                     ┌────────────────┐
     │   PB6/PB7  ► I2C1 (400 kHz) ──────────────────────►│  TMP117        │
     │                                                     │  Ambient Temp  │
     │                                                     │  I2C 0x48      │
     │   PB3      ► SPI3_SCK   ──────►┌──────────┐        └────────────────┘
     │   PB4      ► SPI3_MISO  ──────►│ MicroSD  │
     │   PB5      ► SPI3_MOSI  ──────►│ Card     │
     │   PA15     ► SD_CS      ──────►│ (SPI)    │
     │   PD2      ◄ SD_DETECT  ◄──────│          │
     │                                 └──────────┘
     │   PA2      ► USART2_TX  ──────►┌──────────┐
     │   PA3      ◄ USART2_RX  ◄──────│ SP3485   │──────► RS-485 Bus
     │   PA8      ► RS485_DE   ──────►│ RS-485   │        (Modbus RTU)
     │                                 └──────────┘
     │                                                    ┌──────────────────┐
     │   PD12     ► LED Green  (heartbeat)                │  Power Supply    │
     │   PD13     ► LED Orange (acquisition)              │                  │
     │   PD14     ► LED Red    (fault)                    │  24V DC Input    │
     │   PD15     ► LED Blue   (SD activity)              │       │          │
     │                                                    │       ▼          │
     │   PA13/14  ► SWD Debug (J5 header)                 │  LM2596 5V      │
     │                                                    │       │          │
     └───────────────────────────────────────────────────┐│       ▼          │
                                                         ││  AMS1117 3.3V   │
                               3.3V ◄────────────────────┘│                  │
                                                          └──────────────────┘
```

## 3. Water RTU (WR-PCB-001) Block Diagram

```
     8x 4-20mA Sensor Loops                                   ┌─────────────────────────┐
     ┌─────────────────────┐                                   │                         │
     │ CH0: Pressure       │──250Ω──► AIN0                    │                         │
     │ CH1: pH             │──250Ω──► AIN1                    │     ADS1258             │
     │ CH2: Turbidity      │──250Ω──► AIN2                    │     24-bit ADC          │
     │ CH3: Chlorine       │──250Ω──► AIN3                    │     16-channel          │
     │ CH4: Flow           │──250Ω──► AIN4                    │                         │
     │ CH5: Tank Level     │──250Ω──► AIN5                    │     SPI1 (4 MHz)        │
     │ CH6: Conductivity   │──250Ω──► AIN6                    │     Vref = 5.0V         │
     │ CH7: Dissolved O2   │──250Ω──► AIN7                    │                         │
     └─────────────────────┘                                   └──────────┬──────────────┘
                                                                          │
     ┌───────────────────────────────────────────────────────────────────┐│
     │                                                                   ││
     │         STM32F407VGT6                                             ││
     │         168 MHz Cortex-M4F                                        ││
     │                                                                   ││
     │   PA4      ► ADS1258_CS    ◄──────────────────────────────────────┘│
     │   PA5      ► SPI1_SCK      ◄──────────────────────────────────────┘│
     │   PA6      ► SPI1_MISO     ◄──────────────────────────────────────┘│
     │   PA7      ► SPI1_MOSI     ◄──────────────────────────────────────┘│
     │   PB0      ► ADS1258_DRDY  ◄──────────────────────────────────────┘│
     │   PB1      ► ADS1258_START ◄──────────────────────────────────────┘│
     │   PB2      ► ADS1258_RST   ◄──────────────────────────────────────┘│
     │                                                                    │
     │   PB12     ► W5500_CS    ──────►┌────────────┐                     │
     │   PB13     ► SPI2_SCK   ──────►│            │                     │
     │   PB14     ► SPI2_MISO  ◄──────│   W5500    │                     │
     │   PB15     ► SPI2_MOSI  ──────►│   Ethernet │───► RJ45 Jack       │
     │   PC6      ► W5500_RST  ──────►│   10/100   │    (Modbus TCP)     │
     │   PC7      ◄ W5500_INT  ◄──────│            │                     │
     │                                 └────────────┘                     │
     │   PA2      ► USART2_TX  ──────►┌──────────┐                       │
     │   PA3      ◄ USART2_RX  ◄──────│ SP3485   │──────► RS-485 Bus    │
     │   PA8      ► RS485_DE   ──────►│ RS-485   │   (Modbus RTU)       │
     │                                 └──────────┘                       │
     │                                                                    │
     │   PD12     ► LED Green  (heartbeat)                                │
     │   PD13     ► LED Orange (Modbus activity)                          │
     │   PD14     ► LED Red    (fault)                                    │
     │   PD15     ► LED Blue   (data acquisition)                         │
     │                                                                    │
     │   PA13/14  ► SWD Debug (J5 header)                                 │
     │                                                                    │
     └────────────────────────────────────────────────────────────────────┘
```

## 4. Communication Architecture

```
    ┌─────────────────────────────────────────────────────────────────────────┐
    │                         RS-485 Modbus RTU Bus                           │
    │                     (9600 baud, 8N1, Half-Duplex)                       │
    │                                                                         │
    │   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────────────┐   │
    │   │ Solar #1 │   │ Solar #2 │   │ Solar #N │   │ SCADA Gateway    │   │
    │   │ Addr: 2  │   │ Addr: 3  │   │ Addr: N  │   │ (Modbus Master)  │   │
    │   └────┬─────┘   └────┬─────┘   └────┬─────┘   └────────┬─────────┘   │
    │        │              │              │                    │             │
    ├────────┴──────────────┴──────────────┴────────────────────┴─────────────┤
    │                                                                         │
    │   ┌──────────┐   ┌──────────────────────────────────────────────────┐   │
    │   │ Water #1 │   │ SCADA Gateway                                   │   │
    │   │ Addr: 1  │   │ - Modbus RTU Master (RS-485)                    │   │
    │   └────┬─────┘   │ - Modbus TCP Server (Ethernet, port 502)       │   │
    │        │         │ - MQTT Client → Broker                          │   │
    │        │         └──────────────────────────────────────────────────┘   │
    └────────┴───────────────────────────────────────────────────────────────┘

    ┌─────────────────────────────────────────────────────────────────────────┐
    │                     Ethernet / Modbus TCP Network                       │
    │                                                                         │
    │   ┌──────────┐         ┌──────────────┐      ┌──────────────────┐      │
    │   │ Water #1 │ TCP:502 │ SCADA Server │      │ Mosquitto MQTT   │      │
    │   │ W5500    │────────►│ Go Backend   │◄────►│ Broker           │      │
    │   │ Ethernet │         │              │      │ port 1883        │      │
    │   └──────────┘         └──────────────┘      └──────────────────┘      │
    └─────────────────────────────────────────────────────────────────────────┘
```

### 4.1 MQTT Topic Structure

```
scada/
├── water/
│   └── {device_id}/
│       ├── telemetry          # Sensor readings (pub by device)
│       ├── status             # Device online/offline/fault
│       ├── command            # Commands to device (sub by device)
│       └── response           # Command acknowledgments
├── solar/
│   └── {device_id}/
│       ├── telemetry
│       ├── status
│       ├── command
│       └── response
├── alarms/
│   └── {subsystem}            # Alarm notifications
└── system/
    ├── heartbeat              # System heartbeat
    └── status                 # System status (LWT)
```

### 4.2 Modbus Register Map

#### Solar SMU (Input Registers - FC 0x04, from 30001)

| Address | Count | Type    | Description |
|---------|-------|---------|-------------|
| 30001-30032 | 32 | float32 | String 1-16 Voltages (2 regs each) |
| 30033-30064 | 32 | float32 | String 1-16 Currents (2 regs each) |
| 30065-30096 | 32 | float32 | String 1-16 Powers (2 regs each) |
| 30097-30098 | 2  | float32 | DC Bus Voltage |
| 30099-30100 | 2  | float32 | DC Bus Current |
| 30101-30102 | 2  | float32 | Total Power |
| 30103-30104 | 2  | float32 | Irradiance (W/m2) |
| 30105-30106 | 2  | float32 | Module Temperature (deg C) |
| 30107-30108 | 2  | float32 | Ambient Temperature (deg C) |
| 30109       | 1  | uint16  | String Status Bitmap (1=fault) |
| 30110       | 1  | uint16  | Device Status Word |
| 30111-30112 | 2  | uint32  | Uptime (seconds) |
| 30113-30114 | 2  | float32 | Daily Energy (kWh) |

#### Solar SMU (Holding Registers - FC 0x03, from 40001)

| Address | Count | Type    | Description |
|---------|-------|---------|-------------|
| 40001   | 1     | uint16  | Modbus Slave Address (default: 2) |
| 40002   | 1     | uint16  | Scan Interval (ms, default: 1000) |
| 40003   | 1     | uint16  | Log Interval (s, default: 60) |
| 40004   | 1     | uint16  | Alarm Enable Bitmap |
| 40005-40036 | 32 | float32 | Voltage Cal Offsets (16 strings) |
| 40037-40068 | 32 | float32 | Current Cal Offsets (16 strings) |
| 40100   | 1     | uint16  | Command Register |

#### Device Status Bits (Register 30110)

| Bit  | Mask   | Meaning |
|------|--------|---------|
| 0    | 0x0001 | ADC initialized OK |
| 1    | 0x0002 | TMP117 initialized OK |
| 2    | 0x0004 | SD card OK |
| 3    | 0x0008 | RS-485 OK |
| 4    | 0x0010 | Calibration valid |
| 15   | 0x8000 | Any alarm active |

## 5. Software Architecture (Firmware)

```
┌─────────────────────────────────────────────────────────────────┐
│                    Embassy Async Runtime                         │
│                    (Cortex-M4F Executor)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌─────────────┐  ┌──────────────┐  ┌───────────────────────┐  │
│   │ scan_task    │  │ modbus_task  │  │ diagnostics_task      │  │
│   │              │  │              │  │                       │  │
│   │ 36-ch ADC    │  │ RS-485 slave │  │ TMP117 read (1s)     │  │
│   │ scan (1s)    │  │ FC 03/04/06  │  │ LED heartbeat        │  │
│   │              │  │ FC 10        │  │ Alarm evaluation      │  │
│   │ Signals ─────┤  │              │  │ Performance calc      │  │
│   │ new_scan     │  │              │  │                       │  │
│   └──────┬───────┘  └──────┬───────┘  └───────────┬───────────┘  │
│          │                 │                      │              │
│   ┌──────▼─────────────────▼──────────────────────▼───────────┐  │
│   │                   SharedState                              │  │
│   │                                                            │  │
│   │  registers: Mutex<RegisterStore>                           │  │
│   │  strings[16]: Mutex<[StringData; 16]>                      │  │
│   │  bus_voltage / bus_current: Mutex<f32>                      │  │
│   │  irradiance / module_temp / ambient_temp: Mutex<f32>       │  │
│   │  daily_energy_wh: Mutex<f32>                                │  │
│   │  device_status: Mutex<u16>                                  │  │
│   │  new_scan: Signal<()>                                       │  │
│   └──────┬────────────────────────────────────────────────────┘  │
│          │                                                       │
│   ┌──────▼───────┐                                               │
│   │ logging_task │                                               │
│   │              │                                               │
│   │ SD card CSV  │                                               │
│   │ write (60s)  │                                               │
│   └──────────────┘                                               │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│   HAL Drivers                                                    │
│   ads1263.rs | cd74hc4067.rs | tmp117.rs | sdcard.rs | sp3485   │
├─────────────────────────────────────────────────────────────────┤
│   Embassy-STM32 HAL (SPI, I2C, USART, GPIO, DMA, Timers)       │
├─────────────────────────────────────────────────────────────────┤
│   STM32F407VGT6 Hardware                                         │
└─────────────────────────────────────────────────────────────────┘
```

## 6. Data Flow

```
Sensors → ADC → Calibration → SharedState → Modbus Registers → RS-485
                                    │
                                    ├──────► SD Card Log (CSV)
                                    │
                                    └──────► Alarm Evaluation → LED Indicators
```

### 6.1 Scan Cycle Timing (Solar SMU)

```
Time (ms)    Action
─────────────────────────────────────────────────────
  0          Start scan cycle
  0-0.5      MUX A/B select channel 0
  0.5-1.0    ADC settling + read voltage (AIN0)
  1.0-1.5    ADC read current (AIN1)
  1.5-2.0    MUX A/B select channel 1
             ...
 24.0        MUX A/B select channel 15
 25.5        Complete string scan (16 × ~1.5ms)
 26.0        Disable MUXes
 26.0-26.5   Read AIN2 (bus voltage)
 27.0-27.5   Read AIN3 (bus current)
 28.0-28.5   Read AIN4 (irradiance)
 29.0-29.5   Read AIN5 (module temp)
 30.0        Compute totals, update SharedState
 30.5        Update Modbus registers
 31.0        Signal new_scan
             ─── idle until next 1000ms tick ───
```
