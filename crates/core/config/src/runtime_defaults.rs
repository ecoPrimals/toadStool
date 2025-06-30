// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Runtime configuration defaults
//!
//! This module provides centralized configuration defaults to eliminate
//! hardcoded values throughout the ToadStool codebase.

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Runtime engine default configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeDefaults {
    /// WASM runtime defaults
    pub wasm: WasmDefaults,
    /// Container runtime defaults
    pub container: ContainerDefaults,
    /// GPU runtime defaults
    pub gpu: GpuDefaults,
    /// Native runtime defaults
    pub native: NativeDefaults,
    /// Common runtime defaults
    pub common: CommonDefaults,
}

impl Default for RuntimeDefaults {
    fn default() -> Self {
        Self {
            wasm: WasmDefaults::default(),
            container: ContainerDefaults::default(),
            gpu: GpuDefaults::default(),
            native: NativeDefaults::default(),
            common: CommonDefaults::default(),
        }
    }
}

/// WASM runtime defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmDefaults {
    /// Default memory limit in bytes (128MB)
    pub memory_limit_bytes: u64,
    /// Default fuel limit for execution
    pub fuel_limit: u64,
    /// Default module cache size
    pub cache_size: usize,
    /// Default cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Default maximum module size in bytes
    pub max_module_size_bytes: u64,
    /// Default WASI capabilities
    pub default_wasi_capabilities: Vec<String>,
}

impl Default for WasmDefaults {
    fn default() -> Self {
        Self {
            memory_limit_bytes: 128 * 1024 * 1024, // 128MB
            fuel_limit: 100_000_000, // 100M fuel units
            cache_size: 100, // 100 cached modules
            cache_ttl_secs: 3600, // 1 hour
            max_module_size_bytes: 50 * 1024 * 1024, // 50MB
            default_wasi_capabilities: vec![
                "wasi:filesystem/preopens".to_string(),
                "wasi:clocks/wall-clock".to_string(),
                "wasi:io/streams".to_string(),
            ],
        }
    }
}

/// Container runtime defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDefaults {
    /// Default memory limit in bytes (512MB)
    pub memory_limit_bytes: u64,
    /// Default CPU limit (1.0 cores)
    pub cpu_limit_cores: f64,
    /// Default execution timeout in seconds
    pub execution_timeout_secs: u64,
    /// Default network mode
    pub default_network_mode: String,
    /// Default security options
    pub default_security_opts: Vec<String>,
    /// Default image pull timeout in seconds
    pub image_pull_timeout_secs: u64,
    /// Default container cleanup timeout in seconds
    pub cleanup_timeout_secs: u64,
}

impl Default for ContainerDefaults {
    fn default() -> Self {
        Self {
            memory_limit_bytes: 512 * 1024 * 1024, // 512MB
            cpu_limit_cores: 1.0, // 1 CPU core
            execution_timeout_secs: 300, // 5 minutes
            default_network_mode: "bridge".to_string(),
            default_security_opts: vec![
                "no-new-privileges:true".to_string(),
                "seccomp=runtime/default".to_string(),
            ],
            image_pull_timeout_secs: 600, // 10 minutes
            cleanup_timeout_secs: 30, // 30 seconds
        }
    }
}

/// GPU runtime defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDefaults {
    /// Default GPU memory limit in bytes (1GB)
    pub memory_limit_bytes: u64,
    /// Default device selection strategy
    pub device_selection_strategy: String,
    /// Default compute timeout in seconds
    pub compute_timeout_secs: u64,
    /// Default memory pool size in bytes
    pub memory_pool_size_bytes: u64,
    /// Default monitoring interval in milliseconds
    pub monitoring_interval_ms: u64,
    /// Default enabled frameworks
    pub enabled_frameworks: Vec<String>,
}

impl Default for GpuDefaults {
    fn default() -> Self {
        Self {
            memory_limit_bytes: 1024 * 1024 * 1024, // 1GB
            device_selection_strategy: "auto".to_string(),
            compute_timeout_secs: 600, // 10 minutes
            memory_pool_size_bytes: 256 * 1024 * 1024, // 256MB
            monitoring_interval_ms: 1000, // 1 second
            enabled_frameworks: vec![
                "opencl".to_string(),
                "cuda".to_string(),
            ],
        }
    }
}

/// Native runtime defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeDefaults {
    /// Default execution timeout in seconds
    pub execution_timeout_secs: u64,
    /// Default memory limit in bytes (1GB)
    pub memory_limit_bytes: u64,
    /// Default CPU limit (2.0 cores)
    pub cpu_limit_cores: f64,
    /// Default working directory
    pub default_working_dir: String,
    /// Default environment variables
    pub default_env_vars: Vec<(String, String)>,
    /// Default security capabilities to drop
    pub capabilities_to_drop: Vec<String>,
}

impl Default for NativeDefaults {
    fn default() -> Self {
        Self {
            execution_timeout_secs: 300, // 5 minutes
            memory_limit_bytes: 1024 * 1024 * 1024, // 1GB
            cpu_limit_cores: 2.0, // 2 CPU cores
            default_working_dir: "/tmp".to_string(),
            default_env_vars: vec![
                ("PATH".to_string(), "/usr/local/bin:/usr/bin:/bin".to_string()),
                ("LANG".to_string(), "C.UTF-8".to_string()),
            ],
            capabilities_to_drop: vec![
                "CAP_SYS_ADMIN".to_string(),
                "CAP_NET_ADMIN".to_string(),
                "CAP_SYS_MODULE".to_string(),
            ],
        }
    }
}

/// Common runtime defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDefaults {
    /// Default maximum concurrent executions
    pub max_concurrent_executions: usize,
    /// Default monitoring granularity in milliseconds
    pub monitoring_granularity_ms: u64,
    /// Default metrics retention period in seconds
    pub metrics_retention_secs: u64,
    /// Default log level
    pub log_level: String,
    /// Default cleanup interval in seconds
    pub cleanup_interval_secs: u64,
    /// Default health check interval in seconds
    pub health_check_interval_secs: u64,
}

impl Default for CommonDefaults {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 10,
            monitoring_granularity_ms: 100, // 100ms
            metrics_retention_secs: 3600, // 1 hour
            log_level: "info".to_string(),
            cleanup_interval_secs: 300, // 5 minutes
            health_check_interval_secs: 60, // 1 minute
        }
    }
}

/// Resource limit defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefaults {
    /// CPU defaults
    pub cpu: CpuDefaults,
    /// Memory defaults
    pub memory: MemoryDefaults,
    /// Storage defaults
    pub storage: StorageDefaults,
    /// Network defaults
    pub network: NetworkDefaults,
}

impl Default for ResourceDefaults {
    fn default() -> Self {
        Self {
            cpu: CpuDefaults::default(),
            memory: MemoryDefaults::default(),
            storage: StorageDefaults::default(),
            network: NetworkDefaults::default(),
        }
    }
}

/// CPU resource defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuDefaults {
    /// Default minimum CPU cores
    pub min_cores: f64,
    /// Default maximum CPU cores
    pub max_cores: f64,
    /// Default CPU architecture
    pub default_architecture: String,
    /// Default minimum frequency in MHz
    pub min_frequency_mhz: u32,
}

impl Default for CpuDefaults {
    fn default() -> Self {
        Self {
            min_cores: 0.1, // 100m cores
            max_cores: 16.0, // 16 cores
            default_architecture: "x86_64".to_string(),
            min_frequency_mhz: 1000, // 1GHz
        }
    }
}

/// Memory resource defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDefaults {
    /// Default minimum memory in bytes
    pub min_bytes: u64,
    /// Default maximum memory in bytes
    pub max_bytes: u64,
    /// Default swap allowance
    pub allow_swap: bool,
}

impl Default for MemoryDefaults {
    fn default() -> Self {
        Self {
            min_bytes: 64 * 1024 * 1024, // 64MB
            max_bytes: 8 * 1024 * 1024 * 1024, // 8GB
            allow_swap: false,
        }
    }
}

/// Storage resource defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDefaults {
    /// Default minimum storage in bytes
    pub min_bytes: u64,
    /// Default maximum storage in bytes
    pub max_bytes: u64,
    /// Default minimum IOPS
    pub min_iops: u32,
    /// Default minimum bandwidth in MB/s
    pub min_bandwidth_mbps: u32,
}

impl Default for StorageDefaults {
    fn default() -> Self {
        Self {
            min_bytes: 1024 * 1024, // 1MB
            max_bytes: 100 * 1024 * 1024 * 1024, // 100GB
            min_iops: 100,
            min_bandwidth_mbps: 10,
        }
    }
}

/// Network resource defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDefaults {
    /// Default minimum bandwidth in Mbps
    pub min_bandwidth_mbps: u32,
    /// Default maximum bandwidth in Mbps
    pub max_bandwidth_mbps: u32,
    /// Default maximum latency in milliseconds
    pub max_latency_ms: u32,
    /// Default internet access
    pub internet_access: bool,
    /// Default internal access
    pub internal_access: bool,
}

impl Default for NetworkDefaults {
    fn default() -> Self {
        Self {
            min_bandwidth_mbps: 1, // 1 Mbps
            max_bandwidth_mbps: 1000, // 1 Gbps
            max_latency_ms: 1000, // 1 second
            internet_access: true,
            internal_access: true,
        }
    }
}

/// Security defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityDefaults {
    /// Default isolation level
    pub default_isolation_level: String,
    /// Default network security policies
    pub default_network_policies: Vec<String>,
    /// Default filesystem policies
    pub default_filesystem_policies: Vec<String>,
    /// Default capability restrictions
    pub default_capability_restrictions: Vec<String>,
}

impl Default for SecurityDefaults {
    fn default() -> Self {
        Self {
            default_isolation_level: "sandbox".to_string(),
            default_network_policies: vec![
                "deny-all".to_string(),
                "allow-localhost".to_string(),
            ],
            default_filesystem_policies: vec![
                "read-only-root".to_string(),
                "no-dev-access".to_string(),
                "no-proc-access".to_string(),
            ],
            default_capability_restrictions: vec![
                "no-privileged".to_string(),
                "no-sys-admin".to_string(),
                "no-net-admin".to_string(),
            ],
        }
    }
}

/// Monitoring defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringDefaults {
    /// Default monitoring enabled
    pub enabled: bool,
    /// Default metrics collection interval in milliseconds
    pub collection_interval_ms: u64,
    /// Default metrics retention period in seconds
    pub retention_period_secs: u64,
    /// Default performance profiling enabled
    pub profiling_enabled: bool,
    /// Default memory tracking enabled
    pub memory_tracking_enabled: bool,
    /// Default power monitoring enabled
    pub power_monitoring_enabled: bool,
}

impl Default for MonitoringDefaults {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval_ms: 1000, // 1 second
            retention_period_secs: 3600, // 1 hour
            profiling_enabled: false,
            memory_tracking_enabled: true,
            power_monitoring_enabled: false,
        }
    }
}

/// Get runtime defaults from configuration or use built-in defaults
pub fn get_runtime_defaults() -> RuntimeDefaults {
    // In the future, this could load from configuration files
    RuntimeDefaults::default()
}

/// Get resource defaults from configuration or use built-in defaults
pub fn get_resource_defaults() -> ResourceDefaults {
    // In the future, this could load from configuration files
    ResourceDefaults::default()
}

/// Get security defaults from configuration or use built-in defaults
pub fn get_security_defaults() -> SecurityDefaults {
    // In the future, this could load from configuration files
    SecurityDefaults::default()
}

/// Get monitoring defaults from configuration or use built-in defaults
pub fn get_monitoring_defaults() -> MonitoringDefaults {
    // In the future, this could load from configuration files
    MonitoringDefaults::default()
}

/// Convert duration from seconds to Duration
pub fn duration_from_secs(secs: u64) -> Duration {
    Duration::from_secs(secs)
}

/// Convert duration from milliseconds to Duration
pub fn duration_from_millis(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_defaults() {
        let defaults = RuntimeDefaults::default();
        assert_eq!(defaults.wasm.memory_limit_bytes, 128 * 1024 * 1024);
        assert_eq!(defaults.container.cpu_limit_cores, 1.0);
        assert_eq!(defaults.gpu.device_selection_strategy, "auto");
        assert_eq!(defaults.native.cpu_limit_cores, 2.0);
        assert_eq!(defaults.common.max_concurrent_executions, 10);
    }

    #[test]
    fn test_resource_defaults() {
        let defaults = ResourceDefaults::default();
        assert_eq!(defaults.cpu.min_cores, 0.1);
        assert_eq!(defaults.memory.min_bytes, 64 * 1024 * 1024);
        assert_eq!(defaults.storage.min_iops, 100);
        assert_eq!(defaults.network.min_bandwidth_mbps, 1);
    }

    #[test]
    fn test_security_defaults() {
        let defaults = SecurityDefaults::default();
        assert_eq!(defaults.default_isolation_level, "sandbox");
        assert!(!defaults.default_network_policies.is_empty());
        assert!(!defaults.default_filesystem_policies.is_empty());
    }

    #[test]
    fn test_monitoring_defaults() {
        let defaults = MonitoringDefaults::default();
        assert!(defaults.enabled);
        assert_eq!(defaults.collection_interval_ms, 1000);
        assert!(defaults.memory_tracking_enabled);
    }

    #[test]
    fn test_duration_helpers() {
        assert_eq!(duration_from_secs(60), Duration::from_secs(60));
        assert_eq!(duration_from_millis(1000), Duration::from_millis(1000));
    }
} 