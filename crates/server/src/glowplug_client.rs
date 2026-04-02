// SPDX-License-Identifier: AGPL-3.0-only
//! glowPlug / ember device management service.
//!
//! toadStool-native device lifecycle management. This replaces the
//! former `coral-ember` proxy — toadStool IS the hardware primal and
//! owns the ember subsystem directly.
//!
//! Provides `ember.list`, `ember.status`, `ember.swap`, and
//! `ember.reacquire` operations for GPU passthrough and driver
//! personality management.
//!
//! ## Architecture
//!
//! - `toadstool-ember` (crate): hardware-agnostic device holder (held resources, journals)
//! - `toadstool-glowplug` (crate): hardware-agnostic device lifecycle (personality, swap, discovery)
//! - `toadstool-runtime-gpu/glowplug/`: GPU-specific implementations
//! - This module: server-side JSON-RPC service exposing ember to the ecosystem

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

/// Device entry returned by `ember.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberDeviceList {
    /// PCI BDF addresses of held devices.
    pub devices: Vec<String>,
}

/// Status response from `ember.status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberStatus {
    /// Held device BDFs.
    pub devices: Vec<String>,
    /// Daemon uptime in seconds.
    pub uptime_secs: u64,
}

/// Swap result from `ember.swap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberSwapResult {
    /// BDF of the swapped device.
    pub bdf: String,
    /// New personality after swap (e.g. `"vfio"`, `"nouveau"`, `"unbound"`).
    pub personality: String,
}

/// Reacquire result from `ember.reacquire`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberReacquireResult {
    /// BDF of the reacquired device.
    pub bdf: String,
}

/// toadStool-native device management service.
///
/// Manages GPU device lifecycle directly — no external primal dependency.
/// Uses PCI sysfs for device enumeration and driver bind/unbind.
pub struct GlowPlugClient {
    start_time: std::time::Instant,
}

impl Default for GlowPlugClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GlowPlugClient {
    /// Creates a new glowPlug service.
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
        }
    }

    /// Whether the glowPlug subsystem is operational.
    ///
    /// Always true — toadStool owns this subsystem natively.
    pub fn is_available(&self) -> bool {
        true
    }

    /// List all GPU devices visible via PCI sysfs.
    pub fn list_devices(&self) -> EmberDeviceList {
        let devices = discover_gpu_bdfs();
        debug!(count = devices.len(), "ember.list: enumerated GPU devices");
        EmberDeviceList { devices }
    }

    /// Query service status (held devices + uptime).
    pub fn status(&self) -> EmberStatus {
        EmberStatus {
            devices: discover_gpu_bdfs(),
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Request a driver swap for a device.
    ///
    /// `target` is the driver to bind (e.g. `"vfio-pci"`, `"nouveau"`, `"unbound"`).
    ///
    /// Uses sysfs driver_override + rebind for the swap.
    pub fn swap_device(&self, bdf: &str, target: &str) -> Option<EmberSwapResult> {
        debug!(bdf, target, "ember.swap: requesting personality swap");

        let override_path = format!("/sys/bus/pci/devices/{bdf}/driver_override");
        let unbind_path = find_driver_unbind_path(bdf);
        let bind_path = format!("/sys/bus/pci/drivers/{target}/bind");

        // Unbind current driver
        if let Some(ref unbind) = unbind_path
            && std::fs::write(unbind, bdf).is_err()
        {
            tracing::warn!(bdf, "unbind failed (device may not be bound)");
        }

        // Set driver override
        if target != "unbound" {
            if let Err(e) = std::fs::write(&override_path, target) {
                tracing::warn!(bdf, target, error = %e, "driver_override write failed");
                return None;
            }

            // Bind new driver
            if let Err(e) = std::fs::write(&bind_path, bdf) {
                tracing::warn!(bdf, target, error = %e, "driver bind failed");
            }
        }

        Some(EmberSwapResult {
            bdf: bdf.to_string(),
            personality: target.to_string(),
        })
    }

    /// Reacquire a device (rebind to vfio-pci after a swap).
    pub fn reacquire(&self, bdf: &str) -> Option<EmberReacquireResult> {
        self.swap_device(bdf, "vfio-pci")?;
        Some(EmberReacquireResult {
            bdf: bdf.to_string(),
        })
    }
}

/// Shared glowPlug service wrapped in Arc for handler use.
pub type SharedGlowPlugClient = Arc<GlowPlugClient>;

/// Create a shared glowPlug service instance.
pub fn create_glowplug_client() -> SharedGlowPlugClient {
    Arc::new(GlowPlugClient::new())
}

/// Discover GPU BDF addresses from PCI sysfs (class 0x030000 = VGA).
fn discover_gpu_bdfs() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return Vec::new();
    };

    let mut bdfs: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let class_path = entry.path().join("class");
            let class = std::fs::read_to_string(class_path).ok()?;
            let class_trimmed = class.trim();
            // VGA: 0x030000, 3D: 0x030200
            if class_trimmed.starts_with("0x0302") || class_trimmed.starts_with("0x0300") {
                entry.file_name().to_str().map(String::from)
            } else {
                None
            }
        })
        .collect();

    bdfs.sort();
    bdfs
}

/// Find the driver unbind path for a device (if currently bound).
fn find_driver_unbind_path(bdf: &str) -> Option<String> {
    let driver_link = format!("/sys/bus/pci/devices/{bdf}/driver");
    let driver = std::fs::read_link(&driver_link).ok()?;
    let driver_name = driver.file_name()?.to_str()?;
    Some(format!("/sys/bus/pci/drivers/{driver_name}/unbind"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_creation() {
        let client = GlowPlugClient::new();
        assert!(client.is_available());
    }

    #[test]
    fn shared_client_creation() {
        let client = create_glowplug_client();
        assert!(Arc::strong_count(&client) == 1);
    }

    #[test]
    fn status_has_uptime() {
        let client = GlowPlugClient::new();
        let status = client.status();
        assert!(status.uptime_secs < 10);
    }

    #[test]
    fn list_devices_returns_vec() {
        let client = GlowPlugClient::new();
        let list = client.list_devices();
        // May be empty in CI/test environments without GPUs
        let _ = list.devices;
    }

    #[test]
    fn ember_device_list_deserialization() {
        let json = r#"{"devices":["0000:01:00.0","0000:03:00.0"]}"#;
        let list: EmberDeviceList = serde_json::from_str(json).unwrap();
        assert_eq!(list.devices.len(), 2);
    }

    #[test]
    fn ember_status_deserialization() {
        let json = r#"{"devices":["0000:01:00.0"],"uptime_secs":3600}"#;
        let status: EmberStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.uptime_secs, 3600);
        assert_eq!(status.devices.len(), 1);
    }

    #[test]
    fn ember_swap_result_deserialization() {
        let json = r#"{"bdf":"0000:01:00.0","personality":"nouveau"}"#;
        let result: EmberSwapResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.personality, "nouveau");
    }

    #[test]
    fn ember_reacquire_result_deserialization() {
        let json = r#"{"bdf":"0000:01:00.0"}"#;
        let result: EmberReacquireResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.bdf, "0000:01:00.0");
    }

    #[test]
    fn discover_gpu_bdfs_runs() {
        let bdfs = discover_gpu_bdfs();
        // Should not panic, may be empty in CI
        for bdf in &bdfs {
            assert!(bdf.contains(':'));
        }
    }
}
