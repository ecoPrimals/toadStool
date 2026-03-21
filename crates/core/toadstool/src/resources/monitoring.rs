// SPDX-License-Identifier: AGPL-3.0-only
//! Resource monitoring for ToadStool

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::RwLock;

use crate::ToadStoolResult;

use super::types::{
    CpuMetrics, LoadAverages, MemoryMetrics, NetworkMetrics, NetworkStats, ProcessInfo,
    ProcessStatus, RuntimeMetrics, StorageMetrics, SystemResources, TimingMetrics,
};

/// Resource monitor trait
///
/// **Modern Async Design**: All methods are properly async to avoid blocking
/// the runtime with `block_in_place` or `block_on` anti-patterns.
pub trait ResourceMonitor: Send + Sync {
    /// Start monitoring resources for a workload
    fn start_monitoring(&self, workload_id: &str) -> ToadStoolResult<()>;

    /// Stop monitoring resources for a workload
    fn stop_monitoring(&self, workload_id: &str) -> ToadStoolResult<()>;

    /// Get current resource metrics (async to avoid blocking)
    fn get_metrics(
        &self,
        workload_id: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>>;

    /// Get system resource availability
    fn get_system_resources(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<SystemResources>> + Send + '_>>;
}

/// Real system resource monitor using toadstool-sysmon (pure Rust /proc parsing).
pub struct SystemResourceMonitor {
    workload_metrics: Arc<RwLock<HashMap<String, RuntimeMetrics>>>,
}

impl SystemResourceMonitor {
    /// Creates a new system resource monitor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            workload_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_cpu_usage(&self) -> ToadStoolResult<f64> {
        let usage = tokio::task::spawn_blocking(|| {
            toadstool_sysmon::cpu_usage(std::time::Duration::from_millis(100))
        })
        .await
        .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?
        .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?;
        Ok(f64::from(usage))
    }

    async fn get_memory_info(&self) -> ToadStoolResult<(u64, u64)> {
        let mem = toadstool_sysmon::memory_info()
            .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?;
        Ok((mem.used, mem.total))
    }

    async fn get_disk_info(&self) -> ToadStoolResult<(u64, u64)> {
        let disks = toadstool_sysmon::disk_usage()
            .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?;
        let (mut used_space, mut total_space) = (0u64, 0u64);
        for disk in &disks {
            total_space += disk.total_space;
            used_space += disk.total_space - disk.available_space;
        }
        Ok((used_space, total_space))
    }

    /// Get detailed process information for a specific workload
    pub async fn get_process_info(
        &self,
        workload_id: &str,
    ) -> ToadStoolResult<Option<ProcessInfo>> {
        let procs = toadstool_sysmon::all_processes()
            .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?;
        let process_count = procs.len();
        let total_cpu_time = procs.iter().map(|p| f64::from(p.cpu_usage)).sum::<f64>();
        let memory_usage = procs.iter().map(|p| p.memory).sum::<u64>();

        Ok(Some(ProcessInfo {
            workload_id: workload_id.to_string(),
            process_count,
            total_cpu_time,
            memory_usage,
            status: ProcessStatus::Running,
        }))
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> ToadStoolResult<NetworkStats> {
        let interfaces = toadstool_sysmon::network_stats()
            .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?;

        let mut total_received = 0u64;
        let mut total_transmitted = 0u64;
        let mut total_packets_received = 0u64;
        let mut total_packets_transmitted = 0u64;

        for iface in &interfaces {
            total_received += iface.received;
            total_transmitted += iface.transmitted;
            total_packets_received += iface.packets_received;
            total_packets_transmitted += iface.packets_transmitted;
        }

        Ok(NetworkStats {
            bytes_received: total_received,
            bytes_transmitted: total_transmitted,
            packets_received: total_packets_received,
            packets_transmitted: total_packets_transmitted,
            interfaces: interfaces.len(),
        })
    }

    /// Get system load averages
    pub async fn get_load_averages(&self) -> ToadStoolResult<LoadAverages> {
        let la = toadstool_sysmon::load_average()
            .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?;
        Ok(LoadAverages {
            one_minute: la.one,
            five_minutes: la.five,
            fifteen_minutes: la.fifteen,
        })
    }

    /// Start real-time monitoring for a workload
    pub async fn start_real_time_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        let metrics = RuntimeMetrics {
            cpu: CpuMetrics::default(),
            memory: MemoryMetrics::default(),
            storage: StorageMetrics::default(),
            network: NetworkMetrics::default(),
            gpu: None,
            timing: TimingMetrics::default(),
        };

        self.workload_metrics
            .write()
            .await
            .insert(workload_id.to_string(), metrics);
        tracing::info!("Started real-time monitoring for workload: {workload_id}");
        Ok(())
    }

    /// Update metrics for a workload in real-time
    pub async fn update_workload_metrics(&self, workload_id: &str) -> ToadStoolResult<()> {
        let cpu_usage = self.get_cpu_usage().await?;
        let (used_memory, total_memory) = self.get_memory_info().await?;
        let (used_storage, total_storage) = self.get_disk_info().await?;
        let network_stats = self.get_network_stats().await?;

        let cpu_count = toadstool_sysmon::cpu_count();

        let updated_metrics = RuntimeMetrics {
            cpu: CpuMetrics {
                usage_percent: cpu_usage,
                #[allow(clippy::cast_precision_loss)]
                cores_used: cpu_usage / 100.0 * cpu_count as f64,
                cpu_time_seconds: cpu_usage / 100.0,
            },
            memory: MemoryMetrics {
                #[allow(clippy::cast_precision_loss)]
                usage_percent: if total_memory > 0 {
                    (used_memory as f64 / total_memory as f64) * 100.0
                } else {
                    0.0
                },
                used_bytes: used_memory,
                peak_bytes: used_memory,
            },
            storage: StorageMetrics {
                #[allow(clippy::cast_precision_loss)]
                usage_percent: if total_storage > 0 {
                    (used_storage as f64 / total_storage as f64) * 100.0
                } else {
                    0.0
                },
                used_bytes: used_storage,
                bytes_read: 0,
                bytes_written: 0,
            },
            network: NetworkMetrics {
                bytes_sent: network_stats.bytes_transmitted,
                bytes_received: network_stats.bytes_received,
                packets_sent: network_stats.packets_transmitted,
                packets_received: network_stats.packets_received,
            },
            gpu: None,
            timing: TimingMetrics {
                start_time: SystemTime::now(),
                end_time: None,
                duration: Duration::ZERO,
            },
        };

        self.workload_metrics
            .write()
            .await
            .insert(workload_id.to_string(), updated_metrics);
        Ok(())
    }
}

impl ResourceMonitor for SystemResourceMonitor {
    fn start_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        tracing::info!("Starting resource monitoring for workload: {workload_id}");

        let metrics = RuntimeMetrics {
            cpu: CpuMetrics::default(),
            memory: MemoryMetrics::default(),
            storage: StorageMetrics::default(),
            network: NetworkMetrics::default(),
            gpu: None,
            timing: TimingMetrics {
                start_time: SystemTime::now(),
                end_time: None,
                duration: Duration::ZERO,
            },
        };

        tokio::spawn({
            let workload_metrics = self.workload_metrics.clone();
            let workload_id = workload_id.to_string();
            async move {
                let mut metrics_map = workload_metrics.write().await;
                metrics_map.insert(workload_id, metrics);
            }
        });

        Ok(())
    }

    fn stop_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        tracing::info!("Stopping resource monitoring for workload: {workload_id}");

        tokio::spawn({
            let workload_metrics = self.workload_metrics.clone();
            let workload_id = workload_id.to_string();
            async move {
                let mut metrics_map = workload_metrics.write().await;
                if let Some(metrics) = metrics_map.get_mut(&workload_id) {
                    let end_time = SystemTime::now();
                    metrics.timing.end_time = Some(end_time);
                    metrics.timing.duration = end_time
                        .duration_since(metrics.timing.start_time)
                        .unwrap_or_default();
                }
            }
        });

        Ok(())
    }

    fn get_metrics(
        &self,
        workload_id: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        let workload_metrics = self.workload_metrics.clone();
        let workload_id = workload_id.to_string();

        Box::pin(async move {
            let metrics_map = workload_metrics.read().await;
            Ok(metrics_map.get(&workload_id).cloned().unwrap_or_default())
        })
    }

    fn get_system_resources(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<SystemResources>> + Send + '_>> {
        Box::pin(async move {
            let cpu_usage_percent = self.get_cpu_usage().await?;
            let total_cpu_cores = toadstool_sysmon::cpu_count();
            #[allow(clippy::cast_precision_loss)]
            let cpu_cores = total_cpu_cores as f64;
            let available_cpu_cores = cpu_cores * (1.0 - cpu_usage_percent / 100.0);

            let mem = toadstool_sysmon::memory_info()
                .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?;
            let memory_usage_percent = if mem.total > 0 {
                #[allow(clippy::cast_precision_loss)]
                let pct = (mem.used as f64 / mem.total as f64) * 100.0;
                pct
            } else {
                0.0
            };

            let disks = toadstool_sysmon::disk_usage()
                .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?;
            let (mut used_space, mut total_space) = (0u64, 0u64);
            for disk in &disks {
                total_space += disk.total_space;
                used_space += disk.total_space - disk.available_space;
            }
            let available_disk = total_space - used_space;

            Ok(SystemResources {
                available_cpu_cores,
                available_memory_bytes: mem.available,
                available_storage_bytes: available_disk,
                available_network_bandwidth: None,
                available_gpu_units: 0,
                cpu_usage_percent,
                memory_usage_percent,
                total_cpu_cores,
                total_memory_bytes: mem.total,
            })
        })
    }
}

impl Default for SystemResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
