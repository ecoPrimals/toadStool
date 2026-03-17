// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]

//! `ToadStool` resources component
//!
//! This crate provides resource management and monitoring functionality for the `ToadStool` platform.
//!
//! ## Features
//!
//! - Resource allocation and deallocation
//! - Performance monitoring
//! - Resource usage tracking
//! - Capacity planning utilities

use serde::{Deserialize, Serialize};

/// Current resource usage snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU utilization as a fraction (0.0–1.0).
    pub cpu_percent: f64,
    /// Memory currently used in bytes.
    pub memory_used_bytes: u64,
    /// Total memory available in bytes.
    pub memory_total_bytes: u64,
    /// Disk space used in bytes.
    pub disk_used_bytes: u64,
    /// Total disk space in bytes.
    pub disk_total_bytes: u64,
}

/// Resource limits for enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimit {
    /// Maximum CPU utilization as a fraction (0.0–1.0).
    pub max_cpu_percent: f64,
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Maximum disk usage in bytes.
    pub max_disk_bytes: u64,
}

/// Violation of a resource limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceViolation {
    /// CPU usage exceeded the configured limit.
    CpuExceeded,
    /// Memory usage exceeded the configured limit.
    MemoryExceeded,
    /// Disk usage exceeded the configured limit.
    DiskExceeded,
}

/// Manages resource monitoring and limit enforcement.
pub struct ResourceManager {
    limits: ResourceLimit,
}

impl ResourceManager {
    /// Creates a new resource manager with the given limits.
    #[must_use]
    pub const fn new(limits: ResourceLimit) -> Self {
        Self { limits }
    }

    /// Checks usage against limits and returns any violations.
    #[must_use]
    pub fn check_limits(&self, usage: &ResourceUsage) -> Vec<ResourceViolation> {
        let mut violations = Vec::new();
        if usage.cpu_percent > self.limits.max_cpu_percent {
            violations.push(ResourceViolation::CpuExceeded);
        }
        if usage.memory_used_bytes > self.limits.max_memory_bytes {
            violations.push(ResourceViolation::MemoryExceeded);
        }
        if usage.disk_used_bytes > self.limits.max_disk_bytes {
            violations.push(ResourceViolation::DiskExceeded);
        }
        violations
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn current_usage() -> ResourceUsage {
        let cpu_percent = (f64::from(
            toadstool_sysmon::cpu_usage(std::time::Duration::from_millis(50)).unwrap_or(0.0),
        ) / 100.0)
            .clamp(0.0, 1.0);

        let (memory_used_bytes, memory_total_bytes) = toadstool_sysmon::memory_info()
            .map(|m| (m.used, m.total.max(1)))
            .unwrap_or((0, 1));

        let disks = toadstool_sysmon::disk_usage().unwrap_or_default();
        let (disk_used_bytes, disk_total_bytes) =
            disks.iter().fold((0u64, 0u64), |(used, total), d| {
                (
                    used + d.total_space.saturating_sub(d.available_space),
                    total + d.total_space,
                )
            });
        let disk_total_bytes = disk_total_bytes.max(1);

        ResourceUsage {
            cpu_percent,
            memory_used_bytes,
            memory_total_bytes,
            disk_used_bytes,
            disk_total_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_limits_no_violations() {
        let limits = ResourceLimit {
            max_cpu_percent: 0.9,
            max_memory_bytes: 8 * 1024 * 1024 * 1024,
            max_disk_bytes: 100 * 1024 * 1024 * 1024,
        };
        let mgr = ResourceManager::new(limits);
        let usage = ResourceUsage {
            cpu_percent: 0.5,
            memory_used_bytes: 2 * 1024 * 1024 * 1024,
            memory_total_bytes: 8 * 1024 * 1024 * 1024,
            disk_used_bytes: 50 * 1024 * 1024 * 1024,
            disk_total_bytes: 100 * 1024 * 1024 * 1024,
        };
        assert!(mgr.check_limits(&usage).is_empty());
    }

    #[test]
    fn check_limits_cpu_exceeded() {
        let limits = ResourceLimit {
            max_cpu_percent: 0.5,
            max_memory_bytes: u64::MAX,
            max_disk_bytes: u64::MAX,
        };
        let mgr = ResourceManager::new(limits);
        let usage = ResourceUsage {
            cpu_percent: 0.9,
            memory_used_bytes: 0,
            memory_total_bytes: 1,
            disk_used_bytes: 0,
            disk_total_bytes: 1,
        };
        let v = mgr.check_limits(&usage);
        assert_eq!(v, vec![ResourceViolation::CpuExceeded]);
    }

    #[test]
    fn check_limits_memory_exceeded() {
        let limits = ResourceLimit {
            max_cpu_percent: 1.0,
            max_memory_bytes: 1024,
            max_disk_bytes: u64::MAX,
        };
        let mgr = ResourceManager::new(limits);
        let usage = ResourceUsage {
            cpu_percent: 0.0,
            memory_used_bytes: 2048,
            memory_total_bytes: 4096,
            disk_used_bytes: 0,
            disk_total_bytes: 1,
        };
        let v = mgr.check_limits(&usage);
        assert_eq!(v, vec![ResourceViolation::MemoryExceeded]);
    }

    #[test]
    fn check_limits_disk_exceeded() {
        let limits = ResourceLimit {
            max_cpu_percent: 1.0,
            max_memory_bytes: u64::MAX,
            max_disk_bytes: 1000,
        };
        let mgr = ResourceManager::new(limits);
        let usage = ResourceUsage {
            cpu_percent: 0.0,
            memory_used_bytes: 0,
            memory_total_bytes: 1,
            disk_used_bytes: 2000,
            disk_total_bytes: 5000,
        };
        let v = mgr.check_limits(&usage);
        assert_eq!(v, vec![ResourceViolation::DiskExceeded]);
    }

    #[test]
    fn check_limits_multiple_violations() {
        let limits = ResourceLimit {
            max_cpu_percent: 0.5,
            max_memory_bytes: 100,
            max_disk_bytes: 100,
        };
        let mgr = ResourceManager::new(limits);
        let usage = ResourceUsage {
            cpu_percent: 0.9,
            memory_used_bytes: 200,
            memory_total_bytes: 500,
            disk_used_bytes: 200,
            disk_total_bytes: 500,
        };
        let v = mgr.check_limits(&usage);
        assert_eq!(v.len(), 3);
        assert!(v.contains(&ResourceViolation::CpuExceeded));
        assert!(v.contains(&ResourceViolation::MemoryExceeded));
        assert!(v.contains(&ResourceViolation::DiskExceeded));
    }

    #[test]
    fn current_usage_returns_sensible_values() {
        let usage = ResourceManager::current_usage();
        assert!(usage.cpu_percent >= 0.0 && usage.cpu_percent <= 1.0);
        assert!(usage.memory_total_bytes >= 1);
        assert!(usage.disk_total_bytes >= 1);
    }
}
