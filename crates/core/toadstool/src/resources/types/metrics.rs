// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime and timing metrics (CPU, memory, storage, network, GPU).

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

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
