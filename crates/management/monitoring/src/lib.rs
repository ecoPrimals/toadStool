// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `ToadStool` monitoring component
//!
//! Cross-platform resource monitoring with configurable granularity.

// Module declarations
pub mod platform;
pub mod process;
pub mod thresholds;
pub mod types;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, error, info, warn};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::resources::{
    ResourceMonitor, ResourceRequirements, RuntimeMetrics, SystemResources,
};

// Re-export types for backward compatibility
pub use types::{MonitoringConfig, MonitoringGranularity, ResourceMonitorError, ThresholdAction};

use crate::process::ProcessInfo;

/// Concrete implementation of `ResourceMonitor` trait that provides
/// configurable, high-granularity resource monitoring
#[derive(Debug)]
pub struct SystemResourceMonitor {
    pub(crate) process_map: Arc<RwLock<HashMap<String, ProcessInfo>>>,
    pub(crate) usage_data: Arc<RwLock<HashMap<String, RuntimeMetrics>>>,
    pub(crate) threshold_data: Arc<RwLock<HashMap<String, ResourceRequirements>>>,
    pub(crate) config: MonitoringConfig,
    pub(crate) is_monitoring: Arc<RwLock<bool>>,
}

impl SystemResourceMonitor {
    /// Creates a new `SystemResourceMonitor` instance with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(MonitoringConfig::default())
    }

    /// Creates a new `SystemResourceMonitor` instance with custom configuration
    #[must_use]
    pub fn with_config(config: MonitoringConfig) -> Self {
        Self {
            process_map: Arc::new(RwLock::new(HashMap::new())),
            usage_data: Arc::new(RwLock::new(HashMap::new())),
            threshold_data: Arc::new(RwLock::new(HashMap::new())),
            config,
            is_monitoring: Arc::new(RwLock::new(false)),
        }
    }

    /// Updates the monitoring configuration
    pub async fn update_config(&mut self, config: MonitoringConfig) -> ToadStoolResult<()> {
        self.config = config;

        // Restart monitoring with new configuration if currently monitoring
        let is_monitoring = *self.is_monitoring.read().await;
        if is_monitoring {
            self.stop_monitoring_loop().await?;
            self.start_monitoring_loop().await?;
        }

        Ok(())
    }

    /// Starts the monitoring loop
    pub async fn start_monitoring_loop(&self) -> Result<(), ToadStoolError> {
        let mut is_monitoring = self.is_monitoring.write().await;
        if *is_monitoring {
            return Ok(());
        }

        *is_monitoring = true;
        drop(is_monitoring);
        let interval = self.config.granularity.to_duration();
        info!("Starting resource monitoring with interval {:?}", interval);

        let process_map = Arc::clone(&self.process_map);
        let usage_data = Arc::clone(&self.usage_data);
        let threshold_data = Arc::clone(&self.threshold_data);
        let config = self.config.clone();
        let is_monitoring_flag = Arc::clone(&self.is_monitoring);

        tokio::spawn(async move {
            let mut interval_timer = time::interval(interval);

            while *is_monitoring_flag.read().await {
                interval_timer.tick().await;

                // Snapshot process list and release lock before await (avoid holding lock across .await)
                let process_snapshot: Vec<(String, ProcessInfo)> = {
                    let processes = process_map.read().await;
                    processes
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                };

                let mut updated_metrics = HashMap::new();
                for (workload_id, process_info) in process_snapshot {
                    match Self::measure_process_resources(
                        process_info.pid,
                        &process_info.name,
                        process_info.start_time,
                        &process_info.last_cpu_time,
                        &process_info.memory_usage,
                        &config,
                    )
                    .await
                    {
                        Ok(metrics) => {
                            updated_metrics.insert(workload_id.clone(), metrics);
                        }
                        Err(err) => {
                            warn!(
                                "Failed to measure resources for process {}: {}",
                                workload_id, err
                            );
                        }
                    }
                }

                // Update usage data and check thresholds
                let mut usage_data_guard = usage_data.write().await;
                let thresholds = threshold_data.read().await;

                for (workload_id, metrics) in updated_metrics {
                    usage_data_guard.insert(workload_id.clone(), metrics.clone());

                    // Check thresholds if enabled
                    if config.enable_threshold_monitoring
                        && let Some(requirements) = thresholds.get(&workload_id)
                        && let Err(err) = Self::check_thresholds(
                            &workload_id,
                            &metrics,
                            requirements,
                            &config.threshold_action,
                        )
                    {
                        error!("Threshold violation: {}", err);
                    }
                }
                drop(usage_data_guard);
                drop(thresholds);
            }
        });

        Ok(())
    }

    /// Stops the monitoring loop
    pub async fn stop_monitoring_loop(&self) -> ToadStoolResult<()> {
        {
            let mut is_monitoring = self.is_monitoring.write().await;
            *is_monitoring = false;
        }
        info!("Stopped resource monitoring");
        Ok(())
    }

    /// Measures resources for a specific process
    async fn measure_process_resources(
        pid: u32,
        _name: &str,
        start_time: u64,
        last_cpu_time: &u64,
        memory_usage: &u64,
        config: &MonitoringConfig,
    ) -> Result<RuntimeMetrics, ResourceMonitorError> {
        let elapsed_secs = start_time.saturating_sub(*last_cpu_time);

        // Get platform-specific metrics
        let mut platform_metrics = platform::get_platform_metrics(pid, config).await?;

        // Update timing information
        platform_metrics.timing.start_time = SystemTime::now() - Duration::from_secs(elapsed_secs);
        platform_metrics.timing.end_time = None;
        platform_metrics.timing.duration = Duration::from_secs(elapsed_secs);

        // Update CPU usage
        platform_metrics.cpu.usage_percent = *last_cpu_time as f64 / 100.0;
        platform_metrics.cpu.cores_used = 1.0;
        platform_metrics.cpu.cpu_time_seconds = *last_cpu_time as f64 / 1000.0;

        // Update memory usage
        platform_metrics.memory.used_bytes = *memory_usage;
        platform_metrics.memory.peak_bytes = *memory_usage;
        platform_metrics.memory.usage_percent = (*memory_usage as f64 / 1024.0 / 1024.0) * 100.0;

        // Update storage usage
        platform_metrics.storage.usage_percent = 0.0;
        platform_metrics.storage.used_bytes = 0;
        platform_metrics.storage.bytes_read = 0;
        platform_metrics.storage.bytes_written = 0;

        Ok(platform_metrics)
    }
}

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
            // Get system-wide resource information
            let mut total_cpu_cores = 1usize;
            let mut total_memory_bytes = 1024 * 1024 * 1024u64; // 1GB default
            let mut available_memory_bytes = total_memory_bytes;
            let available_storage_bytes = 10 * 1024 * 1024 * 1024u64; // 10GB default

            #[cfg(target_os = "linux")]
            {
                // Get CPU info from /proc/cpuinfo
                if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
                    total_cpu_cores = cpuinfo
                        .lines()
                        .filter(|line| line.starts_with("processor"))
                        .count()
                        .max(1);
                }

                // Get memory info from /proc/meminfo
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
                // Use sysctl for macOS
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
                            // macOS doesn't have MemAvailable, estimate at 50%
                            available_memory_bytes = mem_bytes / 2;
                        }
                    }
                }
            }

            // Calculate usage percentages
            let memory_usage_percent = if total_memory_bytes > 0 {
                let used = total_memory_bytes.saturating_sub(available_memory_bytes);
                (used as f64 / total_memory_bytes as f64) * 100.0
            } else {
                0.0
            };

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

impl Default for SystemResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Duration;
    use toadstool::resources::{CpuRequirements, MemoryRequirements, StorageRequirements};

    #[test]
    fn system_resource_monitor_new() {
        let monitor = SystemResourceMonitor::new();
        let _ = monitor;
    }

    #[test]
    fn system_resource_monitor_with_config() {
        let config = MonitoringConfig::default();
        let monitor = SystemResourceMonitor::with_config(config);
        let _ = monitor;
    }

    #[test]
    fn system_resource_monitor_default() {
        let monitor = SystemResourceMonitor::default();
        let _ = monitor;
    }

    #[tokio::test]
    async fn register_and_unregister_process() {
        let monitor = SystemResourceMonitor::new();
        let path = std::path::Path::new("test_executable");
        monitor
            .register_process("workload-1", 12345, path)
            .await
            .unwrap();
        monitor.unregister_process("workload-1").await.unwrap();
    }

    #[tokio::test]
    async fn unregister_nonexistent_returns_error() {
        let monitor = SystemResourceMonitor::new();
        let result = monitor.unregister_process("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_metrics_nonexistent_returns_error() {
        let monitor = SystemResourceMonitor::new();
        let result = monitor.get_metrics_async("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn start_and_stop_monitoring_loop() {
        let monitor = SystemResourceMonitor::new();
        monitor.start_monitoring_loop().await.unwrap();
        monitor.stop_monitoring_loop().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // update_config tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn update_config_when_not_monitoring() {
        let mut monitor = SystemResourceMonitor::new();
        let new_config = MonitoringConfig {
            granularity: MonitoringGranularity::LowFrequency,
            enable_network_monitoring: false,
            enable_threshold_monitoring: false,
            threshold_action: ThresholdAction::Log,
            metrics_retention: Duration::from_secs(1800),
        };
        let result = monitor.update_config(new_config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_config_while_monitoring() {
        let mut monitor = SystemResourceMonitor::new();
        monitor.start_monitoring_loop().await.unwrap();

        let new_config = MonitoringConfig {
            granularity: MonitoringGranularity::HighFrequency,
            enable_network_monitoring: true,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Alert,
            metrics_retention: Duration::from_secs(7200),
        };
        let result = monitor.update_config(new_config).await;
        assert!(result.is_ok());

        monitor.stop_monitoring_loop().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // ResourceMonitor trait method tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn resource_monitor_start_monitoring() {
        let monitor = SystemResourceMonitor::new();
        let result = monitor.start_monitoring("workload-1");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn resource_monitor_stop_monitoring() {
        let monitor = SystemResourceMonitor::new();
        let result = monitor.stop_monitoring("workload-1");
        assert!(result.is_ok());
        // Yield to allow the spawned task to complete
        tokio::task::yield_now().await;
    }

    #[tokio::test]
    async fn resource_monitor_get_metrics_trait() {
        let monitor = SystemResourceMonitor::new();
        let result = monitor.get_metrics("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn resource_monitor_get_metrics_registered() {
        let monitor = SystemResourceMonitor::new();
        let path = Path::new("test_exec");
        monitor.register_process("w1", 12345, path).await.unwrap();
        // No metrics yet (monitoring loop not started), so get_metrics still fails
        let result = monitor.get_metrics("w1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn resource_monitor_get_system_resources() {
        let monitor = SystemResourceMonitor::new();
        let result = monitor.get_system_resources().await;
        assert!(result.is_ok());
        let resources = result.unwrap();
        assert!(resources.total_cpu_cores >= 1);
        assert!(resources.total_memory_bytes > 0);
        assert!(resources.available_memory_bytes > 0);
    }

    // -------------------------------------------------------------------------
    // register_process edge cases
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn register_process_path_without_filename() {
        let monitor = SystemResourceMonitor::new();
        // Path::new(".") has no file_name, uses unwrap_or_default -> ""
        let path = Path::new(".");
        let result = monitor.register_process("w1", 12345, path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn register_process_empty_path() {
        let monitor = SystemResourceMonitor::new();
        let path = Path::new("");
        let result = monitor.register_process("w1", 12345, path).await;
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // set_thresholds and get_metrics with monitoring loop
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn set_thresholds_then_get_metrics_after_monitoring_tick() {
        let monitor = SystemResourceMonitor::new();
        let path = Path::new("test_exec");
        let pid = std::process::id();

        monitor.register_process("w1", pid, path).await.unwrap();
        monitor
            .set_thresholds("w1", ResourceRequirements::default())
            .await
            .unwrap();

        monitor.start_monitoring_loop().await.unwrap();
        // Poll until metrics appear (confirms at least one monitoring tick ran)
        let metrics = tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                match monitor.get_metrics_async("w1").await {
                    Ok(m) => break m,
                    Err(_) => tokio::task::yield_now().await,
                }
            }
        })
        .await
        .expect("metrics should appear within 500ms");
        assert!(metrics.cpu.usage_percent >= 0.0);
        assert!(
            metrics.memory.used_bytes <= metrics.memory.peak_bytes
                || metrics.memory.peak_bytes == 0
        );

        monitor.stop_monitoring_loop().await.unwrap();
    }

    #[tokio::test]
    async fn set_thresholds_with_custom_requirements() {
        let monitor = SystemResourceMonitor::new();
        let requirements = ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 0.5,
                max_cores: Some(4.0),
                architecture: None,
            },
            memory: MemoryRequirements {
                min_bytes: 256 * 1024 * 1024,
                max_bytes: Some(8 * 1024 * 1024 * 1024),
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024,
                max_bytes: Some(100 * 1024 * 1024 * 1024),
                storage_type: None,
            },
            gpu: None,
            network: toadstool::resources::NetworkRequirements::default(),
        };
        let result = monitor
            .set_thresholds("workload-threshold", requirements)
            .await;
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // Monitoring loop with threshold checking (exercises check_thresholds)
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn monitoring_loop_with_threshold_violation_log_action() {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::HighFrequency,
            enable_network_monitoring: false,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Log,
            metrics_retention: Duration::from_secs(3600),
        };
        let monitor = SystemResourceMonitor::with_config(config);
        let path = Path::new("test_exec");
        let pid = std::process::id();

        monitor.register_process("w1", pid, path).await.unwrap();
        // Set very low memory threshold so we likely trigger violation
        let requirements = ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 0.1,
                max_cores: Some(0.001),
                architecture: None,
            },
            memory: MemoryRequirements {
                min_bytes: 1024,
                max_bytes: Some(1),
            },
            storage: StorageRequirements {
                min_bytes: 1024,
                max_bytes: Some(0),
                storage_type: None,
            },
            gpu: None,
            network: toadstool::resources::NetworkRequirements::default(),
        };
        monitor.set_thresholds("w1", requirements).await.unwrap();

        monitor.start_monitoring_loop().await.unwrap();
        // Poll until metrics appear (confirms at least one monitoring tick ran)
        tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                if monitor.get_metrics_async("w1").await.is_ok() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("monitoring tick should complete within 500ms");
        monitor.stop_monitoring_loop().await.unwrap();
    }

    #[tokio::test]
    async fn monitoring_loop_with_threshold_alert_action() {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::HighFrequency,
            enable_network_monitoring: false,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Alert,
            metrics_retention: Duration::from_secs(3600),
        };
        let monitor = SystemResourceMonitor::with_config(config);
        let path = Path::new("test_exec");
        let pid = std::process::id();

        monitor.register_process("w1", pid, path).await.unwrap();
        let requirements = ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 0.1,
                max_cores: Some(0.0001),
                architecture: None,
            },
            memory: MemoryRequirements {
                min_bytes: 1024,
                max_bytes: Some(1),
            },
            storage: toadstool::resources::StorageRequirements::default(),
            gpu: None,
            network: toadstool::resources::NetworkRequirements::default(),
        };
        monitor.set_thresholds("w1", requirements).await.unwrap();

        monitor.start_monitoring_loop().await.unwrap();
        // Poll until metrics appear (confirms at least one monitoring tick ran)
        tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                if monitor.get_metrics_async("w1").await.is_ok() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("monitoring tick should complete within 500ms");
        monitor.stop_monitoring_loop().await.unwrap();
    }

    #[tokio::test]
    async fn start_monitoring_loop_idempotent() {
        let monitor = SystemResourceMonitor::new();
        let r1 = monitor.start_monitoring_loop().await;
        let r2 = monitor.start_monitoring_loop().await;
        assert!(r1.is_ok());
        assert!(r2.is_ok());
        monitor.stop_monitoring_loop().await.unwrap();
    }
}
