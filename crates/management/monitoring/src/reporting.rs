// SPDX-License-Identifier: AGPL-3.0-or-later

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use toadstool_sysmon::{DiskInfo, cpu_count, cpu_usage, disk_usage, load_average, memory_info};
use tracing::debug;

use toadstool::error::ToadStoolResult;
use toadstool::resources::{ResourceMonitor, RuntimeMetrics, SystemResources};
use toadstool_common::constants::platform_paths::procfs;

use crate::metric_types::SystemResourceMonitor;
use crate::types::ResourceMonitorError;

impl ResourceMonitor for SystemResourceMonitor {
    fn start_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        self.monitored_workloads
            .lock()
            .map_err(|e| ResourceMonitorError::LockPoisoned(format!("monitored_workloads: {e}")))?
            .insert(workload_id.to_string());
        debug!("Starting monitoring for workload: {}", workload_id);
        Ok(())
    }

    fn stop_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        debug!("Stopping monitoring for workload: {}", workload_id);
        let workload_id = workload_id.to_string();
        self.monitored_workloads
            .lock()
            .map_err(|e| ResourceMonitorError::LockPoisoned(format!("monitored_workloads: {e}")))?
            .remove(&workload_id);

        let process_map = Arc::clone(&self.process_map);
        let usage_data = Arc::clone(&self.usage_data);
        let threshold_data = Arc::clone(&self.threshold_data);

        tokio::spawn(async move {
            process_map.write().await.remove(&workload_id);
            usage_data.write().await.remove(&workload_id);
            threshold_data.write().await.remove(&workload_id);
        });
        Ok(())
    }

    fn get_metrics(
        &self,
        workload_id: &str,
    ) -> impl Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        let workload_id = workload_id.to_string();
        async move {
            // Modern async access - no blocking!
            let usage_data = self.usage_data.read().await;

            usage_data.get(&workload_id).cloned().ok_or_else(|| {
                ResourceMonitorError::ProcessNotRegistered(workload_id.clone()).into()
            })
        }
    }

    fn get_system_resources(
        &self,
    ) -> impl Future<Output = ToadStoolResult<SystemResources>> + Send + '_ {
        async {
            // /proc, sysctl, and statvfs are blocking I/O — run on the blocking
            // pool so we don't stall the async runtime under load.
            let snapshot = if let Ok(tuple) =
                tokio::task::spawn_blocking(collect_host_resource_snapshot).await
            {
                tuple
            } else {
                let (cores, total, avail) = read_system_info();
                (cores, total, avail, 0u64, 0.0f64)
            };

            let (
                total_cpu_cores,
                total_memory_bytes,
                available_memory_bytes,
                available_storage_bytes,
                cpu_usage_percent,
            ) = snapshot;

            let memory_usage_percent =
                memory_usage_percent(total_memory_bytes, available_memory_bytes);

            let available_cpu_cores = total_cpu_cores as f64;

            tracing::debug!(
                available_gpu_units = 0u32,
                "available_gpu_units is discovery-dependent until GPU enumeration is integrated"
            );

            Ok(SystemResources {
                available_cpu_cores,
                available_memory_bytes,
                available_storage_bytes,
                available_network_bandwidth: None,
                available_gpu_units: 0,
                cpu_usage_percent,
                memory_usage_percent,
                total_cpu_cores,
                total_memory_bytes,
            })
        }
    }
}

/// Prefer the root mount (`/`), otherwise the first reported real disk.
#[must_use]
pub(crate) fn root_filesystem_available_bytes(disks: &[DiskInfo]) -> Option<u64> {
    disks
        .iter()
        .find(|d| d.mount_point == "/")
        .or_else(|| disks.first())
        .map(|d| d.available_space)
}

/// Map load average (1-minute) to a rough CPU pressure percentage in `[0, 100]`.
#[must_use]
pub(crate) fn load_to_cpu_usage_percent(load_one: f64, cpu_cores: usize) -> f64 {
    let cores = cpu_cores.max(1) as f64;
    ((load_one / cores) * 100.0).clamp(0.0, 100.0)
}

/// Bytes available to unprivileged users on `/` via `statvfs`, or `0` on failure.
#[cfg(unix)]
fn statvfs_root_available_bytes() -> u64 {
    match rustix::fs::statvfs("/") {
        Ok(s) => s.f_bavail.saturating_mul(s.f_frsize),
        Err(_) => 0,
    }
}

#[cfg(not(unix))]
fn statvfs_root_available_bytes() -> u64 {
    0
}

/// Snapshot host CPU, memory, and root filesystem availability using `toadstool_sysmon`
/// (and `statvfs` on `/` when disk enumeration does not yield a root entry).
///
/// Returns `(total_cpu_cores, total_memory_bytes, available_memory_bytes, available_storage_bytes, cpu_usage_percent)`.
/// Designed to run on a blocking thread pool via `spawn_blocking`.
pub(crate) fn collect_host_resource_snapshot() -> (usize, u64, u64, u64, f64) {
    let total_cpu_cores = cpu_count().max(1);

    let (total_memory_bytes, available_memory_bytes) = if let Ok(m) = memory_info() {
        (m.total, m.available)
    } else {
        let (_, t, a) = read_system_info();
        (t, a)
    };

    let sample = Duration::from_millis(100);
    let cpu_usage_percent = match cpu_usage(sample) {
        Ok(p) => f64::from(p),
        Err(_) => match load_average() {
            Ok(la) => load_to_cpu_usage_percent(la.one, total_cpu_cores),
            Err(_) => memory_usage_percent(total_memory_bytes, available_memory_bytes),
        },
    };

    let available_storage_bytes = match disk_usage() {
        Ok(disks) => {
            root_filesystem_available_bytes(&disks).unwrap_or_else(statvfs_root_available_bytes)
        }
        Err(_) => statvfs_root_available_bytes(),
    };

    (
        total_cpu_cores,
        total_memory_bytes,
        available_memory_bytes,
        available_storage_bytes,
        cpu_usage_percent,
    )
}

/// Used memory percentage from total and available byte counts (`0.0` when total is zero).
#[must_use]
pub(crate) fn memory_usage_percent(total_memory_bytes: u64, available_memory_bytes: u64) -> f64 {
    if total_memory_bytes > 0 {
        let used = total_memory_bytes.saturating_sub(available_memory_bytes);
        (used as f64 / total_memory_bytes as f64) * 100.0
    } else {
        0.0
    }
}

/// Read CPU core count and memory stats from OS interfaces.
///
/// Returns `(total_cpu_cores, total_memory_bytes, available_memory_bytes)`.
/// Designed to run on a blocking thread pool via `spawn_blocking`.
pub(crate) fn read_system_info() -> (usize, u64, u64) {
    let mut total_cpu_cores = 1usize;
    let mut total_memory_bytes = 1024 * 1024 * 1024u64;
    let mut available_memory_bytes = total_memory_bytes;

    #[cfg(target_os = "linux")]
    {
        if let Ok(cpuinfo) = std::fs::read_to_string(procfs::CPUINFO) {
            total_cpu_cores = cpuinfo
                .lines()
                .filter(|line| line.starts_with("processor"))
                .count()
                .max(1);
        }

        if let Ok(meminfo) = std::fs::read_to_string(procfs::MEMINFO) {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(value) = line.split_whitespace().nth(1)
                        && let Ok(mem_kb) = value.parse::<u64>()
                    {
                        total_memory_bytes = mem_kb * 1024;
                    }
                } else if line.starts_with("MemAvailable:")
                    && let Some(value) = line.split_whitespace().nth(1)
                    && let Ok(mem_kb) = value.parse::<u64>()
                {
                    available_memory_bytes = mem_kb * 1024;
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "hw.ncpu"])
            .output()
        {
            if let Ok(cpu_str) = String::from_utf8(output.stdout) {
                if let Ok(cpu_count) = cpu_str.trim().parse::<usize>() {
                    total_cpu_cores = cpu_count.max(1);
                }
            }
        }

        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
        {
            if let Ok(mem_str) = String::from_utf8(output.stdout) {
                if let Ok(mem_bytes) = mem_str.trim().parse::<u64>() {
                    total_memory_bytes = mem_bytes;
                    available_memory_bytes = mem_bytes / 2;
                }
            }
        }
    }

    (total_cpu_cores, total_memory_bytes, available_memory_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metric_types::SystemResourceMonitor;
    use toadstool::resources::ResourceMonitor;
    use toadstool_sysmon::DiskInfo;

    #[test]
    fn memory_usage_percent_zero_total_returns_zero() {
        assert!(memory_usage_percent(0, 0).abs() < f64::EPSILON);
        assert!(memory_usage_percent(0, 10).abs() < f64::EPSILON);
    }

    #[test]
    fn memory_usage_percent_full_usage_when_no_memory_available() {
        let p = memory_usage_percent(8 * 1024 * 1024, 0);
        assert!((p - 100.0).abs() < 1e-9);
    }

    #[test]
    fn memory_usage_percent_when_all_available_is_zero_percent() {
        let total = 4096u64;
        assert!(memory_usage_percent(total, total).abs() < f64::EPSILON);
    }

    #[test]
    fn memory_usage_percent_saturates_when_available_exceeds_total() {
        let p = memory_usage_percent(100, 200);
        assert!(p.abs() < f64::EPSILON);
    }

    #[test]
    fn memory_usage_percent_quarter_used() {
        let p = memory_usage_percent(400, 300);
        assert!((p - 25.0).abs() < 1e-9);
    }

    #[test]
    fn read_system_info_returns_nonzero_memory_and_at_least_one_core() {
        let (cores, total, avail) = read_system_info();
        assert!(cores >= 1);
        assert!(total > 0);
        assert!(avail > 0);
    }

    #[test]
    fn root_filesystem_prefers_slash_mount() {
        let disks = vec![
            DiskInfo {
                mount_point: "/boot".to_string(),
                filesystem: "ext4".to_string(),
                total_space: 100,
                available_space: 10,
            },
            DiskInfo {
                mount_point: "/".to_string(),
                filesystem: "ext4".to_string(),
                total_space: 1000,
                available_space: 500,
            },
        ];
        assert_eq!(root_filesystem_available_bytes(&disks), Some(500));
    }

    #[test]
    fn root_filesystem_falls_back_to_first_disk() {
        let disks = vec![DiskInfo {
            mount_point: "/home".to_string(),
            filesystem: "ext4".to_string(),
            total_space: 200,
            available_space: 20,
        }];
        assert_eq!(root_filesystem_available_bytes(&disks), Some(20));
    }

    #[test]
    fn load_to_cpu_usage_percent_clamps() {
        assert!((load_to_cpu_usage_percent(0.0, 4) - 0.0).abs() < f64::EPSILON);
        assert!((load_to_cpu_usage_percent(4.0, 4) - 100.0).abs() < 1e-9);
        assert!((load_to_cpu_usage_percent(40.0, 4) - 100.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn get_metrics_missing_workload_returns_err() {
        let monitor = SystemResourceMonitor::new();
        let err = monitor.get_metrics("missing-workload").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn start_and_stop_monitoring_registers_workload_ids() {
        let monitor = SystemResourceMonitor::new();
        assert!(monitor.start_monitoring("w-1").is_ok());
        assert!(
            monitor
                .monitored_workloads
                .lock()
                .expect("lock")
                .contains("w-1")
        );
        assert!(monitor.stop_monitoring("w-1").is_ok());
        assert!(
            !monitor
                .monitored_workloads
                .lock()
                .expect("lock")
                .contains("w-1")
        );
        tokio::task::yield_now().await;
    }

    #[tokio::test]
    async fn get_system_resources_reports_live_host_snapshot() {
        let monitor = SystemResourceMonitor::new();
        let res = monitor
            .get_system_resources()
            .await
            .expect("system resources");
        assert!(res.total_cpu_cores >= 1);
        assert!(res.total_memory_bytes > 0);
        assert!(res.memory_usage_percent >= 0.0 && res.memory_usage_percent <= 100.0);
        assert!(res.cpu_usage_percent >= 0.0 && res.cpu_usage_percent <= 100.0);
        assert!(res.available_storage_bytes > 0);
    }
}
