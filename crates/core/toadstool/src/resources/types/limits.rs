// SPDX-License-Identifier: AGPL-3.0-only
//! Execution limits and observed resource usage snapshots.

use serde::{Deserialize, Serialize};
use std::time::Duration;

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
