// SPDX-License-Identifier: AGPL-3.0-only
//! Hardware Transport Layer — any hardware input to any hardware output.
//!
//! Defines the generic [`HardwareTransport`] trait that all physical I/O paths
//! implement: HDMI/DP output, capture-card input, serial, `PCIe`, `NVLink`, etc.
//!
//! ToadStool owns the physical pipe. `BarraCuda` owns the math. Songbird owns
//! network discovery. This module enables toadStool to discover any hardware
//! port, encode data, and route it physically to any other hardware port.

use std::fmt;

/// Direction of data flow through a transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportDirection {
    /// Transmit only (e.g. HDMI output).
    Tx,
    /// Receive only (e.g. capture card input).
    Rx,
    /// Both directions (e.g. serial, `PCIe`).
    Bidirectional,
}

impl fmt::Display for TransportDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tx => write!(f, "Tx"),
            Self::Rx => write!(f, "Rx"),
            Self::Bidirectional => write!(f, "Bidi"),
        }
    }
}

/// Category of the transport medium.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportMedium {
    /// Display output (HDMI, `DisplayPort`).
    Display,
    /// Video capture (V4L2 capture card, UVC).
    Capture,
    /// Serial link (USB serial, UART).
    Serial,
    /// `PCIe` peer-to-peer.
    Pcie,
    /// `NVLink` GPU-to-GPU.
    NvLink,
}

impl fmt::Display for TransportMedium {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Display => write!(f, "Display"),
            Self::Capture => write!(f, "Capture"),
            Self::Serial => write!(f, "Serial"),
            Self::Pcie => write!(f, "PCIe"),
            Self::NvLink => write!(f, "NVLink"),
        }
    }
}

/// Metadata describing a discovered transport endpoint.
#[derive(Debug, Clone)]
pub struct TransportInfo {
    /// Unique identifier (e.g. "/dev/dri/card0:HDMI-A-1", "/dev/video0").
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Transport medium category.
    pub medium: TransportMedium,
    /// Data-flow direction.
    pub direction: TransportDirection,
}

/// Errors from transport operations.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Transport is not available (disconnected, no permissions, etc.).
    #[error("transport unavailable: {0}")]
    Unavailable(String),

    /// Failed to open or configure the transport.
    #[error("open failed: {0}")]
    OpenFailed(String),

    /// I/O error during send or recv.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The operation is not supported in this direction.
    #[error("direction mismatch: transport is {transport_dir} but operation requires {required}")]
    DirectionMismatch {
        /// The transport's actual direction.
        transport_dir: TransportDirection,
        /// The direction the caller needed.
        required: TransportDirection,
    },

    /// Encoding or decoding error in the frame protocol.
    #[error("frame protocol error: {0}")]
    FrameProtocol(String),
}

/// Generic hardware transport — the core abstraction for any-to-any I/O.
///
/// Every physical data path that toadStool manages implements this trait.
/// The [`super::TransportRouter`] composes transports to build pipelines
/// like: GPU framebuffer -> HDMI out -> capture card in -> serial out.
pub trait HardwareTransport: Send + Sync {
    /// Static metadata for this transport endpoint.
    fn info(&self) -> &TransportInfo;

    /// Maximum theoretical bandwidth in bits per second.
    fn bandwidth_bps(&self) -> u64;

    /// Whether the transport is currently usable (plugged in, permissions OK).
    fn is_available(&self) -> bool;

    /// Send data through the transport.
    ///
    /// Returns the number of bytes accepted. For frame-based transports (display),
    /// this is the frame payload size; partial frames are not sent.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::DirectionMismatch`] if the transport is Rx-only.
    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError>;

    /// Receive data from the transport.
    ///
    /// Fills `buf` and returns the number of bytes read.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::DirectionMismatch`] if the transport is Tx-only.
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
}

// ---------------------------------------------------------------------------
// Frame protocol for encoding arbitrary data as display "pixels"
// ---------------------------------------------------------------------------

/// Magic bytes identifying a toadStool transport frame.
const FRAME_MAGIC: [u8; 4] = *b"TSXP";
/// Current frame protocol version.
const FRAME_VERSION: u8 = 1;
/// Header: magic(4) + version(1) + sequence(4) + `payload_len`(4) + checksum(4) = 17 bytes.
pub const FRAME_HEADER_SIZE: usize = 17;

/// Encode a data payload into a pixel buffer.
///
/// Writes a framed header followed by the payload. Returns the total number
/// of bytes written (header + payload), or `None` if `pixel_buf` is too small.
pub fn encode_frame(sequence: u32, payload: &[u8], pixel_buf: &mut [u8]) -> Option<usize> {
    let total = FRAME_HEADER_SIZE + payload.len();
    if pixel_buf.len() < total {
        return None;
    }

    pixel_buf[0..4].copy_from_slice(&FRAME_MAGIC);
    pixel_buf[4] = FRAME_VERSION;
    pixel_buf[5..9].copy_from_slice(&sequence.to_le_bytes());
    // Frame protocol limits payload_len to u32; truncation would require >4GB payload.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation acceptable for this conversion"
    )]
    let payload_len_u32 = payload.len() as u32;
    pixel_buf[9..13].copy_from_slice(&payload_len_u32.to_le_bytes());

    // Simple CRC-32-style checksum (xor-fold for speed).
    let checksum = payload.chunks(4).fold(0u32, |acc, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        acc ^ u32::from_le_bytes(word)
    });
    pixel_buf[13..17].copy_from_slice(&checksum.to_le_bytes());

    pixel_buf[FRAME_HEADER_SIZE..total].copy_from_slice(payload);
    Some(total)
}

/// Decode a transport frame from a pixel buffer.
///
/// Returns `(sequence, payload_slice)` on success, or a descriptive error.
///
/// # Errors
/// Returns [`TransportError::FrameProtocol`] if the buffer is too small, magic
/// is wrong, version is unsupported, or checksum doesn't match.
pub fn decode_frame(pixel_buf: &[u8]) -> Result<(u32, &[u8]), TransportError> {
    if pixel_buf.len() < FRAME_HEADER_SIZE {
        return Err(TransportError::FrameProtocol(
            "buffer too small for header".into(),
        ));
    }

    if pixel_buf[0..4] != FRAME_MAGIC {
        return Err(TransportError::FrameProtocol("bad magic".into()));
    }
    if pixel_buf[4] != FRAME_VERSION {
        return Err(TransportError::FrameProtocol(format!(
            "unsupported version {}",
            pixel_buf[4]
        )));
    }

    let sequence = u32::from_le_bytes([pixel_buf[5], pixel_buf[6], pixel_buf[7], pixel_buf[8]]);
    let payload_len =
        u32::from_le_bytes([pixel_buf[9], pixel_buf[10], pixel_buf[11], pixel_buf[12]]) as usize;
    let expected_checksum =
        u32::from_le_bytes([pixel_buf[13], pixel_buf[14], pixel_buf[15], pixel_buf[16]]);

    let total = FRAME_HEADER_SIZE + payload_len;
    if pixel_buf.len() < total {
        return Err(TransportError::FrameProtocol(
            "buffer too small for payload".into(),
        ));
    }

    let payload = &pixel_buf[FRAME_HEADER_SIZE..total];

    let checksum = payload.chunks(4).fold(0u32, |acc, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        acc ^ u32::from_le_bytes(word)
    });

    if checksum != expected_checksum {
        return Err(TransportError::FrameProtocol(format!(
            "checksum mismatch: expected {expected_checksum:#010x}, got {checksum:#010x}"
        )));
    }

    Ok((sequence, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_frame() {
        let payload = b"Hello, hardware transport!";
        let mut buf = vec![0u8; FRAME_HEADER_SIZE + payload.len() + 64];

        let written = encode_frame(42, payload, &mut buf).unwrap();
        assert_eq!(written, FRAME_HEADER_SIZE + payload.len());

        let (seq, decoded) = decode_frame(&buf[..written]).unwrap();
        assert_eq!(seq, 42);
        assert_eq!(decoded, payload);
    }

    #[test]
    fn bad_magic_rejected() {
        let mut buf = vec![0u8; 64];
        buf[0..4].copy_from_slice(b"NOPE");
        assert!(decode_frame(&buf).is_err());
    }

    #[test]
    fn truncated_buffer_rejected() {
        assert!(decode_frame(&[0; 4]).is_err());
    }

    #[test]
    fn encode_too_small_returns_none() {
        let mut buf = [0u8; 4];
        assert!(encode_frame(0, b"data", &mut buf).is_none());
    }

    #[test]
    fn direction_display() {
        assert_eq!(format!("{}", TransportDirection::Tx), "Tx");
        assert_eq!(format!("{}", TransportDirection::Rx), "Rx");
        assert_eq!(format!("{}", TransportDirection::Bidirectional), "Bidi");
    }

    #[test]
    fn medium_display() {
        assert_eq!(format!("{}", TransportMedium::Display), "Display");
        assert_eq!(format!("{}", TransportMedium::Capture), "Capture");
    }

    #[test]
    fn transport_medium_all_variants() {
        assert_eq!(format!("{}", TransportMedium::Serial), "Serial");
        assert_eq!(format!("{}", TransportMedium::Pcie), "PCIe");
        assert_eq!(format!("{}", TransportMedium::NvLink), "NVLink");
    }

    #[test]
    fn transport_error_open_failed() {
        let e = TransportError::OpenFailed("permission denied".into());
        assert!(e.to_string().contains("open") || e.to_string().contains("permission"));
    }

    #[test]
    fn encode_frame_exact_buffer_size() {
        let payload = b"x";
        let mut buf = vec![0u8; FRAME_HEADER_SIZE + 1];
        let written = encode_frame(0, payload, &mut buf).unwrap();
        assert_eq!(written, FRAME_HEADER_SIZE + 1);
    }

    #[test]
    fn decode_frame_version_zero_rejected() {
        let mut buf = vec![0u8; FRAME_HEADER_SIZE];
        buf[0..4].copy_from_slice(b"TSXP");
        buf[4] = 0;
        assert!(decode_frame(&buf).is_err());
    }
}
