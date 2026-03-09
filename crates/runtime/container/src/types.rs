// SPDX-License-Identifier: AGPL-3.0-only
//! Container Runtime Configuration Types
//!
//! Configuration structs and enums for the container runtime engine

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use toadstool::workload::{PortMapping, RegistryAuth, VolumeMount};

#[cfg(feature = "docker")]
use bollard::API_DEFAULT_VERSION;

/// Container runtime engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRuntimeConfig {
    /// Container engine selection
    pub engine: ContainerEngineType,
    /// Registry configuration for image pulling
    pub registry_config: RegistryConfig,
    /// Network policies and configuration
    pub network_policy: NetworkPolicy,
    /// Volume mounting policies
    pub volume_policy: VolumePolicy,
    /// Security configuration
    pub security_config: ContainerSecurityConfig,
    /// Resource limits
    pub resource_limits: ContainerResourceLimits,
    /// Image management settings
    pub image_config: ImageConfig,
}

impl Default for ContainerRuntimeConfig {
    fn default() -> Self {
        Self {
            engine: ContainerEngineType::Docker {
                socket_path: None,
                #[cfg(feature = "docker")]
                api_version: API_DEFAULT_VERSION.to_string(),
                #[cfg(not(feature = "docker"))]
                api_version: "v1.43".to_string(),
            },
            registry_config: RegistryConfig::default(),
            network_policy: NetworkPolicy::default(),
            volume_policy: VolumePolicy::default(),
            security_config: ContainerSecurityConfig::default(),
            resource_limits: ContainerResourceLimits::default(),
            image_config: ImageConfig::default(),
        }
    }
}

/// Container engine type selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerEngineType {
    /// Docker engine with custom socket path
    Docker {
        /// Docker socket path
        socket_path: Option<String>,
        /// API version
        api_version: String,
    },
    /// Containerd engine
    Containerd {
        /// Containerd socket address
        address: String,
        /// Namespace for containers
        namespace: String,
    },
    /// Podman engine
    Podman {
        /// Podman socket path
        socket_path: String,
        /// Remote connection URL
        remote_url: Option<String>,
    },
}

impl Default for ContainerEngineType {
    fn default() -> Self {
        Self::Docker {
            socket_path: None,
            #[cfg(feature = "docker")]
            api_version: API_DEFAULT_VERSION.to_string(),
            #[cfg(not(feature = "docker"))]
            api_version: "v1.43".to_string(),
        }
    }
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Default registry URL
    pub default_registry: String,
    /// Registry authentication configurations
    pub registries: HashMap<String, RegistryAuth>,
    /// Image pull policy
    pub pull_policy: ImagePullPolicy,
    /// Pull timeout
    pub pull_timeout: Duration,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            default_registry: "docker.io".to_string(),
            registries: HashMap::new(),
            pull_policy: ImagePullPolicy::IfNotPresent,
            pull_timeout: Duration::from_secs(300),
        }
    }
}

/// Image pull policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImagePullPolicy {
    /// Always pull the image
    Always,
    /// Pull if not present locally
    IfNotPresent,
    /// Never pull, use local only
    Never,
}

/// Network policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Default network mode
    pub default_network: NetworkMode,
    /// Allow custom networks
    pub allow_custom_networks: bool,
    /// Allowed port ranges
    pub allowed_port_ranges: Vec<PortRange>,
    /// DNS configuration
    pub dns_config: DnsConfig,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            default_network: NetworkMode::Bridge,
            allow_custom_networks: false,
            allowed_port_ranges: vec![
                PortRange {
                    start: 8000,
                    end: 8999,
                },
                PortRange {
                    start: 3000,
                    end: 3999,
                },
            ],
            dns_config: DnsConfig::default(),
        }
    }
}

/// Network mode for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMode {
    /// Bridge networking
    Bridge,
    /// Host networking
    Host,
    /// No networking
    None,
    /// Custom network
    Custom(String),
}

/// Port range specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

/// DNS configuration.
///
/// Defaults to all-empty: containers inherit the host DNS configuration.
/// Operators supply explicit `nameservers` when network isolation is required.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS servers
    pub nameservers: Vec<String>,
    /// Search domains
    pub search_domains: Vec<String>,
    /// DNS options
    pub options: Vec<String>,
}

/// Volume policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumePolicy {
    /// Allow bind mounts
    pub allow_bind_mounts: bool,
    /// Allowed host paths for bind mounts
    pub allowed_host_paths: Vec<PathBuf>,
    /// Allow tmpfs mounts
    pub allow_tmpfs: bool,
    /// Maximum volume size in MB
    pub max_volume_size_mb: u64,
}

impl Default for VolumePolicy {
    fn default() -> Self {
        Self {
            allow_bind_mounts: false,
            allowed_host_paths: vec![PathBuf::from("/tmp")],
            allow_tmpfs: true,
            max_volume_size_mb: 1024, // 1 GB
        }
    }
}

/// Container security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSecurityConfig {
    /// Run as non-root user
    pub non_root_required: bool,
    /// Drop all capabilities by default
    pub drop_all_capabilities: bool,
    /// Allowed capabilities
    pub allowed_capabilities: Vec<String>,
    /// Security options
    pub security_opts: Vec<String>,
    /// Read-only root filesystem
    pub read_only_root_fs: bool,
}

impl Default for ContainerSecurityConfig {
    fn default() -> Self {
        Self {
            non_root_required: true,
            drop_all_capabilities: true,
            allowed_capabilities: Vec::new(),
            security_opts: vec!["no-new-privileges:true".to_string()],
            read_only_root_fs: false,
        }
    }
}

/// Container resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResourceLimits {
    /// Maximum memory in bytes
    pub max_memory_bytes: u64,
    /// Maximum CPU cores (as millicores)
    pub max_cpu_millicores: u32,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Maximum disk I/O bytes per second
    pub max_io_bps: u64,
}

impl Default for ContainerResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024,           // 512 MB
            max_cpu_millicores: 1000,                      // 1 CPU core
            max_execution_time: Duration::from_secs(3600), // 1 hour
            max_io_bps: 100 * 1024 * 1024,                 // 100 MB/s
        }
    }
}

/// Image configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Enable image caching
    pub cache_enabled: bool,
    /// Image cache directory
    pub cache_dir: Option<PathBuf>,
    /// Maximum cache size in MB
    pub max_cache_size_mb: u64,
    /// Cache cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_dir: None,
            max_cache_size_mb: 5120,                     // 5 GB
            cleanup_interval: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Type alias for container resource limits
pub type ContainerResources = ContainerResourceLimits;

/// Type alias for container security configuration
pub type ContainerSecurity = ContainerSecurityConfig;

/// Configuration for a single container execution
#[derive(Debug, Clone, Default)]
pub struct ContainerExecutionConfig {
    /// Container image to run
    pub image: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Working directory inside the container
    pub working_dir: Option<String>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Resource limits for this execution
    pub resources: ContainerResources,
    /// Security settings for this execution
    pub security: ContainerSecurity,
    /// Registry authentication for private images
    pub registry_auth: Option<RegistryAuth>,
}
