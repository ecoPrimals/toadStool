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

mod gpu;
mod paths;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Primal capabilities and self-knowledge
///
/// Deep debt principle: Self-knowledge only!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalCapabilities {
    /// Unique primal ID (generated at startup)
    pub primal_id: String,
    /// Primal type (e.g., "toadstool", "security", "coordination")
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
    /// Number of CPU cores.
    pub cpu_cores: usize,
    /// Total memory in bytes.
    pub total_memory_bytes: u64,
    /// Available memory in bytes.
    pub available_memory_bytes: u64,
    /// List of GPU devices.
    pub gpu_devices: Vec<GpuDevice>,
    /// System architecture (e.g. x86_64).
    pub architecture: String,
    /// Operating system (e.g. linux).
    pub os: String,
}

/// GPU device information (self-knowledge)
///
/// Fields `render_node`, `driver`, and `arch` are populated on Linux from
/// DRM sysfs and enable the visualization service's `GpuContext::from_descriptor(vendor, arch, driver)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    /// Device index.
    pub device_id: usize,
    /// GPU model name.
    pub name: String,
    /// Vendor (e.g. nvidia, amd, intel, apple).
    pub vendor: String,
    /// VRAM in bytes.
    pub memory_bytes: u64,
    /// Compute capability string (e.g. CUDA sm_86).
    pub compute_capability: Option<String>,
    /// DRM render node path, e.g. `/dev/dri/renderD128`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_node: Option<String>,
    /// Kernel driver name, e.g. `amdgpu`, `nvidia`, `nouveau`, `i915`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    /// GPU micro-architecture, e.g. `rdna2`, `sm_86`, `xe_lpg`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
}

impl PrimalCapabilities {
    /// Discover self (self-knowledge!)
    ///
    /// Deep debt principle: Query local system only!
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // API consistency; may add async discovery in future
    pub async fn discover_self(primal_type: &str) -> Self {
        info!("🔍 Discovering self capabilities (self-knowledge!)");

        let primal_id = Uuid::new_v4().to_string();
        let resources = query_system_resources();
        let capabilities = build_capabilities(&resources);
        let socket_path = paths::default_socket_path(&primal_id);

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
    /// - Also writes to ecoPrimals root for ecosystem-wide discovery
    /// - Peers can read it to discover us
    /// - No centralized registry!
    ///
    /// # Errors
    ///
    /// Returns error string if directory creation, serialization, or file write fails.
    pub fn announce(&self) -> Result<(), String> {
        let discovery_dir = paths::discovery_directory();
        let eco_root = paths::ecoprimals_root_directory();

        fs::create_dir_all(&discovery_dir)
            .map_err(|e| format!("Failed to create discovery directory: {e}"))?;
        fs::create_dir_all(&eco_root)
            .map_err(|e| format!("Failed to create ecoPrimals root: {e}"))?;

        let json = serde_json::to_string_pretty(&self)
            .map_err(|e| format!("Failed to serialize capabilities: {e}"))?;

        let filename = format!("{}.json", self.primal_id);

        let canonical = discovery_dir.join(&filename);
        fs::write(&canonical, &json)
            .map_err(|e| format!("Failed to write capability file: {e}"))?;

        let compat = eco_root.join(&filename);
        fs::write(&compat, &json)
            .map_err(|e| format!("Failed to write compat capability file: {e}"))?;

        info!("📢 Announced capabilities: {}", canonical.display());
        info!("📢 ecoPrimals root entry:   {}", compat.display());

        Ok(())
    }

    /// Find peer with specific capability
    ///
    /// Deep debt principle: Runtime discovery!
    ///
    /// # Errors
    ///
    /// Returns error string if discovery directory read fails or no peer with the capability is found.
    pub fn find_peer_with(capability: &str) -> Result<Self, String> {
        Self::find_peer_with_in(capability, &paths::discovery_directory())
    }

    /// Find peer with specific capability in a given discovery directory.
    ///
    /// Testable variant that avoids global env var mutation.
    ///
    /// # Errors
    ///
    /// Returns error string if directory read fails, file parse fails, or no peer with the capability is found.
    pub fn find_peer_with_in(
        capability: &str,
        discovery_dir: &std::path::Path,
    ) -> Result<Self, String> {
        debug!("🔍 Searching for peer with capability: {}", capability);

        // Read all capability files
        let entries = fs::read_dir(discovery_dir)
            .map_err(|e| format!("Failed to read discovery directory: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Read peer capability file
            let json = fs::read_to_string(&path)
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
    pub fn find_all_peers() -> Result<Vec<Self>, String> {
        Self::find_all_peers_in(&paths::discovery_directory())
    }

    /// Find all peers in a given discovery directory.
    ///
    /// Testable variant that avoids global env var mutation.
    ///
    /// # Errors
    ///
    /// Returns error string if discovery directory read fails.
    pub fn find_all_peers_in(discovery_dir: &std::path::Path) -> Result<Vec<Self>, String> {
        debug!("🔍 Discovering all peers");
        let mut peers = Vec::new();

        // Read all capability files
        let entries = fs::read_dir(discovery_dir)
            .map_err(|e| format!("Failed to read discovery directory: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Read peer capability file
            match fs::read_to_string(&path) {
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

    /// Cleanup announcement on shutdown (removes both canonical and compat entries)
    ///
    /// # Errors
    ///
    /// Returns error string if capability file removal fails.
    pub fn cleanup(&self) -> Result<(), String> {
        let filename = format!("{}.json", self.primal_id);

        let canonical = paths::discovery_directory().join(&filename);
        if canonical.exists() {
            fs::remove_file(&canonical)
                .map_err(|e| format!("Failed to remove capability file: {e}"))?;
        }

        let compat = paths::ecoprimals_root_directory().join(&filename);
        if compat.exists() {
            fs::remove_file(&compat)
                .map_err(|e| format!("Failed to remove compat capability file: {e}"))?;
        }

        info!("🧹 Cleaned up capability announcements");
        Ok(())
    }
}

/// Query local system resources
///
/// Deep debt principle: Self-knowledge only!
#[must_use]
pub fn query_system_resources() -> SystemResources {
    let cpu_cores = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);

    let mem = toadstool_sysmon::memory_info().unwrap_or(toadstool_sysmon::MemoryInfo {
        total: 0,
        available: 0,
        used: 0,
        swap_total: 0,
        swap_free: 0,
    });
    let total_memory = mem.total;
    let available_memory = mem.available;

    let gpu_devices = gpu::query_gpu_devices();

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

/// Capability name constants (avoids repeated literal allocations)
pub(crate) const CAP_COMPUTE: &str = "compute";
pub(crate) const CAP_ORCHESTRATION: &str = "orchestration";
pub(crate) const CAP_JSON_RPC: &str = "jsonrpc";
pub(crate) const CAP_MEMORY_LARGE: &str = "memory-large";
pub(crate) const CAP_MEMORY_MEDIUM: &str = "memory-medium";
pub(crate) const CAP_MEMORY_SMALL: &str = "memory-small";
pub(crate) const CAP_GPU_DISPATCH: &str = "gpu.dispatch";
pub(crate) const CAP_SCIENCE_GPU_DISPATCH: &str = "science.gpu.dispatch";

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
    if !resources.gpu_devices.is_empty() {
        capabilities.push(CAP_GPU_DISPATCH.to_string());
        capabilities.push(CAP_SCIENCE_GPU_DISPATCH.to_string());
    }
    for (i, gpu) in resources.gpu_devices.iter().enumerate() {
        capabilities.push(format!("gpu-{i}"));
        capabilities.push(format!("gpu-{}", gpu.vendor));
        capabilities.push(format!("gpu-{}-{}", gpu.vendor, gpu.name));
    }

    capabilities
}

#[cfg(test)]
mod tests;
