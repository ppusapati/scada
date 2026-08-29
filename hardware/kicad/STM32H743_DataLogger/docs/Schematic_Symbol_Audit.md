# Schematic symbol audit

Checked every `lib_id` in the nine sheets against the KiCad symbol libraries.
**25 of 44 resolve; 19 do not.**

A sheet whose symbols do not resolve will not render — KiCad reports them as
missing on load. This has to be fixed before the sheets can be wired.

| lib_id in the schematic | Status | Correct symbol / action |
|---|---|---|
| `Amplifier_Operational:OPA2376` | MISSING | Amplifier_Operational:OPA4376; Amplifier_Operational:OPA2376xxD |
| `Connector:Conn_ARM_JTAG_SWD_10` | ok | — |
| `Connector:Conn_Coaxial` | ok | — |
| `Connector:Micro_SD_Card_Det` | MISSING | Connector:Micro_SD_Card_Det2; Connector:Micro_SD_Card_Det1 |
| `Connector:RJ45_Shielded_Magnetics` | MISSING | Connector:RJ45_Shielded; Connector:RJ49_Shielded |
| `Connector:SIM_Card` | ok | — |
| `Connector:Screw_Terminal_01x02` | ok | — |
| `Connector:Screw_Terminal_01x03` | ok | — |
| `Connector:Tag-Connect_TC2050-IDC` | MISSING | Connector:Conn_ARM_SWD_TagConnect_TC2030-NL; Connector:Conn_ARM_SWD_TagConnect_TC2030 |
| `Connector:TestPoint` | ok | — |
| `Connector:USB_C_Receptacle_USB2.0` | MISSING | Connector:USB_C_Receptacle_USB2.0_16P; Connector:USB_C_Receptacle_USB2.0_14P |
| `Connector_Generic:Conn_01x03` | ok | — |
| `Connector_Generic:Conn_01x04` | ok | — |
| `Converter_DCDC:NXJ1S0505MC-R13` | MISSING | Converter_DCDC_Isolated:NXE1S0505MC |
| `Device:C` | ok | — |
| `Device:C_Polarized` | ok | — |
| `Device:Crystal` | ok | — |
| `Device:D_Schottky` | ok | — |
| `Device:D_TVS` | ok | — |
| `Device:Ferrite_Bead` | MISSING | Device:FerriteBead; Device:FerriteBead_Small |
| `Device:L` | ok | — |
| `Device:LED` | ok | — |
| `Device:L_Core_Ferrite` | MISSING | Device:L_Ferrite; Device:L_Ferrite_Small |
| `Device:Polyfuse` | ok | — |
| `Device:Q_PMOS_GSD` | MISSING | Device:Q_PMOS; Device:Q_PJFET_GSD |
| `Device:R` | ok | — |
| `Device:R_Pack04` | ok | — |
| `Driver_Motor:ULN2003` | MISSING | Transistor_Array:ULN2003; Transistor_Array:ULN2003A |
| `Interface_CAN_LIN:ISO1042` | MISSING | Interface_CAN_LIN:ISOW1044; Interface_CAN_LIN:ISO1044BD |
| `Interface_Ethernet:W5500` | ok | — |
| `Interface_UART:ISO3082DW` | ok | — |
| `Isolator:TLP293` | ok | — |
| `MCU_ST_STM32H7:STM32H743VITx` | ok | — |
| `Memory_EEPROM:AT24C256C` | MISSING | Memory_EEPROM:CAT24C256; Memory_EEPROM:24LC256 |
| `Memory_Flash:W25Q64JVxxQ` | MISSING | Memory_Flash:W25Q16JVSS; Memory_Flash:W25Q32JVZP |
| `RF_Bluetooth:RN4870` | MISSING | RF_Bluetooth:RN4871; RF_Bluetooth:RN42 |
| `RF_GSM:SIM7600` | MISSING | RF_GSM:SIM7020E; RF_GSM:SIM7020C |
| `RF_Module:RFM95W-868S2` | ok | — |
| `RF_WiFi:ATWINC1500-MR210PB` | MISSING | no close match - needs a custom symbol |
| `Regulator_Linear:TLV1117-33` | ok | — |
| `Regulator_Linear:TPS7A4533` | MISSING | Regulator_Linear:TPS7A39; Regulator_Linear:TPS7133 |
| `Regulator_Switching:TPS54560` | MISSING | Regulator_Switching:TPS54561; Regulator_Switching:TPS54560BDDA |
| `Relay:SANYOU_SRD_Form_C` | ok | — |
| `Timer:TPS3823` | MISSING | Power_Supervisor:TPS3831; Converter_DCDC:TPS82130 |

The parts with no close match are project-specific (the LTE, WiFi and BLE
modules, the isolated CAN transceiver, the flash and EEPROM, the Tag-Connect
pads and the isolated DC-DC). Each needs a symbol drawn for it, or a generic
multi-pin symbol substituted, before schematic capture can proceed.
