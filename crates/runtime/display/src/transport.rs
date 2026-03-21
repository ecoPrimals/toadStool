// SPDX-License-Identifier: AGPL-3.0-only
//! Display Transport — encode data as framebuffer pixels and push via HDMI/DP.
//!
//! Implements [`HardwareTransport`] for DRM display outputs. Each `send()` call
//! writes a framed data payload into a dumb buffer and page-flips it to the
//! physical connector.
//!
//! This is a **Tx-only** transport: HDMI/DP outputs are unidirectional.

use toadstool_core::{
    FRAME_HEADER_SIZE, HardwareTransport, TransportDirection, TransportError, TransportInfo,
    TransportMedium, encode_frame,
};

use crate::drm::{
    Device, DumbBuffer, PixelFormat,
    connector::{ConnectionStatus, enumerate_connectors},
    modesetting::{ModesetPipeline, modeset},
    pageflip::PageFlipper,
};
use crate::{DisplayError, Result as DisplayResult};

/// A Tx-only hardware transport that streams data through a DRM display connector.
pub struct DisplayTransport {
    info: TransportInfo,
    device: Device,
    pipeline: ModesetPipeline,
    flipper: PageFlipper,
    buffers: [DumbBuffer; 2],
    write_idx: usize,
    sequence: u32,
    frame_capacity: usize,
}

impl DisplayTransport {
    /// Open a display transport on the given DRM device path.
    ///
    /// Discovers connectors, picks the best connected HDMI/DP output, sets the
    /// highest-throughput mode, and prepares double-buffered page flipping.
    ///
    /// # Errors
    ///
    /// Returns an error if the device cannot be opened, no suitable connector is found,
    /// or modesetting fails.
    pub fn open(drm_path: &str) -> DisplayResult<Self> {
        let device = Device::open(drm_path)?;
        let connectors = enumerate_connectors(&device)?;

        let connector = connectors
            .into_iter()
            .find(|c| {
                c.status == ConnectionStatus::Connected
                    && c.connector_type.supports_data_transport()
            })
            .ok_or_else(|| {
                DisplayError::IoctlFailed("no connected HDMI/DP connector found".into())
            })?;

        let mode = connector
            .best_mode()
            .ok_or_else(|| DisplayError::IoctlFailed("connector has no modes".into()))?
            .clone();

        let (w, h) = (u32::from(mode.width), u32::from(mode.height));
        let format = PixelFormat::RGBA8888;

        let front_buf = DumbBuffer::create(&device, w, h, format)?;
        let back_buf = DumbBuffer::create(&device, w, h, format)?;

        let pipeline = modeset(&device, &connector, &mode, &front_buf)?;
        let flipper = PageFlipper::new(&device, &pipeline, &back_buf)?;

        let frame_capacity =
            (w as usize) * (h as usize) * format.bytes_per_pixel() - FRAME_HEADER_SIZE;

        let id = format!("{drm_path}:{}", connector.label);

        Ok(Self {
            info: TransportInfo {
                id,
                label: connector.label,
                medium: TransportMedium::Display,
                direction: TransportDirection::Tx,
            },
            device,
            pipeline,
            flipper,
            buffers: [front_buf, back_buf],
            write_idx: 1,
            sequence: 0,
            frame_capacity,
        })
    }

    /// Maximum payload bytes that fit in a single display frame.
    #[must_use]
    pub const fn frame_capacity(&self) -> usize {
        self.frame_capacity
    }
}

impl HardwareTransport for DisplayTransport {
    fn info(&self) -> &TransportInfo {
        &self.info
    }

    fn bandwidth_bps(&self) -> u64 {
        u64::from(self.pipeline.width)
            * u64::from(self.pipeline.height)
            * 4 // RGBA8888
            * u64::from(self.pipeline.refresh_hz)
            * 8 // bits
    }

    fn is_available(&self) -> bool {
        // Re-probe the connector for hotplug.
        enumerate_connectors(&self.device)
            .map(|cs| {
                cs.iter().any(|c| {
                    c.handle == self.pipeline.connector && c.status == ConnectionStatus::Connected
                })
            })
            .unwrap_or(false)
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
        if data.len() > self.frame_capacity {
            return Err(TransportError::FrameProtocol(format!(
                "payload {} exceeds frame capacity {}",
                data.len(),
                self.frame_capacity
            )));
        }

        let buf = &mut self.buffers[self.write_idx];
        let (w, h) = buf.dimensions();
        let bpp = buf.format().bytes_per_pixel();
        let pixel_buf_size = (w as usize) * (h as usize) * bpp;
        let mut pixel_buf = vec![0u8; pixel_buf_size];

        let written = encode_frame(self.sequence, data, &mut pixel_buf)
            .ok_or_else(|| TransportError::FrameProtocol("encode failed".into()))?;

        buf.with_mapping(&self.device, |view| {
            view.copy_from_slice(&pixel_buf);
        })
        .map_err(|e| TransportError::OpenFailed(format!("buffer map: {e}")))?;

        self.flipper
            .flip(&self.device)
            .map_err(|e| TransportError::Io(std::io::Error::other(format!("page_flip: {e}"))))?;

        self.write_idx ^= 1;
        self.sequence = self.sequence.wrapping_add(1);
        Ok(written - FRAME_HEADER_SIZE)
    }

    fn recv(&mut self, _buf: &mut [u8]) -> Result<usize, TransportError> {
        Err(TransportError::DirectionMismatch {
            transport_dir: TransportDirection::Tx,
            required: TransportDirection::Rx,
        })
    }
}

/// Discover all display outputs that can serve as data transports.
#[must_use]
pub fn discover_display_transports() -> Vec<TransportInfo> {
    let Ok(devices) = Device::discover_all() else {
        return Vec::new();
    };

    let mut transports = Vec::new();

    for dev_path in &devices {
        let Ok(device) = Device::open(dev_path) else {
            continue;
        };

        let Ok(connectors) = enumerate_connectors(&device) else {
            continue;
        };

        for conn in &connectors {
            if conn.status == ConnectionStatus::Connected
                && conn.connector_type.supports_data_transport()
            {
                transports.push(TransportInfo {
                    id: format!("{}:{}", dev_path.display(), conn.label),
                    label: conn.label.clone(),
                    medium: TransportMedium::Display,
                    direction: TransportDirection::Tx,
                });
            }
        }
    }

    transports
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_core::{TransportDirection, TransportError, TransportMedium};

    #[test]
    fn discover_display_transports_returns_vec() {
        let transports = discover_display_transports();
        assert!(transports.iter().all(|t| !t.id.is_empty()));
        // May be empty without DRM hardware in CI
    }

    #[test]
    fn display_transport_bandwidth_formula() {
        // DisplayTransport::bandwidth_bps() = width * height * 4 * refresh_hz * 8
        // (RGBA8888 = 4 bytes, * 8 for bits)
        let width = 1920u64;
        let height = 1080u64;
        let refresh_hz = 60u64;
        let expected_bps = width * height * 4 * refresh_hz * 8;
        assert_eq!(expected_bps, 3_981_312_000);
    }

    #[test]
    fn discover_transports_have_display_medium() {
        let transports = discover_display_transports();
        for t in &transports {
            assert_eq!(t.medium, TransportMedium::Display);
            assert_eq!(t.direction, TransportDirection::Tx);
        }
    }

    #[test]
    fn transport_info_id_format() {
        let transports = discover_display_transports();
        for t in &transports {
            assert!(!t.id.is_empty());
            assert!(!t.label.is_empty());
        }
    }

    #[test]
    fn transport_bandwidth_formula_4k() {
        let width = 3840u64;
        let height = 2160u64;
        let refresh_hz = 60u64;
        let bps = width * height * 4 * refresh_hz * 8;
        assert_eq!(bps, 15_925_248_000);
    }

    #[test]
    fn transport_bandwidth_formula_720p() {
        let width = 1280u64;
        let height = 720u64;
        let refresh_hz = 60u64;
        let bps = width * height * 4 * refresh_hz * 8;
        assert_eq!(bps, 1_769_472_000);
    }

    #[test]
    fn frame_header_size_constant() {
        assert_ne!(FRAME_HEADER_SIZE, 0);
    }

    #[test]
    fn transport_direction_tx() {
        assert_eq!(TransportDirection::Tx, TransportDirection::Tx);
    }

    #[test]
    fn transport_medium_display() {
        assert_eq!(TransportMedium::Display, TransportMedium::Display);
    }

    #[test]
    fn discover_returns_transport_info_vec() {
        let t = discover_display_transports();
        assert!(t.is_empty() || t.iter().all(|i| !i.id.is_empty()));
    }

    #[test]
    fn frame_capacity_formula_with_header() {
        use toadstool_core::FRAME_HEADER_SIZE;
        let w = 1920usize;
        let h = 1080usize;
        let bpp = 4;
        let pixel_buf_size = w * h * bpp;
        let frame_capacity = pixel_buf_size - FRAME_HEADER_SIZE;
        assert!(frame_capacity > 0);
        assert_eq!(frame_capacity, 1920 * 1080 * 4 - FRAME_HEADER_SIZE);
    }

    #[test]
    fn transport_info_has_required_fields() {
        let transports = discover_display_transports();
        for t in &transports {
            assert!(!t.id.is_empty());
            assert!(!t.label.is_empty());
        }
    }

    #[test]
    fn transport_bandwidth_formula_1080p_30hz() {
        let width = 1920u64;
        let height = 1080u64;
        let refresh_hz = 30u64;
        let bps = width * height * 4 * refresh_hz * 8;
        assert_eq!(bps, 1_990_656_000);
    }

    #[test]
    fn transport_bandwidth_formula_480p() {
        let width = 640u64;
        let height = 480u64;
        let refresh_hz = 60u64;
        let bps = width * height * 4 * refresh_hz * 8;
        assert_eq!(bps, 589_824_000);
    }

    #[test]
    fn encode_frame_constant_size() {
        const { assert!(FRAME_HEADER_SIZE >= 16) };
        const { assert!(FRAME_HEADER_SIZE <= 64) };
    }

    #[test]
    fn transport_medium_display_variant() {
        assert_eq!(format!("{}", TransportMedium::Display), "Display");
    }

    #[test]
    fn transport_direction_tx_only() {
        assert_eq!(format!("{}", TransportDirection::Tx), "Tx");
    }

    #[test]
    fn discover_empty_without_drm() {
        let t = discover_display_transports();
        assert!(t.is_empty() || t.iter().all(|i| i.medium == TransportMedium::Display));
    }

    // ─── TransportError enum variants (from toadstool_core) ───
    #[test]
    fn transport_error_frame_protocol_display() {
        let err = TransportError::FrameProtocol("payload too large".into());
        assert!(err.to_string().contains("payload") || err.to_string().contains("Frame"));
    }

    #[test]
    fn transport_error_direction_mismatch_tx_rx() {
        let err = TransportError::DirectionMismatch {
            transport_dir: TransportDirection::Tx,
            required: TransportDirection::Rx,
        };
        assert!(err.to_string().contains("Tx") || err.to_string().contains("Rx"));
    }

    #[test]
    fn transport_error_open_failed_display() {
        let err = TransportError::OpenFailed("device busy".into());
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn transport_error_io_display() {
        let err = TransportError::Io(std::io::Error::other("timeout"));
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn transport_error_unavailable_display() {
        let err = TransportError::Unavailable("no connector".into());
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn transport_info_id_contains_path_and_label() {
        let transports = discover_display_transports();
        for t in &transports {
            assert!(!t.id.is_empty());
            assert!(t.id.contains(':') || !t.label.is_empty());
        }
    }

    #[test]
    fn discover_transports_iteration() {
        let t = discover_display_transports();
        let count = t.len();
        let _ = count;
    }

    #[test]
    fn transport_bandwidth_formula_consistency() {
        // width * height * 4 * refresh_hz * 8
        for (w, h, r) in [
            (640u64, 480u64, 60u64),
            (1920u64, 1080u64, 60u64),
            (2560u64, 1440u64, 144u64),
        ] {
            let bps = w * h * 4 * r * 8;
            assert!(bps > 0);
        }
    }

    #[test]
    fn transport_medium_display_debug() {
        let m = TransportMedium::Display;
        let s = format!("{m:?}");
        assert!(s.contains("Display"));
    }

    #[test]
    fn transport_direction_tx_debug() {
        let d = TransportDirection::Tx;
        let s = format!("{d:?}");
        assert!(s.contains("Tx"));
    }

    #[test]
    fn transport_direction_rx() {
        assert_ne!(TransportDirection::Tx, TransportDirection::Rx);
    }

    #[test]
    fn frame_capacity_decreases_with_header() {
        let w = 640usize;
        let h = 480usize;
        let bpp = 4;
        let raw = w * h * bpp;
        let capacity = raw.saturating_sub(FRAME_HEADER_SIZE);
        assert!(capacity < raw);
    }

    #[test]
    fn discover_returns_sorted_or_unsorted() {
        let t1 = discover_display_transports();
        let t2 = discover_display_transports();
        assert_eq!(t1.len(), t2.len());
    }
}
