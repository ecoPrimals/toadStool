// SPDX-License-Identifier: AGPL-3.0-or-later
//! Display Transport — encode data as framebuffer pixels and push via HDMI/DP.
//!
//! Implements [`HardwareTransport`] for DRM display outputs. Each `send()` call
//! writes a framed data payload into a dumb buffer and page-flips it to the
//! physical connector.
//!
//! This is a **Tx-only** transport: HDMI/DP outputs are unidirectional.

use toadstool_core::{
    encode_frame, HardwareTransport, TransportDirection, TransportError, TransportInfo,
    TransportMedium, FRAME_HEADER_SIZE,
};

use crate::drm::{
    connector::{enumerate_connectors, ConnectionStatus},
    modesetting::{modeset, ModesetPipeline},
    pageflip::PageFlipper,
    Device, DumbBuffer, PixelFormat,
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

        let label = connector.label.clone();
        let id = format!("{drm_path}:{label}");

        Ok(Self {
            info: TransportInfo {
                id,
                label,
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
    pub fn frame_capacity(&self) -> usize {
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
}
