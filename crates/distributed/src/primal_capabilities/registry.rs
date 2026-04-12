// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability Registry
//!
//! Maintains the registry of ToadStool's compute capabilities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::DistributedError;

/// A compute capability that ToadStool can provide
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capability {
    /// Capability identifier (e.g., "compute_gpu", "compute_heavy")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this capability provides
    pub description: String,
    /// Resource requirements
    pub resource_requirements: CapabilityResources,
    /// Tags for filtering
    pub tags: Vec<String>,
    /// Whether this capability is currently available
    pub available: bool,
    /// Confidence score (0.0-1.0) for this capability
    pub confidence: f64,
}

/// Resource requirements for a capability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityResources {
    /// Minimum CPU cores
    pub min_cpu_cores: u32,
    /// Minimum memory in MB
    pub min_memory_mb: u64,
    /// GPU required
    pub gpu_required: bool,
    /// GPU memory in MB (if GPU required)
    pub gpu_memory_mb: Option<u64>,
    /// Special hardware requirements
    pub special_hardware: Vec<String>,
}

impl Capability {
    /// Create a GPU compute capability
    pub fn compute_gpu() -> Self {
        Self {
            id: "compute_gpu".to_string(),
            name: "GPU Computing".to_string(),
            description:
                "GPU-accelerated computation (CUDA, Vulkan, WebGPU; OpenCL removed — S198)"
                    .to_string(),
            resource_requirements: CapabilityResources {
                min_cpu_cores: 2,
                min_memory_mb: 2048,
                gpu_required: true,
                gpu_memory_mb: Some(1024),
                special_hardware: vec!["nvidia_gpu".to_string(), "amd_gpu".to_string()],
            },
            tags: vec!["gpu".to_string(), "ml".to_string(), "training".to_string()],
            available: false, // Detected at runtime
            confidence: 0.95,
        }
    }

    /// Create a heavy compute capability
    pub fn compute_heavy() -> Self {
        Self {
            id: "compute_heavy".to_string(),
            name: "Heavy Computing".to_string(),
            description: "CPU-intensive computation with high resource availability".to_string(),
            resource_requirements: CapabilityResources {
                min_cpu_cores: 4,
                min_memory_mb: 4096,
                gpu_required: false,
                gpu_memory_mb: None,
                special_hardware: vec![],
            },
            tags: vec!["cpu".to_string(), "heavy".to_string()],
            available: true,
            confidence: 1.0,
        }
    }

    /// Create an ML training capability
    pub fn compute_ml_training() -> Self {
        Self {
            id: "compute_ml_training".to_string(),
            name: "ML Training".to_string(),
            description: "Machine learning model training with GPU support".to_string(),
            resource_requirements: CapabilityResources {
                min_cpu_cores: 4,
                min_memory_mb: 8192,
                gpu_required: true,
                gpu_memory_mb: Some(4096),
                special_hardware: vec!["cuda".to_string()],
            },
            tags: vec!["ml".to_string(), "training".to_string(), "gpu".to_string()],
            available: false, // Detected at runtime
            confidence: 0.9,
        }
    }

    /// Create a native execution capability
    pub fn compute_native() -> Self {
        Self {
            id: "compute_native".to_string(),
            name: "Native Execution".to_string(),
            description: "Direct native process execution".to_string(),
            resource_requirements: CapabilityResources {
                min_cpu_cores: 1,
                min_memory_mb: 512,
                gpu_required: false,
                gpu_memory_mb: None,
                special_hardware: vec![],
            },
            tags: vec!["native".to_string()],
            available: true,
            confidence: 1.0,
        }
    }

    /// Create a container execution capability
    pub fn compute_container() -> Self {
        Self {
            id: "compute_container".to_string(),
            name: "Container Execution".to_string(),
            description: "Docker/containerd execution".to_string(),
            resource_requirements: CapabilityResources {
                min_cpu_cores: 1,
                min_memory_mb: 1024,
                gpu_required: false,
                gpu_memory_mb: None,
                special_hardware: vec![],
            },
            tags: vec!["container".to_string(), "docker".to_string()],
            available: true,
            confidence: 1.0,
        }
    }

    /// Create a WASM execution capability
    pub fn compute_wasm() -> Self {
        Self {
            id: "compute_wasm".to_string(),
            name: "WebAssembly Execution".to_string(),
            description: "WebAssembly workload execution".to_string(),
            resource_requirements: CapabilityResources {
                min_cpu_cores: 1,
                min_memory_mb: 512,
                gpu_required: false,
                gpu_memory_mb: None,
                special_hardware: vec![],
            },
            tags: vec!["wasm".to_string(), "sandboxed".to_string()],
            available: true,
            confidence: 1.0,
        }
    }

    /// Create a mainframe capability (future - when legacy runtime is fixed)
    pub fn compute_mainframe() -> Self {
        Self {
            id: "compute_mainframe".to_string(),
            name: "Mainframe Computing".to_string(),
            description: "IBM System/360, z/OS, VAX/VMS execution".to_string(),
            resource_requirements: CapabilityResources {
                min_cpu_cores: 1,
                min_memory_mb: 512,
                gpu_required: false,
                gpu_memory_mb: None,
                special_hardware: vec!["mainframe_emulator".to_string()],
            },
            tags: vec!["legacy".to_string(), "mainframe".to_string()],
            available: false, // Will be true when legacy runtime is fixed
            confidence: 0.8,
        }
    }

    /// Create an embedded capability (future - when legacy runtime is fixed)
    pub fn compute_embedded() -> Self {
        Self {
            id: "compute_embedded".to_string(),
            name: "Embedded Systems".to_string(),
            description: "8/16-bit microcontroller, PLC, industrial control".to_string(),
            resource_requirements: CapabilityResources {
                min_cpu_cores: 1,
                min_memory_mb: 256,
                gpu_required: false,
                gpu_memory_mb: None,
                special_hardware: vec!["embedded_emulator".to_string()],
            },
            tags: vec![
                "legacy".to_string(),
                "embedded".to_string(),
                "industrial".to_string(),
            ],
            available: false, // Will be true when legacy runtime is fixed
            confidence: 0.8,
        }
    }
}

/// In-memory registry of [`Capability`] entries keyed by id.
pub struct CapabilityRegistry {
    capabilities: HashMap<String, Capability>,
}

impl CapabilityRegistry {
    /// Create a new registry with initial capabilities
    pub fn new(capabilities: Vec<Capability>) -> Self {
        let mut registry = HashMap::new();
        for cap in capabilities {
            registry.insert(cap.id.clone(), cap);
        }

        Self {
            capabilities: registry,
        }
    }

    /// Get all capabilities
    pub fn all_capabilities(&self) -> Vec<Capability> {
        self.capabilities.values().cloned().collect()
    }

    /// Get available capabilities only
    pub fn available_capabilities(&self) -> Vec<Capability> {
        self.capabilities
            .values()
            .filter(|cap| cap.available)
            .cloned()
            .collect()
    }

    /// Get a specific capability
    pub fn get_capability(&self, id: &str) -> Option<&Capability> {
        self.capabilities.get(id)
    }

    /// Update a capability's availability
    pub fn update_capability(
        &mut self,
        mut capability: Capability,
        available: bool,
    ) -> Result<(), DistributedError> {
        capability.available = available;
        self.capabilities.insert(capability.id.clone(), capability);
        Ok(())
    }

    /// Add a new capability
    pub fn add_capability(&mut self, capability: Capability) {
        self.capabilities.insert(capability.id.clone(), capability);
    }

    /// Remove a capability
    pub fn remove_capability(&mut self, id: &str) -> Option<Capability> {
        self.capabilities.remove(id)
    }

    /// Check if a capability is available
    pub fn is_available(&self, id: &str) -> bool {
        self.capabilities
            .get(id)
            .map(|cap| cap.available)
            .unwrap_or(false)
    }
}

/// Registration from an external provider (e.g. a Spring) that offers
/// a capability through a socket.
///
/// Part of the Spring-as-Provider pattern (ISSUE-007): Springs explicitly
/// register with toadStool at startup rather than relying only on the
/// filesystem convention `$XDG_RUNTIME_DIR/biomeos/{capability}.sock`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRegistration {
    /// Capability domain this provider serves (e.g. "biology", "ecology").
    pub capability: String,
    /// Socket path where the provider listens for JSON-RPC calls.
    pub socket_path: std::path::PathBuf,
    /// JSON-RPC methods the provider supports.
    pub methods: Vec<String>,
    /// Human-readable provider name (e.g. "wetSpring", "airSpring").
    pub provider_name: String,
    /// Provider version string for compatibility tracking.
    pub provider_version: String,
    /// Timestamp of registration (seconds since UNIX epoch).
    pub registered_at: u64,
}

/// Registry of external providers that have registered with toadStool.
///
/// Complements the filesystem-based socket discovery with explicit
/// registrations. When both exist, the explicit registration takes
/// precedence (it has richer metadata and liveness tracking).
pub struct ProviderRegistry {
    providers: HashMap<String, ProviderRegistration>,
}

impl ProviderRegistry {
    /// Creates an empty provider registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider for a capability domain.
    ///
    /// Replaces any existing provider for the same capability.
    pub fn register(&mut self, registration: ProviderRegistration) {
        self.providers
            .insert(registration.capability.clone(), registration);
    }

    /// Deregister a provider by capability domain.
    pub fn deregister(&mut self, capability: &str) -> Option<ProviderRegistration> {
        self.providers.remove(capability)
    }

    /// Look up the socket path for a capability.
    ///
    /// Returns the explicit registration's socket path if present,
    /// falling back to the filesystem convention via `primal_sockets`.
    #[must_use]
    pub fn resolve_socket(&self, capability: &str) -> std::path::PathBuf {
        if let Some(reg) = self.providers.get(capability) {
            return reg.socket_path.clone();
        }
        toadstool_common::primal_sockets::get_socket_path_for_capability(capability)
    }

    /// Whether a provider is explicitly registered for this capability.
    #[must_use]
    pub fn has_provider(&self, capability: &str) -> bool {
        self.providers.contains_key(capability)
    }

    /// Get a provider registration.
    #[must_use]
    pub fn get_provider(&self, capability: &str) -> Option<&ProviderRegistration> {
        self.providers.get(capability)
    }

    /// List all registered providers.
    #[must_use]
    pub fn all_providers(&self) -> Vec<&ProviderRegistration> {
        self.providers.values().collect()
    }

    /// Prune registrations whose socket files no longer exist.
    #[expect(clippy::needless_collect)] // Collect required: cannot mutate self.providers while iterating
    pub fn prune_stale(&mut self) -> Vec<ProviderRegistration> {
        let stale_keys: Vec<String> = self
            .providers
            .iter()
            .filter(|(_, reg)| !reg.socket_path.exists())
            .map(|(k, _)| k.clone())
            .collect();
        stale_keys
            .into_iter()
            .filter_map(|k| self.providers.remove(&k))
            .collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
