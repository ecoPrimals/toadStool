// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capture Transport — receive data from HDMI capture cards via V4L2.
//!
//! Implements [`HardwareTransport`] as an **Rx-only** transport. Reads video
//! frames from a V4L2 capture device, decodes the toadStool frame protocol,
//! and returns the data payload.

use toadstool_core::{
    decode_frame, HardwareTransport, TransportDirection, TransportError, TransportInfo,
    TransportMedium,
};

use crate::v4l2::CaptureDevice;
use crate::Result as DisplayResult;

/// Default RGBA8888 fourcc (`AR24` / Argb8888) — matches `DisplayTransport` output.
const FOURCC_AR24: u32 = u32::from_le_bytes(*b"AR24");
/// Default buffer count for mmap streaming.
const DEFAULT_BUFFERS: u32 = 4;

/// An Rx-only hardware transport backed by a V4L2 capture card.
pub struct CaptureTransport {
    info: TransportInfo,
    device: CaptureDevice,
    frame_buf: Vec<u8>,
}

impl CaptureTransport {
    /// Open a capture transport on the given V4L2 device (e.g. `/dev/video0`).
    ///
    /// Negotiates the capture format to match the sending `DisplayTransport`'s
    /// mode. `width` and `height` should match the display output resolution.
    ///
    /// # Errors
    ///
    /// Returns an error if the device cannot be opened, does not support capture streaming,
    /// or format/buffer negotiation fails.
    pub fn open(v4l2_path: &str, width: u32, height: u32) -> DisplayResult<Self> {
        let mut device = CaptureDevice::open(v4l2_path)?;

        if !device.supports_capture_streaming()? {
            return Err(crate::DisplayError::IoctlFailed(
                "device does not support capture + streaming".into(),
            ));
        }

        let fmt = device.set_format(width, height, FOURCC_AR24)?;
        device.request_buffers(DEFAULT_BUFFERS)?;
        device.start_streaming()?;

        let frame_buf = vec![0u8; fmt.image_size as usize];

        let label = format!("V4L2:{v4l2_path}");

        Ok(Self {
            info: TransportInfo {
                id: v4l2_path.to_string(),
                label,
                medium: TransportMedium::Capture,
                direction: TransportDirection::Rx,
            },
            device,
            frame_buf,
        })
    }
}

impl HardwareTransport for CaptureTransport {
    fn info(&self) -> &TransportInfo {
        &self.info
    }

    fn bandwidth_bps(&self) -> u64 {
        // Estimated from the negotiated format; exact value depends on the source.
        if let Some(fmt) = self.device.format() {
            u64::from(fmt.image_size) * 60 * 8 // assume 60 fps
        } else {
            0
        }
    }

    fn is_available(&self) -> bool {
        self.device.format().is_some()
    }

    fn send(&mut self, _data: &[u8]) -> Result<usize, TransportError> {
        Err(TransportError::DirectionMismatch {
            transport_dir: TransportDirection::Rx,
            required: TransportDirection::Tx,
        })
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        let bytes_read = self
            .device
            .read_frame(&mut self.frame_buf)
            .map_err(|e| TransportError::Io(std::io::Error::other(format!("V4L2 read: {e}"))))?;

        let (_sequence, payload) = decode_frame(&self.frame_buf[..bytes_read])?;
        let copy_len = buf.len().min(payload.len());
        buf[..copy_len].copy_from_slice(&payload[..copy_len]);
        Ok(copy_len)
    }
}

/// Discover V4L2 capture devices that can serve as data transports.
#[must_use]
pub fn discover_capture_transports() -> Vec<TransportInfo> {
    let Ok(devices) = CaptureDevice::discover_all() else {
        return Vec::new();
    };

    let mut transports = Vec::new();

    for dev_path in &devices {
        if let Ok(dev) = CaptureDevice::open(dev_path) {
            if dev.supports_capture_streaming().unwrap_or(false) {
                transports.push(TransportInfo {
                    id: dev_path.display().to_string(),
                    label: format!("V4L2:{}", dev_path.display()),
                    medium: TransportMedium::Capture,
                    direction: TransportDirection::Rx,
                });
            }
        }
    }

    transports
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_core::TransportDirection;

    #[test]
    fn discover_capture_transports_returns_vec() {
        let transports = discover_capture_transports();
        assert!(
            transports
                .iter()
                .all(|t| t.medium == TransportMedium::Capture
                    && t.direction == TransportDirection::Rx)
        );
        // May be empty without V4L2 capture devices in CI
    }

    #[test]
    fn fourcc_ar24_value() {
        assert_eq!(FOURCC_AR24, u32::from_le_bytes(*b"AR24"));
    }
}
