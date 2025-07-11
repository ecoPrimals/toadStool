//! Resource management and monitoring for ToadStool

use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::future::Future;

use crate::{ToadStoolResult};

/// Resource requirements specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU requirements
    pub cpu: CpuRequirements,
    /// Memory requirements  
    pub memory: MemoryRequirements,
    /// Storage requirements
    pub storage: StorageRequirements,
    /// GPU requirements (optional)
    pub gpu: Option<GpuRequirements>,
    /// Network requirements
    pub network: NetworkRequirements,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu: CpuRequirements::default(),
            memory: MemoryRequirements::default(),
            storage: StorageRequirements::default(),
            gpu: None,
            network: NetworkRequirements::default(),
        }
    }
}

/// CPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    /// Minimum CPU cores
    pub min_cores: f64,
    /// Maximum CPU cores
    pub max_cores: Option<f64>,
    /// CPU architecture requirement
    pub architecture: Option<String>,
}

impl Default for CpuRequirements {
    fn default() -> Self {
        Self {
            min_cores: 1.0,
            max_cores: None,
            architecture: None,
        }
    }
}

/// Memory requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    /// Minimum memory in bytes
    pub min_bytes: u64,
    /// Maximum memory in bytes
    pub max_bytes: Option<u64>,
}

impl Default for MemoryRequirements {
    fn default() -> Self {
        Self {
            min_bytes: 1024 * 1024 * 1024, // 1GB
            max_bytes: None,
        }
    }
}

/// Storage requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Minimum storage in bytes
    pub min_bytes: u64,
    /// Maximum storage in bytes
    pub max_bytes: Option<u64>,
    /// Storage type requirement
    pub storage_type: Option<String>,
}

impl Default for StorageRequirements {
    fn default() -> Self {
        Self {
            min_bytes: 1024 * 1024 * 1024, // 1GB
            max_bytes: None,
            storage_type: None,
        }
    }
}

/// Network requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    /// Minimum bandwidth in bytes per second
    pub min_bandwidth: Option<u64>,
    /// Maximum bandwidth in bytes per second
    pub max_bandwidth: Option<u64>,
    /// Latency requirement in milliseconds
    pub max_latency_ms: Option<u64>,
}

impl Default for NetworkRequirements {
    fn default() -> Self {
        Self {
            min_bandwidth: None,
            max_bandwidth: None,
            max_latency_ms: None,
        }
    }
}

/// GPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Minimum GPU units
    pub min_units: u32,
    /// Maximum GPU units
    pub max_units: Option<u32>,
    /// GPU type requirement
    pub gpu_type: Option<String>,
    /// Minimum GPU memory in bytes
    pub min_memory_bytes: Option<u64>,
}

/// Runtime metrics collected during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    /// CPU metrics
    pub cpu: CpuMetrics,
    /// Memory metrics
    pub memory: MemoryMetrics,
    /// Storage metrics
    pub storage: StorageMetrics,
    /// Network metrics
    pub network: NetworkMetrics,
    /// GPU metrics
    pub gpu: Option<GpuMetrics>,
    /// Timing metrics
    pub timing: TimingMetrics,
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self {
            cpu: CpuMetrics::default(),
            memory: MemoryMetrics::default(),
            storage: StorageMetrics::default(),
            network: NetworkMetrics::default(),
            gpu: None,
            timing: TimingMetrics::default(),
        }
    }
}

/// CPU metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    /// CPU usage percentage
    pub usage_percent: f64,
    /// CPU cores used
    pub cores_used: f64,
    /// CPU time in seconds
    pub cpu_time_seconds: f64,
}

impl Default for CpuMetrics {
    fn default() -> Self {
        Self {
            usage_percent: 0.0,
            cores_used: 0.0,
            cpu_time_seconds: 0.0,
        }
    }
}

/// Memory metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// Memory usage percentage
    pub usage_percent: f64,
    /// Memory used in bytes
    pub used_bytes: u64,
    /// Peak memory usage in bytes
    pub peak_bytes: u64,
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self {
            usage_percent: 0.0,
            used_bytes: 0,
            peak_bytes: 0,
        }
    }
}

/// Storage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    /// Storage usage percentage
    pub usage_percent: f64,
    /// Storage used in bytes
    pub used_bytes: u64,
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
}

impl Default for StorageMetrics {
    fn default() -> Self {
        Self {
            usage_percent: 0.0,
            used_bytes: 0,
            bytes_read: 0,
            bytes_written: 0,
        }
    }
}

/// Network metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
        }
    }
}

/// GPU metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    /// GPU usage percentage
    pub usage_percent: f64,
    /// GPU memory usage percentage
    pub memory_usage_percent: f64,
    /// GPU memory used in bytes
    pub memory_used_bytes: u64,
    /// GPU temperature in Celsius
    pub temperature_celsius: Option<f64>,
}

impl Default for GpuMetrics {
    fn default() -> Self {
        Self {
            usage_percent: 0.0,
            memory_usage_percent: 0.0,
            memory_used_bytes: 0,
            temperature_celsius: None,
        }
    }
}

/// Timing metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingMetrics {
    /// Execution start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Execution end time
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Total execution duration
    pub duration: chrono::Duration,
}

impl Default for TimingMetrics {
    fn default() -> Self {
        Self {
            start_time: chrono::Utc::now(),
            end_time: None,
            duration: chrono::Duration::seconds(0),
        }
    }
}

/// Resource limits for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU limits
    pub cpu_limits: CpuLimits,
    /// Memory limits
    pub memory_limits: MemoryLimits,
    /// Storage limits
    pub storage_limits: StorageLimits,
    /// Network limits
    pub network_limits: NetworkLimits,
    /// Execution timeout
    pub execution_timeout: Option<chrono::Duration>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limits: CpuLimits::default(),
            memory_limits: MemoryLimits::default(),
            storage_limits: StorageLimits::default(),
            network_limits: NetworkLimits::default(),
            execution_timeout: Some(chrono::Duration::seconds(300)),
        }
    }
}

/// CPU limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuLimits {
    /// Maximum CPU cores
    pub max_cores: Option<f64>,
    /// CPU throttling
    pub throttle_percent: Option<f64>,
}

impl Default for CpuLimits {
    fn default() -> Self {
        Self {
            max_cores: None,
            throttle_percent: None,
        }
    }
}

/// Memory limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    /// Maximum memory in bytes
    pub max_bytes: Option<u64>,
    /// Memory swap limit
    pub swap_limit_bytes: Option<u64>,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_bytes: None,
            swap_limit_bytes: None,
        }
    }
}

/// Storage limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLimits {
    /// Maximum storage in bytes
    pub max_bytes: Option<u64>,
    /// Read bandwidth limit
    pub read_bandwidth_limit: Option<u64>,
    /// Write bandwidth limit
    pub write_bandwidth_limit: Option<u64>,
}

impl Default for StorageLimits {
    fn default() -> Self {
        Self {
            max_bytes: None,
            read_bandwidth_limit: None,
            write_bandwidth_limit: None,
        }
    }
}

/// Network limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLimits {
    /// Maximum bandwidth in bytes per second
    pub max_bandwidth: Option<u64>,
    /// Connection limits
    pub max_connections: Option<u32>,
}

impl Default for NetworkLimits {
    fn default() -> Self {
        Self {
            max_bandwidth: None,
            max_connections: None,
        }
    }
}

/// Resource monitor trait
pub trait ResourceMonitor: Send + Sync {
    /// Start monitoring resources for a workload
    fn start_monitoring(&self, workload_id: &str) -> ToadStoolResult<()>;
    
    /// Stop monitoring resources for a workload
    fn stop_monitoring(&self, workload_id: &str) -> ToadStoolResult<()>;
    
    /// Get current resource metrics
    fn get_metrics(&self, workload_id: &str) -> ToadStoolResult<RuntimeMetrics>;
    
    /// Get system resource availability
    fn get_system_resources(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<SystemResources>> + Send + '_>>;
}

/// System resources available
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    /// Available CPU cores
    pub available_cpu_cores: f64,
    /// Available memory in bytes
    pub available_memory_bytes: u64,
    /// Available storage in bytes
    pub available_storage_bytes: u64,
    /// Available network bandwidth
    pub available_network_bandwidth: Option<u64>,
    /// Available GPU units
    pub available_gpu_units: u32,
}

impl Default for SystemResources {
    fn default() -> Self {
        Self {
            available_cpu_cores: 1.0,
            available_memory_bytes: 1024 * 1024 * 1024, // 1GB
            available_storage_bytes: 1024 * 1024 * 1024, // 1GB
            available_network_bandwidth: None,
            available_gpu_units: 0,
        }
    }
}
