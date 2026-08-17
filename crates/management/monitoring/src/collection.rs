// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use std::sync::RwLock;
use tokio::time;
use tracing::{error, info, warn};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::resources::RuntimeMetrics;

use crate::metric_types::ProcessInfo;
use crate::metric_types::SystemResourceMonitor;
use crate::platform;
use crate::types::{MonitoringConfig, ResourceMonitorError};

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
            monitored_workloads: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Updates the monitoring configuration
    pub async fn update_config(&mut self, config: MonitoringConfig) -> ToadStoolResult<()> {
        self.config = config;

        // Restart monitoring with new configuration if currently monitoring
        let is_monitoring = *self.is_monitoring.read().unwrap_or_else(|e| e.into_inner());
        if is_monitoring {
            self.stop_monitoring_loop().await?;
            self.start_monitoring_loop().await?;
        }

        Ok(())
    }

    /// Starts the monitoring loop
    pub async fn start_monitoring_loop(&self) -> Result<(), ToadStoolError> {
        let mut is_monitoring = self
            .is_monitoring
            .write()
            .unwrap_or_else(|e| e.into_inner());
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

            while *is_monitoring_flag.read().unwrap_or_else(|e| e.into_inner()) {
                interval_timer.tick().await;

                // Snapshot process list and release lock before await (avoid holding lock across .await)
                let process_snapshot: Vec<(String, ProcessInfo)> = {
                    let processes = process_map.read().unwrap_or_else(|e| e.into_inner());
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
                let mut usage_data_guard = usage_data.write().unwrap_or_else(|e| e.into_inner());
                let thresholds = threshold_data.read().unwrap_or_else(|e| e.into_inner());

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
            let mut is_monitoring = self
                .is_monitoring
                .write()
                .unwrap_or_else(|e| e.into_inner());
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

impl Default for SystemResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[expect(
        unused_imports,
        reason = "wildcard import used selectively by individual tests"
    )]
    use super::*;
    use crate::{MonitoringConfig, MonitoringGranularity, SystemResourceMonitor, ThresholdAction};
    use std::time::Duration;

    #[test]
    fn new_matches_default_monitoring_config() {
        let a = SystemResourceMonitor::new();
        let b = MonitoringConfig::default();
        assert_eq!(a.config, b);
    }

    #[test]
    fn with_config_stores_custom_fields() {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::LowFrequency,
            enable_network_monitoring: false,
            enable_threshold_monitoring: false,
            threshold_action: ThresholdAction::Terminate,
            metrics_retention: Duration::from_secs(42),
        };
        let m = SystemResourceMonitor::with_config(config.clone());
        assert_eq!(m.config, config);
    }

    #[test]
    fn default_impl_matches_new() {
        let a = SystemResourceMonitor::default();
        let b = SystemResourceMonitor::new();
        assert_eq!(a.config, b.config);
    }

    #[tokio::test]
    async fn not_monitoring_after_construction() {
        let m = SystemResourceMonitor::new();
        assert!(!*m.is_monitoring.read().unwrap_or_else(|e| e.into_inner()));
    }

    #[tokio::test]
    async fn stop_monitoring_loop_without_start_succeeds() {
        let m = SystemResourceMonitor::new();
        assert!(m.stop_monitoring_loop().await.is_ok());
        assert!(!*m.is_monitoring.read().unwrap_or_else(|e| e.into_inner()));
    }

    #[tokio::test]
    async fn start_then_stop_monitoring_loop() {
        let m = SystemResourceMonitor::new();
        assert!(m.start_monitoring_loop().await.is_ok());
        assert!(*m.is_monitoring.read().unwrap_or_else(|e| e.into_inner()));
        assert!(m.stop_monitoring_loop().await.is_ok());
        assert!(!*m.is_monitoring.read().unwrap_or_else(|e| e.into_inner()));
    }

    #[tokio::test]
    async fn start_monitoring_loop_twice_is_idempotent() {
        let m = SystemResourceMonitor::new();
        assert!(m.start_monitoring_loop().await.is_ok());
        assert!(m.start_monitoring_loop().await.is_ok());
        assert!(m.stop_monitoring_loop().await.is_ok());
    }

    #[tokio::test]
    async fn update_config_while_idle_updates_fields() {
        let mut m = SystemResourceMonitor::new();
        let new_config = MonitoringConfig {
            granularity: MonitoringGranularity::Millisecond,
            enable_network_monitoring: true,
            enable_threshold_monitoring: false,
            threshold_action: ThresholdAction::Alert,
            metrics_retention: Duration::from_secs(99),
        };
        assert!(m.update_config(new_config.clone()).await.is_ok());
        assert_eq!(m.config.granularity, MonitoringGranularity::Millisecond);
        assert!(m.config.enable_network_monitoring);
        assert!(!m.config.enable_threshold_monitoring);
        assert_eq!(m.config.threshold_action, ThresholdAction::Alert);
        assert_eq!(m.config.metrics_retention, Duration::from_secs(99));
    }
}
