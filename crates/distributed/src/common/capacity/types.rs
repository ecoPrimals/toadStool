//! Common Capacity Management Types
//!
//! Generic capacity tracking abstractions used across Songbird, Cloud, and other distributed systems.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use toadstool_common::constants::timeouts;

/// Capacity information for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityInfo {
    /// Total CPU cores available
    pub total_cpu_cores: f64,
    /// Available CPU cores
    pub available_cpu_cores: f64,
    /// Total memory (bytes)
    pub total_memory_bytes: u64,
    /// Available memory (bytes)
    pub available_memory_bytes: u64,
    /// Total storage (bytes)
    pub total_storage_bytes: u64,
    /// Available storage (bytes)
    pub available_storage_bytes: u64,
    /// Total GPU units (if applicable)
    pub total_gpu_units: Option<u32>,
    /// Available GPU units
    pub available_gpu_units: Option<u32>,
    /// Network bandwidth (bytes/sec)
    pub network_bandwidth_bps: u64,
    /// Timestamp of measurement
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl CapacityInfo {
    /// Calculate CPU utilization (0.0-1.0)
    pub fn cpu_utilization(&self) -> f64 {
        if self.total_cpu_cores == 0.0 {
            return 0.0;
        }
        1.0 - (self.available_cpu_cores / self.total_cpu_cores)
    }

    /// Calculate memory utilization (0.0-1.0)
    pub fn memory_utilization(&self) -> f64 {
        if self.total_memory_bytes == 0 {
            return 0.0;
        }
        1.0 - (self.available_memory_bytes as f64 / self.total_memory_bytes as f64)
    }

    /// Calculate storage utilization (0.0-1.0)
    pub fn storage_utilization(&self) -> f64 {
        if self.total_storage_bytes == 0 {
            return 0.0;
        }
        1.0 - (self.available_storage_bytes as f64 / self.total_storage_bytes as f64)
    }

    /// Check if resource has sufficient capacity for a request
    pub fn has_capacity(&self, required: &CapacityRequirement) -> bool {
        self.available_cpu_cores >= required.cpu_cores
            && self.available_memory_bytes >= required.memory_bytes
            && self.available_storage_bytes >= required.storage_bytes
            && self.network_bandwidth_bps >= required.network_bandwidth_bps
            && match (self.available_gpu_units, required.gpu_units) {
                (Some(available), Some(required)) => available >= required,
                (None, Some(_)) => false,
                _ => true,
            }
    }
}

/// Capacity requirement for a workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityRequirement {
    /// Required CPU cores
    pub cpu_cores: f64,
    /// Required memory (bytes)
    pub memory_bytes: u64,
    /// Required storage (bytes)
    pub storage_bytes: u64,
    /// Required network bandwidth (bytes/sec)
    pub network_bandwidth_bps: u64,
    /// Required GPU units (if applicable)
    pub gpu_units: Option<u32>,
}

/// Capacity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityConfig {
    /// Monitoring interval
    pub monitoring_interval: Duration,
    /// Reserve percentage (keep this % of resources free)
    pub reserve_percent: f64,
    /// Enable automatic scaling
    pub auto_scale: bool,
    /// Scaling thresholds
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
}

impl Default for CapacityConfig {
    fn default() -> Self {
        Self {
            monitoring_interval: timeouts::METRICS_INTERVAL,
            reserve_percent: 10.0,
            auto_scale: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
        }
    }
}

/// Available capacity (remaining resources)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCapacity {
    /// Available CPU cores
    pub cpu_cores: f64,
    /// Available memory (bytes)
    pub memory_bytes: u64,
    /// Available storage (bytes)
    pub storage_bytes: u64,
    /// Available network bandwidth (bytes/sec)
    pub network_bandwidth_bps: u64,
    /// Available GPU units
    pub gpu_units: Option<u32>,
}

/// Network capacity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCapacity {
    /// Total bandwidth (bytes/sec)
    pub total_bandwidth_bps: u64,
    /// Available bandwidth (bytes/sec)
    pub available_bandwidth_bps: u64,
    /// Current connections
    pub active_connections: u64,
    /// Maximum connections
    pub max_connections: u64,
    /// Latency measurements (milliseconds)
    pub latency_ms: f64,
}

/// Resource usage snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageSnapshot {
    /// Target identifier
    pub target_id: String,
    /// Capacity information
    pub capacity: CapacityInfo,
    /// Active workloads count
    pub active_workloads: u64,
    /// Pending workloads count
    pub pending_workloads: u64,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Capacity alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapacityAlert {
    /// Capacity running low
    LowCapacity {
        resource_type: String,
        utilization_percent: f64,
    },
    /// Capacity exhausted
    Exhausted { resource_type: String },
    /// Capacity restored
    Restored { resource_type: String },
}
