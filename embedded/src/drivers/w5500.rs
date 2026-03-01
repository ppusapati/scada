/// W5500 Ethernet Controller Driver
///
/// The W5500 is a hardwired TCP/IP embedded Ethernet controller from WIZnet.
/// It provides 8 independent hardware sockets for TCP/UDP communication,
/// offloading the TCP/IP stack from the MCU.
///
/// Used for:
///   - Modbus TCP server (socket 0)
///   - DDS-XRCE UDP client (socket 1)
///   - MQTT client fallback (socket 2)
///   - General TCP/UDP (sockets 3-7)
///
/// SPI interface:
///   - Mode 0 (CPOL=0, CPHA=0) or Mode 3 (CPOL=1, CPHA=1)
///   - Up to 80 MHz SPI clock
///   - Variable-length data mode (VDM) for flexible access
///
/// Memory map:
///   Block Select Bits (BSB[4:0]) in control byte:
///     00000 = Common registers
///     00001 = Socket 0 register
///     00010 = Socket 0 TX buffer
///     00011 = Socket 0 RX buffer
///     00101 = Socket 1 register
///     ...etc (socket N: register=N*4+1, TX=N*4+2, RX=N*4+3)

use crate::hal::spi::{Spi2Eth, SpiError};
use defmt::{info, warn, error, Format};
use embassy_time::{Timer, Instant};

// ============================================================================
// Common Register Addresses (Block 0)
// ============================================================================

#[allow(dead_code)]
pub mod common_reg {
    pub const MR: u16 = 0x0000;       // Mode register
    pub const GAR0: u16 = 0x0001;     // Gateway address (4 bytes)
    pub const SUBR0: u16 = 0x0005;    // Subnet mask (4 bytes)
    pub const SHAR0: u16 = 0x0009;    // Source hardware address (6 bytes, MAC)
    pub const SIPR0: u16 = 0x000F;    // Source IP address (4 bytes)
    pub const INTLEVEL0: u16 = 0x0013; // Interrupt low level timer (2 bytes)
    pub const IR: u16 = 0x0015;       // Interrupt register
    pub const IMR: u16 = 0x0016;      // Interrupt mask register
    pub const SIR: u16 = 0x0017;      // Socket interrupt register
    pub const SIMR: u16 = 0x0018;     // Socket interrupt mask register
    pub const RTR0: u16 = 0x0019;     // Retry time (2 bytes, 100us units)
    pub const RCR: u16 = 0x001B;      // Retry count
    pub const PTIMER: u16 = 0x001C;   // PPP LCP request timer
    pub const PMAGIC: u16 = 0x001D;   // PPP LCP magic number
    pub const PHAR0: u16 = 0x001E;    // PPP destination MAC (6 bytes)
    pub const PSID0: u16 = 0x0024;    // PPP session ID (2 bytes)
    pub const PMRU0: u16 = 0x0026;    // PPP maximum segment size (2 bytes)
    pub const UIPR0: u16 = 0x0028;    // Unreachable IP address (4 bytes)
    pub const UPORTR0: u16 = 0x002C;  // Unreachable port (2 bytes)
    pub const PHYCFGR: u16 = 0x002E;  // PHY configuration register
    pub const VERSIONR: u16 = 0x0039; // Chip version (should be 0x04)
}

// ============================================================================
// Socket Register Offsets (per socket block)
// ============================================================================

#[allow(dead_code)]
pub mod sock_reg {
    pub const MR: u16 = 0x0000;        // Socket mode register
    pub const CR: u16 = 0x0001;        // Socket command register
    pub const IR: u16 = 0x0002;        // Socket interrupt register
    pub const SR: u16 = 0x0003;        // Socket status register
    pub const PORT0: u16 = 0x0004;     // Source port (2 bytes)
    pub const DHAR0: u16 = 0x0006;     // Destination hardware address (6 bytes)
    pub const DIPR0: u16 = 0x000C;     // Destination IP address (4 bytes)
    pub const DPORT0: u16 = 0x0010;    // Destination port (2 bytes)
    pub const MSSR0: u16 = 0x0012;     // Maximum segment size (2 bytes)
    pub const TOS: u16 = 0x0015;       // IP TOS
    pub const TTL: u16 = 0x0016;       // IP TTL
    pub const RXBUF_SIZE: u16 = 0x001E; // Receive buffer size
    pub const TXBUF_SIZE: u16 = 0x001F; // Transmit buffer size
    pub const TX_FSR0: u16 = 0x0020;   // TX free size (2 bytes)
    pub const TX_RD0: u16 = 0x0022;    // TX read pointer (2 bytes)
    pub const TX_WR0: u16 = 0x0024;    // TX write pointer (2 bytes)
    pub const RX_RSR0: u16 = 0x0026;   // RX received size (2 bytes)
    pub const RX_RD0: u16 = 0x0028;    // RX read pointer (2 bytes)
    pub const RX_WR0: u16 = 0x002A;    // RX write pointer (2 bytes)
    pub const IMR2: u16 = 0x002C;      // Socket interrupt mask
    pub const FRAG0: u16 = 0x002D;     // Fragment offset in IP header (2 bytes)
    pub const KPALVTR: u16 = 0x002F;   // Keep alive timer
}

// ============================================================================
// Socket commands
// ============================================================================

#[allow(dead_code)]
pub mod sock_cmd {
    pub const OPEN: u8 = 0x01;
    pub const LISTEN: u8 = 0x02;
    pub const CONNECT: u8 = 0x04;
    pub const DISCON: u8 = 0x08;
    pub const CLOSE: u8 = 0x10;
    pub const SEND: u8 = 0x20;
    pub const SEND_MAC: u8 = 0x21;
    pub const SEND_KEEP: u8 = 0x22;
    pub const RECV: u8 = 0x40;
}

// ============================================================================
// Socket status values
// ============================================================================

#[allow(dead_code)]
pub mod sock_status {
    pub const CLOSED: u8 = 0x00;
    pub const INIT: u8 = 0x13;
    pub const LISTEN: u8 = 0x14;
    pub const SYNSENT: u8 = 0x15;
    pub const SYNRECV: u8 = 0x16;
    pub const ESTABLISHED: u8 = 0x17;
    pub const FIN_WAIT: u8 = 0x18;
    pub const CLOSING: u8 = 0x1A;
    pub const TIME_WAIT: u8 = 0x1B;
    pub const CLOSE_WAIT: u8 = 0x1C;
    pub const LAST_ACK: u8 = 0x1D;
    pub const UDP: u8 = 0x22;
    pub const MACRAW: u8 = 0x42;
}

// ============================================================================
// Socket mode register bits
// ============================================================================

#[allow(dead_code)]
pub mod sock_mode {
    pub const CLOSED: u8 = 0x00;
    pub const TCP: u8 = 0x01;
    pub const UDP: u8 = 0x02;
    pub const MACRAW: u8 = 0x04;
    pub const MULTI_MFEN: u8 = 0x80;   // Multicasting (UDP) / MAC filter (MACRAW)
    pub const BCASTB: u8 = 0x40;       // Broadcast blocking (UDP)
    pub const ND_MC: u8 = 0x20;        // No delayed ACK (TCP) / Multicast (UDP)
    pub const UCASTB_MIP6B: u8 = 0x10; // Unicast blocking (UDP) / IPv6 blocking
}

// ============================================================================
// Socket interrupt register bits
// ============================================================================

#[allow(dead_code)]
pub mod sock_ir {
    pub const CON: u8 = 0x01;     // Connected
    pub const DISCON: u8 = 0x02;  // Disconnected
    pub const RECV: u8 = 0x04;    // Data received
    pub const TIMEOUT: u8 = 0x08; // Timeout occurred
    pub const SEND_OK: u8 = 0x10; // Send completed
}

// ============================================================================
// PHY configuration register bits
// ============================================================================

#[allow(dead_code)]
pub mod phycfg {
    pub const RST: u8 = 1 << 7;       // PHY reset (write 0, then 1)
    pub const OPMD: u8 = 1 << 6;      // Operation mode: 1=config by register
    pub const OPMDC_AUTO: u8 = 0b111 << 3; // Auto-negotiation
    pub const OPMDC_100F: u8 = 0b100 << 3; // 100Mbps Full-duplex
    pub const OPMDC_100H: u8 = 0b011 << 3; // 100Mbps Half-duplex
    pub const OPMDC_10F: u8 = 0b010 << 3;  // 10Mbps Full-duplex
    pub const OPMDC_10H: u8 = 0b001 << 3;  // 10Mbps Half-duplex
    pub const DPX: u8 = 1 << 2;       // Duplex status (read-only)
    pub const SPD: u8 = 1 << 1;       // Speed status (read-only)
    pub const LNK: u8 = 1 << 0;       // Link status (read-only)
}

// ============================================================================
// Error type
// ============================================================================

#[derive(Clone, Copy, Debug, Format)]
pub enum W5500Error {
    Spi(SpiError),
    SocketClosed,
    Timeout,
    InvalidSocket,
    BufferFull,
    NotConnected,
    LinkDown,
    InvalidState,
}

impl From<SpiError> for W5500Error {
    fn from(e: SpiError) -> Self {
        W5500Error::Spi(e)
    }
}

// ============================================================================
// Control byte construction
// ============================================================================

/// Block Select Bits for the control byte
fn block_select_register(socket: u8) -> u8 {
    (socket * 4 + 1) << 3
}

fn block_select_tx(socket: u8) -> u8 {
    (socket * 4 + 2) << 3
}

fn block_select_rx(socket: u8) -> u8 {
    (socket * 4 + 3) << 3
}

fn block_select_common() -> u8 {
    0x00
}

/// Read/Write mode bits
const RWB_READ: u8 = 0x00;
const RWB_WRITE: u8 = 0x04;

/// Variable Data Length mode
const VDM: u8 = 0x00;

// ============================================================================
// Network configuration
// ============================================================================

/// Network configuration for the W5500
pub struct W5500NetConfig {
    pub mac: [u8; 6],
    pub ip: [u8; 4],
    pub subnet: [u8; 4],
    pub gateway: [u8; 4],
}

impl Default for W5500NetConfig {
    fn default() -> Self {
        Self {
            mac: [0x02, 0x53, 0x43, 0xA0, 0x01, 0x01],
            ip: [192, 168, 1, 100],
            subnet: [255, 255, 255, 0],
            gateway: [192, 168, 1, 1],
        }
    }
}

/// Socket protocol type
#[derive(Clone, Copy, PartialEq, Format)]
pub enum SocketProtocol {
    Tcp,
    Udp,
}

/// Link status
#[derive(Clone, Copy, PartialEq, Format)]
pub enum LinkStatus {
    Down,
    Up10Half,
    Up10Full,
    Up100Half,
    Up100Full,
}

// ============================================================================
// W5500 Driver
// ============================================================================

/// W5500 Ethernet controller driver
pub struct W5500 {
    initialized: bool,
}

impl W5500 {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    /// Initialize the W5500
    pub async fn init(
        &mut self,
        spi: &mut Spi2Eth,
        config: &W5500NetConfig,
    ) -> Result<(), W5500Error> {
        // Software reset via mode register
        self.write_common_reg(spi, common_reg::MR, &[0x80]).await?;
        Timer::after_millis(10).await;

        // Verify chip version
        let version = self.read_common_reg_byte(spi, common_reg::VERSIONR).await?;
        if version != 0x04 {
            error!("W5500: wrong chip version 0x{:02X}, expected 0x04", version);
            return Err(W5500Error::InvalidState);
        }
        info!("W5500: chip version verified (0x{:02X})", version);

        // Configure network parameters
        self.write_common_reg(spi, common_reg::SHAR0, &config.mac).await?;
        self.write_common_reg(spi, common_reg::SIPR0, &config.ip).await?;
        self.write_common_reg(spi, common_reg::SUBR0, &config.subnet).await?;
        self.write_common_reg(spi, common_reg::GAR0, &config.gateway).await?;

        // Configure retry: 200ms timeout, 8 retries
        self.write_common_reg(spi, common_reg::RTR0, &[0x07, 0xD0]).await?; // 2000 * 100us = 200ms
        self.write_common_reg_byte(spi, common_reg::RCR, 8).await?;

        // Configure PHY: auto-negotiation, register-controlled
        self.write_common_reg_byte(
            spi,
            common_reg::PHYCFGR,
            phycfg::RST | phycfg::OPMD | phycfg::OPMDC_AUTO,
        ).await?;

        // Set socket buffer sizes: 2KB each (8 sockets * 2KB = 16KB TX + 16KB RX)
        for s in 0..8u8 {
            self.write_socket_reg_byte(spi, s, sock_reg::RXBUF_SIZE, 2).await?;
            self.write_socket_reg_byte(spi, s, sock_reg::TXBUF_SIZE, 2).await?;
        }

        self.initialized = true;
        info!("W5500: initialized, IP={}.{}.{}.{}",
            config.ip[0], config.ip[1], config.ip[2], config.ip[3]);

        Ok(())
    }

    // ── Link status ──

    /// Check the Ethernet link status
    pub async fn link_status(&mut self, spi: &mut Spi2Eth) -> Result<LinkStatus, W5500Error> {
        let phy = self.read_common_reg_byte(spi, common_reg::PHYCFGR).await?;
        if phy & phycfg::LNK == 0 {
            return Ok(LinkStatus::Down);
        }
        let speed_100 = phy & phycfg::SPD != 0;
        let full_duplex = phy & phycfg::DPX != 0;
        Ok(match (speed_100, full_duplex) {
            (true, true) => LinkStatus::Up100Full,
            (true, false) => LinkStatus::Up100Half,
            (false, true) => LinkStatus::Up10Full,
            (false, false) => LinkStatus::Up10Half,
        })
    }

    /// Wait for link to come up
    pub async fn wait_link_up(
        &mut self,
        spi: &mut Spi2Eth,
        timeout_ms: u32,
    ) -> Result<LinkStatus, W5500Error> {
        let start = Instant::now();
        loop {
            let status = self.link_status(spi).await?;
            if status != LinkStatus::Down {
                info!("W5500: link up ({})", status);
                return Ok(status);
            }
            if Instant::now().as_millis() - start.as_millis() > timeout_ms as u64 {
                return Err(W5500Error::Timeout);
            }
            Timer::after_millis(100).await;
        }
    }

    // ── Socket operations ──

    /// Open a socket with the given protocol and local port
    pub async fn socket_open(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        protocol: SocketProtocol,
        port: u16,
    ) -> Result<(), W5500Error> {
        if socket >= 8 {
            return Err(W5500Error::InvalidSocket);
        }

        // Close first if not already closed
        let status = self.socket_status(spi, socket).await?;
        if status != sock_status::CLOSED {
            self.socket_close(spi, socket).await?;
            Timer::after_millis(5).await;
        }

        // Set protocol mode
        let mode = match protocol {
            SocketProtocol::Tcp => sock_mode::TCP,
            SocketProtocol::Udp => sock_mode::UDP,
        };
        self.write_socket_reg_byte(spi, socket, sock_reg::MR, mode).await?;

        // Set source port
        let port_bytes = port.to_be_bytes();
        self.write_socket_reg(spi, socket, sock_reg::PORT0, &port_bytes).await?;

        // Execute OPEN command
        self.socket_command(spi, socket, sock_cmd::OPEN).await?;

        // Wait for socket to be initialized
        let expected = match protocol {
            SocketProtocol::Tcp => sock_status::INIT,
            SocketProtocol::Udp => sock_status::UDP,
        };
        self.wait_socket_status(spi, socket, expected, 1000).await?;

        info!("W5500: socket {} opened ({}, port {})", socket, protocol, port);
        Ok(())
    }

    /// Close a socket
    pub async fn socket_close(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
    ) -> Result<(), W5500Error> {
        if socket >= 8 {
            return Err(W5500Error::InvalidSocket);
        }
        self.socket_command(spi, socket, sock_cmd::CLOSE).await?;
        self.wait_socket_status(spi, socket, sock_status::CLOSED, 1000).await
    }

    /// Start listening for incoming TCP connections (server mode)
    pub async fn socket_listen(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
    ) -> Result<(), W5500Error> {
        self.socket_command(spi, socket, sock_cmd::LISTEN).await?;
        self.wait_socket_status(spi, socket, sock_status::LISTEN, 1000).await
    }

    /// Connect to a remote TCP server
    pub async fn socket_connect(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        dest_ip: [u8; 4],
        dest_port: u16,
    ) -> Result<(), W5500Error> {
        // Set destination IP
        self.write_socket_reg(spi, socket, sock_reg::DIPR0, &dest_ip).await?;
        // Set destination port
        let port_bytes = dest_port.to_be_bytes();
        self.write_socket_reg(spi, socket, sock_reg::DPORT0, &port_bytes).await?;

        // Execute CONNECT
        self.socket_command(spi, socket, sock_cmd::CONNECT).await?;
        self.wait_socket_status(spi, socket, sock_status::ESTABLISHED, 5000).await
    }

    /// Send data on a socket
    pub async fn socket_send(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        data: &[u8],
    ) -> Result<usize, W5500Error> {
        if socket >= 8 || data.is_empty() {
            return Err(W5500Error::InvalidSocket);
        }

        // Check free TX buffer space
        let free = self.socket_tx_free(spi, socket).await?;
        let send_len = data.len().min(free as usize);
        if send_len == 0 {
            return Err(W5500Error::BufferFull);
        }

        // Get TX write pointer
        let tx_wr = self.read_socket_reg_u16(spi, socket, sock_reg::TX_WR0).await?;

        // Write data to TX buffer
        self.write_socket_buf(spi, socket, tx_wr, &data[..send_len]).await?;

        // Update TX write pointer
        let new_wr = tx_wr.wrapping_add(send_len as u16);
        let wr_bytes = new_wr.to_be_bytes();
        self.write_socket_reg(spi, socket, sock_reg::TX_WR0, &wr_bytes).await?;

        // Execute SEND command
        self.socket_command(spi, socket, sock_cmd::SEND).await?;

        // Wait for send completion
        let start = Instant::now();
        loop {
            let ir = self.read_socket_reg_byte(spi, socket, sock_reg::IR).await?;
            if ir & sock_ir::SEND_OK != 0 {
                // Clear interrupt
                self.write_socket_reg_byte(spi, socket, sock_reg::IR, sock_ir::SEND_OK).await?;
                return Ok(send_len);
            }
            if ir & sock_ir::TIMEOUT != 0 {
                self.write_socket_reg_byte(spi, socket, sock_reg::IR, sock_ir::TIMEOUT).await?;
                return Err(W5500Error::Timeout);
            }
            if Instant::now().as_millis() - start.as_millis() > 5000 {
                return Err(W5500Error::Timeout);
            }
            Timer::after_millis(1).await;
        }
    }

    /// Receive data from a socket
    pub async fn socket_recv(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        buf: &mut [u8],
    ) -> Result<usize, W5500Error> {
        if socket >= 8 || buf.is_empty() {
            return Err(W5500Error::InvalidSocket);
        }

        // Check received data size
        let rx_size = self.socket_rx_size(spi, socket).await?;
        if rx_size == 0 {
            return Ok(0);
        }

        let read_len = buf.len().min(rx_size as usize);

        // Get RX read pointer
        let rx_rd = self.read_socket_reg_u16(spi, socket, sock_reg::RX_RD0).await?;

        // Read data from RX buffer
        self.read_socket_buf(spi, socket, rx_rd, &mut buf[..read_len]).await?;

        // Update RX read pointer
        let new_rd = rx_rd.wrapping_add(read_len as u16);
        let rd_bytes = new_rd.to_be_bytes();
        self.write_socket_reg(spi, socket, sock_reg::RX_RD0, &rd_bytes).await?;

        // Execute RECV command to release buffer
        self.socket_command(spi, socket, sock_cmd::RECV).await?;

        Ok(read_len)
    }

    /// Send UDP datagram
    pub async fn socket_sendto(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        dest_ip: [u8; 4],
        dest_port: u16,
        data: &[u8],
    ) -> Result<usize, W5500Error> {
        // Set destination for UDP
        self.write_socket_reg(spi, socket, sock_reg::DIPR0, &dest_ip).await?;
        let port_bytes = dest_port.to_be_bytes();
        self.write_socket_reg(spi, socket, sock_reg::DPORT0, &port_bytes).await?;

        // Use regular send
        self.socket_send(spi, socket, data).await
    }

    /// Receive UDP datagram (returns data length; destination info from socket registers)
    pub async fn socket_recvfrom(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        buf: &mut [u8],
    ) -> Result<(usize, [u8; 4], u16), W5500Error> {
        let rx_size = self.socket_rx_size(spi, socket).await?;
        if rx_size < 8 {
            return Ok((0, [0; 4], 0));
        }

        // UDP data includes 8-byte header: [IP(4), Port(2), Length(2), Data...]
        let rx_rd = self.read_socket_reg_u16(spi, socket, sock_reg::RX_RD0).await?;

        let mut header = [0u8; 8];
        self.read_socket_buf(spi, socket, rx_rd, &mut header).await?;

        let src_ip = [header[0], header[1], header[2], header[3]];
        let src_port = u16::from_be_bytes([header[4], header[5]]);
        let data_len = u16::from_be_bytes([header[6], header[7]]) as usize;

        let read_len = buf.len().min(data_len);
        self.read_socket_buf(
            spi, socket,
            rx_rd.wrapping_add(8),
            &mut buf[..read_len],
        ).await?;

        // Update RX read pointer past header + data
        let new_rd = rx_rd.wrapping_add(8 + data_len as u16);
        let rd_bytes = new_rd.to_be_bytes();
        self.write_socket_reg(spi, socket, sock_reg::RX_RD0, &rd_bytes).await?;
        self.socket_command(spi, socket, sock_cmd::RECV).await?;

        Ok((read_len, src_ip, src_port))
    }

    // ── Helper functions ──

    /// Get socket status
    pub async fn socket_status(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
    ) -> Result<u8, W5500Error> {
        self.read_socket_reg_byte(spi, socket, sock_reg::SR).await
    }

    /// Get available TX buffer space
    pub async fn socket_tx_free(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
    ) -> Result<u16, W5500Error> {
        self.read_socket_reg_u16(spi, socket, sock_reg::TX_FSR0).await
    }

    /// Get received data size
    pub async fn socket_rx_size(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
    ) -> Result<u16, W5500Error> {
        self.read_socket_reg_u16(spi, socket, sock_reg::RX_RSR0).await
    }

    /// Execute a socket command and wait for completion
    async fn socket_command(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        command: u8,
    ) -> Result<(), W5500Error> {
        self.write_socket_reg_byte(spi, socket, sock_reg::CR, command).await?;
        // Wait for command to be processed (CR reads back as 0x00)
        let start = Instant::now();
        loop {
            let cr = self.read_socket_reg_byte(spi, socket, sock_reg::CR).await?;
            if cr == 0x00 {
                return Ok(());
            }
            if Instant::now().as_millis() - start.as_millis() > 1000 {
                return Err(W5500Error::Timeout);
            }
            Timer::after_micros(50).await;
        }
    }

    /// Wait for a specific socket status
    async fn wait_socket_status(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        expected: u8,
        timeout_ms: u32,
    ) -> Result<(), W5500Error> {
        let start = Instant::now();
        loop {
            let status = self.socket_status(spi, socket).await?;
            if status == expected {
                return Ok(());
            }
            if status == sock_status::CLOSED && expected != sock_status::CLOSED {
                return Err(W5500Error::SocketClosed);
            }
            if Instant::now().as_millis() - start.as_millis() > timeout_ms as u64 {
                return Err(W5500Error::Timeout);
            }
            Timer::after_millis(1).await;
        }
    }

    /// Update MAC address
    pub async fn set_mac(
        &mut self,
        spi: &mut Spi2Eth,
        mac: &[u8; 6],
    ) -> Result<(), W5500Error> {
        self.write_common_reg(spi, common_reg::SHAR0, mac).await
    }

    /// Update IP address
    pub async fn set_ip(
        &mut self,
        spi: &mut Spi2Eth,
        ip: &[u8; 4],
    ) -> Result<(), W5500Error> {
        self.write_common_reg(spi, common_reg::SIPR0, ip).await
    }

    /// Read current IP address
    pub async fn get_ip(&mut self, spi: &mut Spi2Eth) -> Result<[u8; 4], W5500Error> {
        let mut ip = [0u8; 4];
        self.read_common_reg(spi, common_reg::SIPR0, &mut ip).await?;
        Ok(ip)
    }

    // ── Low-level SPI register access ──

    /// Write to common register block
    async fn write_common_reg(
        &mut self,
        spi: &mut Spi2Eth,
        addr: u16,
        data: &[u8],
    ) -> Result<(), W5500Error> {
        let control = block_select_common() | RWB_WRITE | VDM;
        spi.w5500_transfer(addr, control, data, &mut []).await?;
        Ok(())
    }

    /// Write single byte to common register
    async fn write_common_reg_byte(
        &mut self,
        spi: &mut Spi2Eth,
        addr: u16,
        value: u8,
    ) -> Result<(), W5500Error> {
        self.write_common_reg(spi, addr, &[value]).await
    }

    /// Read from common register block
    async fn read_common_reg(
        &mut self,
        spi: &mut Spi2Eth,
        addr: u16,
        buf: &mut [u8],
    ) -> Result<(), W5500Error> {
        let control = block_select_common() | RWB_READ | VDM;
        spi.w5500_transfer(addr, control, &[], buf).await?;
        Ok(())
    }

    /// Read single byte from common register
    async fn read_common_reg_byte(
        &mut self,
        spi: &mut Spi2Eth,
        addr: u16,
    ) -> Result<u8, W5500Error> {
        let mut buf = [0u8; 1];
        self.read_common_reg(spi, addr, &mut buf).await?;
        Ok(buf[0])
    }

    /// Write to socket register block
    async fn write_socket_reg(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        addr: u16,
        data: &[u8],
    ) -> Result<(), W5500Error> {
        let control = block_select_register(socket) | RWB_WRITE | VDM;
        spi.w5500_transfer(addr, control, data, &mut []).await?;
        Ok(())
    }

    /// Write single byte to socket register
    async fn write_socket_reg_byte(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        addr: u16,
        value: u8,
    ) -> Result<(), W5500Error> {
        self.write_socket_reg(spi, socket, addr, &[value]).await
    }

    /// Read from socket register block
    async fn read_socket_reg(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        addr: u16,
        buf: &mut [u8],
    ) -> Result<(), W5500Error> {
        let control = block_select_register(socket) | RWB_READ | VDM;
        spi.w5500_transfer(addr, control, &[], buf).await?;
        Ok(())
    }

    /// Read single byte from socket register
    async fn read_socket_reg_byte(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        addr: u16,
    ) -> Result<u8, W5500Error> {
        let mut buf = [0u8; 1];
        self.read_socket_reg(spi, socket, addr, &mut buf).await?;
        Ok(buf[0])
    }

    /// Read 16-bit value from socket register
    async fn read_socket_reg_u16(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        addr: u16,
    ) -> Result<u16, W5500Error> {
        let mut buf = [0u8; 2];
        self.read_socket_reg(spi, socket, addr, &mut buf).await?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Write to socket TX buffer
    async fn write_socket_buf(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        offset: u16,
        data: &[u8],
    ) -> Result<(), W5500Error> {
        let control = block_select_tx(socket) | RWB_WRITE | VDM;
        spi.w5500_transfer(offset, control, data, &mut []).await?;
        Ok(())
    }

    /// Read from socket RX buffer
    async fn read_socket_buf(
        &mut self,
        spi: &mut Spi2Eth,
        socket: u8,
        offset: u16,
        buf: &mut [u8],
    ) -> Result<(), W5500Error> {
        let control = block_select_rx(socket) | RWB_READ | VDM;
        spi.w5500_transfer(offset, control, &[], buf).await?;
        Ok(())
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
