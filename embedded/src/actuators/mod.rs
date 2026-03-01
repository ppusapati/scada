use std::collections::HashMap;

/// Trait for actuator control
pub trait Actuator: Send {
    fn execute_command(&mut self, action: &str, params: &HashMap<String, String>) -> Result<String, String>;
    fn get_state(&self) -> HashMap<String, String>;
}

/// Water system actuators: pumps and valves
pub struct WaterActuator {
    pump_running: bool,
    valve_positions: HashMap<String, f64>, // valve_id -> position (0-100%)
}

impl WaterActuator {
    pub fn new() -> Self {
        let mut valves = HashMap::new();
        valves.insert("inlet_valve".into(), 100.0);
        valves.insert("outlet_valve".into(), 100.0);
        valves.insert("bypass_valve".into(), 0.0);

        WaterActuator {
            pump_running: true,
            valve_positions: valves,
        }
    }

    pub fn is_pump_running(&self) -> bool {
        self.pump_running
    }
}

impl Actuator for WaterActuator {
    fn execute_command(&mut self, action: &str, params: &HashMap<String, String>) -> Result<String, String> {
        match action {
            "start_pump" => {
                self.pump_running = true;
                log::info!("[Actuator] Pump STARTED");
                Ok("pump_started".into())
            }
            "stop_pump" => {
                self.pump_running = false;
                log::info!("[Actuator] Pump STOPPED");
                Ok("pump_stopped".into())
            }
            "set_valve" => {
                let valve_id = params.get("valve_id").ok_or("missing valve_id")?;
                let position: f64 = params
                    .get("position")
                    .ok_or("missing position")?
                    .parse()
                    .map_err(|_| "invalid position value")?;

                if !(0.0..=100.0).contains(&position) {
                    return Err("position must be 0-100".into());
                }

                self.valve_positions.insert(valve_id.clone(), position);
                log::info!("[Actuator] Valve {} set to {}%", valve_id, position);
                Ok(format!("valve_{}_set_{}", valve_id, position))
            }
            _ => Err(format!("unknown action: {}", action)),
        }
    }

    fn get_state(&self) -> HashMap<String, String> {
        let mut state = HashMap::new();
        state.insert("pump_running".into(), self.pump_running.to_string());
        for (id, pos) in &self.valve_positions {
            state.insert(format!("valve_{}", id), format!("{:.1}%", pos));
        }
        state
    }
}

/// Solar system actuators: inverter control, tracking
pub struct SolarActuator {
    inverter_enabled: bool,
    tracker_angle: f64,
}

impl SolarActuator {
    pub fn new() -> Self {
        SolarActuator {
            inverter_enabled: true,
            tracker_angle: 0.0,
        }
    }

    pub fn is_inverter_enabled(&self) -> bool {
        self.inverter_enabled
    }
}

impl Actuator for SolarActuator {
    fn execute_command(&mut self, action: &str, params: &HashMap<String, String>) -> Result<String, String> {
        match action {
            "enable_inverter" => {
                self.inverter_enabled = true;
                log::info!("[Actuator] Inverter ENABLED");
                Ok("inverter_enabled".into())
            }
            "disable_inverter" => {
                self.inverter_enabled = false;
                log::info!("[Actuator] Inverter DISABLED");
                Ok("inverter_disabled".into())
            }
            "reset_inverter" => {
                log::info!("[Actuator] Inverter RESET - disabling");
                self.inverter_enabled = false;
                // Signal that inverter needs a delayed re-enable
                // The caller should await a delay and then call enable_inverter
                // We don't block the async runtime with thread::sleep
                Ok("inverter_reset_pending".into())
            }
            "complete_reset" => {
                self.inverter_enabled = true;
                log::info!("[Actuator] Inverter RESET - re-enabled");
                Ok("inverter_reset_complete".into())
            }
            "set_tracker_angle" => {
                let angle: f64 = params
                    .get("angle")
                    .ok_or("missing angle")?
                    .parse()
                    .map_err(|_| "invalid angle")?;
                self.tracker_angle = angle.clamp(-60.0, 60.0);
                log::info!("[Actuator] Tracker angle set to {}°", self.tracker_angle);
                Ok(format!("tracker_angle_{}", self.tracker_angle))
            }
            _ => Err(format!("unknown action: {}", action)),
        }
    }

    fn get_state(&self) -> HashMap<String, String> {
        let mut state = HashMap::new();
        state.insert("inverter_enabled".into(), self.inverter_enabled.to_string());
        state.insert("tracker_angle".into(), format!("{:.1}°", self.tracker_angle));
        state
    }
}
