// SPDX-License-Identifier: AGPL-3.0-only
//! Resource type definitions for ToadStool

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use crate::ToadStoolResult;

/// Resource requirements specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkRequirements {
    /// Minimum bandwidth in bytes per second
    pub min_bandwidth: Option<u64>,
    /// Maximum bandwidth in bytes per second
    pub max_bandwidth: Option<u64>,
    /// Latency requirement in milliseconds
    pub max_latency_ms: Option<u64>,
}

impl ResourceRequirements {
    /// Validate that the requirements are internally consistent.
    ///
    /// # Errors
    ///
    /// Returns error if CPU or memory bounds are invalid.
    pub fn validate(&self) -> ToadStoolResult<()> {
        use crate::ToadStoolError;
        if self.cpu.min_cores <= 0.0 {
            return Err(ToadStoolError::validation(
                "cpu.min_cores must be greater than 0",
            ));
        }
        if self.memory.min_bytes == 0 {
            return Err(ToadStoolError::validation(
                "memory.min_bytes must be greater than 0",
            ));
        }
        Ok(())
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    #[serde(with = "toadstool_common::system_time_serde")]
    pub start_time: SystemTime,
    /// Execution end time
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub end_time: Option<SystemTime>,
    /// Total execution duration
    #[serde(with = "humantime_serde")]
    pub duration: Duration,
}

impl Default for TimingMetrics {
    fn default() -> Self {
        Self {
            start_time: SystemTime::now(),
            end_time: None,
            duration: Duration::ZERO,
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
    #[serde(with = "humantime_serde::option")]
    pub execution_timeout: Option<Duration>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limits: CpuLimits::default(),
            memory_limits: MemoryLimits::default(),
            storage_limits: StorageLimits::default(),
            network_limits: NetworkLimits::default(),
            execution_timeout: Some(Duration::from_secs(300)),
        }
    }
}

/// Snapshot of actual resource consumption during or after execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage (0.0–100.0 × `n_cores`)
    pub cpu_usage_percent: f64,
    /// Memory currently in use, in megabytes
    pub memory_used_mb: u64,
    /// Disk I/O read bytes since start
    pub disk_read_bytes: u64,
    /// Disk I/O write bytes since start
    pub disk_write_bytes: u64,
    /// Network bytes received since start
    pub network_rx_bytes: u64,
    /// Network bytes transmitted since start
    pub network_tx_bytes: u64,
    /// Wall-clock execution time in milliseconds
    pub wall_time_ms: u64,
}

impl ResourceUsage {
    /// Returns `true` when all metrics are zero (freshly created or never measured).
    pub fn is_empty(&self) -> bool {
        self.cpu_usage_percent == 0.0
            && self.memory_used_mb == 0
            && self.disk_read_bytes == 0
            && self.disk_write_bytes == 0
            && self.network_rx_bytes == 0
            && self.network_tx_bytes == 0
            && self.wall_time_ms == 0
    }
}

/// CPU limits
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuLimits {
    /// Maximum CPU cores
    pub max_cores: Option<f64>,
    /// CPU throttling
    pub throttle_percent: Option<f64>,
}

/// Memory limits
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryLimits {
    /// Maximum memory in bytes
    pub max_bytes: Option<u64>,
    /// Memory swap limit
    pub swap_limit_bytes: Option<u64>,
}

/// Storage limits
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageLimits {
    /// Maximum storage in bytes
    pub max_bytes: Option<u64>,
    /// Read bandwidth limit
    pub read_bandwidth_limit: Option<u64>,
    /// Write bandwidth limit
    pub write_bandwidth_limit: Option<u64>,
}

/// Network limits
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkLimits {
    /// Maximum bandwidth in bytes per second
    pub max_bandwidth: Option<u64>,
    /// Connection limits
    pub max_connections: Option<u32>,
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
    /// CPU usage percentage (0.0 - 100.0)
    pub cpu_usage_percent: f64,
    /// Memory usage percentage (0.0 - 100.0)
    pub memory_usage_percent: f64,
    /// Total CPU cores
    pub total_cpu_cores: usize,
    /// Total memory in bytes
    pub total_memory_bytes: u64,
}

impl Default for SystemResources {
    fn default() -> Self {
        Self {
            available_cpu_cores: 1.0,
            available_memory_bytes: 1024 * 1024 * 1024, // 1GB
            available_storage_bytes: 1024 * 1024 * 1024, // 1GB
            available_network_bandwidth: None,
            available_gpu_units: 0,
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            total_cpu_cores: 1,
            total_memory_bytes: 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// Process information for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Workload identifier.
    pub workload_id: String,
    /// Number of processes.
    pub process_count: usize,
    /// Total CPU time in seconds.
    pub total_cpu_time: f64,
    /// Memory usage in bytes.
    pub memory_usage: u64,
    /// Aggregate process status.
    pub status: ProcessStatus,
}

/// Process status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    /// Processes running.
    Running,
    /// Processes sleeping/waiting.
    Sleeping,
    /// Processes stopped.
    Stopped,
    /// Zombie processes.
    Zombie,
    /// Unknown status.
    Unknown,
}

/// Network statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Bytes received.
    pub bytes_received: u64,
    /// Bytes transmitted.
    pub bytes_transmitted: u64,
    /// Packets received.
    pub packets_received: u64,
    /// Packets transmitted.
    pub packets_transmitted: u64,
    /// Number of interfaces.
    pub interfaces: usize,
}

/// System load averages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadAverages {
    /// 1-minute load average.
    pub one_minute: f64,
    /// 5-minute load average.
    pub five_minutes: f64,
    /// 15-minute load average.
    pub fifteen_minutes: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_requirements_default_is_valid() {
        let req = ResourceRequirements::default();
        assert!(req.validate().is_ok());
        assert!(req.cpu.min_cores > 0.0);
        assert!(req.memory.min_bytes > 0);
        assert!(req.gpu.is_none());
    }

    #[test]
    fn resource_requirements_validate_zero_cpu() {
        let mut req = ResourceRequirements::default();
        req.cpu.min_cores = 0.0;
        let err = req.validate().unwrap_err();
        assert!(err.to_string().contains("cpu.min_cores"));
    }

    #[test]
    fn resource_requirements_validate_zero_memory() {
        let mut req = ResourceRequirements::default();
        req.memory.min_bytes = 0;
        let err = req.validate().unwrap_err();
        assert!(err.to_string().contains("memory.min_bytes"));
    }

    #[test]
    fn resource_usage_default_is_empty() {
        let usage = ResourceUsage::default();
        assert!(usage.is_empty());
    }

    #[test]
    fn resource_usage_nonzero_not_empty() {
        let usage = ResourceUsage {
            memory_used_mb: 128,
            ..ResourceUsage::default()
        };
        assert!(!usage.is_empty());
    }

    #[test]
    fn defaults_round_trip_serde() {
        let metrics = RuntimeMetrics::default();
        let json = serde_json::to_string(&metrics).unwrap();
        let deser: RuntimeMetrics = serde_json::from_str(&json).unwrap();
        assert!((deser.cpu.cores_used - metrics.cpu.cores_used).abs() < f64::EPSILON);
        assert_eq!(deser.memory.used_bytes, metrics.memory.used_bytes);
    }

    #[test]
    fn resource_limits_default_has_timeout() {
        let limits = ResourceLimits::default();
        assert!(limits.execution_timeout.is_some());
    }

    #[test]
    fn system_resources_default() {
        let res = SystemResources::default();
        assert!(res.available_cpu_cores > 0.0);
        assert!(res.available_memory_bytes > 0);
        assert_eq!(res.available_gpu_units, 0);
    }

    #[test]
    fn process_status_debug() {
        let statuses = [
            ProcessStatus::Running,
            ProcessStatus::Sleeping,
            ProcessStatus::Stopped,
            ProcessStatus::Zombie,
            ProcessStatus::Unknown,
        ];
        for s in &statuses {
            assert!(!format!("{s:?}").is_empty());
        }
    }

    #[test]
    fn cpu_metrics_default() {
        let m = CpuMetrics::default();
        assert!((m.usage_percent - 0.0).abs() < f64::EPSILON);
        assert!((m.cores_used - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn gpu_metrics_default() {
        let m = GpuMetrics::default();
        assert!((m.usage_percent - 0.0).abs() < f64::EPSILON);
        assert!(m.temperature_celsius.is_none());
    }

    #[test]
    fn storage_requirements_default() {
        let s = StorageRequirements::default();
        assert!(s.min_bytes > 0);
        assert!(s.max_bytes.is_none());
    }

    #[test]
    fn network_requirements_default() {
        let n = NetworkRequirements::default();
        assert!(n.min_bandwidth.is_none());
        assert!(n.max_latency_ms.is_none());
    }

    #[test]
    fn timing_metrics_default() {
        let t = TimingMetrics::default();
        assert!(t.end_time.is_none());
        assert_eq!(t.duration, Duration::ZERO);
    }

    #[test]
    fn load_averages_serde() {
        let la = LoadAverages {
            one_minute: 1.5,
            five_minutes: 2.0,
            fifteen_minutes: 1.8,
        };
        let json = serde_json::to_string(&la).unwrap();
        let deser: LoadAverages = serde_json::from_str(&json).unwrap();
        assert!((deser.one_minute - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn network_stats_serde() {
        let ns = NetworkStats {
            bytes_received: 100,
            bytes_transmitted: 200,
            packets_received: 10,
            packets_transmitted: 20,
            interfaces: 2,
        };
        let json = serde_json::to_string(&ns).unwrap();
        let deser: NetworkStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.interfaces, 2);
    }

    #[test]
    fn process_info_serde() {
        let pi = ProcessInfo {
            workload_id: "w1".to_string(),
            process_count: 3,
            total_cpu_time: 10.5,
            memory_usage: 1024,
            status: ProcessStatus::Running,
        };
        let json = serde_json::to_string(&pi).unwrap();
        let deser: ProcessInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.workload_id, "w1");
    }
}
