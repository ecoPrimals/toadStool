// SPDX-License-Identifier: AGPL-3.0-or-later
//! Host/system snapshots, process monitoring, and load/network aggregates.

use serde::{Deserialize, Serialize};

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
