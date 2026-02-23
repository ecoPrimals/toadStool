//! DRM/KMS abstraction layer
//!
//! Provides safe wrappers around Direct Rendering Manager (DRM) and
//! Kernel Mode Setting (KMS) APIs for display hardware control.
//!
//! Uses `linux-drm` for 100% Pure Rust implementation.

pub mod buffer;
pub mod device;

// Re-exports
pub use buffer::{DumbBuffer, MappedBuffer, PixelFormat};
pub use device::{Device, DeviceCapabilities};

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
    pub fn discover_all() -> Result<Vec<std::path::PathBuf>> {
        Device::discover_all()
    }

    /// Get device capabilities
    pub fn capabilities(&self) -> Result<DeviceCapabilities> {
        self.device.query_capabilities()
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
    pub fn create_dumb_buffer(&self, width: u32, height: u32, bpp: u32) -> Result<DumbBuffer> {
        // Map bpp to pixel format
        let format = match bpp {
            32 => PixelFormat::RGBA8888,
            24 => PixelFormat::RGB888,
            16 => PixelFormat::RGB565,
            _ => PixelFormat::RGBA8888, // Default to 32-bit
        };

        DumbBuffer::create(&self.device, width, height, format)
    }
}
