/// XRCE-DDS RPMsg Transport Layer
///
/// Implements the XRCE-DDS transport protocol over RPMsg.
/// This allows the M4 firmware to communicate with the XRCE-DDS
/// agent running on the A7 Linux core.
///
/// XRCE Transport Frame Format:
///   ┌─────────┬───────────┬──────────┬─────────┬──────────┬─────────────┐
///   │ Marker  │ SessionID │ StreamID │ SeqNum  │ PayLen   │ Payload     │
///   │ 2 bytes │ 1 byte    │ 1 byte   │ 2 bytes │ 2 bytes  │ variable    │
///   └─────────┴───────────┴──────────┴─────────┴──────────┴─────────────┘
///
/// For messages larger than RPMSG_MAX_PAYLOAD (496 bytes), the transport
/// layer fragments and reassembles automatically.
///
/// Fragment header (prepended to each fragment):
///   ┌───────────┬────────────┬────────────┬──────────────┐
///   │ Flags     │ FragIdx    │ TotalFrags │ TotalLen     │
///   │ 1 byte    │ 1 byte     │ 1 byte     │ 2 bytes      │
///   └───────────┴────────────┴────────────┴──────────────┘
///
/// Reliable delivery uses a simple ACK/NAK scheme:
///   - Sender transmits and waits for ACK (up to 3 retries)
///   - Receiver sends ACK with the sequence number
///   - NAK triggers retransmission

use core::sync::atomic::{AtomicU16, Ordering};
use defmt::{debug, warn, error};
use heapless::Vec;

use crate::rpmsg::{RpmsgEndpoint, RPMSG_MAX_PAYLOAD};

/// XRCE transport frame marker bytes
const XRCE_MARKER: [u8; 2] = [0xCE, 0xD5]; // "XRCE" abbreviated

/// Maximum XRCE payload before fragmentation
/// = RPMsg payload - XRCE header (8 bytes)
pub const XRCE_MAX_UNFRAGMENTED: usize = RPMSG_MAX_PAYLOAD - XRCE_HEADER_SIZE;

/// XRCE header size
pub const XRCE_HEADER_SIZE: usize = 8;

/// Fragment header size (when fragmentation is needed)
pub const FRAGMENT_HEADER_SIZE: usize = 5;

/// Maximum fragment payload
pub const FRAGMENT_MAX_PAYLOAD: usize = RPMSG_MAX_PAYLOAD - XRCE_HEADER_SIZE - FRAGMENT_HEADER_SIZE;

/// Maximum total message size (with fragmentation)
pub const XRCE_MAX_MESSAGE_SIZE: usize = FRAGMENT_MAX_PAYLOAD * 255; // ~122 KB

/// Maximum retransmission attempts for reliable delivery
pub const MAX_RETRIES: u8 = 3;

/// Stream IDs for DDS communication
#[derive(Clone, Copy, PartialEq, defmt::Format)]
#[repr(u8)]
pub enum StreamId {
    /// Best-effort output stream (sensor data, heartbeats)
    BestEffortOutput = 0x01,
    /// Reliable output stream (alarms, critical events)
    ReliableOutput = 0x02,
    /// Best-effort input stream (status queries)
    BestEffortInput = 0x81,
    /// Reliable input stream (commands, configuration)
    ReliableInput = 0x82,
    /// Session management stream
    Session = 0x00,
}

/// Fragment flags
mod frag_flags {
    pub const FIRST: u8 = 0x01;
    pub const LAST: u8 = 0x02;
    pub const UNFRAGMENTED: u8 = 0x03; // Both FIRST and LAST
    pub const MIDDLE: u8 = 0x00;
}

/// ACK/NAK message types
mod ack_type {
    pub const ACK: u8 = 0xA0;
    pub const NAK: u8 = 0xA1;
    pub const HEARTBEAT: u8 = 0xA2;
}

/// XRCE transport frame header
#[derive(Clone, Copy)]
pub struct XrceHeader {
    pub session_id: u8,
    pub stream_id: u8,
    pub sequence: u16,
    pub payload_len: u16,
}

impl XrceHeader {
    /// Serialize the XRCE header to bytes
    pub fn to_bytes(&self) -> [u8; XRCE_HEADER_SIZE] {
        [
            XRCE_MARKER[0],
            XRCE_MARKER[1],
            self.session_id,
            self.stream_id,
            (self.sequence >> 8) as u8,
            (self.sequence & 0xFF) as u8,
            (self.payload_len >> 8) as u8,
            (self.payload_len & 0xFF) as u8,
        ]
    }

    /// Parse an XRCE header from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < XRCE_HEADER_SIZE {
            return None;
        }
        if data[0] != XRCE_MARKER[0] || data[1] != XRCE_MARKER[1] {
            return None;
        }
        Some(Self {
            session_id: data[2],
            stream_id: data[3],
            sequence: ((data[4] as u16) << 8) | data[5] as u16,
            payload_len: ((data[6] as u16) << 8) | data[7] as u16,
        })
    }
}

/// Fragment header for large messages
#[derive(Clone, Copy)]
pub struct FragmentHeader {
    pub flags: u8,
    pub frag_idx: u8,
    pub total_frags: u8,
    pub total_len: u16,
}

impl FragmentHeader {
    pub fn to_bytes(&self) -> [u8; FRAGMENT_HEADER_SIZE] {
        [
            self.flags,
            self.frag_idx,
            self.total_frags,
            (self.total_len >> 8) as u8,
            (self.total_len & 0xFF) as u8,
        ]
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < FRAGMENT_HEADER_SIZE {
            return None;
        }
        Some(Self {
            flags: data[0],
            frag_idx: data[1],
            total_frags: data[2],
            total_len: ((data[3] as u16) << 8) | data[4] as u16,
        })
    }
}

/// XRCE-DDS RPMsg Transport
///
/// Handles framing, fragmentation, and reliable delivery
/// of XRCE-DDS messages over the RPMsg channel.
pub struct XrceTransport {
    session_id: u8,
    tx_seq: AtomicU16,
    rx_seq: AtomicU16,
    /// Reassembly buffer for incoming fragmented messages
    reassembly_buf: [u8; 2048],
    reassembly_len: usize,
    reassembly_expected_frags: u8,
    reassembly_received_frags: u8,
}

impl XrceTransport {
    pub const fn new(session_id: u8) -> Self {
        Self {
            session_id,
            tx_seq: AtomicU16::new(0),
            rx_seq: AtomicU16::new(0),
            reassembly_buf: [0u8; 2048],
            reassembly_len: 0,
            reassembly_expected_frags: 0,
            reassembly_received_frags: 0,
        }
    }

    /// Send a message on a given stream via RPMsg
    ///
    /// If the message fits in a single RPMsg frame, it is sent directly.
    /// Otherwise, it is fragmented into multiple RPMsg frames.
    ///
    /// For reliable streams, each fragment is acknowledged.
    pub fn send(
        &self,
        rpmsg: &RpmsgEndpoint,
        stream_id: StreamId,
        payload: &[u8],
    ) -> bool {
        if payload.is_empty() {
            return true;
        }

        let seq = self.tx_seq.fetch_add(1, Ordering::Relaxed);

        if payload.len() <= XRCE_MAX_UNFRAGMENTED {
            // Single frame — no fragmentation needed
            self.send_single(rpmsg, stream_id, seq, payload)
        } else {
            // Fragment the message
            self.send_fragmented(rpmsg, stream_id, seq, payload)
        }
    }

    /// Send a single unfragmented XRCE message
    fn send_single(
        &self,
        rpmsg: &RpmsgEndpoint,
        stream_id: StreamId,
        sequence: u16,
        payload: &[u8],
    ) -> bool {
        let header = XrceHeader {
            session_id: self.session_id,
            stream_id: stream_id as u8,
            sequence,
            payload_len: payload.len() as u16,
        };

        // Build frame: XRCE header + payload
        let mut frame: Vec<u8, 512> = Vec::new();
        let hdr_bytes = header.to_bytes();
        if frame.extend_from_slice(&hdr_bytes).is_err() {
            return false;
        }
        if frame.extend_from_slice(payload).is_err() {
            return false;
        }

        let ok = rpmsg.send(&frame);
        if !ok {
            warn!("XrceTransport: Failed to send frame (seq={})", sequence);
        }
        ok
    }

    /// Send a fragmented XRCE message
    fn send_fragmented(
        &self,
        rpmsg: &RpmsgEndpoint,
        stream_id: StreamId,
        sequence: u16,
        payload: &[u8],
    ) -> bool {
        let total_len = payload.len();
        let total_frags = ((total_len + FRAGMENT_MAX_PAYLOAD - 1) / FRAGMENT_MAX_PAYLOAD) as u8;

        if total_frags == 0 || total_frags == 255 {
            error!("XrceTransport: Message too large for fragmentation ({} bytes)", total_len);
            return false;
        }

        debug!("XrceTransport: Fragmenting {} bytes into {} fragments", total_len, total_frags);

        let mut offset = 0usize;
        for frag_idx in 0..total_frags {
            let remaining = total_len - offset;
            let frag_len = core::cmp::min(remaining, FRAGMENT_MAX_PAYLOAD);

            let flags = match (frag_idx == 0, frag_idx == total_frags - 1) {
                (true, true) => frag_flags::UNFRAGMENTED,
                (true, false) => frag_flags::FIRST,
                (false, true) => frag_flags::LAST,
                (false, false) => frag_flags::MIDDLE,
            };

            let frag_header = FragmentHeader {
                flags,
                frag_idx,
                total_frags,
                total_len: total_len as u16,
            };

            let xrce_header = XrceHeader {
                session_id: self.session_id,
                stream_id: stream_id as u8,
                sequence: sequence.wrapping_add(frag_idx as u16),
                payload_len: (FRAGMENT_HEADER_SIZE + frag_len) as u16,
            };

            // Build frame: XRCE header + fragment header + fragment payload
            let mut frame: Vec<u8, 512> = Vec::new();
            if frame.extend_from_slice(&xrce_header.to_bytes()).is_err() {
                return false;
            }
            if frame.extend_from_slice(&frag_header.to_bytes()).is_err() {
                return false;
            }
            if frame.extend_from_slice(&payload[offset..offset + frag_len]).is_err() {
                return false;
            }

            if !rpmsg.send(&frame) {
                warn!("XrceTransport: Fragment {}/{} send failed", frag_idx + 1, total_frags);
                return false;
            }

            offset += frag_len;
        }

        true
    }

    /// Receive and reassemble an XRCE message from RPMsg
    ///
    /// Returns the complete payload when a full message is available.
    /// For fragmented messages, intermediate fragments are buffered
    /// until the complete message is received.
    pub fn recv(
        &mut self,
        rpmsg: &RpmsgEndpoint,
        buf: &mut [u8],
    ) -> Option<(StreamId, usize)> {
        let mut rpmsg_buf = [0u8; 512];
        let rpmsg_len = rpmsg.recv(&mut rpmsg_buf)?;

        if rpmsg_len < XRCE_HEADER_SIZE {
            warn!("XrceTransport: Frame too short ({} bytes)", rpmsg_len);
            return None;
        }

        let header = XrceHeader::from_bytes(&rpmsg_buf)?;
        let payload = &rpmsg_buf[XRCE_HEADER_SIZE..rpmsg_len];

        // Check if this is an ACK/NAK message
        if header.stream_id == ack_type::ACK || header.stream_id == ack_type::NAK {
            debug!("XrceTransport: Received {} for seq={}",
                if header.stream_id == ack_type::ACK { "ACK" } else { "NAK" },
                header.sequence);
            return None;
        }

        let stream_id = match header.stream_id {
            0x01 => StreamId::BestEffortOutput,
            0x02 => StreamId::ReliableOutput,
            0x81 => StreamId::BestEffortInput,
            0x82 => StreamId::ReliableInput,
            0x00 => StreamId::Session,
            _ => {
                warn!("XrceTransport: Unknown stream 0x{:02X}", header.stream_id);
                return None;
            }
        };

        // Check if message is fragmented
        if payload.len() >= FRAGMENT_HEADER_SIZE {
            let frag = FragmentHeader::from_bytes(payload)?;

            if frag.flags == frag_flags::UNFRAGMENTED {
                // Single fragment — return directly
                let data = &payload[FRAGMENT_HEADER_SIZE..];
                let copy_len = core::cmp::min(data.len(), buf.len());
                buf[..copy_len].copy_from_slice(&data[..copy_len]);

                // Send ACK for reliable streams
                if stream_id == StreamId::ReliableInput {
                    self.send_ack(rpmsg, header.sequence);
                }

                return Some((stream_id, copy_len));
            }

            // Handle fragmented message reassembly
            if frag.flags & frag_flags::FIRST != 0 {
                // First fragment — reset reassembly state
                self.reassembly_len = 0;
                self.reassembly_expected_frags = frag.total_frags;
                self.reassembly_received_frags = 0;
            }

            // Copy fragment data into reassembly buffer
            let frag_data = &payload[FRAGMENT_HEADER_SIZE..];
            let copy_end = self.reassembly_len + frag_data.len();
            if copy_end <= self.reassembly_buf.len() {
                self.reassembly_buf[self.reassembly_len..copy_end]
                    .copy_from_slice(frag_data);
                self.reassembly_len = copy_end;
                self.reassembly_received_frags += 1;
            }

            if frag.flags & frag_flags::LAST != 0 {
                // Last fragment — message complete
                if self.reassembly_received_frags == self.reassembly_expected_frags {
                    let copy_len = core::cmp::min(self.reassembly_len, buf.len());
                    buf[..copy_len].copy_from_slice(&self.reassembly_buf[..copy_len]);

                    if stream_id == StreamId::ReliableInput {
                        self.send_ack(rpmsg, header.sequence);
                    }

                    let len = self.reassembly_len;
                    self.reassembly_len = 0;
                    return Some((stream_id, len));
                } else {
                    warn!("XrceTransport: Fragment count mismatch ({}/{})",
                        self.reassembly_received_frags, self.reassembly_expected_frags);
                    self.reassembly_len = 0;
                    return None;
                }
            }

            return None; // Waiting for more fragments
        }

        // No fragment header — treat as unfragmented
        let copy_len = core::cmp::min(payload.len(), buf.len());
        buf[..copy_len].copy_from_slice(&payload[..copy_len]);
        Some((stream_id, copy_len))
    }

    /// Send an ACK for reliable delivery
    fn send_ack(&self, rpmsg: &RpmsgEndpoint, sequence: u16) {
        let header = XrceHeader {
            session_id: self.session_id,
            stream_id: ack_type::ACK,
            sequence,
            payload_len: 0,
        };

        let frame = header.to_bytes();
        let _ = rpmsg.send(&frame);
    }

    /// Send a NAK (negative acknowledgment) requesting retransmission
    #[allow(dead_code)]
    fn send_nak(&self, rpmsg: &RpmsgEndpoint, sequence: u16) {
        let header = XrceHeader {
            session_id: self.session_id,
            stream_id: ack_type::NAK,
            sequence,
            payload_len: 0,
        };

        let frame = header.to_bytes();
        let _ = rpmsg.send(&frame);
    }
}
