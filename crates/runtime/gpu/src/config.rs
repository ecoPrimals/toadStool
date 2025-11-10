//! Configuration structures for Universal GPU Compute Runtime
//!
//! This module provides configuration types for the GPU runtime engine.
//! Key configurations use base config patterns for consistency:
//!
//! - **ExecutionConfig**: Uses `RetryConfig` for retry policies
//! - **FaultToleranceConfig**: Uses `HealthCheckConfig` for device health monitoring
//!
//! # Example
//!
//! ```rust
//! use toadstool_runtime_gpu::config::UniversalGpuConfig;
//!
//! let config = UniversalGpuConfig::default();
//! // Retry configuration is now using base RetryConfig
//! assert!(config.execution.retry_enabled);
//! assert_eq!(config.execution.retries.max_retries, 3);
//! ```

use std::time::Duration;

use serde::{Deserialize, Serialize};
use toadstool_common::config_bases::{HealthCheckConfig, RetryConfig};

use crate::types::GpuFramework;

/// Configuration for the universal GPU runtime
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UniversalGpuConfig {
    /// GPU hardware auto-discovery settings
    pub discovery: GpuDiscoveryConfig,
    /// Resource management settings
    pub resources: ResourceConfig,
    /// Kernel compilation settings
    pub compilation: CompilationConfig,
    /// Execution settings
    pub execution: ExecutionConfig,
    /// Monitoring and telemetry settings
    pub monitoring: MonitoringConfig,
    /// Recursive execution settings
    pub recursion: RecursionConfig,
}

/// GPU hardware auto-discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDiscoveryConfig {
    /// Frameworks to attempt discovery for
    pub enabled_frameworks: Vec<GpuFramework>,
    /// Discovery timeout
    pub discovery_timeout: Duration,
    /// Automatic fallback on failures
    pub auto_fallback: bool,
    /// Minimum device requirements
    pub min_requirements: super::types::DeviceRequirements,
}

impl Default for GpuDiscoveryConfig {
    fn default() -> Self {
        use super::types::GpuFramework;
        Self {
            enabled_frameworks: vec![
                GpuFramework::WebGpu,        // Universal, future-ready
                GpuFramework::Vulkan,        // Cross-platform
                GpuFramework::OpenCl,        // Widely supported
                GpuFramework::Cuda,          // NVIDIA performance
                GpuFramework::Metal,         // Apple optimization
                GpuFramework::Rocm,          // AMD optimization
                GpuFramework::DirectCompute, // Windows optimization
            ],
            discovery_timeout: Duration::from_secs(10),
            auto_fallback: true,
            min_requirements: super::types::DeviceRequirements::minimal(),
        }
    }
}

/// Resource management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Maximum memory usage per device (percentage)
    pub max_memory_usage_percent: f32,
    /// Memory allocation strategy
    pub allocation_strategy: AllocationStrategy,
    /// Device selection strategy
    pub device_selection: DeviceSelectionStrategy,
    /// Load balancing settings
    pub load_balancing: LoadBalancingConfig,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max_memory_usage_percent: 80.0,
            allocation_strategy: AllocationStrategy::Adaptive,
            device_selection: DeviceSelectionStrategy::Optimal,
            load_balancing: LoadBalancingConfig::default(),
        }
    }
}

/// Kernel compilation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationConfig {
    /// Optimization level
    pub optimization_level: OptimizationLevel,
    /// Target architectures to optimize for
    pub target_architectures: Vec<String>,
    /// Enable just-in-time compilation
    pub jit_enabled: bool,
    /// Kernel caching settings
    pub caching: CachingConfig,
    /// Universal IR settings
    pub universal_ir: UniversalIrConfig,
}

impl Default for CompilationConfig {
    fn default() -> Self {
        Self {
            optimization_level: OptimizationLevel::Adaptive,
            target_architectures: vec!["universal".to_string()],
            jit_enabled: true,
            caching: CachingConfig::default(),
            universal_ir: UniversalIrConfig::default(),
        }
    }
}

/// Execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum execution time per kernel
    pub max_execution_time: Duration,
    /// Enable automatic retries for transient failures
    pub retry_enabled: bool,
    /// Retry configuration (max attempts, backoff, jitter)
    #[serde(flatten)]
    pub retries: RetryConfig,
    /// Fault tolerance settings
    pub fault_tolerance: FaultToleranceConfig,
    /// Asynchronous execution settings
    pub async_execution: AsyncExecutionConfig,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_secs(300), // 5 minutes
            retry_enabled: true,
            retries: RetryConfig::default(),
            fault_tolerance: FaultToleranceConfig::default(),
            async_execution: AsyncExecutionConfig::default(),
        }
    }
}

/// Monitoring and telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable performance profiling
    pub profiling_enabled: bool,
    /// Enable memory tracking
    pub memory_tracking: bool,
    /// Enable power monitoring
    pub power_monitoring: bool,
    /// Monitoring interval
    pub monitoring_interval: Duration,
    /// Metrics retention period
    pub metrics_retention: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            profiling_enabled: true,
            memory_tracking: true,
            power_monitoring: false, // May not be available on all platforms
            monitoring_interval: Duration::from_secs(1),
            metrics_retention: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Recursive execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursionConfig {
    /// Enable recursive GPU-on-GPU execution
    pub recursive_enabled: bool,
    /// Maximum recursion depth
    pub max_recursion_depth: u32,
    /// Resource allocation for recursive jobs
    pub recursive_resource_allocation: f32,
    /// Recursive job scheduling strategy
    pub recursive_scheduling: RecursiveSchedulingStrategy,
}

impl Default for RecursionConfig {
    fn default() -> Self {
        Self {
            recursive_enabled: true,
            max_recursion_depth: 10,
            recursive_resource_allocation: 0.5, // 50% of device resources
            recursive_scheduling: RecursiveSchedulingStrategy::Cooperative,
        }
    }
}

/// Memory allocation strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllocationStrategy {
    /// Allocate memory on-demand
    OnDemand,
    /// Pre-allocate memory pools
    Pooled,
    /// Use adaptive allocation based on usage patterns
    Adaptive,
    /// Use unified memory where available
    Unified,
}

/// Device selection strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceSelectionStrategy {
    /// Automatically select optimal device
    Optimal,
    /// Round-robin across available devices
    RoundRobin,
    /// Select device with most available memory
    MaxMemory,
    /// Select device with highest compute capability
    MaxCompute,
    /// Load balance across devices
    LoadBalance,
    /// Use specific device
    Specific(super::types::DeviceId),
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// Enable automatic load balancing
    pub enabled: bool,
    /// Load balancing algorithm
    pub algorithm: LoadBalancingAlgorithm,
    /// Rebalancing interval
    pub rebalance_interval: Duration,
    /// Load threshold for rebalancing
    pub load_threshold: f32,
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: LoadBalancingAlgorithm::WeightedRoundRobin,
            rebalance_interval: Duration::from_secs(30),
            load_threshold: 0.8, // 80% utilization
        }
    }
}

/// Load balancing algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    /// Simple round-robin
    RoundRobin,
    /// Weighted round-robin based on device capabilities
    WeightedRoundRobin,
    /// Least connections
    LeastConnections,
    /// Least utilization
    LeastUtilization,
    /// Consistent hashing
    ConsistentHashing,
}

/// Kernel optimization levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    /// No optimization (fastest compilation)
    None,
    /// Basic optimizations
    Basic,
    /// Adaptive optimization based on device characteristics
    Adaptive,
    /// Aggressive optimization (slower compilation, best performance)
    Aggressive,
}

/// Kernel caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingConfig {
    /// Enable kernel caching
    pub enabled: bool,
    /// Cache size limit in MB
    pub cache_size_mb: u64,
    /// Cache TTL
    pub cache_ttl: Duration,
    /// Cache storage location
    pub cache_path: Option<String>,
}

impl Default for CachingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_size_mb: 1024,                   // 1GB
            cache_ttl: Duration::from_secs(86400), // 24 hours
            cache_path: None,                      // Use system temp directory
        }
    }
}

/// Universal IR configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalIrConfig {
    /// Enable universal IR compilation
    pub enabled: bool,
    /// Target IR format
    pub target_format: UniversalIrFormat,
    /// IR optimization level
    pub optimization_level: OptimizationLevel,
}

impl Default for UniversalIrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target_format: UniversalIrFormat::Spirv,
            optimization_level: OptimizationLevel::Adaptive,
        }
    }
}

/// Universal IR formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UniversalIrFormat {
    /// SPIR-V (Khronos standard)
    Spirv,
    /// LLVM IR
    Llvm,
    /// WebAssembly
    Wasm,
    /// Custom IR
    Custom(String),
}

// Retry policy is now using base RetryConfig
// See ExecutionConfig.retries field for retry configuration

/// Fault tolerance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultToleranceConfig {
    /// Enable automatic failover to other devices
    pub auto_failover: bool,
    /// Enable checkpointing for long-running kernels
    pub checkpointing_enabled: bool,
    /// Checkpoint interval
    pub checkpoint_interval: Duration,
    /// Health check configuration (interval, timeout, thresholds)
    #[serde(flatten)]
    pub health_check: HealthCheckConfig,
}

impl Default for FaultToleranceConfig {
    fn default() -> Self {
        Self {
            auto_failover: true,
            checkpointing_enabled: false, // Disabled by default due to overhead
            checkpoint_interval: Duration::from_secs(60),
            health_check: HealthCheckConfig::default(),
        }
    }
}

/// Asynchronous execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncExecutionConfig {
    /// Enable asynchronous execution
    pub enabled: bool,
    /// Maximum concurrent executions per device
    pub max_concurrent_per_device: u32,
    /// Queue size for pending executions
    pub queue_size: u32,
    /// Priority scheduling enabled
    pub priority_scheduling: bool,
}

impl Default for AsyncExecutionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_per_device: 4,
            queue_size: 64,
            priority_scheduling: true,
        }
    }
}

/// Recursive scheduling strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecursiveSchedulingStrategy {
    /// Cooperative scheduling (yield resources)
    Cooperative,
    /// Preemptive scheduling
    Preemptive,
    /// Isolated scheduling (separate resource pools)
    Isolated,
}
