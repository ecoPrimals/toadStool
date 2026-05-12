// SPDX-License-Identifier: AGPL-3.0-or-later
//! glowPlug / ember device management service.
//!
//! toadStool-native device lifecycle management. This replaces the
//! former `coral-ember` proxy — toadStool IS the hardware primal and
//! owns the ember subsystem directly.
//!
//! Provides `ember.list` and `ember.status` JSON-RPC operations for GPU
//! passthrough and driver personality management. Device lifecycle swaps
//! use [`SwapOrchestrator`] exclusively (legacy synchronous path removed S243).
//!
//! ## Architecture
//!
//! - `toadstool-ember` (crate): hardware-agnostic device holder (held resources, journals)
//! - `toadstool-glowplug` (crate): hardware-agnostic device lifecycle (personality, swap, discovery)
//! - `toadstool-runtime-gpu/glowplug/`: GPU-specific implementations
//! - This module: server-side JSON-RPC service exposing ember to the ecosystem

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use toadstool_glowplug::boot::BootResult;
use toadstool_glowplug::device_id::DeviceId;
use toadstool_glowplug::sysfs_executor::SysfsSwapExecutor;
use toadstool_glowplug::swap::SwapOrchestrator;
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

/// Reacquire result from `ember.reacquire`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberReacquireResult {
    /// BDF of the reacquired device.
    pub bdf: String,
}

/// toadStool-native device management service.
///
/// Manages GPU device lifecycle directly — no external primal dependency.
/// Uses PCI sysfs for device enumeration and the [`SwapOrchestrator`] for
/// lifecycle-managed personality swaps (quiesce → persist → swap → restore → health).
///
/// This replaces `coral-glowplug`'s `EmberClient` — all operations that
/// formerly went through cross-process Unix socket IPC to `coral-ember`
/// are now performed internally via `SysfsSwapExecutor`.
pub struct GlowPlugClient {
    start_time: std::time::Instant,
    orchestrator: SwapOrchestrator<SysfsSwapExecutor>,
}

impl Default for GlowPlugClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GlowPlugClient {
    /// Creates a new glowPlug service with the sysfs-based swap orchestrator.
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            orchestrator: SwapOrchestrator::new(SysfsSwapExecutor),
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

    /// Request a lifecycle-managed driver swap for a device.
    ///
    /// Executes the full 7-step swap lifecycle via [`SwapOrchestrator`]:
    /// quiesce → persist → drop → delegate → reacquire → restore → health.
    ///
    /// Returns the full [`BootResult`] with per-step timing.
    pub async fn swap_device_orchestrated(&self, bdf: &str, target: &str) -> BootResult {
        let device = DeviceId::PciBdf(bdf.to_string());
        let current = read_current_driver(bdf);
        self.orchestrator
            .execute_boot(&device, current.as_deref(), target)
            .await
    }

    /// Reacquire a device (rebind to vfio-pci via orchestrated lifecycle).
    pub async fn reacquire(&self, bdf: &str) -> EmberReacquireResult {
        let result = self.swap_device_orchestrated(bdf, "vfio-pci").await;
        debug!(
            bdf,
            success = result.success,
            "ember.reacquire via orchestrator"
        );
        EmberReacquireResult {
            bdf: bdf.to_string(),
        }
    }

    /// Access the underlying swap orchestrator.
    pub fn orchestrator(&self) -> &SwapOrchestrator<SysfsSwapExecutor> {
        &self.orchestrator
    }
}

/// Shared glowPlug service wrapped in Arc for handler use.
pub type SharedGlowPlugClient = Arc<GlowPlugClient>;

/// Create a shared glowPlug service instance.
pub fn create_glowplug_client() -> SharedGlowPlugClient {
    Arc::new(GlowPlugClient::new())
}

/// Read the current driver bound to a PCI device.
fn read_current_driver(bdf: &str) -> Option<String> {
    let link = format!("/sys/bus/pci/devices/{bdf}/driver");
    std::fs::read_link(&link)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
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
