use std::collections::HashMap;
use super::SensorArray;

/// Water treatment sensor array
/// In hardware mode, this would read from I2C/SPI/ADC sensors
/// In simulator mode, generates realistic test data
pub struct WaterSensorArray {
    // Simulated state
    tank_level: f64,
    flow_rate_in: f64,
    flow_rate_out: f64,
    pressure: f64,
    ph_level: f64,
    turbidity: f64,
    chlorine_level: f64,
    temperature: f64,
    pump_running: bool,
    quality: u8,
}

impl WaterSensorArray {
    pub fn new() -> Self {
        WaterSensorArray {
            tank_level: 65.0,
            flow_rate_in: 45.0,
            flow_rate_out: 42.0,
            pressure: 3.5,
            ph_level: 7.2,
            turbidity: 1.5,
            chlorine_level: 0.8,
            temperature: 22.0,
            pump_running: true,
            quality: 0,
        }
    }

    #[cfg(feature = "simulator")]
    fn simulate_step(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Simulate tank level changes based on flow rates
        let net_flow = self.flow_rate_in - self.flow_rate_out;
        self.tank_level += net_flow * 0.001; // Slow change
        self.tank_level = self.tank_level.clamp(0.0, 100.0);

        // Flow rate variations
        if self.pump_running {
            self.flow_rate_in = 45.0 + rng.gen_range(-3.0..3.0);
            self.flow_rate_out = 42.0 + rng.gen_range(-2.0..2.0);
        } else {
            self.flow_rate_in = rng.gen_range(0.0..2.0);
            self.flow_rate_out = rng.gen_range(0.0..1.0);
        }

        // Pressure with slight fluctuation
        self.pressure = if self.pump_running {
            3.5 + rng.gen_range(-0.3..0.3)
        } else {
            1.0 + rng.gen_range(-0.1..0.1)
        };

        // pH drift
        self.ph_level += rng.gen_range(-0.05..0.05);
        self.ph_level = self.ph_level.clamp(5.0, 9.0);

        // Turbidity with occasional spikes
        if rng.gen_range(0..100) < 5 {
            self.turbidity = rng.gen_range(2.0..5.0); // Spike
        } else {
            self.turbidity = 1.5 + rng.gen_range(-0.5..0.5);
            self.turbidity = self.turbidity.max(0.1);
        }

        // Chlorine level
        self.chlorine_level += rng.gen_range(-0.02..0.02);
        self.chlorine_level = self.chlorine_level.clamp(0.0, 3.0);

        // Temperature drift
        self.temperature += rng.gen_range(-0.2..0.2);
        self.temperature = self.temperature.clamp(5.0, 40.0);
    }

    #[cfg(feature = "hardware")]
    fn read_hardware(&mut self) {
        // TODO: Read from actual I2C/SPI/ADC hardware
        // Example for Raspberry Pi / STM32:
        // self.tank_level = adc_channel_0.read_percentage();
        // self.pressure = i2c_pressure_sensor.read_bar();
        // self.ph_level = adc_channel_1.read_ph();
        unimplemented!("Hardware sensor reading not yet implemented")
    }

    pub fn set_pump_running(&mut self, running: bool) {
        self.pump_running = running;
        log::info!("Pump status set to: {}", if running { "running" } else { "stopped" });
    }

    pub fn is_pump_running(&self) -> bool {
        self.pump_running
    }
}

impl SensorArray for WaterSensorArray {
    fn read_all(&mut self) -> HashMap<String, f64> {
        #[cfg(feature = "simulator")]
        self.simulate_step();

        #[cfg(feature = "hardware")]
        self.read_hardware();

        let mut metrics = HashMap::new();
        metrics.insert("tank_level".into(), self.tank_level);
        metrics.insert("flow_rate_in".into(), self.flow_rate_in);
        metrics.insert("flow_rate_out".into(), self.flow_rate_out);
        metrics.insert("pressure".into(), self.pressure);
        metrics.insert("ph_level".into(), self.ph_level);
        metrics.insert("turbidity".into(), self.turbidity);
        metrics.insert("chlorine_level".into(), self.chlorine_level);
        metrics.insert("temperature".into(), self.temperature);
        metrics
    }

    fn quality(&self) -> u8 {
        self.quality
    }

    fn self_check(&mut self) -> bool {
        // Validate sensor readings are within physical limits
        let checks = vec![
            self.tank_level >= 0.0 && self.tank_level <= 100.0,
            self.pressure >= 0.0 && self.pressure <= 20.0,
            self.ph_level >= 0.0 && self.ph_level <= 14.0,
            self.turbidity >= 0.0,
            self.chlorine_level >= 0.0,
        ];

        let all_ok = checks.iter().all(|&c| c);
        self.quality = if all_ok { 0 } else { 2 };
        all_ok
    }
}
