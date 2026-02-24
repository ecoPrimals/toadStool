//! Runtime execution configuration
//!
//! This module contains configuration types for workload execution including:
//! - General runtime settings (timeouts, concurrency)
//! - Resource limits (CPU, memory, disk, network)
//! - Container runtime (Docker/Podman)
//! - WASM runtime (Wasmtime/WASI)
//! - Python runtime (`PyO3`)
//! - GPU compute (CUDA/OpenCL)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::app;

/// Runtime configuration
///
/// Controls workload execution settings across all runtime engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Maximum execution timeout for workloads
    pub execution_timeout: Duration,

    /// Maximum number of concurrent executions
    pub max_concurrent_executions: u32,

    /// Resource limits for all workloads
    pub resource_limits: ResourceLimits,

    /// Container runtime settings
    pub container: ContainerConfig,

    /// WASM runtime settings
    pub wasm: WasmConfig,

    /// Python runtime settings
    pub python: PythonConfig,

    /// GPU compute settings (optional)
    pub gpu: Option<GpuConfig>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            execution_timeout: Duration::from_secs(app::DEFAULT_EXECUTION_TIMEOUT_SECS),
            max_concurrent_executions: app::DEFAULT_MAX_CONCURRENT_EXECUTIONS,
            resource_limits: ResourceLimits::default(),
            container: ContainerConfig::default(),
            wasm: WasmConfig::default(),
            python: PythonConfig::default(),
            gpu: None,
        }
    }
}

/// Resource limits configuration
///
/// Defines resource constraints for workload execution to prevent
/// resource exhaustion and ensure fair scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU usage percentage (0.0 - 100.0)
    pub max_cpu_usage: f64,

    /// Maximum memory usage percentage (0.0 - 100.0)
    pub max_memory_usage: f64,

    /// Maximum disk usage percentage (0.0 - 100.0)
    pub max_disk_usage: f64,

    /// Maximum network bandwidth in bytes per second
    pub max_network_bandwidth: u64,

    /// Maximum number of open file descriptors
    pub max_open_files: u64,

    /// Maximum number of processes
    pub max_processes: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_usage: app::DEFAULT_MAX_CPU_USAGE,
            max_memory_usage: app::DEFAULT_MAX_MEMORY_USAGE,
            max_disk_usage: app::DEFAULT_MAX_DISK_USAGE,
            max_network_bandwidth: 1024 * 1024 * 1024, // 1 GB/s
            max_open_files: 1024,
            max_processes: 100,
        }
    }
}

/// Container runtime configuration
///
/// Settings for Docker/Podman container execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container runtime backend (docker, podman, containerd)
    pub runtime: String,

    /// Default image registry URL
    pub default_registry: String,

    /// Port range for container port mapping (start, end)
    pub port_range: (u16, u16),

    /// Network mode (bridge, host, none)
    pub network_mode: String,

    /// Security options (e.g., no-new-privileges)
    pub security_opts: Vec<String>,

    /// Volume mounts (host:container)
    pub volume_mounts: Vec<String>,

    /// Environment variables to pass to containers
    pub environment: HashMap<String, String>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            runtime: "docker".to_string(),
            default_registry: "docker.io".to_string(),
            port_range: crate::config_utils::ConfigUtils::get_container_port_range(),
            network_mode: "bridge".to_string(),
            security_opts: vec!["no-new-privileges".to_string()],
            volume_mounts: vec![],
            environment: HashMap::new(),
        }
    }
}

/// WASM runtime configuration
///
/// Settings for WebAssembly execution via Wasmtime with WASI support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    /// WASM runtime engine (wasmtime, wasmer, wasmtime-jit)
    pub engine: String,

    /// Maximum memory size in bytes
    pub max_memory: u64,

    /// Maximum execution time in seconds
    pub max_execution_time: u64,

    /// Enable WASI (WebAssembly System Interface)
    pub enable_wasi: bool,

    /// WASI allowed directories for file access
    pub wasi_allowed_dirs: Vec<String>,

    /// WASI environment variables
    pub wasi_env: HashMap<String, String>,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            engine: "wasmtime".to_string(),
            max_memory: 64 * 1024 * 1024, // 64MB
            max_execution_time: 300,      // 5 minutes
            enable_wasi: true,
            wasi_allowed_dirs: vec!["/tmp".to_string()],
            wasi_env: HashMap::new(),
        }
    }
}

/// Python runtime configuration
///
/// Settings for Python code execution via `PyO3`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonConfig {
    /// Python executable path
    pub executable: String,

    /// Virtual environment path (optional)
    pub venv_path: Option<String>,

    /// Package index URL; must be explicitly configured (sovereignty: no external service defaults).
    pub index_url: String,

    /// Maximum memory size in bytes
    pub max_memory: u64,

    /// Maximum execution time in seconds
    pub max_execution_time: u64,

    /// Allowed modules (allowlist)
    pub allowed_modules: Vec<String>,

    /// Restricted modules (blocklist)
    pub restricted_modules: Vec<String>,
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            executable: "python3".to_string(),
            venv_path: None,
            index_url: String::new(),
            max_memory: 128 * 1024 * 1024, // 128MB
            max_execution_time: 300,       // 5 minutes
            allowed_modules: vec!["numpy".to_string(), "pandas".to_string()],
            restricted_modules: vec!["os".to_string(), "subprocess".to_string()],
        }
    }
}

/// GPU compute configuration
///
/// Settings for GPU-accelerated workloads via CUDA/OpenCL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// GPU runtime backend (cuda, opencl, metal, vulkan)
    pub runtime: String,

    /// GPU device IDs to use (e.g., [0, 1] for first two GPUs)
    pub device_ids: Vec<u32>,

    /// Maximum memory usage per device in bytes
    pub max_memory_per_device: u64,

    /// Maximum execution time in seconds
    pub max_execution_time: u64,

    /// Enable profiling and metrics collection
    pub enable_profiling: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            runtime: "cuda".to_string(),
            device_ids: vec![0],
            max_memory_per_device: 2 * 1024 * 1024 * 1024, // 2GB
            max_execution_time: 300,                       // 5 minutes
            enable_profiling: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_runtime_config() {
        let config = RuntimeConfig::default();
        assert!(config.execution_timeout.as_secs() > 0);
        assert!(config.max_concurrent_executions > 0);
    }

    #[test]
    fn test_resource_limits_defaults() {
        let limits = ResourceLimits::default();
        assert!(limits.max_cpu_usage > 0.0 && limits.max_cpu_usage <= 100.0);
        assert!(limits.max_memory_usage > 0.0 && limits.max_memory_usage <= 100.0);
    }

    #[test]
    fn test_container_config_defaults() {
        let config = ContainerConfig::default();
        assert_eq!(config.runtime, "docker");
        assert!(config
            .security_opts
            .contains(&"no-new-privileges".to_string()));
    }

    #[test]
    fn test_wasm_config_defaults() {
        let config = WasmConfig::default();
        assert_eq!(config.engine, "wasmtime");
        assert!(config.enable_wasi);
        assert!(config.max_memory > 0);
    }

    #[test]
    fn test_python_config_defaults() {
        let config = PythonConfig::default();
        assert_eq!(config.executable, "python3");
        assert!(!config.allowed_modules.is_empty());
    }

    #[test]
    fn test_gpu_config_defaults() {
        let config = GpuConfig::default();
        assert_eq!(config.runtime, "cuda");
        assert!(!config.device_ids.is_empty());
    }
}
