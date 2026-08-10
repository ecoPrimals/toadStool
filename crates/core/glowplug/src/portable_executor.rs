// SPDX-License-Identifier: AGPL-3.0-or-later

//! Platform-agnostic swap executor for non-sysfs platforms.
//!
//! On Linux, [`SysfsSwapExecutor`](crate::SysfsSwapExecutor) drives real
//! kernel driver unbind/rebind via sysfs. On platforms without sysfs
//! (Windows, Android, macOS), personality swaps operate at a higher
//! abstraction: the executor tracks logical personality state and delegates
//! the actual backend transition to the resource handle layer (ember).
//!
//! Phase 2 Silicon Atheism: abstraction over gating.

use std::sync::Arc;
use std::time::Instant;

use std::sync::RwLock;

use crate::device_id::DeviceId;
use crate::swap::{SwapExecutor, SwapObservation};

/// Error type for portable swap operations.
#[derive(Debug, thiserror::Error)]
pub enum PortableSwapError {
    /// The requested personality is not supported on this platform.
    #[error("unsupported personality: {0}")]
    UnsupportedPersonality(String),

    /// The device is not in a state where it can be swapped.
    #[error("device not swappable: {0}")]
    NotSwappable(String),

    /// Release failed.
    #[error("release failed: {0}")]
    ReleaseFailed(String),
}

/// Portable swap executor for platforms without kernel driver swap (sysfs).
///
/// Instead of unbinding/rebinding kernel drivers, this executor tracks
/// personality as logical state. The actual hardware transition is
/// delegated to the resource handle layer — e.g. dropping and recreating
/// a Vulkan `VkDevice` with different features, or re-initializing an
/// Android GPU context.
///
/// Valid personalities for the portable executor:
/// - `"compute"` — general-purpose compute (default)
/// - `"graphics"` — graphics/rendering mode
/// - `"low-power"` — power-saving mode
/// - `"unbound"` — released/idle state
#[derive(Debug)]
pub struct PortableSwapExecutor {
    personalities: Arc<RwLock<std::collections::HashMap<String, String>>>,
}

impl Default for PortableSwapExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl PortableSwapExecutor {
    /// Supported personality names for portable platforms.
    pub const PORTABLE_PERSONALITIES: &[&str] = &["compute", "graphics", "low-power", "unbound"];

    /// Create a new portable swap executor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            personalities: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Query the current personality for a device, if tracked.
    pub async fn current_personality(&self, device: &DeviceId) -> Option<String> {
        let map = self.personalities.read().unwrap_or_else(|e| e.into_inner());
        map.get(&device.to_string()).cloned()
    }
}

impl SwapExecutor for PortableSwapExecutor {
    type Error = PortableSwapError;

    async fn execute_swap(
        &self,
        device: &DeviceId,
        target_personality: &str,
    ) -> Result<SwapObservation, Self::Error> {
        if !Self::PORTABLE_PERSONALITIES.contains(&target_personality) {
            return Err(PortableSwapError::UnsupportedPersonality(
                target_personality.to_string(),
            ));
        }

        let start = Instant::now();
        let key = device.to_string();

        let from = {
            let map = self.personalities.read().unwrap_or_else(|e| e.into_inner());
            map.get(&key).cloned().unwrap_or_else(|| "unbound".into())
        };

        {
            let mut map = self.personalities.write().unwrap_or_else(|e| e.into_inner());
            map.insert(key, target_personality.to_string());
        }

        Ok(SwapObservation {
            device_id: device.short_label(),
            from,
            to: target_personality.to_string(),
            success: true,
            duration: start.elapsed(),
            error: None,
            detail: Some(serde_json::json!({
                "executor": "portable",
                "note": "logical swap — no kernel driver transition"
            })),
        })
    }

    async fn release(&self, device: &DeviceId) -> Result<(), Self::Error> {
        let key = device.to_string();
        let mut map = self.personalities.write().unwrap_or_else(|e| e.into_inner());
        map.insert(key, "unbound".to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_device() -> DeviceId {
        DeviceId::Platform("wgpu:Vulkan:0x10de:0x1b80:GTX 1080".into())
    }

    #[tokio::test]
    async fn swap_to_compute() {
        let exec = PortableSwapExecutor::new();
        let obs = exec.execute_swap(&test_device(), "compute").await.unwrap();
        assert!(obs.success);
        assert_eq!(obs.to, "compute");
        assert_eq!(obs.from, "unbound");
    }

    #[tokio::test]
    async fn swap_tracks_personality() {
        let exec = PortableSwapExecutor::new();
        let dev = test_device();

        exec.execute_swap(&dev, "compute").await.unwrap();
        assert_eq!(exec.current_personality(&dev).await, Some("compute".into()));

        let obs = exec.execute_swap(&dev, "graphics").await.unwrap();
        assert_eq!(obs.from, "compute");
        assert_eq!(obs.to, "graphics");
    }

    #[tokio::test]
    async fn unsupported_personality_errors() {
        let exec = PortableSwapExecutor::new();
        let result = exec.execute_swap(&test_device(), "vfio-pci").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unsupported personality"));
    }

    #[tokio::test]
    async fn release_sets_unbound() {
        let exec = PortableSwapExecutor::new();
        let dev = test_device();

        exec.execute_swap(&dev, "compute").await.unwrap();
        exec.release(&dev).await.unwrap();
        assert_eq!(exec.current_personality(&dev).await, Some("unbound".into()));
    }

    #[tokio::test]
    async fn swap_observation_has_detail() {
        let exec = PortableSwapExecutor::new();
        let obs = exec.execute_swap(&test_device(), "compute").await.unwrap();
        let detail = obs.detail.unwrap();
        assert_eq!(detail["executor"], "portable");
    }

    #[test]
    fn portable_personalities_list() {
        assert!(PortableSwapExecutor::PORTABLE_PERSONALITIES.contains(&"compute"));
        assert!(PortableSwapExecutor::PORTABLE_PERSONALITIES.contains(&"graphics"));
        assert!(PortableSwapExecutor::PORTABLE_PERSONALITIES.contains(&"low-power"));
        assert!(PortableSwapExecutor::PORTABLE_PERSONALITIES.contains(&"unbound"));
        assert!(!PortableSwapExecutor::PORTABLE_PERSONALITIES.contains(&"vfio-pci"));
    }

    #[test]
    fn default_creates_empty_state() {
        let exec = PortableSwapExecutor::default();
        assert!(format!("{exec:?}").contains("PortableSwapExecutor"));
    }
}
