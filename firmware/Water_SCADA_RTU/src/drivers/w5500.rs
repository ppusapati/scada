/// W5500 — Hardwired TCP/IP Ethernet Controller Driver
///
/// Datasheet: WIZnet W5500
///
/// The W5500 provides:
///   - Hardwired TCP/IP stack (offloaded from MCU)
///   - 8 independent sockets
///   - 10/100 Mbps Ethernet PHY
///   - SPI interface up to 80 MHz
///
/// On WS-PCB-001:
///   - Socket 0: Modbus TCP server (port 502)
///   - Socket 1-7: Reserved for future use

use embassy_stm32::gpio::Output;
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};
use defmt::{info, warn, error};

/// W5500 register blocks (Block Select Bits)
mod bsb {
    pub const COMMON: u8 = 0x00;
    pub const SOCKET0_REG: u8 = 0x01;
    pub const SOCKET0_TX: u8 = 0x02;
    pub const SOCKET0_RX: u8 = 0x03;
}

/// W5500 common registers
mod common_reg {
    pub const MR: u16 = 0x0000;        // Mode register
    pub const GAR: u16 = 0x0001;       // Gateway address (4 bytes)
    pub const SUBR: u16 = 0x0005;      // Subnet mask (4 bytes)
    pub const SHAR: u16 = 0x0009;      // Source hardware address / MAC (6 bytes)
    pub const SIPR: u16 = 0x000F;      // Source IP address (4 bytes)
    pub const PHYCFGR: u16 = 0x002E;   // PHY configuration
    pub const VERSIONR: u16 = 0x0039;  // Chip version (should be 0x04)
}

/// W5500 socket registers (offsets within socket register block)
mod sock_reg {
    pub const SN_MR: u16 = 0x0000;     // Socket mode
    pub const SN_CR: u16 = 0x0001;     // Socket command
    pub const SN_IR: u16 = 0x0002;     // Socket interrupt
    pub const SN_SR: u16 = 0x0003;     // Socket status
    pub const SN_PORT: u16 = 0x0004;   // Source port (2 bytes)
    pub const SN_TX_FSR: u16 = 0x0020; // TX free size (2 bytes)
    pub const SN_TX_WR: u16 = 0x0024;  // TX write pointer (2 bytes)
    pub const SN_RX_RSR: u16 = 0x0026; // RX received size (2 bytes)
    pub const SN_RX_RD: u16 = 0x0028;  // RX read pointer (2 bytes)
}

/// Socket commands
mod sock_cmd {
    pub const OPEN: u8 = 0x01;
    pub const LISTEN: u8 = 0x02;
    pub const CONNECT: u8 = 0x04;
    pub const DISCONNECT: u8 = 0x08;
    pub const CLOSE: u8 = 0x10;
    pub const SEND: u8 = 0x20;
    pub const RECV: u8 = 0x40;
}

/// Socket status values
mod sock_status {
    pub const CLOSED: u8 = 0x00;
    pub const INIT: u8 = 0x13;
    pub const LISTEN: u8 = 0x14;
    pub const ESTABLISHED: u8 = 0x17;
    pub const CLOSE_WAIT: u8 = 0x1C;
}

/// Network configuration
pub struct NetConfig {
    pub mac: [u8; 6],
    pub ip: [u8; 4],
    pub subnet: [u8; 4],
    pub gateway: [u8; 4],
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            mac: [0x02, 0x50, 0x39, 0x65, 0x00, 0x01], // P9e = 0x50,0x39,0x65
            ip: [192, 168, 1, 100],
            subnet: [255, 255, 255, 0],
            gateway: [192, 168, 1, 1],
        }
    }
}

/// W5500 driver
pub struct W5500<'a> {
    spi: Spi<'a, Async>,
    cs: Output<'a>,
    rst: Output<'a>,
}

impl<'a> W5500<'a> {
    pub fn new(
        spi: Spi<'a, Async>,
        cs: Output<'a>,
        rst: Output<'a>,
    ) -> Self {
        Self { spi, cs, rst }
    }

    /// Hardware reset and initialize with network config
    pub async fn init(&mut self, config: &NetConfig) -> bool {
        info!("W5500: Initializing...");

        // Hardware reset
        self.rst.set_low();
        Timer::after(Duration::from_millis(10)).await;
        self.rst.set_high();
        Timer::after(Duration::from_millis(150)).await;

        // Software reset
        self.write_common(common_reg::MR, &[0x80]).await;
        Timer::after(Duration::from_millis(10)).await;

        // Verify chip version
        let mut ver = [0u8; 1];
        self.read_common(common_reg::VERSIONR, &mut ver).await;
        if ver[0] != 0x04 {
            error!("W5500: Unexpected version 0x{:02X} (expected 0x04)", ver[0]);
            return false;
        }

        // Set network configuration
        self.write_common(common_reg::GAR, &config.gateway).await;
        self.write_common(common_reg::SUBR, &config.subnet).await;
        self.write_common(common_reg::SHAR, &config.mac).await;
        self.write_common(common_reg::SIPR, &config.ip).await;

        // Verify IP was set
        let mut ip_check = [0u8; 4];
        self.read_common(common_reg::SIPR, &mut ip_check).await;
        if ip_check != config.ip {
            error!("W5500: IP verify failed");
            return false;
        }

        info!(
            "W5500: Initialized IP={}.{}.{}.{}",
            config.ip[0], config.ip[1], config.ip[2], config.ip[3]
        );

        true
    }

    /// Check if Ethernet link is up
    pub async fn link_up(&mut self) -> bool {
        let mut phy = [0u8; 1];
        self.read_common(common_reg::PHYCFGR, &mut phy).await;
        (phy[0] & 0x01) != 0
    }

    /// Open socket 0 as TCP server on given port
    pub async fn socket_listen(&mut self, socket: u8, port: u16) -> bool {
        let bsb = (socket * 4 + 1) as u8; // Socket register block

        // Close any existing connection
        self.write_socket(bsb, sock_reg::SN_CR, &[sock_cmd::CLOSE]).await;
        Timer::after(Duration::from_millis(5)).await;

        // Set TCP mode
        self.write_socket(bsb, sock_reg::SN_MR, &[0x01]).await; // TCP mode

        // Set port
        let port_bytes = port.to_be_bytes();
        self.write_socket(bsb, sock_reg::SN_PORT, &port_bytes).await;

        // Open socket
        self.write_socket(bsb, sock_reg::SN_CR, &[sock_cmd::OPEN]).await;
        Timer::after(Duration::from_millis(5)).await;

        // Verify INIT state
        let mut status = [0u8; 1];
        self.read_socket(bsb, sock_reg::SN_SR, &mut status).await;
        if status[0] != sock_status::INIT {
            warn!("W5500: Socket {} failed to INIT (status=0x{:02X})", socket, status[0]);
            return false;
        }

        // Listen
        self.write_socket(bsb, sock_reg::SN_CR, &[sock_cmd::LISTEN]).await;
        Timer::after(Duration::from_millis(5)).await;

        self.read_socket(bsb, sock_reg::SN_SR, &mut status).await;
        if status[0] != sock_status::LISTEN {
            warn!("W5500: Socket {} failed to LISTEN", socket);
            return false;
        }

        info!("W5500: Socket {} listening on port {}", socket, port);
        true
    }

    /// Check socket status
    pub async fn socket_status(&mut self, socket: u8) -> u8 {
        let bsb = (socket * 4 + 1) as u8;
        let mut status = [0u8; 1];
        self.read_socket(bsb, sock_reg::SN_SR, &mut status).await;
        status[0]
    }

    /// Get number of bytes available to read
    pub async fn socket_rx_available(&mut self, socket: u8) -> u16 {
        let bsb = (socket * 4 + 1) as u8;
        let mut buf = [0u8; 2];
        self.read_socket(bsb, sock_reg::SN_RX_RSR, &mut buf).await;
        u16::from_be_bytes(buf)
    }

    /// Read data from socket RX buffer
    pub async fn socket_recv(&mut self, socket: u8, data: &mut [u8]) -> usize {
        let rx_size = self.socket_rx_available(socket).await as usize;
        if rx_size == 0 {
            return 0;
        }

        let read_len = core::cmp::min(rx_size, data.len());
        let bsb_rx = (socket * 4 + 3) as u8; // RX buffer block
        let bsb_reg = (socket * 4 + 1) as u8;

        // Get read pointer
        let mut ptr_buf = [0u8; 2];
        self.read_socket(bsb_reg, sock_reg::SN_RX_RD, &mut ptr_buf).await;
        let rx_ptr = u16::from_be_bytes(ptr_buf);

        // Read data from RX buffer
        self.read_block(bsb_rx, rx_ptr, &mut data[..read_len]).await;

        // Update read pointer
        let new_ptr = rx_ptr.wrapping_add(read_len as u16);
        self.write_socket(bsb_reg, sock_reg::SN_RX_RD, &new_ptr.to_be_bytes()).await;

        // Issue RECV command
        self.write_socket(bsb_reg, sock_reg::SN_CR, &[sock_cmd::RECV]).await;

        read_len
    }

    /// Write data to socket TX buffer
    pub async fn socket_send(&mut self, socket: u8, data: &[u8]) -> bool {
        let bsb_tx = (socket * 4 + 2) as u8;  // TX buffer block
        let bsb_reg = (socket * 4 + 1) as u8;

        // Check free TX buffer space
        let mut fsr = [0u8; 2];
        self.read_socket(bsb_reg, sock_reg::SN_TX_FSR, &mut fsr).await;
        let free = u16::from_be_bytes(fsr) as usize;
        if free < data.len() {
            warn!("W5500: TX buffer full ({} free, {} needed)", free, data.len());
            return false;
        }

        // Get write pointer
        let mut ptr_buf = [0u8; 2];
        self.read_socket(bsb_reg, sock_reg::SN_TX_WR, &mut ptr_buf).await;
        let tx_ptr = u16::from_be_bytes(ptr_buf);

        // Write data to TX buffer
        self.write_block(bsb_tx, tx_ptr, data).await;

        // Update write pointer
        let new_ptr = tx_ptr.wrapping_add(data.len() as u16);
        self.write_socket(bsb_reg, sock_reg::SN_TX_WR, &new_ptr.to_be_bytes()).await;

        // Issue SEND command
        self.write_socket(bsb_reg, sock_reg::SN_CR, &[sock_cmd::SEND]).await;

        true
    }

    /// Disconnect socket
    pub async fn socket_disconnect(&mut self, socket: u8) {
        let bsb = (socket * 4 + 1) as u8;
        self.write_socket(bsb, sock_reg::SN_CR, &[sock_cmd::DISCONNECT]).await;
    }

    // ── Low-level SPI register access ──

    async fn write_common(&mut self, addr: u16, data: &[u8]) {
        self.write_block(bsb::COMMON, addr, data).await;
    }

    async fn read_common(&mut self, addr: u16, data: &mut [u8]) {
        self.read_block(bsb::COMMON, addr, data).await;
    }

    async fn write_socket(&mut self, bsb: u8, addr: u16, data: &[u8]) {
        self.write_block(bsb, addr, data).await;
    }

    async fn read_socket(&mut self, bsb: u8, addr: u16, data: &mut [u8]) {
        self.read_block(bsb, addr, data).await;
    }

    /// W5500 SPI frame: [addr_hi, addr_lo, control_byte, data...]
    /// Control byte: BSB(5) | R/W(1) | OM(2)
    async fn write_block(&mut self, bsb: u8, addr: u16, data: &[u8]) {
        let header = [
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            (bsb << 3) | 0x04, // Write, VDM mode
        ];
        self.cs.set_low();
        let _ = self.spi.write(&header).await;
        let _ = self.spi.write(data).await;
        self.cs.set_high();
    }

    async fn read_block(&mut self, bsb: u8, addr: u16, data: &mut [u8]) {
        let header = [
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            (bsb << 3) | 0x00, // Read, VDM mode
        ];
        self.cs.set_low();
        let _ = self.spi.write(&header).await;
        let _ = self.spi.read(data).await;
        self.cs.set_high();
    }
}
