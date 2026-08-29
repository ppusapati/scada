# Schematic symbol audit

Every `lib_id` in the nine sheets, checked against the KiCad symbol libraries.
**35 of 44 resolve.**

Twelve wrong ids were corrected against the real library (`Device:Ferrite_Bead`
is `Device:FerriteBead`, `Driver_Motor:ULN2003` lives in `Transistor_Array`,
`Timer:TPS3823` in `Power_Supervisor`, and so on).

## Still unresolved - each needs a symbol drawn

| lib_id | Part | Note |
|---|---|---|
| `Connector:RJ45_Shielded_Magnetics` | Pulse J0011D21BNL | RJ45 with integrated magnetics; the stock library has no magnetics variant |
| `Converter_DCDC:NXJ1S0505MC-R13` | Murata NXJ1S0505MC-R13 | 5.2kVDC isolated 1W. `Converter_DCDC_Isolated:NXE1S0505MC` is the 3kV sibling - verify the pinout before substituting |
| `Interface_CAN_LIN:ISO1042` | TI ISO1042BQDWRQ1 | isolated CAN FD; library has ISO1044 only, which is a different pinout |
| `Memory_Flash:W25Q64JVxxQ` | Winbond W25Q64JVSSIQ | library has W25Q16/W25Q32 in the same SOIC-8 family |
| `RF_Bluetooth:RN4870` | Microchip RN4870-I/RM128 | library has RN4871, a different module |
| `RF_GSM:SIM7600` | SIMCom SIM7600G-H |  |
| `RF_Module:SX1276` | Semtech SX1276IMLTRT | bare QFN-28 die. The library's RFM95W symbols are modules containing an SX1276, not the die |
| `RF_WiFi:ATWINC1500-MR210PB` | Microchip ATWINC1500-MR210PB |  |
| `Regulator_Linear:TPS7A4533DGN` | TI TPS7A4533DGNR | MSOP-8-EP. The library only has the KTT (TO-263-5) variant, which has a different pinout - do not substitute it |
