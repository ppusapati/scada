/// OpenAMP RPMsg Driver for STM32MP157 Cortex-M4
///
/// This module implements the RPMsg (Remote Processor Messaging) protocol
/// over OpenAMP's VirtIO transport. It enables zero-copy message passing
/// between the M4 co-processor and the A7 Linux host.
///
/// Architecture:
///   M4 Firmware ←→ RPMsg ←→ VirtIO Rings ←→ IPCC IRQ ←→ A7 Linux
///
/// Shared Memory Layout (MCUSRAM2: 0x10020000 - 0x1003FFFF):
///   ┌────────────────┬─────────────────────────────────────┐
///   │ 0x10020000     │ VirtIO Ring 0 (M4→A7 TX) — 32 KB   │
///   │ 0x10028000     │ VirtIO Ring 1 (A7→M4 RX) — 32 KB   │
///   │ 0x10030000     │ RPMsg buffer pool — 64 KB           │
///   └────────────────┴─────────────────────────────────────┘
///
/// RPMsg message format (per OpenAMP spec):
///   struct rpmsg_hdr {
///       u32 src;           // source endpoint address
///       u32 dst;           // destination endpoint address
///       u32 reserved;      // reserved, must be 0
///       u16 len;           // payload length in bytes
///       u16 flags;         // message flags
///       u8  data[];        // payload (up to 496 bytes)
///   };
///
/// IPCC (Inter-Processor Communication Controller):
///   Channel 1: M4→A7 notification (TX complete / new message)
///   Channel 2: A7→M4 notification (new message available)

pub mod transport;

use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use defmt::{info, warn, error, debug};
use heapless::Vec;

/// Maximum RPMsg payload size (512 - 16 byte header = 496 bytes)
pub const RPMSG_MAX_PAYLOAD: usize = 496;

/// RPMsg header size
pub const RPMSG_HEADER_SIZE: usize = 16;

/// Total RPMsg buffer size (header + payload)
pub const RPMSG_BUF_SIZE: usize = 512;

/// Number of buffers in the VirtIO ring
pub const VRING_NUM_BUFS: usize = 64;

/// VirtIO ring alignment
pub const VRING_ALIGN: usize = 4096;

/// IPCC register base address (STM32MP157)
const IPCC_BASE: u32 = 0x4400_C000;

/// IPCC registers
mod ipcc_reg {
    pub const C1CR: u32 = 0x00;    // CPU1 (M4) control register
    pub const C1MR: u32 = 0x04;    // CPU1 mask register
    pub const C1SCR: u32 = 0x08;   // CPU1 status clear register
    pub const C1TOC2SR: u32 = 0x0C; // CPU1 to CPU2 status register
    pub const C2CR: u32 = 0x10;    // CPU2 (A7) control register
    pub const C2MR: u32 = 0x14;    // CPU2 mask register
    pub const C2SCR: u32 = 0x18;   // CPU2 status clear register
    pub const C2TOC1SR: u32 = 0x1C; // CPU2 to CPU1 status register
}

/// IPCC channel assignments
const IPCC_CHANNEL_RPMSG: u32 = 0; // Channel 1 (bit 0) for RPMsg

/// Shared memory base addresses (from linker script)
const SHMEM_BASE: u32 = 0x1002_0000;
const VRING0_BASE: u32 = 0x1002_0000;  // M4→A7 (TX)
const VRING1_BASE: u32 = 0x1002_8000;  // A7→M4 (RX)
const RPMSG_BUF_BASE: u32 = 0x1003_0000;

/// RPMsg message header (wire format, matches OpenAMP)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct RpmsgHeader {
    pub src: u32,
    pub dst: u32,
    pub reserved: u32,
    pub len: u16,
    pub flags: u16,
}

impl RpmsgHeader {
    pub const fn new(src: u32, dst: u32, len: u16) -> Self {
        Self {
            src,
            dst,
            reserved: 0,
            len,
            flags: 0,
        }
    }
}

/// VirtIO ring descriptor (matches VirtIO spec 1.0)
#[repr(C)]
#[derive(Clone, Copy)]
struct VringDesc {
    addr: u64,    // Physical address of buffer
    len: u32,     // Buffer length
    flags: u16,   // VRING_DESC_F_NEXT, VRING_DESC_F_WRITE
    next: u16,    // Index of next descriptor (if chained)
}

/// VirtIO available ring
#[repr(C)]
struct VringAvail {
    flags: u16,
    idx: u16,
    ring: [u16; VRING_NUM_BUFS],
}

/// VirtIO used ring element
#[repr(C)]
#[derive(Clone, Copy)]
struct VringUsedElem {
    id: u32,
    len: u32,
}

/// VirtIO used ring
#[repr(C)]
struct VringUsed {
    flags: u16,
    idx: u16,
    ring: [VringUsedElem; VRING_NUM_BUFS],
}

/// RPMsg endpoint state
pub struct RpmsgEndpoint {
    /// Local endpoint address
    local_addr: AtomicU32,
    /// Remote endpoint address (A7 XRCE agent)
    remote_addr: AtomicU32,
    /// TX sequence number
    tx_seq: AtomicU16,
    /// RX sequence number
    rx_seq: AtomicU16,
    /// Endpoint is connected
    connected: AtomicBool,
    /// Last available index we processed (TX ring)
    tx_last_avail: AtomicU16,
    /// Last used index we processed (RX ring)
    rx_last_used: AtomicU16,
}

impl RpmsgEndpoint {
    pub const fn new() -> Self {
        Self {
            local_addr: AtomicU32::new(1024), // XRCE-DDS endpoint
            remote_addr: AtomicU32::new(1025),
            tx_seq: AtomicU16::new(0),
            rx_seq: AtomicU16::new(0),
            connected: AtomicBool::new(false),
            tx_last_avail: AtomicU16::new(0),
            rx_last_used: AtomicU16::new(0),
        }
    }

    /// Initialize the RPMsg subsystem
    ///
    /// Sets up VirtIO rings in shared memory and configures IPCC
    /// for interrupt-driven notification between M4 and A7.
    pub async fn init(&self) {
        info!("RPMsg: Initializing OpenAMP transport...");

        // Initialize VirtIO ring 0 (M4→A7 TX)
        self.init_vring(VRING0_BASE, true);

        // Initialize VirtIO ring 1 (A7→M4 RX)
        self.init_vring(VRING1_BASE, false);

        // Initialize RPMsg buffer pool
        self.init_buffer_pool();

        // Configure IPCC for RPMsg notifications
        self.init_ipcc();

        // Wait for A7 to acknowledge the resource table
        // The A7 remoteproc driver will set up the VirtIO device
        // and notify us via IPCC when ready
        let mut timeout = 0u32;
        while !self.connected.load(Ordering::Acquire) {
            embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
            timeout += 1;

            // Check IPCC for A7 connection notification
            self.check_ipcc_rx();

            if timeout > 100 {
                warn!("RPMsg: Timeout waiting for A7 connection, proceeding...");
                // In production, A7 may not be ready yet — we still initialize
                self.connected.store(true, Ordering::Release);
                break;
            }
        }

        info!("RPMsg: Transport ready (local={}, remote={})",
            self.local_addr.load(Ordering::Relaxed),
            self.remote_addr.load(Ordering::Relaxed));
    }

    /// Initialize a VirtIO ring in shared memory
    fn init_vring(&self, base: u32, is_tx: bool) {
        let desc_ptr = base as *mut VringDesc;
        let avail_offset = core::mem::size_of::<VringDesc>() * VRING_NUM_BUFS;
        let avail_ptr = (base as usize + avail_offset) as *mut VringAvail;

        // Zero out descriptors
        for i in 0..VRING_NUM_BUFS {
            unsafe {
                let desc = &mut *desc_ptr.add(i);
                desc.addr = 0;
                desc.len = 0;
                desc.flags = 0;
                desc.next = 0;
            }
        }

        // Zero out available ring
        unsafe {
            let avail = &mut *avail_ptr;
            avail.flags = 0;
            avail.idx = 0;
            for slot in avail.ring.iter_mut() {
                *slot = 0;
            }
        }

        if is_tx {
            // For TX ring, pre-populate descriptors with buffer addresses
            for i in 0..VRING_NUM_BUFS {
                let buf_addr = RPMSG_BUF_BASE as u64 + (i as u64 * RPMSG_BUF_SIZE as u64);
                unsafe {
                    let desc = &mut *desc_ptr.add(i);
                    desc.addr = buf_addr;
                    desc.len = RPMSG_BUF_SIZE as u32;
                    desc.flags = 0;
                    desc.next = 0;
                }
            }
        }

        debug!("RPMsg: VRing initialized at 0x{:08X} ({})", base,
            if is_tx { "TX" } else { "RX" });
    }

    /// Initialize the RPMsg buffer pool in shared memory
    fn init_buffer_pool(&self) {
        let pool_ptr = RPMSG_BUF_BASE as *mut u8;
        let pool_size = VRING_NUM_BUFS * RPMSG_BUF_SIZE;

        // Zero the entire buffer pool
        unsafe {
            core::ptr::write_bytes(pool_ptr, 0, pool_size);
        }

        debug!("RPMsg: Buffer pool initialized ({} × {} bytes)", VRING_NUM_BUFS, RPMSG_BUF_SIZE);
    }

    /// Configure IPCC for RPMsg notifications
    fn init_ipcc(&self) {
        unsafe {
            // Enable IPCC clock (RCC)
            // Note: On STM32MP157, IPCC clock is typically enabled by A7 bootloader

            // Enable receive interrupt on channel 1 (A7→M4 direction)
            // C1MR: unmask receive occupied interrupt for channel 1
            let c1mr = core::ptr::read_volatile((IPCC_BASE + ipcc_reg::C1MR) as *const u32);
            core::ptr::write_volatile(
                (IPCC_BASE + ipcc_reg::C1MR) as *mut u32,
                c1mr & !(1 << (IPCC_CHANNEL_RPMSG + 16)), // Clear mask bit for RX occupied
            );

            // Enable IPCC RX occupied interrupt in C1CR
            let c1cr = core::ptr::read_volatile((IPCC_BASE + ipcc_reg::C1CR) as *const u32);
            core::ptr::write_volatile(
                (IPCC_BASE + ipcc_reg::C1CR) as *mut u32,
                c1cr | (1 << 0), // RXOIE: RX occupied interrupt enable
            );
        }

        debug!("RPMsg: IPCC configured for RPMsg notifications");
    }

    /// Check IPCC for incoming A7 notifications
    fn check_ipcc_rx(&self) {
        unsafe {
            let c2toc1sr = core::ptr::read_volatile(
                (IPCC_BASE + ipcc_reg::C2TOC1SR) as *const u32
            );

            if c2toc1sr & (1 << IPCC_CHANNEL_RPMSG) != 0 {
                // A7 has sent us a notification — mark connected
                self.connected.store(true, Ordering::Release);

                // Clear the status flag
                core::ptr::write_volatile(
                    (IPCC_BASE + ipcc_reg::C1SCR) as *mut u32,
                    1 << IPCC_CHANNEL_RPMSG,
                );

                debug!("RPMsg: A7 notification received");
            }
        }
    }

    /// Send an RPMsg message to the A7 core
    ///
    /// This writes the message into a VirtIO TX ring buffer and
    /// notifies the A7 via IPCC interrupt.
    ///
    /// # Arguments
    /// * `payload` - Message payload (max 496 bytes)
    ///
    /// # Returns
    /// `true` if message was queued successfully
    pub fn send(&self, payload: &[u8]) -> bool {
        if payload.len() > RPMSG_MAX_PAYLOAD {
            warn!("RPMsg: Payload too large ({} > {})", payload.len(), RPMSG_MAX_PAYLOAD);
            return false;
        }

        let seq = self.tx_seq.fetch_add(1, Ordering::Relaxed);
        let buf_idx = (seq as usize) % VRING_NUM_BUFS;

        // Write RPMsg header + payload into buffer
        let buf_addr = RPMSG_BUF_BASE + (buf_idx * RPMSG_BUF_SIZE) as u32;
        let header = RpmsgHeader::new(
            self.local_addr.load(Ordering::Relaxed),
            self.remote_addr.load(Ordering::Relaxed),
            payload.len() as u16,
        );

        unsafe {
            // Write header
            let hdr_ptr = buf_addr as *mut RpmsgHeader;
            core::ptr::write_volatile(hdr_ptr, header);

            // Write payload after header
            let data_ptr = (buf_addr as usize + RPMSG_HEADER_SIZE) as *mut u8;
            core::ptr::copy_nonoverlapping(payload.as_ptr(), data_ptr, payload.len());
        }

        // Update VirtIO available ring
        let avail_offset = core::mem::size_of::<VringDesc>() * VRING_NUM_BUFS;
        let avail_ptr = (VRING0_BASE as usize + avail_offset) as *mut VringAvail;

        unsafe {
            let avail = &mut *avail_ptr;
            let avail_idx = avail.idx;
            avail.ring[(avail_idx as usize) % VRING_NUM_BUFS] = buf_idx as u16;

            // Memory barrier before updating index
            core::sync::atomic::fence(Ordering::Release);
            avail.idx = avail_idx.wrapping_add(1);
        }

        // Notify A7 via IPCC
        self.notify_a7();

        debug!("RPMsg: Sent {} bytes (seq={})", payload.len(), seq);
        true
    }

    /// Receive an RPMsg message from the A7 core
    ///
    /// Checks the VirtIO RX ring for available messages.
    ///
    /// # Returns
    /// `Some(data)` if a message is available, `None` otherwise
    pub fn recv(&self, buf: &mut [u8]) -> Option<usize> {
        // Check VirtIO used ring for RX
        let used_offset = core::mem::size_of::<VringDesc>() * VRING_NUM_BUFS
            + core::mem::size_of::<VringAvail>();
        // Align to next page boundary
        let used_offset = (used_offset + VRING_ALIGN - 1) & !(VRING_ALIGN - 1);
        let used_ptr = (VRING1_BASE as usize + used_offset) as *const VringUsed;

        let last_used = self.rx_last_used.load(Ordering::Relaxed);

        let used_idx = unsafe { core::ptr::read_volatile(&(*used_ptr).idx) };

        if last_used == used_idx {
            return None; // No new messages
        }

        // Read the used ring element
        let elem_idx = (last_used as usize) % VRING_NUM_BUFS;
        let elem = unsafe {
            core::ptr::read_volatile(&(*used_ptr).ring[elem_idx])
        };

        // Read the RPMsg buffer
        let desc_ptr = VRING1_BASE as *const VringDesc;
        let desc = unsafe { core::ptr::read_volatile(desc_ptr.add(elem.id as usize)) };

        let hdr_ptr = desc.addr as *const RpmsgHeader;
        let header = unsafe { core::ptr::read_volatile(hdr_ptr) };

        let payload_len = header.len as usize;
        if payload_len > buf.len() || payload_len > RPMSG_MAX_PAYLOAD {
            warn!("RPMsg: RX buffer too small or payload invalid");
            self.rx_last_used.store(last_used.wrapping_add(1), Ordering::Release);
            return None;
        }

        // Copy payload
        let data_ptr = (desc.addr as usize + RPMSG_HEADER_SIZE) as *const u8;
        unsafe {
            core::ptr::copy_nonoverlapping(data_ptr, buf.as_mut_ptr(), payload_len);
        }

        // Advance the used index
        self.rx_last_used.store(last_used.wrapping_add(1), Ordering::Release);

        // Return buffer to available ring (for A7 to reuse)
        self.return_rx_buffer(elem.id as u16);

        debug!("RPMsg: Received {} bytes from endpoint {}", payload_len, header.src);
        Some(payload_len)
    }

    /// Return a consumed RX buffer to the available ring
    fn return_rx_buffer(&self, desc_idx: u16) {
        let avail_offset = core::mem::size_of::<VringDesc>() * VRING_NUM_BUFS;
        let avail_ptr = (VRING1_BASE as usize + avail_offset) as *mut VringAvail;

        unsafe {
            let avail = &mut *avail_ptr;
            let idx = avail.idx;
            avail.ring[(idx as usize) % VRING_NUM_BUFS] = desc_idx;
            core::sync::atomic::fence(Ordering::Release);
            avail.idx = idx.wrapping_add(1);
        }
    }

    /// Notify the A7 core via IPCC that a new message is available
    fn notify_a7(&self) {
        unsafe {
            // Set the channel occupied flag in C1TOC2SR
            // This triggers an interrupt on the A7 side
            core::ptr::write_volatile(
                (IPCC_BASE + ipcc_reg::C1SCR) as *mut u32,
                1 << (IPCC_CHANNEL_RPMSG + 16), // Set TX free flag (bit 16+ch)
            );
        }
    }

    /// Check if the endpoint is connected to the A7
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}
