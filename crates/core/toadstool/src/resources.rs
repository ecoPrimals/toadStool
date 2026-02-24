//! Resource management and monitoring for `ToadStool`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::RwLock;

use crate::ToadStoolResult;
// ToadStoolError is available from crate::ToadStoolResult import

/// Resource requirements specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceRequirements {
    /// CPU requirements
    pub cpu: CpuRequirements,
    /// Memory requirements  
    pub memory: MemoryRequirements,
    /// Storage requirements
    pub storage: StorageRequirements,
    /// GPU requirements (optional)
    pub gpu: Option<GpuRequirements>,
    /// Network requirements
    pub network: NetworkRequirements,
}

/// CPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    /// Minimum CPU cores
    pub min_cores: f64,
    /// Maximum CPU cores
    pub max_cores: Option<f64>,
    /// CPU architecture requirement
    pub architecture: Option<String>,
}

impl Default for CpuRequirements {
    fn default() -> Self {
        Self {
            min_cores: 1.0,
            max_cores: None,
            architecture: None,
        }
    }
}

/// Memory requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    /// Minimum memory in bytes
    pub min_bytes: u64,
    /// Maximum memory in bytes
    pub max_bytes: Option<u64>,
}

impl Default for MemoryRequirements {
    fn default() -> Self {
        Self {
            min_bytes: 1024 * 1024 * 1024, // 1GB
            max_bytes: None,
        }
    }
}

/// Storage requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Minimum storage in bytes
    pub min_bytes: u64,
    /// Maximum storage in bytes
    pub max_bytes: Option<u64>,
    /// Storage type requirement
    pub storage_type: Option<String>,
}

impl Default for StorageRequirements {
    fn default() -> Self {
        Self {
            min_bytes: 1024 * 1024 * 1024, // 1GB
            max_bytes: None,
            storage_type: None,
        }
    }
}

/// Network requirements
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkRequirements {
    /// Minimum bandwidth in bytes per second
    pub min_bandwidth: Option<u64>,
    /// Maximum bandwidth in bytes per second
    pub max_bandwidth: Option<u64>,
    /// Latency requirement in milliseconds
    pub max_latency_ms: Option<u64>,
}

impl ResourceRequirements {
    /// Validate that the requirements are internally consistent.
    ///
    /// Returns `Err` if any mandatory field contains an obviously invalid value
    /// (e.g. zero CPU cores or zero memory).
    pub fn validate(&self) -> ToadStoolResult<()> {
        use crate::ToadStoolError;
        if self.cpu.min_cores <= 0.0 {
            return Err(ToadStoolError::validation(
                "cpu.min_cores must be greater than 0",
            ));
        }
        if self.memory.min_bytes == 0 {
            return Err(ToadStoolError::validation(
                "memory.min_bytes must be greater than 0",
            ));
        }
        Ok(())
    }
}

/// GPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Minimum GPU units
    pub min_units: u32,
    /// Maximum GPU units
    pub max_units: Option<u32>,
    /// GPU type requirement
    pub gpu_type: Option<String>,
    /// Minimum GPU memory in bytes
    pub min_memory_bytes: Option<u64>,
}

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
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Execution end time
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Total execution duration
    pub duration: chrono::Duration,
}

impl Default for TimingMetrics {
    fn default() -> Self {
        Self {
            start_time: chrono::Utc::now(),
            end_time: None,
            duration: chrono::Duration::seconds(0),
        }
    }
}

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
    pub execution_timeout: Option<chrono::Duration>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limits: CpuLimits::default(),
            memory_limits: MemoryLimits::default(),
            storage_limits: StorageLimits::default(),
            network_limits: NetworkLimits::default(),
            execution_timeout: Some(chrono::Duration::seconds(300)),
        }
    }
}

/// Snapshot of actual resource consumption during or after execution.
///
/// Tracks observed resource usage in contrast to `ResourceRequirements` (requested)
/// and `ResourceLimits` (maximum permitted).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage (0.0–100.0 × n_cores)
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
    pub workload_id: String,
    pub process_count: usize,
    pub total_cpu_time: f64,
    pub memory_usage: u64,
    pub status: ProcessStatus,
}

/// Process status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Unknown,
}

/// Network statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub packets_received: u64,
    pub packets_transmitted: u64,
    pub interfaces: usize,
}

/// System load averages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadAverages {
    pub one_minute: f64,
    pub five_minutes: f64,
    pub fifteen_minutes: f64,
}

/// Real system resource monitor using sysinfo
pub struct SystemResourceMonitor {
    system: Arc<RwLock<System>>,
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

        // In a real implementation, we would track PIDs associated with workloads
        // For now, we'll return aggregate process information
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

        // Load averages are platform-specific
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
            // For non-Unix systems, estimate load from CPU usage
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
                cpu_time_seconds: cpu_usage / 100.0, // Approximation
            },
            memory: MemoryMetrics {
                usage_percent: (used_memory as f64 / total_memory as f64) * 100.0,
                used_bytes: used_memory,
                peak_bytes: used_memory, // Would track peak in real implementation
            },
            storage: StorageMetrics {
                usage_percent: (used_storage as f64 / total_storage as f64) * 100.0,
                used_bytes: used_storage,
                bytes_read: 0, // Would track I/O in real implementation
                bytes_written: 0,
            },
            network: NetworkMetrics {
                bytes_sent: network_stats.bytes_transmitted,
                bytes_received: network_stats.bytes_received,
                packets_sent: network_stats.packets_transmitted,
                packets_received: network_stats.packets_received,
            },
            gpu: None, // Would integrate with GPU monitoring libraries
            timing: TimingMetrics {
                start_time: chrono::Utc::now(),
                end_time: None,
                duration: chrono::Duration::zero(),
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

        // Initialize metrics for this workload
        let metrics = RuntimeMetrics {
            cpu: CpuMetrics::default(),
            memory: MemoryMetrics::default(),
            storage: StorageMetrics::default(),
            network: NetworkMetrics::default(),
            gpu: None,
            timing: TimingMetrics {
                start_time: chrono::Utc::now(),
                end_time: None,
                duration: chrono::Duration::zero(),
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
                    metrics.timing.end_time = Some(chrono::Utc::now());
                    if let Some(end_time) = metrics.timing.end_time {
                        metrics.timing.duration = end_time - metrics.timing.start_time;
                    }
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
            // Refresh system information ONCE (optimization: was called 4 times before!)
            let mut system = self.system.write().await;
            system.refresh_all();

            // Get CPU info (without additional refresh)
            // sysinfo 0.30 API: refresh then get usage
            system.refresh_cpu();
            let total_cpu_cores = system.cpus().len();
            let cpu_usage_percent = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>()
                / total_cpu_cores as f32;
            let cpu_usage_percent = f64::from(cpu_usage_percent);
            let cpu_cores = total_cpu_cores as f64;
            let available_cpu_cores = cpu_cores * (1.0 - cpu_usage_percent / 100.0);

            // Get memory info (without additional refresh)
            let total_memory = system.total_memory();
            let used_memory = system.used_memory();
            let available_memory = total_memory - used_memory;
            let memory_usage_percent = if total_memory > 0 {
                (used_memory as f64 / total_memory as f64) * 100.0
            } else {
                0.0
            };

            // Release the write lock before disk I/O
            drop(system);

            // Get disk info (this can be slow, do it last)
            let disks = sysinfo::Disks::new_with_refreshed_list();
            let (mut used_space, mut total_space) = (0, 0);

            for disk in &disks {
                total_space += disk.total_space();
                used_space += disk.total_space() - disk.available_space();
            }
            let available_disk = total_space - used_space;

            // For now, we don't have a reliable way to detect GPU units without additional crates
            // This could be enhanced with nvidia-ml-py or similar
            let available_gpu_units = 0;

            Ok(SystemResources {
                available_cpu_cores,
                available_memory_bytes: available_memory,
                available_storage_bytes: available_disk,
                available_network_bandwidth: None, // Would need additional network monitoring
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_requirements_default() {
        let req = ResourceRequirements::default();
        assert_eq!(req.cpu.min_cores, 1.0);
        assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024);
        assert_eq!(req.storage.min_bytes, 1024 * 1024 * 1024);
        assert!(req.gpu.is_none());
    }

    #[test]
    fn test_cpu_requirements_default() {
        let cpu = CpuRequirements::default();
        assert_eq!(cpu.min_cores, 1.0);
        assert!(cpu.max_cores.is_none());
        assert!(cpu.architecture.is_none());
    }

    #[test]
    fn test_memory_requirements_default() {
        let mem = MemoryRequirements::default();
        assert_eq!(mem.min_bytes, 1024 * 1024 * 1024);
        assert!(mem.max_bytes.is_none());
    }

    #[test]
    fn test_storage_requirements_default() {
        let storage = StorageRequirements::default();
        assert_eq!(storage.min_bytes, 1024 * 1024 * 1024);
        assert!(storage.max_bytes.is_none());
        assert!(storage.storage_type.is_none());
    }

    #[test]
    fn test_network_requirements_default() {
        let network = NetworkRequirements::default();
        assert!(network.min_bandwidth.is_none());
        assert!(network.max_bandwidth.is_none());
        assert!(network.max_latency_ms.is_none());
    }

    #[test]
    fn test_gpu_requirements() {
        let gpu = GpuRequirements {
            min_units: 1,
            max_units: Some(4),
            gpu_type: Some("NVIDIA".to_string()),
            min_memory_bytes: Some(2 * 1024 * 1024 * 1024),
        };

        assert_eq!(gpu.min_memory_bytes, Some(2 * 1024 * 1024 * 1024));
        assert_eq!(gpu.gpu_type, Some("NVIDIA".to_string()));
    }

    #[test]
    fn test_resource_requirements_serialization() {
        let req = ResourceRequirements::default();
        let json = serde_json::to_string(&req).expect("Failed to serialize");
        let deserialized: ResourceRequirements =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.cpu.min_cores, req.cpu.min_cores);
        assert_eq!(deserialized.memory.min_bytes, req.memory.min_bytes);
    }

    #[test]
    fn test_cpu_requirements_with_architecture() {
        let cpu = CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(8.0),
            architecture: Some("x86_64".to_string()),
        };

        assert_eq!(cpu.min_cores, 4.0);
        assert_eq!(cpu.max_cores, Some(8.0));
        assert_eq!(cpu.architecture, Some("x86_64".to_string()));
    }

    #[test]
    fn test_memory_requirements_with_max() {
        let mem = MemoryRequirements {
            min_bytes: 2 * 1024 * 1024 * 1024,
            max_bytes: Some(4 * 1024 * 1024 * 1024),
        };

        assert_eq!(mem.min_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(mem.max_bytes, Some(4 * 1024 * 1024 * 1024));
    }

    #[test]
    fn test_storage_requirements_with_type() {
        let storage = StorageRequirements {
            min_bytes: 10 * 1024 * 1024 * 1024,
            max_bytes: Some(50 * 1024 * 1024 * 1024),
            storage_type: Some("SSD".to_string()),
        };

        assert_eq!(storage.storage_type, Some("SSD".to_string()));
    }

    #[test]
    fn test_network_requirements_with_constraints() {
        let network = NetworkRequirements {
            min_bandwidth: Some(1024 * 1024),
            max_bandwidth: Some(10 * 1024 * 1024),
            max_latency_ms: Some(100),
        };

        assert_eq!(network.min_bandwidth, Some(1024 * 1024));
        assert_eq!(network.max_latency_ms, Some(100));
    }

    #[test]
    fn test_resource_requirements_validate_ok() {
        let req = ResourceRequirements::default();
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_resource_requirements_validate_zero_cpu() {
        let mut req = ResourceRequirements::default();
        req.cpu.min_cores = 0.0;
        let result = req.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("cpu"));
    }

    #[test]
    fn test_resource_requirements_validate_zero_memory() {
        let mut req = ResourceRequirements::default();
        req.memory.min_bytes = 0;
        let result = req.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("memory"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_system_resource_monitor_creation() {
        let monitor = SystemResourceMonitor::new();
        let _guard = monitor.system.read().await;
        // If we can read the lock, the monitor was created successfully
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_system_resource_monitor_default() {
        let monitor = SystemResourceMonitor::default();
        let _guard = monitor.system.read().await;
        // If we can read the lock, the monitor was created successfully
    }

    // ─── Resource tracking, limits, and monitoring tests ───────────────────────────

    #[test]
    fn test_resource_usage_is_empty() {
        let usage = ResourceUsage::default();
        assert!(usage.is_empty());
    }

    #[test]
    fn test_resource_usage_not_empty_when_used() {
        let usage = ResourceUsage {
            cpu_usage_percent: 1.0,
            ..Default::default()
        };
        assert!(!usage.is_empty());
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert!(limits.execution_timeout.is_some());
    }

    #[test]
    fn test_resource_limits_serialization() {
        let limits = ResourceLimits::default();
        let json = serde_json::to_string(&limits).expect("serialize");
        let _: ResourceLimits = serde_json::from_str(&json).expect("deserialize");
    }

    #[test]
    fn test_cpu_limits_default() {
        let cpu = CpuLimits::default();
        assert!(cpu.max_cores.is_none());
    }

    #[test]
    fn test_memory_limits_default() {
        let mem = MemoryLimits::default();
        assert!(mem.max_bytes.is_none());
    }

    #[test]
    fn test_storage_limits_default() {
        let storage = StorageLimits::default();
        assert!(storage.max_bytes.is_none());
    }

    #[test]
    fn test_network_limits_default() {
        let net = NetworkLimits::default();
        assert!(net.max_bandwidth.is_none());
    }

    #[test]
    fn test_runtime_metrics_default() {
        let m = RuntimeMetrics::default();
        assert_eq!(m.cpu.usage_percent, 0.0);
        assert_eq!(m.memory.used_bytes, 0);
    }

    #[test]
    fn test_runtime_metrics_serialization() {
        let m = RuntimeMetrics::default();
        let json = serde_json::to_string(&m).expect("serialize");
        let _: RuntimeMetrics = serde_json::from_str(&json).expect("deserialize");
    }

    #[test]
    fn test_system_resources_default() {
        let sr = SystemResources::default();
        assert_eq!(sr.available_cpu_cores, 1.0);
        assert_eq!(sr.total_cpu_cores, 1);
    }

    #[test]
    fn test_system_resources_serialization() {
        let sr = SystemResources::default();
        let json = serde_json::to_string(&sr).expect("serialize");
        let _: SystemResources = serde_json::from_str(&json).expect("deserialize");
    }

    #[test]
    fn test_process_status_variants() {
        for s in [
            ProcessStatus::Running,
            ProcessStatus::Sleeping,
            ProcessStatus::Stopped,
            ProcessStatus::Zombie,
            ProcessStatus::Unknown,
        ] {
            let json = serde_json::to_value(&s).expect("serialize");
            let _: ProcessStatus = serde_json::from_value(json).expect("deserialize");
        }
    }

    #[test]
    fn test_process_info_serialization() {
        let info = ProcessInfo {
            workload_id: "w1".to_string(),
            process_count: 5,
            total_cpu_time: 10.5,
            memory_usage: 1024,
            status: ProcessStatus::Running,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let _: ProcessInfo = serde_json::from_str(&json).expect("deserialize");
    }

    #[test]
    fn test_network_stats_constructor() {
        let stats = NetworkStats {
            bytes_received: 100,
            bytes_transmitted: 200,
            packets_received: 10,
            packets_transmitted: 20,
            interfaces: 2,
        };
        assert_eq!(stats.bytes_received, 100);
        assert_eq!(stats.interfaces, 2);
    }

    #[test]
    fn test_load_averages_constructor() {
        let load = LoadAverages {
            one_minute: 0.5,
            five_minutes: 0.4,
            fifteen_minutes: 0.3,
        };
        assert!((load.one_minute - 0.5).abs() < 0.01);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_system_resource_monitor_get_metrics_empty() {
        let monitor = SystemResourceMonitor::new();
        let metrics = monitor.get_metrics("nonexistent").await.unwrap();
        assert_eq!(metrics.cpu.usage_percent, 0.0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_system_resource_monitor_start_real_time_monitoring() {
        let monitor = SystemResourceMonitor::new();
        let result = monitor.start_real_time_monitoring("wl-1").await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_system_resource_monitor_get_process_info() {
        let monitor = SystemResourceMonitor::new();
        let result = monitor.get_process_info("wl-1").await;
        assert!(result.is_ok());
        let info = result.unwrap().unwrap();
        assert_eq!(info.workload_id, "wl-1");
    }

    // ─── Additional resource tracking, limits, monitoring tests ───────────────

    #[test]
    fn test_resource_usage_not_empty_disk_io() {
        let usage = ResourceUsage {
            disk_read_bytes: 1,
            ..Default::default()
        };
        assert!(!usage.is_empty());
    }

    #[test]
    fn test_resource_usage_not_empty_network() {
        let usage = ResourceUsage {
            network_rx_bytes: 100,
            ..Default::default()
        };
        assert!(!usage.is_empty());
    }

    #[test]
    fn test_resource_usage_not_empty_memory() {
        let usage = ResourceUsage {
            memory_used_mb: 1,
            ..Default::default()
        };
        assert!(!usage.is_empty());
    }

    #[test]
    fn test_resource_usage_not_empty_wall_time() {
        let usage = ResourceUsage {
            wall_time_ms: 1,
            ..Default::default()
        };
        assert!(!usage.is_empty());
    }

    #[test]
    fn test_resource_usage_serialization() {
        let usage = ResourceUsage {
            cpu_usage_percent: 50.0,
            memory_used_mb: 1024,
            disk_read_bytes: 1000,
            disk_write_bytes: 500,
            network_rx_bytes: 2000,
            network_tx_bytes: 1000,
            wall_time_ms: 5000,
        };
        let json = serde_json::to_string(&usage).expect("serialize");
        let parsed: ResourceUsage = serde_json::from_str(&json).expect("deserialize");
        assert!((parsed.cpu_usage_percent - 50.0).abs() < 0.01);
        assert_eq!(parsed.memory_used_mb, 1024);
    }

    #[test]
    fn test_gpu_requirements_default_construction() {
        let gpu = GpuRequirements {
            min_units: 0,
            max_units: None,
            gpu_type: None,
            min_memory_bytes: None,
        };
        assert_eq!(gpu.min_units, 0);
    }

    #[test]
    fn test_gpu_requirements_serialization() {
        let gpu = GpuRequirements {
            min_units: 2,
            max_units: Some(4),
            gpu_type: Some("NVIDIA A100".to_string()),
            min_memory_bytes: Some(40 * 1024 * 1024 * 1024),
        };
        let json = serde_json::to_string(&gpu).expect("serialize");
        let _: GpuRequirements = serde_json::from_str(&json).expect("deserialize");
    }

    #[test]
    fn test_cpu_metrics_constructor() {
        let m = CpuMetrics {
            usage_percent: 75.5,
            cores_used: 6.0,
            cpu_time_seconds: 120.0,
        };
        assert!((m.usage_percent - 75.5).abs() < 0.01);
    }

    #[test]
    fn test_storage_metrics_default() {
        let m = StorageMetrics::default();
        assert_eq!(m.bytes_read, 0);
        assert_eq!(m.bytes_written, 0);
    }

    #[test]
    fn test_timing_metrics_serialization() {
        let t = TimingMetrics {
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
            duration: chrono::Duration::seconds(60),
        };
        let json = serde_json::to_string(&t).expect("serialize");
        let _: TimingMetrics = serde_json::from_str(&json).expect("deserialize");
    }

    #[test]
    fn test_load_averages_serialization() {
        let load = LoadAverages {
            one_minute: 1.5,
            five_minutes: 1.2,
            fifteen_minutes: 0.9,
        };
        let json = serde_json::to_string(&load).expect("serialize");
        let parsed: LoadAverages = serde_json::from_str(&json).expect("deserialize");
        assert!((parsed.one_minute - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_resource_limits_with_timeout() {
        let limits = ResourceLimits {
            cpu_limits: CpuLimits::default(),
            memory_limits: MemoryLimits::default(),
            storage_limits: StorageLimits::default(),
            network_limits: NetworkLimits::default(),
            execution_timeout: Some(chrono::Duration::seconds(600)),
        };
        assert_eq!(limits.execution_timeout.unwrap().num_seconds(), 600);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_system_resource_monitor_update_workload_metrics() {
        let monitor = SystemResourceMonitor::new();
        let _ = monitor.start_real_time_monitoring("wl-metrics").await;
        let result = monitor.update_workload_metrics("wl-metrics").await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_resource_monitor_stop_monitoring() {
        let monitor = SystemResourceMonitor::new();
        let _ = monitor.start_monitoring("wl-stop");
        let result = monitor.stop_monitoring("wl-stop");
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_resource_monitor_get_metrics_after_start() {
        let monitor = SystemResourceMonitor::new();
        let _ = monitor.start_monitoring("wl-get");
        let metrics = monitor.get_metrics("wl-get").await.unwrap();
        assert!(metrics.timing.start_time <= chrono::Utc::now());
    }
}
