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
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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

/// Enriched device info for `ember.list` / `device.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberDeviceInfo {
    /// PCI BDF address.
    pub bdf: String,
    /// Human-readable device name, if known.
    pub name: Option<String>,
    /// PCI vendor ID.
    pub vendor_id: u16,
    /// Active personality (e.g. compute, display).
    pub personality: String,
    /// Whether the device is protected from experiment takeover.
    #[serde(default)]
    pub protected: bool,
    /// Whether VRAM is responding to probes.
    pub vram_alive: bool,
    /// Number of faulted engine domains.
    pub domains_faulted: usize,
}

/// Enriched list response from `ember.list` / `device.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberDeviceListEnriched {
    /// Held devices with enriched metadata.
    pub devices: Vec<EmberDeviceInfo>,
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

/// Swap result returned by `device.swap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSwapResult {
    /// BDF of the swapped device.
    pub bdf: String,
    /// Target personality the device was swapped to.
    pub target: String,
    /// Whether the swap succeeded.
    pub success: bool,
    /// Per-step timing from the orchestrator.
    pub steps: Vec<DeviceSwapStep>,
}

/// Active experiment session on a held device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSession {
    /// PCI BDF address under experiment.
    pub bdf: String,
    /// Session start time (seconds since daemon start).
    pub started_at: u64,
    /// Whether the session is still active.
    pub active: bool,
}

/// Result of `experiment.start` / `experiment.stop` lifecycle calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentLifecycleResult {
    /// PCI BDF address.
    pub bdf: String,
    /// Lifecycle action performed (`start` or `stop`).
    pub action: String,
    /// Whether the action succeeded.
    pub success: bool,
    /// Current session state, if applicable.
    pub session: Option<ExperimentSession>,
}

/// Single step in a device swap lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSwapStep {
    /// Step identifier (e.g. "detect_driver", "swap_to_vfio").
    pub name: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Whether this step succeeded.
    pub success: bool,
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
        let resource_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "resource0");
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
        let mut experiments = self.experiments.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
    std::env::var(toadstool_common::interned_strings::socket_env::TOADSTOOL_RUN_DIR)
        .map_or_else(|_| std::path::PathBuf::from("/run/toadstool"), std::path::PathBuf::from)
}

/// Shared glowPlug service wrapped in Arc for handler use.
pub type SharedGlowPlugClient = Arc<GlowPlugClient>;

/// Create a shared glowPlug service instance.
pub fn create_glowplug_client() -> SharedGlowPlugClient {
    Arc::new(GlowPlugClient::new())
}

/// Read the current driver bound to a PCI device.
fn read_current_driver(bdf: &str) -> Option<String> {
    let link = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "driver");
    std::fs::read_link(&link)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
}

/// Read a 32-bit value from PCI config space via sysfs.
///
/// Falls back to `0` if the device or offset is inaccessible.
#[cfg(test)]
fn read_pci_config_u32(bdf: &str, offset: u64) -> u32 {
    use std::io::{Read, Seek, SeekFrom};
    let path = format!("/sys/bus/pci/devices/{bdf}/config");
    let Ok(mut f) = std::fs::File::open(&path) else {
        return 0;
    };
    if f.seek(SeekFrom::Start(offset)).is_err() {
        return 0;
    }
    let mut buf = [0u8; 4];
    if f.read_exact(&mut buf).is_err() {
        return 0;
    }
    u32::from_le_bytes(buf)
}

/// Probe GPU registers from BAR0 via `nvpmu::bar0::Bar0Access`.
///
/// Returns (PMC_ENABLE at 0x200, FECS_CPUCTL at 0x409100).
/// Falls back to (0, 0) if the BAR0 resource is inaccessible.
fn read_bar0_registers(bdf: &str) -> (u32, u32) {
    let Ok(bar0) = nvpmu::bar0::Bar0Access::open(bdf) else {
        return (0, 0);
    };
    let pmc_enable = bar0.read_u32(0x200).unwrap_or(0);
    let fecs_cpuctl = bar0.read_u32(0x40_9100).unwrap_or(0);
    (pmc_enable, fecs_cpuctl)
}

/// Discover enriched GPU device info for all visible GPUs.
fn discover_gpu_devices() -> Vec<EmberDeviceInfo> {
    discover_gpu_bdfs()
        .into_iter()
        .filter_map(|bdf| discover_single_device(&bdf))
        .collect()
}

/// Probe enriched metadata for a single GPU BDF.
fn discover_single_device(bdf: &str) -> Option<EmberDeviceInfo> {
    let pci_path = toadstool_cylinder::linux_paths::sysfs_pci_device_path(bdf);
    if !std::path::Path::new(&pci_path).exists() || !is_gpu_bdf(bdf) {
        return None;
    }

    Some(EmberDeviceInfo {
        bdf: bdf.to_string(),
        name: read_device_name(bdf),
        vendor_id: toadstool_ember::sysfs::read_pci_id(bdf, "vendor"),
        personality: read_current_driver(bdf).unwrap_or_else(|| "unbound".into()),
        protected: is_display_connected(bdf),
        vram_alive: probe_vram_alive(bdf),
        domains_faulted: 0,
    })
}

fn is_gpu_bdf(bdf: &str) -> bool {
    let class_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "class");
    let Ok(class) = std::fs::read_to_string(class_path) else {
        return false;
    };
    let class_trimmed = class.trim();
    class_trimmed.starts_with("0x0302") || class_trimmed.starts_with("0x0300")
}

fn read_device_name(bdf: &str) -> Option<String> {
    let label_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "label");
    if let Ok(label) = std::fs::read_to_string(&label_path) {
        let trimmed = label.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    let device_id = toadstool_ember::sysfs::read_pci_id(bdf, "device");
    if device_id == 0 {
        None
    } else {
        Some(format!("0x{device_id:04x}"))
    }
}

fn probe_vram_alive(bdf: &str) -> bool {
    let Ok(bar0) = nvpmu::bar0::Bar0Access::open(bdf) else {
        return false;
    };
    match bar0.read_u32(0) {
        Ok(val) => val != 0xFFFF_FFFF,
        Err(_) => false,
    }
}

/// True when a physical display connector is connected on this GPU's DRM card.
fn is_display_connected(bdf: &str) -> bool {
    let drm_dir = std::path::Path::new("/sys/class/drm");
    let Ok(entries) = std::fs::read_dir(drm_dir) else {
        return false;
    };

    let mut card_names = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.contains('-') || !name_str.starts_with("card") {
            continue;
        }
        let device_link = entry.path().join("device");
        if pci_bdf_matches(&device_link, bdf) {
            card_names.push(name_str.into_owned());
        }
    }

    if card_names.is_empty() {
        return false;
    }

    let Ok(entries) = std::fs::read_dir(drm_dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.contains('-') {
            continue;
        }
        let is_connector = card_names
            .iter()
            .any(|card| name_str.starts_with(&format!("{card}-")));
        if !is_connector {
            continue;
        }
        let status_path = entry.path().join("status");
        if let Ok(status) = std::fs::read_to_string(status_path)
            && status.trim() == "connected"
        {
            return true;
        }
    }
    false
}

fn pci_bdf_matches(device_link: &std::path::Path, bdf: &str) -> bool {
    let Ok(canonical) = std::fs::canonicalize(device_link) else {
        return false;
    };
    canonical
        .file_name()
        .is_some_and(|name| name.to_string_lossy() == bdf)
}

/// Discover GPU BDF addresses from PCI sysfs (class 0x030000 = VGA).
pub fn discover_gpu_bdfs() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(toadstool_cylinder::linux_paths::sysfs_pci_devices()) else {
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
}
