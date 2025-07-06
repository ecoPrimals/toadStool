//! Resource management and monitoring
//!
//! This module defines resource requirements, limits, metrics, and monitoring
//! interfaces for workload execution.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Resource requirements and limits for workload execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceRequirements {
    /// CPU requirements
    pub cpu: CpuRequirements,
    /// Memory requirements
    pub memory: MemoryRequirements,
    /// Storage requirements
    pub storage: StorageRequirements,
    /// Network requirements
    pub network: NetworkRequirements,
    /// GPU requirements (if applicable)
    pub gpu: Option<GpuRequirements>,
    /// Custom resource requirements
    pub custom: HashMap<String, serde_json::Value>,
}

/// CPU resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    /// Minimum CPU cores (fractional allowed)
    pub min_cores: f64,
    /// Maximum CPU cores (None = unlimited)
    pub max_cores: Option<f64>,
    /// CPU architecture preference
    pub architecture: Option<String>,
    /// Minimum CPU frequency in MHz
    pub min_frequency_mhz: Option<u32>,
    /// Required CPU features
    pub required_features: Vec<String>,
}

impl Default for CpuRequirements {
    fn default() -> Self {
        Self {
            min_cores: 0.1, // 100m cores
            max_cores: Some(2.0),
            architecture: None,
            min_frequency_mhz: None,
            required_features: Vec::new(),
        }
    }
}

/// Memory resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    /// Minimum memory in bytes
    pub min_bytes: u64,
    /// Maximum memory in bytes (None = unlimited)
    pub max_bytes: Option<u64>,
    /// Memory type preference
    pub memory_type: Option<MemoryType>,
    /// Swap allowance
    pub allow_swap: bool,
}

impl Default for MemoryRequirements {
    fn default() -> Self {
        Self {
            min_bytes: 64 * 1024 * 1024,         // 64 MB
            max_bytes: Some(1024 * 1024 * 1024), // 1 GB
            memory_type: None,
            allow_swap: false,
        }
    }
}

/// Memory types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    /// Standard system RAM
    Ram,
    /// High bandwidth memory
    Hbm,
    /// Persistent memory
    Persistent,
    /// GPU memory
    GpuMemory,
}

/// Storage resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Minimum storage space in bytes
    pub min_bytes: u64,
    /// Maximum storage space in bytes (None = unlimited)
    pub max_bytes: Option<u64>,
    /// Storage type preference
    pub storage_type: Option<StorageType>,
    /// IOPS requirements
    pub min_iops: Option<u32>,
    /// Bandwidth requirements in MB/s
    pub min_bandwidth_mbps: Option<u32>,
}

impl Default for StorageRequirements {
    fn default() -> Self {
        Self {
            min_bytes: 100 * 1024 * 1024,             // 100 MB
            max_bytes: Some(10 * 1024 * 1024 * 1024), // 10 GB
            storage_type: None,
            min_iops: None,
            min_bandwidth_mbps: None,
        }
    }
}

/// Storage types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    /// Solid state drive
    Ssd,
    /// Hard disk drive
    Hdd,
    /// Network attached storage
    Network,
    /// In-memory storage
    Memory,
    /// Object storage
    Object,
}

/// Network resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    /// Minimum bandwidth in Mbps
    pub min_bandwidth_mbps: Option<u32>,
    /// Maximum bandwidth in Mbps (None = unlimited)
    pub max_bandwidth_mbps: Option<u32>,
    /// Latency requirements in milliseconds
    pub max_latency_ms: Option<u32>,
    /// Required network access
    pub internet_access: bool,
    /// Internal network access
    pub internal_access: bool,
}

impl Default for NetworkRequirements {
    fn default() -> Self {
        Self {
            min_bandwidth_mbps: None,
            max_bandwidth_mbps: None,
            max_latency_ms: None,
            internet_access: false,
            internal_access: true,
        }
    }
}

/// GPU resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Minimum GPU memory in MB
    pub min_memory_mb: u64,
    /// GPU compute capability
    pub min_compute_capability: Option<String>,
    /// Required GPU count
    pub min_gpu_count: u32,
    /// GPU vendor preference
    pub vendor_preference: Option<GpuVendor>,
    /// CUDA support required
    pub requires_cuda: bool,
    /// OpenCL support required
    pub requires_opencl: bool,
}

/// GPU vendors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuVendor {
    /// NVIDIA GPUs
    Nvidia,
    /// AMD GPUs
    Amd,
    /// Intel GPUs
    Intel,
    /// Apple GPUs
    Apple,
}

/// Runtime metrics and performance data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeMetrics {
    /// CPU usage metrics
    pub cpu: CpuMetrics,
    /// Memory usage metrics
    pub memory: MemoryMetrics,
    /// Storage usage metrics
    pub storage: StorageMetrics,
    /// Network usage metrics
    pub network: NetworkMetrics,
    /// GPU usage metrics (if applicable)
    pub gpu: Option<GpuMetrics>,
    /// Execution timing metrics
    pub timing: TimingMetrics,
    /// Custom metrics
    pub custom: HashMap<String, serde_json::Value>,
}

/// CPU usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    /// Current CPU usage percentage (0-100)
    pub usage_percent: f64,
    /// Peak CPU usage percentage
    pub peak_usage_percent: f64,
    /// Average CPU usage percentage
    pub average_usage_percent: f64,
    /// CPU time used in milliseconds
    pub cpu_time_ms: u64,
    /// CPU cycles used
    pub cpu_cycles: Option<u64>,
    /// CPU throttling events
    pub throttle_events: u32,
}

impl Default for CpuMetrics {
    fn default() -> Self {
        Self {
            usage_percent: 0.0,
            peak_usage_percent: 0.0,
            average_usage_percent: 0.0,
            cpu_time_ms: 0,
            cpu_cycles: None,
            throttle_events: 0,
        }
    }
}

/// Memory usage metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryMetrics {
    /// Current memory usage in bytes
    pub usage_bytes: u64,
    /// Peak memory usage in bytes
    pub peak_usage_bytes: u64,
    /// Average memory usage in bytes
    pub average_usage_bytes: u64,
    /// Memory allocation count
    pub allocation_count: u64,
    /// Memory deallocation count
    pub deallocation_count: u64,
    /// Page faults
    pub page_faults: u64,
    /// Swap usage in bytes
    pub swap_usage_bytes: u64,
}

/// Storage usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    /// Bytes read from storage
    pub bytes_read: u64,
    /// Bytes written to storage  
    pub bytes_written: u64,
    /// Read operations count
    pub read_ops: u64,
    /// Write operations count
    pub write_ops: u64,
    /// Read IOPS
    pub read_iops: f64,
    /// Write IOPS
    pub write_iops: f64,
    /// Average read latency in microseconds
    pub avg_read_latency_us: f64,
    /// Average write latency in microseconds
    pub avg_write_latency_us: f64,
}

impl Default for StorageMetrics {
    fn default() -> Self {
        Self {
            bytes_read: 0,
            bytes_written: 0,
            read_ops: 0,
            write_ops: 0,
            read_iops: 0.0,
            write_iops: 0.0,
            avg_read_latency_us: 0.0,
            avg_write_latency_us: 0.0,
        }
    }
}

/// Network usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Bytes received
    pub bytes_received: u64,
    /// Bytes transmitted
    pub bytes_transmitted: u64,
    /// Packets received
    pub packets_received: u64,
    /// Packets transmitted
    pub packets_transmitted: u64,
    /// Network errors
    pub errors: u64,
    /// Network drops
    pub drops: u64,
    /// Average latency in microseconds
    pub avg_latency_us: f64,
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self {
            bytes_received: 0,
            bytes_transmitted: 0,
            packets_received: 0,
            packets_transmitted: 0,
            errors: 0,
            drops: 0,
            avg_latency_us: 0.0,
        }
    }
}

/// GPU usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    /// GPU utilization percentage (0-100)
    pub utilization_percent: f64,
    /// GPU memory usage in bytes
    pub memory_usage_bytes: u64,
    /// GPU memory utilization percentage
    pub memory_utilization_percent: f64,
    /// GPU temperature in Celsius
    pub temperature_celsius: f64,
    /// GPU power usage in watts
    pub power_usage_watts: f64,
    /// GPU compute operations
    pub compute_ops: u64,
    /// GPU memory operations
    pub memory_ops: u64,
}

impl Default for GpuMetrics {
    fn default() -> Self {
        Self {
            utilization_percent: 0.0,
            memory_usage_bytes: 0,
            memory_utilization_percent: 0.0,
            temperature_celsius: 0.0,
            power_usage_watts: 0.0,
            compute_ops: 0,
            memory_ops: 0,
        }
    }
}

/// Execution timing metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingMetrics {
    /// Execution start time
    pub start_time: DateTime<Utc>,
    /// Execution end time (None if still running)
    pub end_time: Option<DateTime<Utc>>,
    /// Total execution duration
    pub duration: Duration,
    /// Initialization time
    pub init_duration: Duration,
    /// Cleanup time
    pub cleanup_duration: Duration,
    /// Queue wait time
    pub queue_wait_duration: Duration,
}

impl Default for TimingMetrics {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            start_time: now,
            end_time: None,
            duration: Duration::from_secs(0),
            init_duration: Duration::from_secs(0),
            cleanup_duration: Duration::from_secs(0),
            queue_wait_duration: Duration::from_secs(0),
        }
    }
}

/// Resource monitoring interface
pub trait ResourceMonitor: Send + Sync + std::fmt::Debug {
    /// Start monitoring a workload
    fn start_monitoring(&self, workload_id: &str) -> crate::error::ToadStoolResult<()>;

    /// Stop monitoring a workload
    fn stop_monitoring(&self, workload_id: &str) -> crate::error::ToadStoolResult<()>;

    /// Get current metrics for a workload
    fn get_metrics(&self, workload_id: &str) -> crate::error::ToadStoolResult<RuntimeMetrics>;

    /// Check if resource limits are exceeded
    fn check_limits(
        &self,
        workload_id: &str,
        requirements: &ResourceRequirements,
    ) -> crate::error::ToadStoolResult<bool>;
}
