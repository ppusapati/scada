/// DDS Domain Participant
///
/// Creates and manages the DDS domain participant for the SCADA gateway.
/// The participant is the entry point for all DDS communication.

use anyhow::Result;
use rustdds::*;
use rustdds::policy::*;
use std::sync::Arc;
use tracing::info;

use crate::config::ScadaConfig;

/// Create a DDS domain participant with publisher and subscriber
pub fn create_participant(
    domain_id: u16,
    config: &ScadaConfig,
) -> Result<(Arc<Publisher>, Arc<Subscriber>)> {
    info!("Creating DDS participant on domain {}", domain_id);

    // Create domain participant
    let domain_participant = DomainParticipant::new(domain_id)
        .map_err(|e| anyhow::anyhow!("Failed to create DDS participant: {:?}", e))?;

    // Create publisher with partition QoS
    let publisher_qos = QosPolicyBuilder::new()
        .partition(vec![config.dds.partition.clone()])
        .build();

    let publisher = domain_participant
        .create_publisher(&publisher_qos)
        .map_err(|e| anyhow::anyhow!("Failed to create publisher: {:?}", e))?;

    // Create subscriber with same partition
    let subscriber_qos = QosPolicyBuilder::new()
        .partition(vec![config.dds.partition.clone()])
        .build();

    let subscriber = domain_participant
        .create_subscriber(&subscriber_qos)
        .map_err(|e| anyhow::anyhow!("Failed to create subscriber: {:?}", e))?;

    info!("DDS participant created (partition='{}')", config.dds.partition);

    Ok((Arc::new(publisher), Arc::new(subscriber)))
}

/// Create QoS for sensor data topics (best-effort, volatile)
pub fn sensor_data_qos() -> QosPolicies {
    QosPolicyBuilder::new()
        .reliability(Reliability::BestEffort)
        .durability(Durability::Volatile)
        .history(History::KeepLast { depth: 1 })
        .deadline(Deadline(rustdds::Duration::from_millis(1000)))
        .lifespan(Lifespan {
            duration: rustdds::Duration::from_millis(5000),
        })
        .build()
}

/// Create QoS for alarm topics (reliable, transient-local)
pub fn alarm_qos() -> QosPolicies {
    QosPolicyBuilder::new()
        .reliability(Reliability::Reliable {
            max_blocking_time: rustdds::Duration::from_millis(100),
        })
        .durability(Durability::TransientLocal)
        .history(History::KeepLast { depth: 32 })
        .lifespan(Lifespan {
            duration: rustdds::Duration::from_millis(60000),
        })
        .build()
}

/// Create QoS for heartbeat topics (best-effort, volatile)
pub fn heartbeat_qos() -> QosPolicies {
    QosPolicyBuilder::new()
        .reliability(Reliability::BestEffort)
        .durability(Durability::Volatile)
        .history(History::KeepLast { depth: 1 })
        .deadline(Deadline(rustdds::Duration::from_millis(10000)))
        .lifespan(Lifespan {
            duration: rustdds::Duration::from_millis(30000),
        })
        .build()
}

/// Create QoS for command topics (reliable, volatile)
pub fn command_qos() -> QosPolicies {
    QosPolicyBuilder::new()
        .reliability(Reliability::Reliable {
            max_blocking_time: rustdds::Duration::from_millis(500),
        })
        .durability(Durability::Volatile)
        .history(History::KeepAll)
        .lifespan(Lifespan {
            duration: rustdds::Duration::from_millis(30000),
        })
        .build()
}
