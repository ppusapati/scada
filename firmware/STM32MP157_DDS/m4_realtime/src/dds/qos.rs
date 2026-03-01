/// DDS Quality of Service (QoS) Policies
///
/// Defines QoS profiles for each topic in the SCADA system.
/// These are communicated to the XRCE agent during entity creation
/// and determine how DDS handles reliability, durability, deadlines, etc.
///
/// QoS choices for SCADA:
///   - Sensor data: Best-effort, 1-second deadline (tolerate drops)
///   - Alarms: Reliable, transient-local durability (must not be lost)
///   - Heartbeats: Best-effort, 10-second lifespan (stale data expires)
///   - Commands: Reliable, volatile durability (one-shot delivery)

/// Reliability policy
#[derive(Clone, Copy, PartialEq, defmt::Format)]
#[repr(u8)]
pub enum Reliability {
    /// Best-effort: no retransmission, lowest latency
    BestEffort = 0x01,
    /// Reliable: retransmit until acknowledged
    Reliable = 0x02,
}

/// Durability policy
#[derive(Clone, Copy, PartialEq, defmt::Format)]
#[repr(u8)]
pub enum Durability {
    /// Volatile: no persistence, only delivers to currently active subscribers
    Volatile = 0x00,
    /// TransientLocal: retains last N samples for late-joining subscribers
    TransientLocal = 0x01,
}

/// History policy
#[derive(Clone, Copy, PartialEq, defmt::Format)]
#[repr(u8)]
pub enum HistoryKind {
    /// Keep only the last sample
    KeepLast = 0x00,
    /// Keep all samples (bounded by resource limits)
    KeepAll = 0x01,
}

/// Complete QoS profile for a topic
#[derive(Clone, Copy)]
pub struct TopicQos {
    /// Reliability: best-effort or reliable
    pub reliability: Reliability,
    /// Durability: volatile or transient-local
    pub durability: Durability,
    /// History kind
    pub history: HistoryKind,
    /// History depth (number of samples to keep)
    pub history_depth: u16,
    /// Deadline in milliseconds (0 = no deadline)
    /// If a sample is not published within this period, a deadline missed event occurs
    pub deadline_ms: u32,
    /// Lifespan in milliseconds (0 = infinite)
    /// Samples older than this are automatically discarded
    pub lifespan_ms: u32,
}

impl TopicQos {
    pub const fn best_effort() -> Self {
        Self {
            reliability: Reliability::BestEffort,
            durability: Durability::Volatile,
            history: HistoryKind::KeepLast,
            history_depth: 1,
            deadline_ms: 0,
            lifespan_ms: 0,
        }
    }

    pub const fn reliable() -> Self {
        Self {
            reliability: Reliability::Reliable,
            durability: Durability::TransientLocal,
            history: HistoryKind::KeepLast,
            history_depth: 10,
            deadline_ms: 0,
            lifespan_ms: 0,
        }
    }
}

/// Pre-defined QoS profiles for SCADA topics

/// Sensor data QoS: best-effort, 1s deadline, 5s lifespan
/// Used for: string_voltage, string_current, bus_power, water_level, flow, pressure, quality
pub const QOS_SENSOR_DATA: TopicQos = TopicQos {
    reliability: Reliability::BestEffort,
    durability: Durability::Volatile,
    history: HistoryKind::KeepLast,
    history_depth: 1,
    deadline_ms: 1000,   // Expect data every 1 second
    lifespan_ms: 5000,   // Discard if older than 5 seconds
};

/// Alarm QoS: reliable, transient-local, no deadline, 60s lifespan
/// Used for: alarm events (must be delivered even if subscriber joins late)
pub const QOS_ALARM: TopicQos = TopicQos {
    reliability: Reliability::Reliable,
    durability: Durability::TransientLocal,
    history: HistoryKind::KeepLast,
    history_depth: 32,    // Keep last 32 alarms
    deadline_ms: 0,       // No periodic deadline (event-driven)
    lifespan_ms: 60000,   // Alarms valid for 60 seconds
};

/// Heartbeat QoS: best-effort, 10s deadline, 30s lifespan
/// Used for: heartbeat / system health (periodic, tolerate drops)
pub const QOS_HEARTBEAT: TopicQos = TopicQos {
    reliability: Reliability::BestEffort,
    durability: Durability::Volatile,
    history: HistoryKind::KeepLast,
    history_depth: 1,
    deadline_ms: 10000,   // Expect heartbeat every 10 seconds
    lifespan_ms: 30000,   // Discard after 30 seconds
};

/// Command QoS: reliable, volatile, no deadline
/// Used for: configuration commands (must be delivered exactly once)
pub const QOS_COMMAND: TopicQos = TopicQos {
    reliability: Reliability::Reliable,
    durability: Durability::Volatile,
    history: HistoryKind::KeepAll,
    history_depth: 16,
    deadline_ms: 0,
    lifespan_ms: 30000,  // Commands expire after 30 seconds
};

/// Configurable publish rates per topic (in milliseconds)
pub struct PublishRates {
    /// String voltage publish interval
    pub string_voltage_ms: u32,
    /// String current publish interval
    pub string_current_ms: u32,
    /// Bus power publish interval
    pub bus_power_ms: u32,
    /// Water level publish interval
    pub water_level_ms: u32,
    /// Flow rate publish interval
    pub flow_rate_ms: u32,
    /// Pressure publish interval
    pub pressure_ms: u32,
    /// Water quality publish interval
    pub water_quality_ms: u32,
    /// Heartbeat publish interval
    pub heartbeat_ms: u32,
}

impl Default for PublishRates {
    fn default() -> Self {
        Self {
            string_voltage_ms: 1000,   // 1 Hz
            string_current_ms: 1000,   // 1 Hz
            bus_power_ms: 1000,        // 1 Hz
            water_level_ms: 1000,      // 1 Hz
            flow_rate_ms: 1000,        // 1 Hz
            pressure_ms: 1000,         // 1 Hz
            water_quality_ms: 5000,    // 0.2 Hz (slow-changing)
            heartbeat_ms: 10000,       // 0.1 Hz
        }
    }
}
