//! Resource monitoring for ToadStool

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use sysinfo::System;
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

/// Real system resource monitor using sysinfo
pub struct SystemResourceMonitor {
    pub(super) system: Arc<RwLock<System>>,
    workload_metrics: Arc<RwLock<HashMap<String, RuntimeMetrics>>>,
}

impl SystemResourceMonitor {
    #[must_use]
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system: Arc::new(RwLock::new(system)),
            workload_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn refresh_system(&self) -> ToadStoolResult<()> {
        let mut system = self.system.write().await;
        system.refresh_all();
        Ok(())
    }

    async fn get_cpu_usage(&self) -> ToadStoolResult<f64> {
        self.refresh_system().await?;
        let mut system = self.system.write().await;
        system.refresh_cpu();
        let cpu_usage = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>()
            / system.cpus().len().max(1) as f32;
        Ok(f64::from(cpu_usage))
    }

    async fn get_memory_info(&self) -> ToadStoolResult<(u64, u64)> {
        self.refresh_system().await?;
        let system = self.system.read().await;
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        Ok((used_memory, total_memory))
    }

    async fn get_disk_info(&self) -> ToadStoolResult<(u64, u64)> {
        self.refresh_system().await?;
        let _system = self.system.read().await;
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let (mut used_space, mut total_space) = (0, 0);

        for disk in &disks {
            total_space += disk.total_space();
            used_space += disk.total_space() - disk.available_space();
        }

        Ok((used_space, total_space))
    }

    /// Get detailed process information for a specific workload
    pub async fn get_process_info(
        &self,
        workload_id: &str,
    ) -> ToadStoolResult<Option<ProcessInfo>> {
        self.refresh_system().await?;
        let system = self.system.read().await;

        let processes = system.processes();
        let process_count = processes.len();
        let total_cpu_time = processes
            .values()
            .map(|p| f64::from(p.cpu_usage()))
            .sum::<f64>();

        Ok(Some(ProcessInfo {
            workload_id: workload_id.to_string(),
            process_count,
            total_cpu_time,
            memory_usage: processes
                .values()
                .map(sysinfo::Process::memory)
                .sum::<u64>(),
            status: ProcessStatus::Running,
        }))
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> ToadStoolResult<NetworkStats> {
        self.refresh_system().await?;
        let _system = self.system.read().await;
        let networks = sysinfo::Networks::new_with_refreshed_list();

        let mut total_received = 0;
        let mut total_transmitted = 0;
        let mut total_packets_received = 0;
        let mut total_packets_transmitted = 0;

        for (_interface_name, data) in &networks {
            total_received += data.received();
            total_transmitted += data.transmitted();
            total_packets_received += data.packets_received();
            total_packets_transmitted += data.packets_transmitted();
        }

        Ok(NetworkStats {
            bytes_received: total_received,
            bytes_transmitted: total_transmitted,
            packets_received: total_packets_received,
            packets_transmitted: total_packets_transmitted,
            interfaces: networks.len(),
        })
    }

    /// Get system load averages (Unix-like systems)
    pub async fn get_load_averages(&self) -> ToadStoolResult<LoadAverages> {
        self.refresh_system().await?;
        let _system = self.system.read().await;

        #[cfg(unix)]
        {
            let load_avg = System::load_average();
            Ok(LoadAverages {
                one_minute: load_avg.one,
                five_minutes: load_avg.five,
                fifteen_minutes: load_avg.fifteen,
            })
        }

        #[cfg(not(unix))]
        {
            let cpu_usage = self.get_cpu_usage().await?;
            let estimated_load = cpu_usage / 100.0 * _system.cpus().len() as f64;
            Ok(LoadAverages {
                one_minute: estimated_load,
                five_minutes: estimated_load,
                fifteen_minutes: estimated_load,
            })
        }
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
        tracing::info!("Started real-time monitoring for workload: {}", workload_id);
        Ok(())
    }

    /// Update metrics for a workload in real-time
    pub async fn update_workload_metrics(&self, workload_id: &str) -> ToadStoolResult<()> {
        let cpu_usage = self.get_cpu_usage().await?;
        let (used_memory, total_memory) = self.get_memory_info().await?;
        let (used_storage, total_storage) = self.get_disk_info().await?;
        let network_stats = self.get_network_stats().await?;

        let updated_metrics = RuntimeMetrics {
            cpu: CpuMetrics {
                usage_percent: cpu_usage,
                cores_used: cpu_usage / 100.0 * self.system.read().await.cpus().len() as f64,
                cpu_time_seconds: cpu_usage / 100.0,
            },
            memory: MemoryMetrics {
                usage_percent: (used_memory as f64 / total_memory as f64) * 100.0,
                used_bytes: used_memory,
                peak_bytes: used_memory,
            },
            storage: StorageMetrics {
                usage_percent: (used_storage as f64 / total_storage as f64) * 100.0,
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
        tracing::info!("Starting resource monitoring for workload: {}", workload_id);

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
        tracing::info!("Stopping resource monitoring for workload: {}", workload_id);

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
            let mut system = self.system.write().await;
            system.refresh_all();

            system.refresh_cpu();
            let total_cpu_cores = system.cpus().len();
            let cpu_usage_percent = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>()
                / total_cpu_cores as f32;
            let cpu_usage_percent = f64::from(cpu_usage_percent);
            let cpu_cores = total_cpu_cores as f64;
            let available_cpu_cores = cpu_cores * (1.0 - cpu_usage_percent / 100.0);

            let total_memory = system.total_memory();
            let used_memory = system.used_memory();
            let available_memory = total_memory - used_memory;
            let memory_usage_percent = if total_memory > 0 {
                (used_memory as f64 / total_memory as f64) * 100.0
            } else {
                0.0
            };

            drop(system);

            let disks = sysinfo::Disks::new_with_refreshed_list();
            let (mut used_space, mut total_space) = (0, 0);

            for disk in &disks {
                total_space += disk.total_space();
                used_space += disk.total_space() - disk.available_space();
            }
            let available_disk = total_space - used_space;

            let available_gpu_units = 0;

            Ok(SystemResources {
                available_cpu_cores,
                available_memory_bytes: available_memory,
                available_storage_bytes: available_disk,
                available_network_bandwidth: None,
                available_gpu_units,
                cpu_usage_percent,
                memory_usage_percent,
                total_cpu_cores,
                total_memory_bytes: total_memory,
            })
        })
    }
}

impl Default for SystemResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
