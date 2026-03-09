// SPDX-License-Identifier: AGPL-3.0-only
//! DRM/KMS abstraction layer
//!
//! Provides safe wrappers around Direct Rendering Manager (DRM) and
//! Kernel Mode Setting (KMS) APIs for display hardware control.
//!
//! Uses `linux-drm` for 100% Pure Rust implementation.

pub mod buffer;
pub mod connector;
pub mod device;
pub mod modesetting;
pub mod pageflip;

// Re-exports
pub use buffer::{DumbBuffer, MappedBuffer, MappedBufferView, PixelFormat};
pub use connector::{ConnectionStatus, ConnectorInfo, ConnectorType, DisplayMode};
pub use device::{Device, DeviceCapabilities};
pub use modesetting::{modeset, ModesetPipeline};
pub use pageflip::PageFlipper;

use crate::Result;
use std::path::Path;

/// DRM backend for display hardware control
///
/// This is the main entry point for DRM operations.
/// Wraps the lower-level Device type with a simpler API.
pub struct DrmBackend {
    device: Device,
}

impl DrmBackend {
    /// Open a DRM device
    ///
    /// # Errors
    ///
    /// Returns an error if the device cannot be opened or is not a valid DRM device.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::DrmBackend;
    /// # fn main() -> toadstool_display::Result<()> {
    /// let drm = DrmBackend::open("/dev/dri/card0")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let device = Device::open(path)?;
        Ok(Self { device })
    }

    /// Discover all DRM devices (self-knowledge!)
    ///
    /// Returns paths to all available DRM devices.
    /// No hardcoding - pure runtime discovery!
    ///
    /// # Errors
    ///
    /// Returns an error if `/dev/dri` cannot be read.
    pub fn discover_all() -> Result<Vec<std::path::PathBuf>> {
        Device::discover_all()
    }

    /// Get device capabilities
    ///
    /// # Errors
    ///
    /// Returns an error if capability queries fail.
    pub fn capabilities(&self) -> Result<DeviceCapabilities> {
        self.device.query_capabilities()
    }

    /// Get the underlying DRM device (for buffer mapping)
    #[must_use]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Create a dumb buffer
    ///
    /// Allocates a simple framebuffer for rendering.
    ///
    /// # Arguments
    ///
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    /// * `bpp` - Bits per pixel (typically 32 for RGBA)
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer cannot be allocated.
    pub fn create_dumb_buffer(&self, width: u32, height: u32, bpp: u32) -> Result<DumbBuffer> {
        // Map bpp to pixel format
        let format = match bpp {
            24 => PixelFormat::RGB888,
            16 => PixelFormat::RGB565,
            _ => PixelFormat::RGBA8888, // Default to 32-bit (includes 32)
        };

        DumbBuffer::create(&self.device, width, height, format)
    }
}
