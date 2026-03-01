/// Modbus Protocol Stack — shared between RTU and TCP
///
/// Implements Modbus Application Protocol (MBAP) per specification:
///   - Function Code 0x03: Read Holding Registers
///   - Function Code 0x04: Read Input Registers
///   - Function Code 0x06: Write Single Register
///   - Function Code 0x10: Write Multiple Registers
///
/// Register Map (Water SCADA RTU — WS-PCB-001):
///   ┌──────────────┬──────────────────────────────────────────────┐
///   │ Address      │ Description                                  │
///   ├──────────────┼──────────────────────────────────────────────┤
///   │ 30001-30016  │ Raw ADC values (2 regs per ch, 32-bit)      │
///   │ 30017-30024  │ Loop current mA (×100 integer)              │
///   │ 30025-30040  │ Engineering values (2 regs per ch, float32) │
///   │ 30041-30048  │ Channel status codes                        │
///   │ 30049-30050  │ Alarm bitmap (16 bits each)                 │
///   │ 30051        │ Device status word                          │
///   │ 30052-30053  │ Uptime seconds (32-bit)                     │
///   │ 40001-40016  │ Calibration zero codes (2 regs × 8 ch)     │
///   │ 40017-40032  │ Calibration span codes (2 regs × 8 ch)     │
///   │ 40033-40048  │ Eng min values (float32, 2 regs × 8 ch)    │
///   │ 40049-40064  │ Eng max values (float32, 2 regs × 8 ch)    │
///   │ 40065        │ Modbus slave address                        │
///   │ 40066        │ Baud rate selector                          │
///   │ 40067-40070  │ IP address (4 regs)                         │
///   │ 40071        │ Alarm enable bitmap                         │
///   │ 40100        │ Calibration save command (write 0xCAFE)     │
///   └──────────────┴──────────────────────────────────────────────┘

pub mod registers;
pub mod protocol;
