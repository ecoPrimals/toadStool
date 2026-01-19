//! DRM device management
//!
//! Safe wrappers around DRM device operations.

#[allow(unused_imports)]
use crate::{DisplayError, Result};
use std::path::{Path, PathBuf};

/// DRM device handle
///
/// Provides safe access to Direct Rendering Manager (DRM) functionality
/// for display control.
///
/// ## Safety
///
/// All unsafe operations are isolated and documented with SAFETY comments.
/// Public API is 100% safe.
///
/// ## Example
///
/// ```rust,no_run
/// use toadstool_display::drm::Device;
///
/// let device = Device::open("/dev/dri/card0")?;
/// let caps = device.query_capabilities()?;
/// ```
#[allow(dead_code)] // TODO: Phase 0 - Remove when fully implemented
pub struct Device {
    path: PathBuf,
    // TODO: Add linux-drm device handle
}

impl Device {
    /// Open a DRM device
    ///
    /// # Arguments
    ///
    /// * `path` - Path to DRM device (e.g., `/dev/dri/card0`)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Device doesn't exist
    /// - Permission denied
    /// - Device is not a DRM device
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Validate path exists
        if !path.exists() {
            return Err(DisplayError::DeviceNotFound(path));
        }
        
        tracing::info!("Opening DRM device: {}", path.display());
        
        // TODO: Phase 0 - Implement device opening
        // let fd = std::fs::OpenOptions::new()
        //     .read(true)
        //     .write(true)
        //     .open(&path)?;
        
        Ok(Self {
            path,
        })
    }
    
    /// Query device capabilities
    ///
    /// Returns information about what the device supports.
    pub fn query_capabilities(&self) -> Result<DeviceCapabilities> {
        tracing::debug!("Querying capabilities for: {}", self.path.display());
        
        // TODO: Phase 0 - Implement capability queries
        Ok(DeviceCapabilities {
            supports_dumb_buffers: true,
            supports_atomic_modesetting: false,
            preferred_depth: 32,
        })
    }
    
    /// Discover all DRM devices on the system
    ///
    /// This implements **self-knowledge** - the primal discovers its own
    /// hardware capabilities at runtime.
    ///
    /// No hardcoding! Agnostic discovery!
    pub fn discover_all() -> Result<Vec<PathBuf>> {
        tracing::info!("🔍 Discovering DRM devices (self-knowledge)...");
        
        let mut devices = Vec::new();
        
        // Capability-based discovery: scan /dev/dri/
        let drm_dir = Path::new("/dev/dri");
        if !drm_dir.exists() {
            tracing::warn!("No /dev/dri directory - no DRM devices available");
            return Ok(devices);
        }
        
        // Read directory entries
        let entries = std::fs::read_dir(drm_dir)
            .map_err(|e| DisplayError::IoctlFailed(format!("Failed to read /dev/dri: {}", e)))?;
            
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // Only card* devices (not renderD* or controlD*)
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("card") {
                    tracing::debug!("  Found: {}", path.display());
                    devices.push(path);
                }
            }
        }
        
        tracing::info!("✅ Discovered {} DRM device(s)", devices.len());
        Ok(devices)
    }
}

/// Device capabilities
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    /// Supports dumb buffer allocation
    pub supports_dumb_buffers: bool,
    /// Supports atomic modesetting
    pub supports_atomic_modesetting: bool,
    /// Preferred color depth
    pub preferred_depth: u32,
}

// TODO: Phase 0 Implementation
//
// 1. Device opening:
//    - Use linux_drm::Device or rustix for file operations
//    - Validate it's actually a DRM device (DRM_IOCTL_VERSION)
//    - Store file descriptor
//
// 2. Capability queries:
//    - DRM_CAP_DUMB_BUFFER
//    - DRM_CAP_DUMB_PREFERRED_DEPTH
//    - DRM_CAP_ATOMIC
//
// 3. Resource enumeration:
//    - Get connectors (displays)
//    - Get CRTCs (scanout engines)
//    - Get encoders
//    - Get modes (resolutions/refresh rates)
//
// Safety notes:
// - File descriptor must be properly closed (use Drop)
// - ioctl calls are unsafe but wrapped in safe API
// - All pointers validated before dereferencing
