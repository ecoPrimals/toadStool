// SPDX-License-Identifier: AGPL-3.0-or-later
//! Data types for display capability discovery and advertisement.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Display backend capabilities
///
/// Advertises what the display backend can provide.
/// Used for runtime discovery by UI primals (petalTongue).
///
/// ## Deep Debt Compliance
///
/// - **Self-knowledge**: Discovers own hardware at runtime
/// - **No hardcoding**: All paths discovered dynamically
/// - **Capability-based**: Other primals discover via capabilities
/// - **Runtime discovery**: No compile-time dependencies
///
/// ## Example
///
/// ```rust,no_run
/// # use toadstool_display::DisplayCapabilities;
/// #[tokio::main]
/// async fn main() -> toadstool_display::Result<()> {
///     // Discover own capabilities (self-knowledge!)
///     let caps = DisplayCapabilities::discover_self()?;
///     
///     // Announce to ecosystem
///     caps.announce().await?;
///     
///     // Other primals can now discover us!
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayCapabilities {
    /// Primal identifier (unique ID)
    pub primal_id: String,

    /// Primal type (always "toadstool" for us)
    pub primal_type: String,

    /// Unix socket path for communication
    pub socket_path: PathBuf,

    /// Maximum windows supported
    pub max_windows: usize,

    /// Supported pixel formats
    pub supported_formats: Vec<String>,

    /// GPU acceleration available
    pub has_gpu_acceleration: bool,

    /// `VSync` available
    pub vsync_available: bool,

    /// Detected displays
    pub displays: Vec<DisplayInfo>,

    /// Input devices
    pub input_devices: Vec<InputDeviceInfo>,

    /// Metadata
    pub metadata: CapabilityMetadata,
}

/// Display information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    /// Display name (e.g., "eDP-1")
    pub name: String,

    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// Refresh rate in Hz
    pub refresh_rate: f32,

    /// Connected status
    pub connected: bool,
}

/// Input device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDeviceInfo {
    /// Device name
    pub name: String,

    /// Device type (keyboard, mouse, etc.)
    pub device_type: String,
}

/// Capability metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    /// Version of display backend
    pub version: String,

    /// Pure Rust status
    pub pure_rust: bool,

    /// Timestamp of capability announcement
    pub timestamp: String,
}
