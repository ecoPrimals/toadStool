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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use toadstool_glowplug::boot::BootResult;
use toadstool_glowplug::device_id::DeviceId;
use toadstool_glowplug::swap::SwapOrchestrator;
use toadstool_glowplug::sysfs_executor::SysfsSwapExecutor;
use tracing::debug;

pub use crate::glowplug_types::{
    DeviceSwapResult, DeviceSwapStep, EmberDeviceInfo, EmberDeviceList, EmberDeviceListEnriched,
    EmberReacquireResult, EmberStatus, ExperimentLifecycleResult, ExperimentSession,
};

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
    experiments: Mutex<HashMap<String, ExperimentSession>>,
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
            experiments: Mutex::new(HashMap::new()),
        }
    }

    /// Whether the glowPlug subsystem is operational.
    ///
    /// Always true — toadStool owns this subsystem natively.
    pub fn is_available(&self) -> bool {
        true
    }

    /// List all GPU devices visible via PCI sysfs with enriched metadata.
    pub fn list_devices(&self) -> EmberDeviceListEnriched {
        let devices = discover_gpu_devices();
        debug!(count = devices.len(), "ember.list: enumerated GPU devices");
        EmberDeviceListEnriched { devices }
    }

    /// Fetch enriched metadata for a single GPU by BDF.
    pub fn get_device(&self, bdf: &str) -> Option<EmberDeviceInfo> {
        discover_single_device(bdf)
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

    /// Swap a device to an arbitrary target personality via orchestrated lifecycle.
    ///
    /// Activates PCIe keepalive burst mode for the duration of the swap to
    /// prevent PLX bridge D3cold during the unbind window.
    ///
    /// Returns a [`DeviceSwapResult`] with per-step timing and success status.
    pub async fn swap(&self, bdf: &str, target: &str) -> DeviceSwapResult {
        let _keepalive_guard = crate::background::pcie_keepalive::SwapGuard::enter();
        let result = self.swap_device_orchestrated(bdf, target).await;
        debug!(
            bdf,
            target,
            success = result.success,
            "device.swap via orchestrator"
        );
        DeviceSwapResult {
            bdf: bdf.to_string(),
            target: target.to_string(),
            success: result.success,
            steps: result
                .steps
                .iter()
                .map(|s| DeviceSwapStep {
                    name: s.name.clone(),
                    duration_ms: s.duration_ms,
                    success: s.status == toadstool_glowplug::boot::StepStatus::Ok,
                })
                .collect(),
        }
    }

    /// Detect whether a GPU is in a warm state (HBM/GDDR trained, engines enabled).
    ///
    /// Reads PMC_ENABLE at BAR0 offset 0x200 via sysfs resource0 mmap to
    /// determine if the GPU was previously initialized (e.g. by nouveau
    /// warm-handoff). Also probes FECS CPUCTL (0x409100) for falcon state.
    pub fn warm_detect(&self, bdf: &str) -> serde_json::Value {
        let resource_path =
            toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "resource0");
        let resource0_exists = std::path::Path::new(&resource_path).exists();

        let (pmc_enable, fecs_cpuctl) = if resource0_exists {
            read_bar0_registers(bdf)
        } else {
            (0, 0)
        };
        let popcount = pmc_enable.count_ones();
        let fecs_halted = fecs_cpuctl & 0x20 != 0;
        let warm = popcount >= 8;

        debug!(
            bdf,
            pmc_enable = format!("{pmc_enable:#010x}"),
            popcount,
            fecs_halted,
            warm,
            "device.warm_catch"
        );

        serde_json::json!({
            "bdf": bdf,
            "warm_detected": warm,
            "pmc_enable": format!("{pmc_enable:#010x}"),
            "pmc_popcount": popcount,
            "fecs_cpuctl": format!("{fecs_cpuctl:#010x}"),
            "fecs_halted": fecs_halted,
            "fecs_ready": warm && !fecs_halted,
            "resource0_exists": resource0_exists,
        })
    }

    /// Start or stop an experiment session on a held device.
    pub fn experiment_lifecycle(&self, bdf: &str, action: &str) -> ExperimentLifecycleResult {
        let mut experiments = self
            .experiments
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match action {
            "start" => {
                let session = ExperimentSession {
                    bdf: bdf.to_string(),
                    started_at: self.start_time.elapsed().as_secs(),
                    active: true,
                };
                experiments.insert(bdf.to_string(), session.clone());
                ExperimentLifecycleResult {
                    bdf: bdf.to_string(),
                    action: "start".to_string(),
                    success: true,
                    session: Some(session),
                }
            }
            "end" => {
                let session = experiments.remove(bdf);
                ExperimentLifecycleResult {
                    bdf: bdf.to_string(),
                    action: "end".to_string(),
                    success: session.is_some(),
                    session,
                }
            }
            _ => ExperimentLifecycleResult {
                bdf: bdf.to_string(),
                action: action.to_string(),
                success: false,
                session: None,
            },
        }
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

/// Resolve the runtime directory for toadStool's socket tree.
///
/// Priority: `TOADSTOOL_RUN_DIR` env → `/run/toadstool`.
pub fn run_dir() -> std::path::PathBuf {
    std::env::var(toadstool_common::interned_strings::socket_env::TOADSTOOL_RUN_DIR).map_or_else(
        |_| std::path::PathBuf::from("/run/toadstool"),
        std::path::PathBuf::from,
    )
}

/// Shared glowPlug service wrapped in Arc for handler use.
pub type SharedGlowPlugClient = Arc<GlowPlugClient>;

/// Create a shared glowPlug service instance.
pub fn create_glowplug_client() -> SharedGlowPlugClient {
    Arc::new(GlowPlugClient::new())
}

// PCI discovery extracted to `glowplug_discovery.rs`
pub use crate::glowplug_discovery::discover_gpu_bdfs;
use crate::glowplug_discovery::{
    discover_gpu_devices, discover_single_device, read_bar0_registers, read_current_driver,
};
#[cfg(test)]
use crate::glowplug_discovery::{is_gpu_bdf, pci_bdf_matches, read_pci_config_u32};

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
        let _ = list.devices;
    }

    #[test]
    fn ember_device_list_deserialization() {
        let json = r#"{"devices":[{"bdf":"0000:01:00.0","name":"0x1b06","vendor_id":4318,"personality":"vfio-pci","protected":false,"vram_alive":true,"domains_faulted":0},{"bdf":"0000:03:00.0","name":null,"vendor_id":4318,"personality":"unbound","protected":true,"vram_alive":false,"domains_faulted":0}]}"#;
        let list: EmberDeviceListEnriched = serde_json::from_str(json).unwrap();
        assert_eq!(list.devices.len(), 2);
        assert_eq!(list.devices[0].bdf, "0000:01:00.0");
        assert!(list.devices[0].vram_alive);
        assert!(list.devices[1].protected);
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
        for bdf in &bdfs {
            assert!(bdf.contains(':'));
        }
    }

    #[tokio::test]
    async fn reacquire_returns_bdf() {
        let client = GlowPlugClient::new();
        let result = client.reacquire("0000:99:00.0").await;
        assert_eq!(result.bdf, "0000:99:00.0");
    }

    #[tokio::test]
    async fn swap_device_orchestrated_returns_boot_result() {
        let client = GlowPlugClient::new();
        let result = client
            .swap_device_orchestrated("0000:99:00.0", "vfio-pci")
            .await;
        assert!(!result.steps.is_empty());
    }

    #[test]
    fn orchestrator_accessible() {
        let client = GlowPlugClient::new();
        let _orch = client.orchestrator();
    }

    #[test]
    fn read_current_driver_nonexistent_device() {
        let result = read_current_driver("ffff:ff:ff.f");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn swap_returns_result_with_steps() {
        let client = GlowPlugClient::new();
        let result = client.swap("0000:99:00.0", "vfio-pci").await;
        assert_eq!(result.bdf, "0000:99:00.0");
        assert_eq!(result.target, "vfio-pci");
        assert!(!result.steps.is_empty());
    }

    #[test]
    fn warm_detect_nonexistent_device() {
        let client = GlowPlugClient::new();
        let result = client.warm_detect("ffff:ff:ff.f");
        assert_eq!(result["bdf"], "ffff:ff:ff.f");
        assert_eq!(result["warm_detected"], false);
        assert_eq!(result["resource0_exists"], false);
    }

    #[test]
    fn device_swap_result_serialization() {
        let result = DeviceSwapResult {
            bdf: "0000:01:00.0".to_string(),
            target: "nouveau".to_string(),
            success: true,
            steps: vec![DeviceSwapStep {
                name: "detect_driver".to_string(),
                duration_ms: 5,
                success: true,
            }],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["bdf"], "0000:01:00.0");
        assert_eq!(json["target"], "nouveau");
        assert_eq!(json["success"], true);
        assert_eq!(json["steps"][0]["name"], "detect_driver");
    }

    #[test]
    fn run_dir_default() {
        temp_env::with_var_unset("TOADSTOOL_RUN_DIR", || {
            let dir = super::run_dir();
            assert_eq!(dir, std::path::PathBuf::from("/run/toadstool"));
        });
    }

    #[test]
    fn run_dir_override() {
        temp_env::with_var("TOADSTOOL_RUN_DIR", Some("/custom/toadstool"), || {
            let dir = super::run_dir();
            assert_eq!(dir, std::path::PathBuf::from("/custom/toadstool"));
        });
    }

    #[test]
    fn read_pci_config_u32_nonexistent() {
        let val = read_pci_config_u32("ffff:ff:ff.f", 0x200);
        assert_eq!(val, 0);
    }

    #[test]
    fn get_device_nonexistent_bdf_returns_none() {
        let client = GlowPlugClient::new();
        assert!(client.get_device("ffff:ff:ff.f").is_none());
    }

    #[test]
    fn discover_single_device_rejects_non_gpu_bdf() {
        assert!(discover_single_device("ffff:ff:ff.f").is_none());
    }

    #[test]
    fn is_gpu_bdf_rejects_missing_device() {
        assert!(!is_gpu_bdf("ffff:ff:ff.f"));
    }

    #[test]
    fn experiment_lifecycle_unknown_action_fails() {
        let client = GlowPlugClient::new();
        let result = client.experiment_lifecycle("0000:01:00.0", "pause");
        assert!(!result.success);
        assert_eq!(result.action, "pause");
        assert!(result.session.is_none());
    }

    #[test]
    fn experiment_lifecycle_start_and_end() {
        let client = GlowPlugClient::new();
        let start = client.experiment_lifecycle("0000:01:00.0", "start");
        assert!(start.success);
        assert!(start.session.as_ref().is_some_and(|s| s.active));

        let end = client.experiment_lifecycle("0000:01:00.0", "end");
        assert!(end.success);
        assert!(end.session.is_some());

        let end_again = client.experiment_lifecycle("0000:01:00.0", "end");
        assert!(!end_again.success);
        assert!(end_again.session.is_none());
    }

    #[test]
    fn pci_bdf_matches_nonexistent_link_returns_false() {
        let path = std::path::Path::new("/nonexistent/device/link");
        assert!(!pci_bdf_matches(path, "0000:01:00.0"));
    }

    #[test]
    fn warm_detect_invalid_bdf_has_zero_registers() {
        let client = GlowPlugClient::new();
        let result = client.warm_detect("ffff:ff:ff.f");
        assert_eq!(result["pmc_enable"], "0x00000000");
        assert_eq!(result["fecs_cpuctl"], "0x00000000");
        assert_eq!(result["pmc_popcount"], 0);
    }
}
