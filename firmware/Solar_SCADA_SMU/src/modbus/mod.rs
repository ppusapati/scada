/// Modbus Protocol Stack — Solar SCADA SMU
///
/// Register Map (Solar SCADA SMU — SS-PCB-001):
///   ┌──────────────┬──────────────────────────────────────────────────┐
///   │ Address      │ Description                                      │
///   ├──────────────┼──────────────────────────────────────────────────┤
///   │ 30001-30032  │ String voltages (float32, 2 regs × 16 strings)  │
///   │ 30033-30064  │ String currents (float32, 2 regs × 16 strings)  │
///   │ 30065-30096  │ String powers   (float32, 2 regs × 16 strings)  │
///   │ 30097-30098  │ Bus voltage (float32)                            │
///   │ 30099-30100  │ Bus current (float32)                            │
///   │ 30101-30102  │ Total power (float32)                            │
///   │ 30103-30104  │ Irradiance W/m² (float32)                       │
///   │ 30105-30106  │ Module temperature °C (float32)                  │
///   │ 30107-30108  │ Ambient temperature °C (float32, from TMP117)   │
///   │ 30109        │ String status bitmap (16 bits, 1=fault)         │
///   │ 30110        │ Device status word                               │
///   │ 30111-30112  │ Uptime seconds (32-bit)                         │
///   │ 30113-30114  │ Daily energy kWh (float32)                      │
///   │ 40001        │ Modbus slave address                             │
///   │ 40002        │ Scan interval ms                                 │
///   │ 40003        │ Log interval seconds                             │
///   │ 40004        │ Alarm enable bitmap                              │
///   │ 40005-40036  │ Voltage cal offset/gain (2 regs × 16)           │
///   │ 40037-40068  │ Current cal offset/gain (2 regs × 16)           │
///   │ 40100        │ Command register (0xCAFE=save cal, 0xDEAD=reset)│
///   └──────────────┴──────────────────────────────────────────────────┘

pub mod registers;
pub mod protocol;
