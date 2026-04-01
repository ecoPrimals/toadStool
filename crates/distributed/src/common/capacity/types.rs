// SPDX-License-Identifier: AGPL-3.0-only
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
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
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
    /// Utilization fraction above which to scale up.
    pub scale_up_threshold: f64,
    /// Utilization fraction below which to scale down.
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
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

/// Capacity alert for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapacityAlert {
    /// Capacity running low.
    LowCapacity {
        /// Resource type (cpu, memory, etc.).
        resource_type: String,
        /// Utilization percentage.
        utilization_percent: f64,
    },
    /// Capacity exhausted.
    Exhausted {
        /// Resource type.
        resource_type: String,
    },
    /// Capacity restored.
    Restored {
        /// Resource type.
        resource_type: String,
    },
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
mod tests {
    use super::*;

    fn make_capacity_info(
        total: (f64, u64, u64),
        available: (f64, u64, u64),
        network_bps: u64,
    ) -> CapacityInfo {
        CapacityInfo {
            total_cpu_cores: total.0,
            available_cpu_cores: available.0,
            total_memory_bytes: total.1,
            available_memory_bytes: available.1,
            total_storage_bytes: total.2,
            available_storage_bytes: available.2,
            total_gpu_units: Some(2),
            available_gpu_units: Some(1),
            network_bandwidth_bps: network_bps,
            timestamp: std::time::SystemTime::now(),
        }
    }

    #[test]
    fn test_capacity_info_cpu_utilization_full() {
        let info = make_capacity_info((4.0, 8, 100), (0.0, 0, 0), 1000);
        assert_eq!(info.cpu_utilization(), 1.0);
    }

    #[test]
    fn test_capacity_info_cpu_utilization_half() {
        let info = make_capacity_info((4.0, 8, 100), (2.0, 4, 50), 1000);
        assert_eq!(info.cpu_utilization(), 0.5);
    }

    #[test]
    fn test_capacity_info_cpu_utilization_zero_total() {
        let info = make_capacity_info((0.0, 0, 0), (0.0, 0, 0), 0);
        assert_eq!(info.cpu_utilization(), 0.0);
    }

    #[test]
    fn test_capacity_info_memory_utilization() {
        let info = make_capacity_info((4.0, 8, 100), (4.0, 2, 50), 1000);
        assert_eq!(info.memory_utilization(), 0.75);
    }

    #[test]
    fn test_capacity_info_storage_utilization() {
        let info = make_capacity_info((4.0, 8, 100), (4.0, 8, 25), 1000);
        assert_eq!(info.storage_utilization(), 0.75);
    }

    #[test]
    fn test_capacity_info_has_capacity_true() {
        let info = make_capacity_info((4.0, 8, 100), (2.0, 4, 50), 1000);
        let req = CapacityRequirement {
            cpu_cores: 1.0,
            memory_bytes: 2,
            storage_bytes: 25,
            network_bandwidth_bps: 500,
            gpu_units: Some(1),
        };
        assert!(info.has_capacity(&req));
    }

    #[test]
    fn test_capacity_info_has_capacity_false_insufficient_cpu() {
        let info = make_capacity_info((4.0, 8, 100), (0.5, 8, 100), 1000);
        let req = CapacityRequirement {
            cpu_cores: 2.0,
            memory_bytes: 1,
            storage_bytes: 1,
            network_bandwidth_bps: 1,
            gpu_units: None,
        };
        assert!(!info.has_capacity(&req));
    }

    #[test]
    fn test_capacity_info_has_capacity_false_insufficient_gpu() {
        let info = CapacityInfo {
            total_cpu_cores: 4.0,
            available_cpu_cores: 4.0,
            total_memory_bytes: 8,
            available_memory_bytes: 8,
            total_storage_bytes: 100,
            available_storage_bytes: 100,
            total_gpu_units: None,
            available_gpu_units: None,
            network_bandwidth_bps: 1000,
            timestamp: std::time::SystemTime::now(),
        };
        let req = CapacityRequirement {
            cpu_cores: 1.0,
            memory_bytes: 1,
            storage_bytes: 1,
            network_bandwidth_bps: 1,
            gpu_units: Some(1),
        };
        assert!(!info.has_capacity(&req));
    }

    #[test]
    fn test_capacity_config_default() {
        let config = CapacityConfig::default();
        assert_eq!(config.reserve_percent, 10.0);
        assert!(config.auto_scale);
        assert_eq!(config.scale_up_threshold, 0.8);
        assert_eq!(config.scale_down_threshold, 0.3);
    }

    #[test]
    fn test_network_capacity_construction() {
        let nc = NetworkCapacity {
            total_bandwidth_bps: 1_000_000_000,
            available_bandwidth_bps: 500_000_000,
            active_connections: 10,
            max_connections: 100,
            latency_ms: 5.2,
        };
        assert_eq!(nc.total_bandwidth_bps, 1_000_000_000);
        assert_eq!(nc.latency_ms, 5.2);
    }

    #[test]
    fn test_capacity_alert_variants() {
        let _low = CapacityAlert::LowCapacity {
            resource_type: "cpu".to_string(),
            utilization_percent: 85.0,
        };
        let _exhausted = CapacityAlert::Exhausted {
            resource_type: "memory".to_string(),
        };
        let _restored = CapacityAlert::Restored {
            resource_type: "storage".to_string(),
        };
    }
}
