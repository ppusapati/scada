/// Modbus Slave Task -- listens on USART2 for Modbus RTU requests
///
/// This task runs the Modbus RTU slave on the RS-485 bus:
///   1. Listen for incoming frames on USART2 (via SP3485 transceiver)
///   2. Validate address and CRC
///   3. Process function codes (read/write registers)
///   4. Update the shared register map with latest sensor data
///   5. Build and send response frames
///
/// The register map is shared between this task and the sensor_read task:
///   - sensor_read updates INPUT registers with sensor data
///   - modbus_task reads INPUT registers in response to FC04
///   - modbus_task reads/writes HOLDING registers for FC03/FC06/FC10
///   - command_handler watches for write events on HOLDING registers

#[cfg(feature = "modbus")]
use crate::net::modbus::{ModbusSlave, RegisterMap, SharedRegisterMap};
use crate::drivers::sp3485::{Sp3485, ModbusFrame, Rs485Error};
use crate::hal::gpio::Rs485Control;
use embassy_stm32::usart::Uart;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer, Instant};
use defmt::{info, warn, error, Format};

// ============================================================================
// Modbus task configuration
// ============================================================================

/// Modbus task configuration
pub struct ModbusTaskConfig {
    /// Frame receive timeout (milliseconds)
    pub frame_timeout_ms: u32,
    /// Maximum consecutive errors before restarting USART
    pub max_consecutive_errors: u32,
    /// Delay between processing frames (milliseconds, for rate limiting)
    pub inter_frame_delay_ms: u64,
}

impl Default for ModbusTaskConfig {
    fn default() -> Self {
        Self {
            frame_timeout_ms: 1000,
            max_consecutive_errors: 10,
            inter_frame_delay_ms: 5,
        }
    }
}

// ============================================================================
// Modbus task statistics
// ============================================================================

/// Modbus task runtime statistics
pub struct ModbusTaskStats {
    /// Total frames received
    pub frames_received: u32,
    /// Frames addressed to us
    pub frames_processed: u32,
    /// Frames with CRC errors
    pub crc_errors: u32,
    /// Frames with other errors (timeout, framing, etc.)
    pub rx_errors: u32,
    /// Responses sent
    pub responses_sent: u32,
    /// Exception responses sent
    pub exceptions_sent: u32,
    /// TX errors
    pub tx_errors: u32,
    /// Consecutive error counter (resets on success)
    pub consecutive_errors: u32,
    /// Last activity timestamp (ms)
    pub last_activity_ms: u64,
}

impl ModbusTaskStats {
    pub const fn new() -> Self {
        Self {
            frames_received: 0,
            frames_processed: 0,
            crc_errors: 0,
            rx_errors: 0,
            responses_sent: 0,
            exceptions_sent: 0,
            tx_errors: 0,
            consecutive_errors: 0,
            last_activity_ms: 0,
        }
    }

    /// Record a successful frame reception
    pub fn record_success(&mut self) {
        self.frames_received += 1;
        self.consecutive_errors = 0;
        self.last_activity_ms = Instant::now().as_millis();
    }

    /// Record an error
    pub fn record_error(&mut self, is_crc: bool) {
        if is_crc {
            self.crc_errors += 1;
        } else {
            self.rx_errors += 1;
        }
        self.consecutive_errors += 1;
    }
}

// ============================================================================
// Register update helper functions
// ============================================================================

/// Update the register map with the latest water sensor data
///
/// Called by the sensor_read task after acquiring new readings.
#[cfg(feature = "modbus")]
pub fn update_water_input_registers(
    regs: &mut RegisterMap,
    pressure_bar: f32,
    ph: f32,
    turbidity_ntu: f32,
    chlorine_mgl: f32,
    tank_level_pct: f32,
    water_temp_c: f32,
    ambient_temp_c: f32,
    flow_in_lpm: f32,
    flow_out_lpm: f32,
    total_in_l: f32,
    total_out_l: f32,
    pump1_on: bool,
    valve1_open: bool,
    overflow: bool,
) {
    use crate::net::modbus::water_input;

    // Tank level
    regs.set_input_scaled(water_input::TANK1_LEVEL, tank_level_pct, 10.0);
    regs.set_input(water_input::TANK1_OVERFLOW, if overflow { 1 } else { 0 });

    // Flow rates (as 32-bit values split across two registers)
    regs.set_input_u32(water_input::FLOW_IN_HI, (flow_in_lpm * 10.0) as u32);
    regs.set_input_u32(water_input::FLOW_OUT_HI, (flow_out_lpm * 10.0) as u32);
    regs.set_input_u32(water_input::TOTAL_IN_HI, total_in_l as u32);
    regs.set_input_u32(water_input::TOTAL_OUT_HI, total_out_l as u32);

    // Pressure
    regs.set_input_scaled(water_input::PRESSURE_IN, pressure_bar, 100.0);

    // Water quality
    regs.set_input_scaled(water_input::PH, ph, 100.0);
    regs.set_input_scaled(water_input::TURBIDITY, turbidity_ntu, 100.0);
    regs.set_input_scaled(water_input::CHLORINE, chlorine_mgl, 1000.0);
    regs.set_input_scaled(water_input::WATER_TEMP, water_temp_c, 10.0);
    regs.set_input_scaled(water_input::AMBIENT_TEMP, ambient_temp_c, 10.0);

    // Pump/valve status
    regs.set_input(water_input::PUMP1_STATUS, if pump1_on { 1 } else { 0 });
    regs.set_input(water_input::VALVE1_STATUS, if valve1_open { 1 } else { 0 });
}

/// Update the register map with the latest solar sensor data
#[cfg(feature = "modbus")]
pub fn update_solar_input_registers(
    regs: &mut RegisterMap,
    string1_voltage_v: f32,
    string1_current_a: f32,
    string1_power_w: f32,
    string1_temp_c: f32,
    bus_voltage_v: f32,
    bus_current_a: f32,
    bus_power_w: f32,
    irradiance_wm2: f32,
    ambient_temp_c: f32,
    humidity_pct: f32,
) {
    use crate::net::modbus::solar_input;

    // String 1
    regs.set_input_u32(solar_input::STR1_VOLTAGE_HI, (string1_voltage_v * 10.0) as u32);
    regs.set_input_u32(solar_input::STR1_CURRENT_HI, (string1_current_a * 100.0) as u32);
    regs.set_input_u32(solar_input::STR1_POWER_HI, string1_power_w as u32);
    regs.set_input_scaled(solar_input::STR1_TEMP, string1_temp_c, 10.0);

    // Bus totals
    regs.set_input_u32(solar_input::BUS_VOLTAGE_HI, (bus_voltage_v * 10.0) as u32);
    regs.set_input_u32(solar_input::BUS_CURRENT_HI, (bus_current_a * 100.0) as u32);
    regs.set_input_u32(solar_input::BUS_POWER_HI, bus_power_w as u32);

    // Environmental
    regs.set_input(solar_input::IRRADIANCE, irradiance_wm2 as u16);
    regs.set_input_scaled(solar_input::AMBIENT_TEMP, ambient_temp_c, 10.0);
    regs.set_input_scaled(solar_input::HUMIDITY, humidity_pct, 10.0);
}

/// Update device info registers (common to both node types)
#[cfg(feature = "modbus")]
pub fn update_device_info_registers(
    regs: &mut RegisterMap,
    uptime_s: u32,
    fw_major: u16,
    fw_minor: u16,
    hw_version: u16,
    serial: u32,
    node_id: u16,
    fault_count: u16,
    alarm_word_0: u16,
    alarm_word_1: u16,
) {
    // These register addresses are the same for both solar and water
    regs.set_input_u32(90, uptime_s);     // UPTIME_HI/LO
    regs.set_input(92, fw_major);          // FW_VERSION_MAJOR
    regs.set_input(93, fw_minor);          // FW_VERSION_MINOR
    regs.set_input(94, hw_version);        // HW_VERSION
    regs.set_input_u32(95, serial);        // SERIAL_HI/LO
    regs.set_input(97, node_id);           // NODE_ID
    regs.set_input(60, alarm_word_0);      // ALARM_WORD_0
    regs.set_input(61, alarm_word_1);      // ALARM_WORD_1
    regs.set_input(62, fault_count);       // FAULT_COUNT
}

// ============================================================================
// Holding register write handler
// ============================================================================

/// Actions resulting from holding register writes
#[cfg(feature = "modbus")]
#[derive(Clone, Copy, Format)]
pub enum ModbusWriteAction {
    /// No action needed
    None,
    /// Slave address was changed
    AddressChanged(u8),
    /// Baud rate was changed
    BaudRateChanged(u16),
    /// Pump enable/disable
    PumpControl(bool),
    /// Valve control
    ValveControl(bool),
    /// Mode change (auto/manual/maintenance)
    ModeChange(u16),
    /// Energy/volume counter reset
    CounterReset,
    /// Calibration command
    CalibrationCmd(u16),
    /// Reboot requested
    RebootRequested,
}

/// Check holding registers for write actions
#[cfg(feature = "modbus")]
pub fn check_holding_write_actions(regs: &mut RegisterMap) -> ModbusWriteAction {
    if !regs.write_pending {
        return ModbusWriteAction::None;
    }
    regs.clear_write_pending();

    let addr = regs.last_write_addr;

    // Check for reboot command (common address 14 or 19)
    if (addr == 14 || addr == 19) && regs.holding[addr as usize] == 0xAA55 {
        return ModbusWriteAction::RebootRequested;
    }

    // Check for slave address change
    if addr == 0 {
        return ModbusWriteAction::AddressChanged(regs.holding[0] as u8);
    }

    // Check for baud rate change
    if addr == 1 {
        return ModbusWriteAction::BaudRateChanged(regs.holding[1]);
    }

    // Check for pump/valve control (Water RTU specific)
    if addr == 12 {
        return ModbusWriteAction::PumpControl(regs.holding[12] != 0);
    }
    if addr == 14 || addr == 15 {
        return ModbusWriteAction::ValveControl(regs.holding[14] != 0);
    }

    // Check for mode change
    if addr == 11 || addr == 16 {
        return ModbusWriteAction::ModeChange(regs.holding[addr as usize]);
    }

    // Check for counter reset
    if (addr == 9 || addr == 17) && regs.holding[addr as usize] == 1 {
        regs.holding[addr as usize] = 0; // Auto-clear
        return ModbusWriteAction::CounterReset;
    }

    // Check for calibration command
    if addr == 12 || addr == 18 {
        return ModbusWriteAction::CalibrationCmd(regs.holding[addr as usize]);
    }

    ModbusWriteAction::None
}
