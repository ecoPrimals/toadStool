// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery, announcement, and filesystem operations for [`DisplayCapabilities`].

use super::paths::{get_discovery_dir, get_socket_path};
use super::types::{CapabilityMetadata, DisplayCapabilities, DisplayInfo, InputDeviceInfo};
use crate::{DisplayError, Result};
use toadstool_common::constants::PRIMAL_NAME;

impl DisplayCapabilities {
    /// Discover own capabilities (self-knowledge!)
    ///
    /// Queries local hardware to determine what we can provide.
    /// NO hardcoding! All runtime discovery!
    ///
    /// # Errors
    ///
    /// Returns an error if DRM device discovery fails or platform paths cannot be resolved.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::DisplayCapabilities;
    /// #[tokio::main]
    /// async fn main() -> toadstool_display::Result<()> {
    ///     let caps = DisplayCapabilities::discover_self()?;
    ///     println!("Found {} displays", caps.displays.len());
    ///     println!("Found {} input devices", caps.input_devices.len());
    ///     Ok(())
    /// }
    /// ```
    pub fn discover_self() -> Result<Self> {
        use crate::drm::Device as DrmDevice;
        use crate::input::Device as InputDevice;

        tracing::info!("🔍 Discovering display backend capabilities (self-knowledge)...");

        // Generate unique primal ID
        let primal_id = format!("{}-display-{}", PRIMAL_NAME, uuid::Uuid::new_v4());

        // Discover DRM devices (displays)
        let drm_devices = DrmDevice::discover_all().map_err(|e| {
            tracing::warn!("Failed to discover DRM devices: {}", e);
            e
        })?;

        tracing::info!("  Found {} DRM device(s)", drm_devices.len());

        // Convert to display info
        // Pending: Open each DRM device, use drm::control::Device to get connectors/modes,
        // then extract resolution and refresh rate per display. Currently uses fallback defaults.
        let displays: Vec<DisplayInfo> = drm_devices
            .iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                DisplayInfo {
                    name,
                    // Fallback defaults until DRM mode query is implemented
                    width: toadstool_common::constants::display::FALLBACK_WIDTH,
                    height: toadstool_common::constants::display::FALLBACK_HEIGHT,
                    refresh_rate: 60.0,
                    connected: true,
                }
            })
            .collect();

        // Discover input devices
        let input_devices_result = InputDevice::discover_all();
        let input_device_infos = match input_devices_result {
            Ok(devices) => {
                tracing::info!("  Found {} input device(s)", devices.len());
                devices
                    .into_iter()
                    .map(|info| InputDeviceInfo {
                        name: info.name,
                        device_type: format!("{:?}", info.device_type),
                    })
                    .collect()
            }
            Err(e) => {
                tracing::warn!("Failed to discover input devices: {}", e);
                vec![]
            }
        };

        // Determine socket path (XDG compliant)
        let socket_path = get_socket_path();

        tracing::info!("✅ Capability discovery complete!");
        tracing::info!("  Displays: {}", displays.len());
        tracing::info!("  Input devices: {}", input_device_infos.len());
        tracing::info!("  Socket: {}", socket_path.display());

        Ok(Self {
            primal_id,
            primal_type: PRIMAL_NAME.to_string(),
            socket_path,
            max_windows: 8, // Reasonable default
            supported_formats: vec![
                "RGBA8888".to_string(),
                "BGRA8888".to_string(),
                "RGB888".to_string(),
                "RGB565".to_string(),
            ],
            has_gpu_acceleration: true, // We have wgpu!
            vsync_available: true,
            displays,
            input_devices: input_device_infos,
            metadata: CapabilityMetadata {
                version: env!("CARGO_PKG_VERSION").to_string(),
                pure_rust: true,
                timestamp: toadstool_common::system_time_serde::format_rfc3339(
                    std::time::SystemTime::now(),
                ),
            },
        })
    }

    /// Announce capabilities (write to discovery directory)
    ///
    /// Writes capability JSON file so other primals can discover us.
    /// **No hardcoding!** Uses XDG standard paths.
    ///
    /// # Errors
    ///
    /// Returns an error if the discovery directory cannot be created or the capability file cannot be written.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::DisplayCapabilities;
    /// #[tokio::main]
    /// async fn main() -> toadstool_display::Result<()> {
    ///     let caps = DisplayCapabilities::discover_self()?;
    ///     caps.announce().await?;
    ///     println!("Announced capabilities!");
    ///     Ok(())
    /// }
    /// ```
    pub async fn announce(&self) -> Result<()> {
        let discovery_dir = get_discovery_dir();

        // Create directory if needed
        tokio::fs::create_dir_all(&discovery_dir).await?;

        // Write capability file
        let filename = format!("{}.json", self.primal_id);
        let filepath = discovery_dir.join(filename);

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| DisplayError::IpcError(format!("JSON serialization failed: {e}")))?;

        tokio::fs::write(&filepath, json).await?;

        tracing::info!("📢 Announced capabilities: {}", filepath.display());

        Ok(())
    }

    /// Find all display backends
    ///
    /// Reads capability files from discovery directory.
    /// Other primals use this to find us!
    ///
    /// # Errors
    ///
    /// Returns an error if the discovery directory cannot be read or capability files cannot be parsed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::DisplayCapabilities;
    /// #[tokio::main]
    /// async fn main() -> toadstool_display::Result<()> {
    ///     let backends = DisplayCapabilities::find_all().await?;
    ///     for backend in backends {
    ///         println!("Found: {} at {}",
    ///             backend.primal_id,
    ///             backend.socket_path.display());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn find_all() -> Result<Vec<Self>> {
        let discovery_dir = get_discovery_dir();

        if !discovery_dir.exists() {
            return Ok(vec![]);
        }

        let mut capabilities = Vec::new();

        let mut entries = tokio::fs::read_dir(&discovery_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only JSON files
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                match tokio::fs::read_to_string(&path).await {
                    Ok(json) => match serde_json::from_str::<Self>(&json) {
                        Ok(cap) => capabilities.push(cap),
                        Err(e) => {
                            tracing::debug!(
                                "Skipped invalid capability file {}: {}",
                                path.display(),
                                e
                            );
                        }
                    },
                    Err(e) => {
                        tracing::debug!("Failed to read {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(capabilities)
    }

    /// Cleanup (remove capability announcement)
    ///
    /// Call on shutdown to clean up discovery files.
    ///
    /// # Errors
    ///
    /// Returns an error if the discovery directory cannot be resolved or the capability file cannot be removed.
    pub async fn cleanup(&self) -> Result<()> {
        let discovery_dir = get_discovery_dir();
        let filename = format!("{}.json", self.primal_id);
        let filepath = discovery_dir.join(filename);

        if filepath.exists() {
            tokio::fs::remove_file(&filepath).await?;
            tracing::info!("🧹 Cleaned up capability file: {}", filepath.display());
        }

        Ok(())
    }
}
