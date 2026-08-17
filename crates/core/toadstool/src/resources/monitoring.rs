// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource monitoring for ToadStool

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[cfg(feature = "runtime")]
use std::sync::RwLock;

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
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`; the `Result` type is reserved for future failures.
    fn start_monitoring(&self, workload_id: &str) -> ToadStoolResult<()>;

    /// Stop monitoring resources for a workload
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`; the `Result` type is reserved for future failures.
    fn stop_monitoring(&self, workload_id: &str) -> ToadStoolResult<()>;

    /// Get current resource metrics (async to avoid blocking)
    fn get_metrics(
        &self,
        workload_id: &str,
    ) -> impl Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_;

    /// Get system resource availability
    fn get_system_resources(
        &self,
    ) -> impl Future<Output = ToadStoolResult<SystemResources>> + Send + '_;
}

/// Real system resource monitor using toadstool-sysmon (pure Rust /proc parsing).
#[derive(Clone)]
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
    ///
    /// # Errors
    ///
    /// Returns error if process enumeration fails.
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
    ///
    /// # Errors
    ///
    /// Returns error if network statistics cannot be read.
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
    ///
    /// # Errors
    ///
    /// Returns error if load averages cannot be read.
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
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`.
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
            .unwrap_or_else(|e| e.into_inner())
            .insert(workload_id.to_string(), metrics);
        tracing::info!("Started real-time monitoring for workload: {workload_id}");
        Ok(())
    }

    /// Update metrics for a workload in real-time
    ///
    /// # Errors
    ///
    /// Returns error if any underlying system metric query fails.
    pub async fn update_workload_metrics(&self, workload_id: &str) -> ToadStoolResult<()> {
        let cpu_usage = self.get_cpu_usage().await?;
        let (used_memory, total_memory) = self.get_memory_info().await?;
        let (used_storage, total_storage) = self.get_disk_info().await?;
        let network_stats = self.get_network_stats().await?;

        let cpu_count = toadstool_sysmon::cpu_count();

        let updated_metrics = RuntimeMetrics {
            cpu: CpuMetrics {
                usage_percent: cpu_usage,
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "precision loss acceptable for this conversion"
                )]
                cores_used: cpu_usage / 100.0 * cpu_count as f64,
                cpu_time_seconds: cpu_usage / 100.0,
            },
            memory: MemoryMetrics {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "precision loss acceptable for this conversion"
                )]
                usage_percent: if total_memory > 0 {
                    (used_memory as f64 / total_memory as f64) * 100.0
                } else {
                    0.0
                },
                used_bytes: used_memory,
                peak_bytes: used_memory,
            },
            storage: StorageMetrics {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "precision loss acceptable for this conversion"
                )]
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
            .unwrap_or_else(|e| e.into_inner())
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
                let mut metrics_map = workload_metrics.write().unwrap_or_else(|e| e.into_inner());
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
                let mut metrics_map = workload_metrics.write().unwrap_or_else(|e| e.into_inner());
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
    ) -> impl Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        let workload_metrics = self.workload_metrics.clone();
        let workload_id = workload_id.to_string();

        async move {
            let metrics_map = workload_metrics.read().unwrap_or_else(|e| e.into_inner());
            Ok(metrics_map.get(&workload_id).cloned().unwrap_or_default())
        }
    }

    fn get_system_resources(
        &self,
    ) -> impl Future<Output = ToadStoolResult<SystemResources>> + Send + '_ {
        async {
            let cpu_usage_percent = self.get_cpu_usage().await?;
            let total_cpu_cores = toadstool_sysmon::cpu_count();
            #[expect(
                clippy::cast_precision_loss,
                reason = "precision loss acceptable for this conversion"
            )]
            let cpu_cores = total_cpu_cores as f64;
            let available_cpu_cores = cpu_cores * (1.0 - cpu_usage_percent / 100.0);

            let mem = toadstool_sysmon::memory_info()
                .map_err(|e| crate::ToadStoolError::Runtime(e.to_string()))?;
            let memory_usage_percent = if mem.total > 0 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "precision loss acceptable for this conversion"
                )]
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
        }
    }
}

/// Test-only monitor behavior (mirrors `toadstool_testing::MockResourceMonitor` presets).
#[cfg(any(test, feature = "test-mocks"))]
#[derive(Debug, Clone, Copy)]
pub enum TestResourceMonitorBehavior {
    /// Successful paths with moderate resource readings.
    Successful,
    /// High usage / constrained resources.
    LimitViolations,
    /// All operations fail with resource errors.
    MonitoringFailure,
}

/// Lightweight resource monitor for tests and the `test-mocks` feature.
#[cfg(any(test, feature = "test-mocks"))]
#[derive(Debug, Clone)]
pub struct TestResourceMonitor {
    behavior: TestResourceMonitorBehavior,
}

#[cfg(any(test, feature = "test-mocks"))]
impl TestResourceMonitor {
    /// Successful monitor (default test readings).
    #[must_use]
    pub fn successful() -> Self {
        Self {
            behavior: TestResourceMonitorBehavior::Successful,
        }
    }

    /// Monitor reporting constrained resources.
    #[must_use]
    pub fn limit_violations() -> Self {
        Self {
            behavior: TestResourceMonitorBehavior::LimitViolations,
        }
    }

    /// Monitor that fails all operations.
    #[must_use]
    pub fn monitoring_failure() -> Self {
        Self {
            behavior: TestResourceMonitorBehavior::MonitoringFailure,
        }
    }
}

#[cfg(any(test, feature = "test-mocks"))]
impl ResourceMonitor for TestResourceMonitor {
    fn start_monitoring(&self, _workload_id: &str) -> ToadStoolResult<()> {
        match self.behavior {
            TestResourceMonitorBehavior::MonitoringFailure => Err(crate::ToadStoolError::resource(
                "Failed to start monitoring",
            )),
            _ => Ok(()),
        }
    }

    fn stop_monitoring(&self, _workload_id: &str) -> ToadStoolResult<()> {
        match self.behavior {
            TestResourceMonitorBehavior::MonitoringFailure => {
                Err(crate::ToadStoolError::resource("Failed to stop monitoring"))
            }
            _ => Ok(()),
        }
    }

    fn get_metrics(
        &self,
        workload_id: &str,
    ) -> impl Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        let behavior = self.behavior;
        let workload_id = workload_id.to_string();
        async move {
            match behavior {
                TestResourceMonitorBehavior::MonitoringFailure => {
                    Err(crate::ToadStoolError::resource("Failed to get metrics"))
                }
                TestResourceMonitorBehavior::LimitViolations => {
                    let mut metrics = RuntimeMetrics::default();
                    metrics.cpu.usage_percent = 95.0;
                    metrics.memory.used_bytes = 1024 * 1024 * 1024 * 7;
                    Ok(metrics)
                }
                TestResourceMonitorBehavior::Successful => {
                    let mut metrics = RuntimeMetrics::default();
                    metrics.cpu.usage_percent = 42.0;
                    let _ = workload_id;
                    Ok(metrics)
                }
            }
        }
    }

    fn get_system_resources(
        &self,
    ) -> impl Future<Output = ToadStoolResult<SystemResources>> + Send + '_ {
        let behavior = self.behavior;
        async move {
            match behavior {
                TestResourceMonitorBehavior::MonitoringFailure => Err(
                    crate::ToadStoolError::resource("Failed to get system resources"),
                ),
                TestResourceMonitorBehavior::LimitViolations => Ok(SystemResources {
                    available_cpu_cores: 2.0,
                    available_memory_bytes: 4 * 1024 * 1024 * 1024,
                    available_storage_bytes: 100 * 1024 * 1024 * 1024,
                    available_network_bandwidth: Some(100_000_000),
                    available_gpu_units: 0,
                    cpu_usage_percent: 75.0,
                    memory_usage_percent: 87.5,
                    total_cpu_cores: 8,
                    total_memory_bytes: 8 * 1024 * 1024 * 1024,
                }),
                TestResourceMonitorBehavior::Successful => Ok(SystemResources {
                    available_cpu_cores: 8.0,
                    available_memory_bytes: 16 * 1024 * 1024 * 1024,
                    available_storage_bytes: 1024 * 1024 * 1024 * 1024,
                    available_network_bandwidth: Some(1_000_000_000),
                    available_gpu_units: 1,
                    cpu_usage_percent: 25.0,
                    memory_usage_percent: 50.0,
                    total_cpu_cores: 16,
                    total_memory_bytes: 32 * 1024 * 1024 * 1024,
                }),
            }
        }
    }
}

/// Enum dispatch for [`ResourceMonitor`] (replaces `dyn ResourceMonitor` at type-erased boundaries).
#[derive(Clone)]
pub enum ResourceMonitorDispatch {
    /// Live system monitor.
    System(SystemResourceMonitor),
    /// Test monitor (`test-mocks` / unit tests).
    #[cfg(any(test, feature = "test-mocks"))]
    Test(TestResourceMonitor),
}

impl std::fmt::Debug for ResourceMonitorDispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceMonitorDispatch::System(_) => f.write_str("ResourceMonitorDispatch::System"),
            #[cfg(any(test, feature = "test-mocks"))]
            ResourceMonitorDispatch::Test(t) => f
                .debug_tuple("ResourceMonitorDispatch::Test")
                .field(t)
                .finish(),
        }
    }
}

impl ResourceMonitor for ResourceMonitorDispatch {
    fn start_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        match self {
            Self::System(m) => m.start_monitoring(workload_id),
            #[cfg(any(test, feature = "test-mocks"))]
            Self::Test(t) => t.start_monitoring(workload_id),
        }
    }

    fn stop_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        match self {
            Self::System(m) => m.stop_monitoring(workload_id),
            #[cfg(any(test, feature = "test-mocks"))]
            Self::Test(t) => t.stop_monitoring(workload_id),
        }
    }

    fn get_metrics(
        &self,
        workload_id: &str,
    ) -> impl Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        let dispatch = self.clone();
        let workload_id = workload_id.to_string();
        async move {
            match &dispatch {
                ResourceMonitorDispatch::System(m) => m.get_metrics(&workload_id).await,
                #[cfg(any(test, feature = "test-mocks"))]
                ResourceMonitorDispatch::Test(t) => t.get_metrics(&workload_id).await,
            }
        }
    }

    fn get_system_resources(
        &self,
    ) -> impl Future<Output = ToadStoolResult<SystemResources>> + Send + '_ {
        let dispatch = self.clone();
        async move {
            match &dispatch {
                ResourceMonitorDispatch::System(m) => m.get_system_resources().await,
                #[cfg(any(test, feature = "test-mocks"))]
                ResourceMonitorDispatch::Test(t) => t.get_system_resources().await,
            }
        }
    }
}

impl From<SystemResourceMonitor> for ResourceMonitorDispatch {
    fn from(m: SystemResourceMonitor) -> Self {
        Self::System(m)
    }
}

#[cfg(any(test, feature = "test-mocks"))]
impl From<TestResourceMonitor> for ResourceMonitorDispatch {
    fn from(m: TestResourceMonitor) -> Self {
        Self::Test(m)
    }
}

impl Default for SystemResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
