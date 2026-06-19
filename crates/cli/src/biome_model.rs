// SPDX-License-Identifier: AGPL-3.0-or-later
//! Biome manifest schema and runtime status types shared by the CLI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Biome manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeManifest {
    /// Biome metadata
    pub metadata: BiomeMetadata,

    /// Primal configurations
    pub primals: HashMap<String, PrimalConfig>,

    /// Service definitions
    pub services: HashMap<String, ServiceConfig>,

    /// Resource requirements
    pub resources: BiomeResources,

    /// Security policies
    pub security: BiomeSecurity,

    /// Network configuration
    pub networking: BiomeNetworking,

    /// Storage configuration
    pub storage: BiomeStorage,
}

/// Biome manifest metadata (name, version, timestamps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMetadata {
    /// Biome display name
    pub name: String,
    /// Semantic version string
    pub version: String,
    /// Optional human-readable description
    pub description: Option<String>,
    /// Optional author or maintainer
    pub author: Option<String>,
    /// Creation timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created: std::time::SystemTime,
    /// Last update timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub updated: std::time::SystemTime,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Configuration for a primal (workload) in the biome manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    /// Primal version
    pub version: String,
    /// Where to load the workload from (container, wasm, git, etc.)
    pub source: WorkloadSource,
    /// Whether the primal is enabled
    pub enabled: bool,
    /// Arbitrary config key-value pairs
    pub config: HashMap<String, serde_yaml_ng::Value>,
    /// Names of other primals or services this depends on
    pub dependencies: Vec<String>,
    /// Optional health check configuration
    pub health_check: Option<HealthCheck>,
}

impl PrimalConfig {
    /// Returns true if this config declares the given capability.
    ///
    /// Checks the manifest `capabilities` config field (comma-separated labels).
    pub fn has_capability(&self, capability: &str) -> bool {
        let target = capability.to_ascii_lowercase();
        let Some(serde_yaml_ng::Value::String(caps)) = self.config.get("capabilities") else {
            return false;
        };
        caps.split(',')
            .map(str::trim)
            .any(|declared| capability_labels_match(declared, &target))
    }
}

fn capability_labels_match(declared: &str, target: &str) -> bool {
    let declared_lower = declared.to_ascii_lowercase();
    if declared_lower == target {
        return true;
    }
    use toadstool_common::interned_strings::CapabilityDomain;
    CapabilityDomain::from_label(&declared_lower).is_some_and(|domain| domain.as_str() == target)
}

impl BiomeManifest {
    /// Returns true if any primal entry provides the given capability.
    pub fn has_primal_with_capability(&self, capability: &str) -> bool {
        self.primals
            .iter()
            .any(|(name, config)| Self::entry_provides_capability(name, config, capability))
    }

    /// Find the first manifest primal that provides the given capability.
    pub fn find_primal_with_capability(&self, capability: &str) -> Option<(&str, &PrimalConfig)> {
        self.primals.iter().find_map(|(name, config)| {
            Self::entry_provides_capability(name, config, capability)
                .then_some((name.as_str(), config))
        })
    }

    fn entry_provides_capability(name: &str, config: &PrimalConfig, capability: &str) -> bool {
        if config.has_capability(capability) {
            return true;
        }
        let target = capability.to_ascii_lowercase();
        use toadstool_common::interned_strings::CapabilityDomain;
        if CapabilityDomain::from_label(name).is_some_and(|domain| domain.as_str() == target) {
            return true;
        }
        let name_lower = name.to_ascii_lowercase();
        name_lower.contains(&target)
            || (target == "crypto"
                && (name_lower.contains("pki") || name_lower.contains("security")))
    }
}

/// Configuration for a service in the biome manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service version
    pub version: String,
    /// Where to load the service workload from
    pub source: WorkloadSource,
    /// Number of replicas (None = 1)
    pub replicas: Option<u32>,
    /// CPU, memory, storage limits
    pub resources: ServiceResources,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Port mappings
    pub ports: Vec<ServicePort>,
    /// Volume mounts
    pub volumes: Vec<ServiceVolume>,
    /// Names of services this depends on
    pub dependencies: Vec<String>,
    /// Optional health check configuration
    pub health_check: Option<HealthCheck>,
}

/// Source type for loading a workload (container, wasm, git, ipfs, local)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkloadSource {
    /// OCI container registry
    Container {
        /// Registry host (e.g. docker.io)
        registry: String,
        /// Image name
        image: String,
        /// Image tag
        tag: String,
        /// Optional digest for pinning
        digest: Option<String>,
    },
    /// WebAssembly module
    Wasm {
        /// URL or path to the WASM module
        source: String,
        /// SHA256 or similar checksum for verification
        checksum: String,
        /// Optional WASI runtime config
        wasi_config: Option<HashMap<String, serde_yaml_ng::Value>>,
    },
    /// Git repository
    Git {
        /// Repository URL
        repository: String,
        /// Branch to checkout
        branch: Option<String>,
        /// Commit or tag to pin
        commit: Option<String>,
        /// Subpath within the repo
        path: Option<String>,
    },
    /// IPFS content
    Ipfs {
        /// IPFS CID
        hash: String,
        /// Gateway URL for fetching
        gateway: Option<String>,
    },
    /// Local file path
    Local {
        /// Path to the workload file or directory
        path: PathBuf,
    },
}

/// Resource limits for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeResources {
    /// CPU cores limit
    pub cpu_limit: Option<f64>,
    /// Memory limit (e.g. "512Mi")
    pub memory_limit: Option<String>,
    /// Storage limit (e.g. "10Gi")
    pub storage_limit: Option<String>,
    /// GPU count limit
    pub gpu_limit: Option<u32>,
    /// Network bandwidth limit
    pub network_bandwidth: Option<String>,
}

/// Security policies for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeSecurity {
    /// Isolation level (e.g. process, container)
    pub isolation_level: String,
    /// Trust level for workload execution
    pub trust_level: String,
    /// Whether a security/crypto service is required
    #[serde(alias = "beardog_required")]
    pub security_required: bool,
    /// Crypto policy names
    pub crypto_policies: Vec<String>,
    /// Allowed network CIDRs
    pub allowed_networks: Vec<String>,
    /// Syscalls to forbid
    pub forbidden_syscalls: Vec<String>,
}

/// Network configuration for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeNetworking {
    /// Network mode (bridge, host, none)
    pub mode: String,
    /// DNS server addresses
    pub dns_servers: Vec<String>,
    /// Host-to-container port mappings
    pub port_mappings: Vec<PortMapping>,
    /// Network policy names
    pub network_policies: Vec<String>,
}

/// Storage configuration for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStorage {
    /// Storage service integration version or config
    #[serde(alias = "nestgate_integration")]
    pub storage_integration: Option<String>,
    /// Dataset definitions
    pub datasets: Vec<DatasetConfig>,
    /// Volume definitions
    pub volumes: Vec<VolumeConfig>,
    /// Backup policy name or config
    pub backup_policy: Option<String>,
}

/// Resource limits for a single service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResources {
    /// CPU cores limit
    pub cpu_limit: Option<f64>,
    /// Memory limit (e.g. "256Mi")
    pub memory_limit: Option<String>,
    /// Storage limit
    pub storage_limit: Option<String>,
}

/// Port mapping for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePort {
    /// Port inside the container
    pub container_port: u16,
    /// Host port (None = same as container)
    pub host_port: Option<u16>,
    /// Protocol (tcp, udp)
    pub protocol: String,
}

/// Volume mount for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceVolume {
    /// Host path or volume name
    pub source: String,
    /// Mount path inside the container
    pub target: String,
    /// Whether the mount is read-only
    pub read_only: bool,
}

/// Health check configuration for a service or primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Command and args to run (e.g. `["curl", "-f", "http://localhost/health"]`)
    pub command: Vec<String>,
    /// Interval between checks in seconds
    pub interval: u64,
    /// Timeout per check in seconds
    pub timeout: u64,
    /// Consecutive failures before unhealthy
    pub retries: u32,
    /// Grace period before first check in seconds
    pub start_period: u64,
}

/// Host-to-container port mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Port on the host
    pub host_port: u16,
    /// Port in the container
    pub container_port: u16,
    /// Protocol (tcp, udp)
    pub protocol: String,
}

/// Dataset configuration for storage service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    /// Dataset name
    pub name: String,
    /// Size limit (e.g. "10Gi")
    pub size: Option<String>,
    /// Compression algorithm
    pub compression: Option<String>,
    /// Whether to encrypt at rest
    pub encryption: bool,
}

/// Volume driver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    /// Volume name
    pub name: String,
    /// Storage driver (e.g. local, nfs)
    pub driver: String,
    /// Driver-specific options
    pub options: HashMap<String, String>,
}

/// Running biome information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeInfo {
    /// Unique biome instance ID
    pub id: Uuid,
    /// Biome name from manifest
    pub name: String,
    /// Current lifecycle status
    pub status: BiomeStatus,
    /// When the biome was created
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created: std::time::SystemTime,
    /// When the biome was started (if running)
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "toadstool_common::system_time_serde::opt"
    )]
    pub started: Option<std::time::SystemTime>,
    /// Path to the biome manifest file
    pub manifest_path: PathBuf,
    /// Current CPU, memory, network usage
    pub resource_usage: ResourceUsage,
    /// Status of each service
    pub services: Vec<ServiceInfo>,
}

/// Lifecycle status of a biome instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiomeStatus {
    /// Biome is bootstrapping
    Starting,
    /// Biome is running
    Running,
    /// Biome is shutting down
    Stopping,
    /// Biome has stopped
    Stopped,
    /// Biome failed with error message
    Error(String),
    /// Biome is being migrated
    Migrating,
}

/// Current resource usage for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU utilization percentage
    pub cpu_percent: f64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Storage usage in bytes
    pub storage_bytes: u64,
    /// Network bytes received
    pub network_rx_bytes: u64,
    /// Network bytes transmitted
    pub network_tx_bytes: u64,
}

/// Status of a single service within a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Current status (running, stopped, etc.)
    pub status: String,
    /// Number of replicas
    pub replicas: u32,
    /// Exposed port numbers
    pub ports: Vec<u16>,
    /// Health status (healthy, unhealthy, unknown)
    pub health: String,
}
