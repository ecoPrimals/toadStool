// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Primal Capability Infrastructure
//!
//! Self-knowledge and capability-based discovery for primals.
//!
//! ## Deep Debt Principles ✅
//! - **Self-Knowledge Only**: Query only local system state
//! - **No External Registration**: No centralized registry!
//! - **Capability-Based**: Discover peers by what they can do
//! - **Runtime Discovery**: Find primals at runtime, not compile-time
//! - **Pure Rust**: Zero C dependencies!
//!
//! ## Architecture
//!
//! Each primal:
//! 1. Knows itself (resources, capabilities)
//! 2. Announces capabilities (optional, for discovery)
//! 3. Discovers peers by capabilities needed
//! 4. No centralized registry (peer-to-peer!)
//!
//! ## Example
//!
//! ```rust,ignore
//! // Query self (always works!)
//! let my_caps = PrimalCapabilities::discover_self().await;
//! println!("I have: {} CPU cores, {} GPUs", my_caps.resources.cpu_cores, my_caps.resources.gpu_devices.len());
//!
//! // Announce capabilities (optional, for peer discovery)
//! my_caps.announce().await?;
//!
//! // Find peer with specific capability
//! let gpu_primal = PrimalCapabilities::find_peer_with("gpu-nvidia").await?;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Primal capabilities and self-knowledge
///
/// Deep debt principle: Self-knowledge only!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalCapabilities {
    /// Unique primal ID (generated at startup)
    pub primal_id: String,
    /// Primal type (e.g., "toadstool", "beardog", "songbird")
    pub primal_type: String,
    /// Version
    pub version: String,
    /// Local system resources (self-knowledge!)
    pub resources: SystemResources,
    /// Capabilities (derived from resources)
    pub capabilities: Vec<String>,
    /// Unix socket path (for peer communication)
    pub socket_path: PathBuf,
    /// Optional metadata
    pub metadata: HashMap<String, String>,
}

/// System resources (self-knowledge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    pub cpu_cores: usize,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub gpu_devices: Vec<GpuDevice>,
    pub architecture: String,
    pub os: String,
}

/// GPU device information (self-knowledge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub device_id: usize,
    pub name: String,
    pub vendor: String, // "nvidia", "amd", "intel", "apple"
    pub memory_bytes: u64,
    pub compute_capability: Option<String>,
}

impl PrimalCapabilities {
    /// Discover self (self-knowledge!)
    ///
    /// Deep debt principle: Query local system only!
    #[allow(clippy::unused_async)] // API consistency; may add async discovery in future
    pub async fn discover_self(primal_type: &str) -> Self {
        info!("🔍 Discovering self capabilities (self-knowledge!)");

        let primal_id = Uuid::new_v4().to_string();
        let resources = query_system_resources();
        let capabilities = build_capabilities(&resources);
        let socket_path = default_socket_path(&primal_id);

        info!("✅ Self-knowledge acquired:");
        info!("   - Primal ID: {}", primal_id);
        info!("   - Type: {}", primal_type);
        info!("   - CPU Cores: {}", resources.cpu_cores);
        info!(
            "   - Memory: {} GB",
            resources.total_memory_bytes / (1024 * 1024 * 1024)
        );
        info!("   - GPUs: {}", resources.gpu_devices.len());
        info!("   - Capabilities: {}", capabilities.len());
        info!("   - Socket: {}", socket_path.display());

        Self {
            primal_id,
            primal_type: String::from(primal_type),
            version: String::from(env!("CARGO_PKG_VERSION")),
            resources,
            capabilities,
            socket_path,
            metadata: HashMap::new(),
        }
    }

    /// Announce capabilities (optional, for peer discovery)
    ///
    /// Deep debt principle: Announcement, not registration!
    /// - Writes capability file to shared discovery directory
    /// - Peers can read it to discover us
    /// - No centralized registry!
    ///
    /// # Errors
    ///
    /// Returns error string if directory creation, serialization, or file write fails.
    pub async fn announce(&self) -> Result<(), String> {
        let discovery_dir = discovery_directory();

        // Create discovery directory if needed
        fs::create_dir_all(&discovery_dir)
            .await
            .map_err(|e| format!("Failed to create discovery directory: {e}"))?;

        // Write capability file
        let capability_file = discovery_dir.join(format!("{}.json", self.primal_id));
        let json = serde_json::to_string_pretty(&self)
            .map_err(|e| format!("Failed to serialize capabilities: {e}"))?;

        fs::write(&capability_file, json)
            .await
            .map_err(|e| format!("Failed to write capability file: {e}"))?;

        info!("📢 Announced capabilities: {}", capability_file.display());
        info!("   Deep debt principle: Peer discovery, not centralized registry!");

        Ok(())
    }

    /// Find peer with specific capability
    ///
    /// Deep debt principle: Runtime discovery!
    ///
    /// # Errors
    ///
    /// Returns error string if discovery directory read fails or no peer with the capability is found.
    pub async fn find_peer_with(capability: &str) -> Result<Self, String> {
        Self::find_peer_with_in(capability, &discovery_directory()).await
    }

    /// Find peer with specific capability in a given discovery directory.
    ///
    /// Testable variant that avoids global env var mutation.
    ///
    /// # Errors
    ///
    /// Returns error string if directory read fails, file parse fails, or no peer with the capability is found.
    pub async fn find_peer_with_in(
        capability: &str,
        discovery_dir: &std::path::Path,
    ) -> Result<Self, String> {
        debug!("🔍 Searching for peer with capability: {}", capability);

        // Read all capability files
        let mut entries = fs::read_dir(&discovery_dir)
            .await
            .map_err(|e| format!("Failed to read discovery directory: {e}"))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read entry: {e}"))?
        {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Read peer capability file
            let json = fs::read_to_string(&path)
                .await
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

            let peer: PrimalCapabilities = serde_json::from_slice(json.as_bytes())
                .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

            // Check if peer has the capability
            if peer.capabilities.iter().any(|c| c.contains(capability)) {
                info!(
                    "✅ Found peer with capability '{}': {}",
                    capability, peer.primal_id
                );
                return Ok(peer);
            }
        }

        Err(format!("No peer found with capability '{capability}'"))
    }

    /// Find all peers
    ///
    /// Deep debt principle: Peer discovery at runtime!
    ///
    /// # Errors
    ///
    /// Returns error string if discovery directory read fails.
    pub async fn find_all_peers() -> Result<Vec<Self>, String> {
        Self::find_all_peers_in(&discovery_directory()).await
    }

    /// Find all peers in a given discovery directory.
    ///
    /// Testable variant that avoids global env var mutation.
    ///
    /// # Errors
    ///
    /// Returns error string if discovery directory read fails.
    pub async fn find_all_peers_in(discovery_dir: &std::path::Path) -> Result<Vec<Self>, String> {
        debug!("🔍 Discovering all peers");
        let mut peers = Vec::new();

        // Read all capability files
        let mut entries = fs::read_dir(&discovery_dir)
            .await
            .map_err(|e| format!("Failed to read discovery directory: {e}"))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read entry: {e}"))?
        {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Read peer capability file
            match fs::read_to_string(&path).await {
                Ok(json) => match serde_json::from_slice::<PrimalCapabilities>(json.as_bytes()) {
                    Ok(peer) => peers.push(peer),
                    Err(e) => warn!("Failed to parse {}: {}", path.display(), e),
                },
                Err(e) => warn!("Failed to read {}: {}", path.display(), e),
            }
        }

        info!("✅ Discovered {} peers", peers.len());
        Ok(peers)
    }

    /// Cleanup announcement on shutdown
    ///
    /// # Errors
    ///
    /// Returns error string if capability file removal fails.
    pub async fn cleanup(&self) -> Result<(), String> {
        let discovery_dir = discovery_directory();
        let capability_file = discovery_dir.join(format!("{}.json", self.primal_id));

        if capability_file.exists() {
            fs::remove_file(&capability_file)
                .await
                .map_err(|e| format!("Failed to remove capability file: {e}"))?;

            info!("🧹 Cleaned up capability announcement");
        }

        Ok(())
    }
}

/// Query local system resources
///
/// Deep debt principle: Self-knowledge only!
#[must_use]
pub fn query_system_resources() -> SystemResources {
    let cpu_cores = std::thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(4);

    let mem = toadstool_sysmon::memory_info().unwrap_or(toadstool_sysmon::MemoryInfo {
        total: 0, available: 0, used: 0, swap_total: 0, swap_free: 0,
    });
    let total_memory = mem.total;
    let available_memory = mem.available;

    // Query GPU devices (self-knowledge)
    let gpu_devices = query_gpu_devices();

    // System architecture and OS
    let architecture = String::from(std::env::consts::ARCH);
    let os = String::from(std::env::consts::OS);

    SystemResources {
        cpu_cores,
        total_memory_bytes: total_memory,
        available_memory_bytes: available_memory,
        gpu_devices,
        architecture,
        os,
    }
}

/// Query GPU devices (self-knowledge)
///
/// Deep debt principle: No vendor lock-in, query all available GPUs
///
/// Query GPU devices (self-knowledge)
///
/// Detects GPUs via platform-specific mechanisms:
/// - Linux: /sys/class/drm for all GPUs, /proc/driver/nvidia for NVIDIA details
/// - macOS: System Profiler for Apple Silicon/discrete GPUs
///
/// **Design**: Vendor-agnostic, graceful degradation if no GPUs found
fn query_gpu_devices() -> Vec<GpuDevice> {
    let mut devices = Vec::new();
    let mut device_id = 0;

    #[cfg(target_os = "linux")]
    {
        // Check for NVIDIA GPUs via /proc/driver/nvidia
        if let Ok(entries) = std::fs::read_dir("/proc/driver/nvidia/gpus") {
            for entry in entries.flatten() {
                let gpu_path = entry.path();
                let pci_id = entry.file_name().to_string_lossy().to_string();

                // Try to read GPU info
                let info_path = gpu_path.join("information");
                let mut name = format!("NVIDIA GPU {device_id}");
                let mut memory_bytes = 0u64;

                if let Ok(info) = std::fs::read_to_string(&info_path) {
                    for line in info.lines() {
                        if line.starts_with("Model:") {
                            name = line.trim_start_matches("Model:").trim().to_string();
                        }
                    }
                }

                // Try to get memory from nvidia-smi
                if let Ok(output) = std::process::Command::new("nvidia-smi")
                    .args([
                        "--query-gpu=memory.total",
                        "--format=csv,noheader,nounits",
                        "-i",
                        &device_id.to_string(),
                    ])
                    .output()
                {
                    if output.status.success() {
                        if let Ok(mem_str) = String::from_utf8(output.stdout) {
                            if let Ok(mem_mb) = mem_str.trim().parse::<u64>() {
                                memory_bytes = mem_mb * 1024 * 1024;
                            }
                        }
                    }
                }

                devices.push(GpuDevice {
                    device_id,
                    name,
                    vendor: "nvidia".to_string(),
                    memory_bytes,
                    compute_capability: Some(pci_id),
                });
                device_id += 1;
            }
        }

        // Check for AMD/Intel GPUs via /sys/class/drm
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Only look at card* entries (skip renderD*)
                if !name.starts_with("card") || name.contains("render") {
                    continue;
                }

                let card_path = entry.path();
                let device_path = card_path.join("device");

                // Check vendor
                let vendor_path = device_path.join("vendor");
                if let Ok(vendor_id) = std::fs::read_to_string(&vendor_path) {
                    let vendor_id = vendor_id.trim();

                    // Skip if it's an NVIDIA card (already detected above)
                    if vendor_id == "0x10de" {
                        continue;
                    }

                    let vendor = match vendor_id {
                        "0x1002" => "amd",
                        "0x8086" => "intel",
                        _ => continue, // Unknown vendor, skip
                    };

                    // Try to get device name
                    let mut gpu_name = format!("{} GPU {}", vendor.to_uppercase(), device_id);

                    // Read uevent for more details
                    let uevent_path = device_path.join("uevent");
                    if let Ok(uevent) = std::fs::read_to_string(&uevent_path) {
                        for line in uevent.lines() {
                            if line.starts_with("PCI_ID=") {
                                let pci_id = line.trim_start_matches("PCI_ID=");
                                gpu_name = format!("{} GPU ({})", vendor.to_uppercase(), pci_id);
                            }
                        }
                    }

                    // Try to get memory (AMD exposes this in mem_info_vram_total)
                    let mut memory_bytes = 0u64;
                    let mem_path = device_path.join("mem_info_vram_total");
                    if let Ok(mem_str) = std::fs::read_to_string(&mem_path) {
                        if let Ok(mem) = mem_str.trim().parse::<u64>() {
                            memory_bytes = mem;
                        }
                    }

                    devices.push(GpuDevice {
                        device_id,
                        name: gpu_name,
                        vendor: vendor.to_string(),
                        memory_bytes,
                        compute_capability: None,
                    });
                    device_id += 1;
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, use system_profiler to detect GPUs
        if let Ok(output) = std::process::Command::new("system_profiler")
            .args(["SPDisplaysDataType", "-json"])
            .output()
        {
            if output.status.success() {
                if let Ok(json_str) = String::from_utf8(output.stdout) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        if let Some(displays) =
                            json.get("SPDisplaysDataType").and_then(|d| d.as_array())
                        {
                            for display in displays {
                                let name = display
                                    .get("sppci_model")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown GPU")
                                    .to_string();

                                let vendor = if name.contains("Apple")
                                    || name.contains("M1")
                                    || name.contains("M2")
                                    || name.contains("M3")
                                {
                                    "apple"
                                } else if name.contains("AMD") || name.contains("Radeon") {
                                    "amd"
                                } else if name.contains("Intel") {
                                    "intel"
                                } else if name.contains("NVIDIA") {
                                    "nvidia"
                                } else {
                                    "unknown"
                                };

                                // Try to get VRAM
                                let memory_bytes = display
                                    .get("sppci_vram")
                                    .and_then(|v| v.as_str())
                                    .and_then(|s| {
                                        // Parse strings like "8 GB" or "16384 MB"
                                        let parts: Vec<&str> = s.split_whitespace().collect();
                                        if parts.len() >= 2 {
                                            let num: u64 = parts[0].parse().ok()?;
                                            let unit = parts[1].to_uppercase();
                                            match unit.as_str() {
                                                "GB" => Some(num * 1024 * 1024 * 1024),
                                                "MB" => Some(num * 1024 * 1024),
                                                _ => None,
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(0);

                                devices.push(GpuDevice {
                                    device_id,
                                    name,
                                    vendor: vendor.to_string(),
                                    memory_bytes,
                                    compute_capability: None,
                                });
                                device_id += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // Graceful degradation: ToadStool works without GPU detection
    // GPU capabilities are optional enhancement, not required
    if !devices.is_empty() {
        info!("🎮 Detected {} GPU(s) via self-knowledge", devices.len());
        for device in &devices {
            info!(
                "   - {}: {} ({} MB)",
                device.vendor,
                device.name,
                device.memory_bytes / (1024 * 1024)
            );
        }
    }

    devices
}

/// Capability name constants (avoids repeated literal allocations)
pub(crate) const CAP_COMPUTE: &str = "compute";
pub(crate) const CAP_ORCHESTRATION: &str = "orchestration";
pub(crate) const CAP_JSON_RPC: &str = "jsonrpc";
pub(crate) const CAP_MEMORY_LARGE: &str = "memory-large";
pub(crate) const CAP_MEMORY_MEDIUM: &str = "memory-medium";
pub(crate) const CAP_MEMORY_SMALL: &str = "memory-small";

/// Build capabilities list from resources
///
/// Deep debt principle: Self-knowledge - report only what we have!
#[must_use]
pub fn build_capabilities(resources: &SystemResources) -> Vec<String> {
    let mut capabilities = vec![
        CAP_COMPUTE.to_string(),
        CAP_ORCHESTRATION.to_string(),
        CAP_JSON_RPC.to_string(),
    ];

    // Architecture capability
    capabilities.push(format!("arch-{}", resources.architecture));
    capabilities.push(format!("os-{}", resources.os));

    // CPU capabilities
    capabilities.push(format!("cpu-cores-{}", resources.cpu_cores));

    // Memory tiers
    let gb = resources.total_memory_bytes / (1024 * 1024 * 1024);
    if gb >= 64 {
        capabilities.push(CAP_MEMORY_LARGE.to_string());
    } else if gb >= 16 {
        capabilities.push(CAP_MEMORY_MEDIUM.to_string());
    } else {
        capabilities.push(CAP_MEMORY_SMALL.to_string());
    }

    // GPU capabilities
    for (i, gpu) in resources.gpu_devices.iter().enumerate() {
        capabilities.push(format!("gpu-{i}"));
        capabilities.push(format!("gpu-{}", gpu.vendor));
        capabilities.push(format!("gpu-{}-{}", gpu.vendor, gpu.name));
    }

    capabilities
}

/// Runtime base directory: XDG_RUNTIME_DIR or platform temp dir.
fn runtime_base_dir() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
}

/// Get discovery directory
///
/// Prefers XDG_RUNTIME_DIR, falls back to platform temp directory.
fn discovery_directory() -> PathBuf {
    runtime_base_dir().join("ecoPrimals").join("discovery")
}

/// Get default socket path for this primal
fn default_socket_path(primal_id: &str) -> PathBuf {
    runtime_base_dir()
        .join("ecoPrimals")
        .join("sockets")
        .join(format!("{primal_id}.sock"))
}

#[cfg(test)]
mod tests;
