//! DRM/KMS abstraction layer
//!
//! Provides safe wrappers around Direct Rendering Manager (DRM) and
//! Kernel Mode Setting (KMS) APIs for display hardware control.
//!
//! Uses `linux-drm` for 100% Pure Rust implementation.

pub mod device;
pub mod buffer;

// Re-exports
pub use device::{Device, DeviceCapabilities};
pub use buffer::{DumbBuffer, MappedBuffer, PixelFormat};

#[allow(unused_imports)]
use crate::{DisplayError, Result};
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
    /// use toadstool_display::DrmBackend;
    ///
    /// let drm = DrmBackend::open("/dev/dri/card0")?;
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
}
