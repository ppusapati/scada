/// Configuration Management
///
/// Loads SCADA gateway configuration from /etc/scada/config.toml.
/// Supports runtime reconfiguration via DDS commands.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Top-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScadaConfig {
    /// Device identity
    pub device: DeviceConfig,
    /// DDS configuration
    pub dds: DdsConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// Modbus TCP gateway configuration
    pub modbus: ModbusConfig,
    /// PTP configuration
    pub ptp: PtpConfig,
    /// OTA update configuration
    pub ota: OtaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Unique device identifier (e.g., "SMU-PLANT01-STR01")
    pub device_id: String,
    /// Device type: "solar_smu" or "water_rtu"
    pub device_type: String,
    /// Physical location description
    pub location: String,
    /// Plant/district identifier
    pub plant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdsConfig {
    /// DDS domain ID (0-232)
    pub domain_id: u16,
    /// Network interface for DDS multicast
    pub interface: String,
    /// Multicast group address
    pub multicast_group: String,
    /// Partition name (e.g., "Solar/Plant_01" or "Water/District_03")
    pub partition: String,
    /// Topic QoS profile name
    pub qos_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Primary network interface (e.g., "eth0")
    pub interface: String,
    /// Static IP address (if DHCP not used)
    pub ip_address: Option<String>,
    /// Subnet mask
    pub netmask: Option<String>,
    /// Default gateway
    pub gateway: Option<String>,
    /// DNS servers
    pub dns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusConfig {
    /// Enable Modbus TCP gateway
    pub enabled: bool,
    /// TCP port for Modbus server
    pub tcp_port: u16,
    /// Maximum simultaneous connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Modbus unit ID
    pub unit_id: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtpConfig {
    /// Enable PTP synchronization
    pub enabled: bool,
    /// PTP network interface
    pub interface: String,
    /// PTP domain number
    pub domain: u8,
    /// Path to ptp4l binary
    pub ptp4l_path: String,
    /// Path to phc2sys binary
    pub phc2sys_path: String,
    /// PTP4L configuration file
    pub ptp4l_config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtaConfig {
    /// Enable OTA updates
    pub enabled: bool,
    /// OTA server URL
    pub server_url: String,
    /// Check interval in seconds
    pub check_interval_secs: u64,
    /// M4 firmware path on filesystem
    pub m4_firmware_path: String,
    /// Remoteproc sysfs path
    pub remoteproc_path: String,
}

impl ScadaConfig {
    /// Load configuration from a TOML file
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path))?;
        let config: ScadaConfig = toml::from_str(&content)
            .context("Failed to parse config TOML")?;
        Ok(config)
    }

    /// Get default configuration
    pub fn default_config() -> Self {
        Self {
            device: DeviceConfig {
                device_id: "MP157-001".to_string(),
                device_type: "solar_smu".to_string(),
                location: "Plant 01, Combiner Box 01".to_string(),
                plant_id: "PLANT01".to_string(),
            },
            dds: DdsConfig {
                domain_id: 0,
                interface: "eth0".to_string(),
                multicast_group: "239.255.0.1".to_string(),
                partition: "Solar/Plant_01".to_string(),
                qos_profile: "default".to_string(),
            },
            network: NetworkConfig {
                interface: "eth0".to_string(),
                ip_address: Some("192.168.1.100".to_string()),
                netmask: Some("255.255.255.0".to_string()),
                gateway: Some("192.168.1.1".to_string()),
                dns: None,
            },
            modbus: ModbusConfig {
                enabled: true,
                tcp_port: 502,
                max_connections: 4,
                timeout_secs: 30,
                unit_id: 1,
            },
            ptp: PtpConfig {
                enabled: true,
                interface: "eth0".to_string(),
                domain: 0,
                ptp4l_path: "/usr/sbin/ptp4l".to_string(),
                phc2sys_path: "/usr/sbin/phc2sys".to_string(),
                ptp4l_config: "/etc/linuxptp/ptp4l.conf".to_string(),
            },
            ota: OtaConfig {
                enabled: true,
                server_url: "https://ota.p9e.io/firmware".to_string(),
                check_interval_secs: 3600,
                m4_firmware_path: "/lib/firmware/stm32mp157-m4.elf".to_string(),
                remoteproc_path: "/sys/class/remoteproc/remoteproc0".to_string(),
            },
        }
    }
}
