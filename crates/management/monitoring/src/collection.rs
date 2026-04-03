// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::RwLock;
use tokio::time;
use tracing::{error, info, warn};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::resources::RuntimeMetrics;

use crate::metric_types::SystemResourceMonitor;
use crate::platform;
use crate::process::ProcessInfo;
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

impl Default for SystemResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
