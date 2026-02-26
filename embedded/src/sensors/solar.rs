use std::collections::HashMap;
use super::SensorArray;

/// Solar panel sensor array
/// Monitors solar irradiance, panel voltage/current, temperature, and grid parameters
pub struct SolarSensorArray {
    irradiance: f64,           // W/m²
    panel_temperature: f64,    // °C
    panel_voltage: f64,        // V (DC)
    panel_current: f64,        // A (DC)
    grid_voltage: f64,         // V (AC)
    grid_frequency: f64,       // Hz
    inverter_output_kw: f64,   // kW
    daily_energy_kwh: f64,     // kWh accumulated today
    efficiency: f64,           // %
    inverter_online: bool,
    quality: u8,
    hour_of_day: f64,         // Simulated time for irradiance curve
}

impl SolarSensorArray {
    pub fn new() -> Self {
        SolarSensorArray {
            irradiance: 800.0,
            panel_temperature: 45.0,
            panel_voltage: 380.0,
            panel_current: 8.5,
            grid_voltage: 230.0,
            grid_frequency: 50.0,
            inverter_output_kw: 3.2,
            daily_energy_kwh: 0.0,
            efficiency: 19.5,
            inverter_online: true,
            quality: 0,
            hour_of_day: 12.0,
        }
    }

    #[cfg(feature = "simulator")]
    fn simulate_step(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Advance simulated time
        self.hour_of_day += 0.01;
        if self.hour_of_day > 24.0 {
            self.hour_of_day = 0.0;
            self.daily_energy_kwh = 0.0; // Reset at midnight
        }

        // Solar irradiance follows a bell curve (sunrise ~6, peak ~12, sunset ~18)
        let solar_angle = ((self.hour_of_day - 12.0) / 6.0 * std::f64::consts::FRAC_PI_2).cos();
        let base_irradiance = if self.hour_of_day > 6.0 && self.hour_of_day < 18.0 {
            1000.0 * solar_angle.max(0.0)
        } else {
            0.0
        };

        // Add cloud effects
        let cloud_factor = if rng.gen_range(0..100) < 15 {
            rng.gen_range(0.3..0.7) // Occasional cloud
        } else {
            rng.gen_range(0.9..1.0)
        };

        self.irradiance = (base_irradiance * cloud_factor + rng.gen_range(-20.0..20.0)).max(0.0);

        // Panel temperature correlates with irradiance + ambient
        let ambient = 25.0 + rng.gen_range(-2.0..2.0);
        self.panel_temperature = ambient + (self.irradiance / 1000.0) * 30.0 + rng.gen_range(-1.0..1.0);

        // Electrical output based on irradiance and temperature
        if self.inverter_online && self.irradiance > 50.0 {
            // Temperature derating: -0.4% per °C above 25°C
            let temp_factor = 1.0 - ((self.panel_temperature - 25.0).max(0.0) * 0.004);
            let irradiance_factor = self.irradiance / 1000.0;

            self.panel_voltage = 380.0 * irradiance_factor.powf(0.3) * temp_factor + rng.gen_range(-5.0..5.0);
            self.panel_current = 10.0 * irradiance_factor * temp_factor + rng.gen_range(-0.3..0.3);
            self.inverter_output_kw = (self.panel_voltage * self.panel_current / 1000.0 * 0.96).max(0.0);

            self.efficiency = if self.irradiance > 100.0 {
                (self.inverter_output_kw / (self.irradiance / 1000.0 * 20.0)) * 100.0 // Assume 20m² panel area
            } else {
                0.0
            };

            // Accumulate energy
            self.daily_energy_kwh += self.inverter_output_kw * (0.01 / 60.0); // fraction of an hour
        } else {
            self.panel_voltage = rng.gen_range(0.0..5.0);
            self.panel_current = 0.0;
            self.inverter_output_kw = 0.0;
            self.efficiency = 0.0;
        }

        // Grid parameters
        self.grid_voltage = 230.0 + rng.gen_range(-3.0..3.0);
        self.grid_frequency = 50.0 + rng.gen_range(-0.05..0.05);
    }

    #[cfg(feature = "hardware")]
    fn read_hardware(&mut self) {
        // TODO: Read from actual solar monitoring hardware
        // pyranometer for irradiance, voltage/current sensors, etc.
        unimplemented!("Hardware sensor reading not yet implemented")
    }

    pub fn set_inverter_online(&mut self, online: bool) {
        self.inverter_online = online;
        log::info!("Inverter set to: {}", if online { "online" } else { "offline" });
    }
}

impl SensorArray for SolarSensorArray {
    fn read_all(&mut self) -> HashMap<String, f64> {
        #[cfg(feature = "simulator")]
        self.simulate_step();

        #[cfg(feature = "hardware")]
        self.read_hardware();

        let mut metrics = HashMap::new();
        metrics.insert("irradiance".into(), self.irradiance);
        metrics.insert("panel_temperature".into(), self.panel_temperature);
        metrics.insert("current_output_kw".into(), self.inverter_output_kw);
        metrics.insert("grid_voltage".into(), self.grid_voltage);
        metrics.insert("grid_frequency".into(), self.grid_frequency);
        metrics.insert("daily_energy_kwh".into(), self.daily_energy_kwh);
        metrics.insert("efficiency".into(), self.efficiency);
        metrics
    }

    fn quality(&self) -> u8 {
        self.quality
    }

    fn self_check(&mut self) -> bool {
        let checks = vec![
            self.irradiance >= 0.0 && self.irradiance <= 1500.0,
            self.panel_temperature > -40.0 && self.panel_temperature < 100.0,
            self.grid_voltage > 180.0 && self.grid_voltage < 270.0,
            self.grid_frequency > 48.0 && self.grid_frequency < 52.0,
        ];

        let all_ok = checks.iter().all(|&c| c);
        self.quality = if all_ok { 0 } else { 2 };
        all_ok
    }
}
