/// Command handler task — processes incoming MQTT commands
///
/// Subscribes to the device command topic and dispatches received
/// commands to the appropriate driver (relay control, mode changes,
/// calibration updates, emergency stop).

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use defmt::{info, warn, error};

/// Maximum pending commands in the queue
const CMD_QUEUE_SIZE: usize = 8;

/// Command types that can be received from the SCADA backend
#[derive(Clone)]
pub enum DeviceCommand {
    /// Set relay output state (relay_index, on/off)
    SetRelay(u8, bool),
    /// Set operating mode (auto/manual/maintenance)
    SetMode(OperatingMode),
    /// Emergency stop — all outputs off
    EmergencyStop,
    /// Update calibration value (register_addr, value)
    SetCalibration(u16, u16),
    /// Save current calibration to flash
    SaveCalibration,
    /// Restart the device
    Restart,
}

/// Operating modes
#[derive(Clone, Copy, PartialEq, defmt::Format)]
pub enum OperatingMode {
    Auto,
    Manual,
    Maintenance,
}

/// Command queue for inter-task communication
pub type CommandChannel = Channel<CriticalSectionRawMutex, DeviceCommand, CMD_QUEUE_SIZE>;

/// Parse a command from an MQTT payload (simple string matching for no_std)
pub fn parse_command(payload: &[u8]) -> Option<DeviceCommand> {
    if let Ok(s) = core::str::from_utf8(payload) {
        if s.contains("emergency_stop") {
            return Some(DeviceCommand::EmergencyStop);
        }
        if s.contains("pump_on") {
            return Some(DeviceCommand::SetRelay(0, true));
        }
        if s.contains("pump_off") {
            return Some(DeviceCommand::SetRelay(0, false));
        }
        if s.contains("valve_on") {
            return Some(DeviceCommand::SetRelay(1, true));
        }
        if s.contains("valve_off") {
            return Some(DeviceCommand::SetRelay(1, false));
        }
        if s.contains("mode_auto") {
            return Some(DeviceCommand::SetMode(OperatingMode::Auto));
        }
        if s.contains("mode_manual") {
            return Some(DeviceCommand::SetMode(OperatingMode::Manual));
        }
        if s.contains("mode_maintenance") {
            return Some(DeviceCommand::SetMode(OperatingMode::Maintenance));
        }
        if s.contains("save_calibration") {
            return Some(DeviceCommand::SaveCalibration);
        }
        if s.contains("restart") {
            return Some(DeviceCommand::Restart);
        }
    }
    None
}
