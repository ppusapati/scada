/// Ethernet driver setup for STM32F407 RMII
///
/// The STM32F407 has a built-in Ethernet MAC that connects to an
/// external PHY (typically LAN8720A) via RMII interface.
///
/// RMII Pin Assignments:
///   PA1  = ETH_REF_CLK (50MHz from PHY)
///   PA2  = ETH_MDIO
///   PA7  = ETH_CRS_DV
///   PC1  = ETH_MDC
///   PC4  = ETH_RXD0
///   PC5  = ETH_RXD1
///   PB11 = ETH_TX_EN
///   PB12 = ETH_TXD0
///   PB13 = ETH_TXD1

use embassy_stm32::eth::{self, Ethernet, PacketQueue};
use embassy_stm32::eth::generic_smi::GenericSMI;
use embassy_stm32::peripherals;
use embassy_net::{Stack, StackResources};
use defmt::info;
use static_cell::StaticCell;

/// MAC address for this node (should be unique per device)
/// Format: locally administered unicast (bit 1 of first octet = 1, bit 0 = 0)
/// In production, derive from STM32 unique device ID
pub fn generate_mac_address(node_type: u8, node_id: u8) -> [u8; 6] {
    // Locally administered MAC: 02:SC:AD:Ax:TT:ID
    // SC:AD:A = "SCADA", TT = node type, ID = node id
    [0x02, 0x53, 0x43, 0xA0 | (node_type & 0x0F), node_type, node_id]
}

/// Ethernet peripheral pins bundle
pub struct EthPins {
    pub eth: peripherals::ETH,
    pub ref_clk: peripherals::PA1,
    pub mdio: peripherals::PA2,
    pub crs_dv: peripherals::PA7,
    pub mdc: peripherals::PC1,
    pub rxd0: peripherals::PC4,
    pub rxd1: peripherals::PC5,
    pub tx_en: peripherals::PB11,
    pub txd0: peripherals::PB12,
    pub txd1: peripherals::PB13,
}

/// Create the Embassy Ethernet device
///
/// Returns the Ethernet device that embassy-net's Stack will use.
/// The caller must spawn the `embassy_net::Stack::run()` task.
pub fn create_ethernet(
    pins: EthPins,
    mac_addr: [u8; 6],
    packet_queue: &'static PacketQueue<4, 4>,
) -> Ethernet<'static, peripherals::ETH, GenericSMI> {
    info!(
        "ETH: Initializing with MAC {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac_addr[0], mac_addr[1], mac_addr[2],
        mac_addr[3], mac_addr[4], mac_addr[5]
    );

    eth::Ethernet::new(
        packet_queue,
        pins.eth,
        embassy_stm32::Irqs,
        pins.ref_clk,
        pins.mdio,
        pins.crs_dv,
        pins.mdc,
        pins.rxd0,
        pins.rxd1,
        pins.tx_en,
        pins.txd0,
        pins.txd1,
        GenericSMI::new(0), // PHY address 0
        mac_addr,
    )
}

/// Network configuration
pub struct NetConfig {
    pub use_dhcp: bool,
    pub static_ip: Option<embassy_net::Ipv4Cidr>,
    pub gateway: Option<embassy_net::Ipv4Address>,
    pub dns: Option<embassy_net::Ipv4Address>,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            use_dhcp: true,
            static_ip: None,
            gateway: None,
            dns: None,
        }
    }
}

/// Wait for the network link to come up and obtain an IP address
pub async fn wait_for_ip(stack: &Stack<'_>) {
    info!("ETH: Waiting for link...");
    stack.wait_link_up().await;
    info!("ETH: Link up, waiting for IP...");
    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        info!(
            "ETH: Got IP {}.{}.{}.{}",
            config.address.address().0[0],
            config.address.address().0[1],
            config.address.address().0[2],
            config.address.address().0[3],
        );
    }
}
