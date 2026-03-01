pub mod water;
pub mod solar;

use std::collections::HashMap;

/// Trait for all sensor implementations
pub trait SensorArray: Send {
    /// Read all sensor values and return as metric name -> value map
    fn read_all(&mut self) -> HashMap<String, f64>;

    /// Get the quality indicator (0 = good, 1 = uncertain, 2 = bad)
    fn quality(&self) -> u8;

    /// Perform self-check on sensors
    fn self_check(&mut self) -> bool;

    /// Get the sensor type identifier
    fn sensor_type(&self) -> &'static str;
}
