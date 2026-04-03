// SPDX-License-Identifier: AGPL-3.0-only

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tracing::debug;

use toadstool::error::ToadStoolResult;
use toadstool::resources::{ResourceMonitor, RuntimeMetrics, SystemResources};

use crate::metric_types::SystemResourceMonitor;
use crate::types::ResourceMonitorError;

impl ResourceMonitor for SystemResourceMonitor {
    fn start_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        debug!("Starting monitoring for workload: {}", workload_id);
        // Individual workload monitoring is handled by the background loop
        // This could be extended to enable per-workload monitoring configuration
        Ok(())
    }

    fn stop_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        debug!("Stopping monitoring for workload: {}", workload_id);
        // Remove from tracking maps
        let process_map = Arc::clone(&self.process_map);
        let usage_data = Arc::clone(&self.usage_data);
        let threshold_data = Arc::clone(&self.threshold_data);
        let workload_id = workload_id.to_string();

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
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        let workload_id = workload_id.to_string();
        Box::pin(async move {
            // Modern async access - no blocking!
            let usage_data = self.usage_data.read().await;

            usage_data.get(&workload_id).cloned().ok_or_else(|| {
                ResourceMonitorError::ProcessNotRegistered(workload_id.clone()).into()
            })
        })
    }

    fn get_system_resources(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<SystemResources>> + Send + '_>> {
        Box::pin(async move {
            // /proc and sysctl reads are blocking I/O — run on the blocking
            // pool so we don't stall the async runtime under load.
            let (total_cpu_cores, total_memory_bytes, available_memory_bytes) =
                tokio::task::spawn_blocking(read_system_info)
                    .await
                    .unwrap_or((1, 1024 * 1024 * 1024, 1024 * 1024 * 1024));

            let available_storage_bytes = 10 * 1024 * 1024 * 1024u64; // 10GB default

            // Calculate usage percentages
            let memory_usage_percent =
                memory_usage_percent(total_memory_bytes, available_memory_bytes);

            // CPU usage requires sampling over time - use 0% as snapshot
            // Real usage tracking would need historical data
            let cpu_usage_percent = 0.0;
            let available_cpu_cores = total_cpu_cores as f64;

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
        })
    }
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
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            total_cpu_cores = cpuinfo
                .lines()
                .filter(|line| line.starts_with("processor"))
                .count()
                .max(1);
        }

        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
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
