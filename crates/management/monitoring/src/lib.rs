#![deny(unsafe_code)]

//! `ToadStool` monitoring component
//!
//! Cross-platform resource monitoring with configurable granularity.

// Module declarations
pub mod types;

use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, error, info, warn};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::resources::{
    ResourceMonitor, ResourceRequirements, RuntimeMetrics, SystemResources,
};

// Re-export types for backward compatibility
pub use types::{MonitoringConfig, MonitoringGranularity, ResourceMonitorError, ThresholdAction};

/// Concrete implementation of `ResourceMonitor` trait that provides
/// configurable, high-granularity resource monitoring
#[derive(Debug)]
pub struct SystemResourceMonitor {
    process_map: Arc<RwLock<HashMap<String, ProcessInfo>>>,
    usage_data: Arc<RwLock<HashMap<String, RuntimeMetrics>>>,
    threshold_data: Arc<RwLock<HashMap<String, ResourceRequirements>>>,
    config: MonitoringConfig,
    is_monitoring: Arc<RwLock<bool>>,
}

#[derive(Clone, Debug)]
struct ProcessInfo {
    pid: u32,
    name: String,
    #[allow(dead_code)] // Used when computing RuntimeMetrics in monitoring loop
    cpu_usage: f64,
    last_cpu_time: u64,
    memory_usage: u64,
    start_time: u64,
}

#[derive(Default, Clone, Debug)]
#[allow(dead_code)] // Reserved for future network monitoring aggregation
struct NetworkStats {
    bytes_received: u64,
    bytes_transmitted: u64,
    packets_received: u64,
    packets_transmitted: u64,
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
        SystemResourceMonitor {
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

    /// Registers a process for resource monitoring
    pub async fn register_process(
        &self,
        workload_id: &str,
        process_handle: u32,
        executable_path: &Path,
    ) -> Result<(), ToadStoolError> {
        let mut process_map = self.process_map.write().await;
        let process_info = ProcessInfo {
            pid: process_handle,
            name: executable_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            cpu_usage: 0.0,
            last_cpu_time: 0,
            memory_usage: 0,
            start_time: Instant::now().elapsed().as_secs(),
        };

        process_map.insert(workload_id.to_string(), process_info);
        info!(
            "Registered process {} with PID {} for monitoring",
            workload_id, process_handle
        );
        Ok(())
    }

    /// Unregisters a process from monitoring
    pub async fn unregister_process(&self, workload_id: &str) -> Result<(), ToadStoolError> {
        let mut process_map = self.process_map.write().await;
        let mut usage_data = self.usage_data.write().await;
        let mut threshold_data = self.threshold_data.write().await;

        if process_map.remove(workload_id).is_some() {
            usage_data.remove(workload_id);
            threshold_data.remove(workload_id);
            info!("Unregistered process {} from monitoring", workload_id);
            Ok(())
        } else {
            Err(ResourceMonitorError::ProcessNotRegistered(workload_id.to_string()).into())
        }
    }

    /// Sets resource thresholds for a workload
    pub async fn set_thresholds(
        &self,
        workload_id: &str,
        requirements: ResourceRequirements,
    ) -> ToadStoolResult<()> {
        let mut threshold_data = self.threshold_data.write().await;
        threshold_data.insert(workload_id.to_string(), requirements);
        debug!("Set thresholds for workload: {}", workload_id);
        Ok(())
    }

    /// Starts the monitoring loop
    pub async fn start_monitoring_loop(&self) -> Result<(), ToadStoolError> {
        let mut is_monitoring = self.is_monitoring.write().await;
        if *is_monitoring {
            return Ok(());
        }

        *is_monitoring = true;
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

                let processes = process_map.read().await;
                let mut updated_metrics = HashMap::new();

                for (workload_id, process_info) in processes.iter() {
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
                    if config.enable_threshold_monitoring {
                        if let Some(requirements) = thresholds.get(&workload_id) {
                            if let Err(err) = Self::check_thresholds(
                                &workload_id,
                                &metrics,
                                requirements,
                                &config.threshold_action,
                            ) {
                                error!("Threshold violation: {}", err);
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stops the monitoring loop
    pub async fn stop_monitoring_loop(&self) -> ToadStoolResult<()> {
        let mut is_monitoring = self.is_monitoring.write().await;
        *is_monitoring = false;
        info!("Stopped resource monitoring");
        Ok(())
    }

    /// Gets current metrics for a workload (async version)
    pub async fn get_metrics_async(&self, workload_id: &str) -> ToadStoolResult<RuntimeMetrics> {
        let usage_data = self.usage_data.read().await;
        usage_data.get(workload_id).cloned().ok_or_else(|| {
            ResourceMonitorError::ProcessNotRegistered(workload_id.to_string()).into()
        })
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
        let mut platform_metrics = Self::get_platform_metrics(pid, config).await?;

        // Update timing information
        platform_metrics.timing.start_time =
            Utc::now() - chrono::Duration::seconds(elapsed_secs as i64);
        platform_metrics.timing.end_time = None;
        platform_metrics.timing.duration =
            chrono::Duration::from_std(std::time::Duration::from_secs(elapsed_secs))
                .unwrap_or_default();

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

        // Network monitoring if enabled
        let network_metrics = if config.enable_network_monitoring {
            Self::measure_linux_network_stats(pid)
                .await
                .unwrap_or_default()
        } else {
            toadstool::resources::NetworkMetrics::default()
        };

        platform_metrics.network = network_metrics;

        Ok(platform_metrics)
    }

    /// Gets platform-specific metrics
    async fn get_platform_metrics(
        pid: u32,
        config: &MonitoringConfig,
    ) -> Result<RuntimeMetrics, ResourceMonitorError> {
        #[cfg(target_os = "linux")]
        {
            Self::measure_linux_resources(pid, config).await
        }
        #[cfg(target_os = "macos")]
        {
            Self::measure_macos_resources(pid, config).await
        }
        #[cfg(target_os = "windows")]
        {
            Self::measure_windows_resources(pid, config).await
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(ResourceMonitorError::PlatformNotSupported(
                std::env::consts::OS.to_string(),
            ))
        }
    }

    #[cfg(target_os = "linux")]
    async fn measure_linux_resources(
        pid: u32,
        config: &MonitoringConfig,
    ) -> Result<RuntimeMetrics, ResourceMonitorError> {
        use std::fs;

        // Read from /proc/[pid]/stat for CPU info
        let stat_path = format!("/proc/{pid}/stat");
        let stat_content = fs::read_to_string(&stat_path)
            .map_err(|e| ResourceMonitorError::CommandExecutionFailed(e.to_string()))?;

        let stat_fields: Vec<&str> = stat_content.split_whitespace().collect();
        if stat_fields.len() < 24 {
            return Err(ResourceMonitorError::ParseError(
                "Invalid stat format".to_string(),
            ));
        }

        // Parse CPU times (user time + system time)
        let utime: u64 = stat_fields[13]
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                ResourceMonitorError::ParseError(e.to_string())
            })?;
        let stime: u64 = stat_fields[14]
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                ResourceMonitorError::ParseError(e.to_string())
            })?;

        // Read memory info from /proc/[pid]/status
        let status_path = format!("/proc/{pid}/status");
        let status_content = fs::read_to_string(&status_path)
            .map_err(|e| ResourceMonitorError::CommandExecutionFailed(e.to_string()))?;

        let vm_rss = Self::parse_proc_status_value(&status_content, "VmRSS")?;
        let _vm_size = Self::parse_proc_status_value(&status_content, "VmSize")?;

        // Read IO stats from /proc/[pid]/io
        let io_path = format!("/proc/{pid}/io");
        let io_content = fs::read_to_string(&io_path).unwrap_or_default();

        let read_bytes = Self::parse_proc_io_value(&io_content, "read_bytes").unwrap_or(0);
        let write_bytes = Self::parse_proc_io_value(&io_content, "write_bytes").unwrap_or(0);

        // Network monitoring if enabled
        let network_metrics = if config.enable_network_monitoring {
            Self::measure_linux_network_stats(pid)
                .await
                .unwrap_or_default()
        } else {
            toadstool::resources::NetworkMetrics::default()
        };

        Ok(RuntimeMetrics {
            cpu: toadstool::resources::CpuMetrics {
                usage_percent: (utime + stime) as f64 / 100.0, // Simplified CPU calculation
                cores_used: 1.0,
                cpu_time_seconds: (utime + stime) as f64 / 100.0,
            },
            memory: toadstool::resources::MemoryMetrics {
                used_bytes: (vm_rss * 1024.0) as u64, // VmRSS is in KB
                peak_bytes: (vm_rss * 1024.0) as u64,
                usage_percent: (vm_rss * 1024.0 * 100.0) / (4.0 * 1024.0 * 1024.0 * 1024.0), // Assume 4GB total
            },
            storage: toadstool::resources::StorageMetrics {
                usage_percent: 0.0,
                used_bytes: 0,
                bytes_read: read_bytes,
                bytes_written: write_bytes,
            },
            network: network_metrics,
            gpu: None,
            timing: toadstool::resources::TimingMetrics::default(),
        })
    }

    #[cfg(target_os = "linux")]
    async fn measure_linux_network_stats(
        pid: u32,
    ) -> Result<toadstool::resources::NetworkMetrics, ResourceMonitorError> {
        use std::fs;

        // Read network stats from /proc/[pid]/net/dev
        let net_dev_path = format!("/proc/{pid}/net/dev");
        let net_content = fs::read_to_string(&net_dev_path)
            .or_else(|_| fs::read_to_string("/proc/net/dev")) // Fallback to system-wide stats
            .map_err(|_e| ResourceMonitorError::NetworkMonitoringNotAvailable)?;

        let mut total_rx_bytes = 0u64;
        let mut total_tx_bytes = 0u64;
        let mut total_rx_packets = 0u64;
        let mut total_tx_packets = 0u64;

        // Parse network interface statistics
        for line in net_content.lines().skip(2) {
            // Skip header lines
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 17 {
                // RX bytes, packets, TX bytes, packets
                if let (Ok(rx_bytes), Ok(rx_packets), Ok(tx_bytes), Ok(tx_packets)) = (
                    parts[1].parse::<u64>(),
                    parts[2].parse::<u64>(),
                    parts[9].parse::<u64>(),
                    parts[10].parse::<u64>(),
                ) {
                    total_rx_bytes += rx_bytes;
                    total_rx_packets += rx_packets;
                    total_tx_bytes += tx_bytes;
                    total_tx_packets += tx_packets;
                }
            }
        }

        Ok(toadstool::resources::NetworkMetrics {
            bytes_received: total_rx_bytes,
            bytes_sent: total_tx_bytes,
            packets_received: total_rx_packets,
            packets_sent: total_tx_packets,
        })
    }

    #[cfg(target_os = "linux")]
    fn parse_proc_status_value(status: &str, field: &str) -> Result<f64, ResourceMonitorError> {
        for line in status.lines() {
            if line.starts_with(field) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1]
                        .parse::<f64>()
                        .map_err(|e| ResourceMonitorError::ParseError(e.to_string()));
                }
            }
        }
        Err(ResourceMonitorError::ParseError(format!(
            "Field {field} not found"
        )))
    }

    #[cfg(target_os = "linux")]
    fn parse_proc_io_value(io_content: &str, field: &str) -> Result<u64, ResourceMonitorError> {
        for line in io_content.lines() {
            if line.starts_with(field) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1]
                        .parse::<u64>()
                        .map_err(|e| ResourceMonitorError::ParseError(e.to_string()));
                }
            }
        }
        Err(ResourceMonitorError::ParseError(format!(
            "Field {field} not found"
        )))
    }

    #[cfg(target_os = "macos")]
    async fn measure_macos_resources(
        pid: u32,
        config: &MonitoringConfig,
    ) -> Result<RuntimeMetrics, ResourceMonitorError> {
        // Use ps command for macOS
        let output = Command::new("ps")
            .args(&["-p", &pid.to_string(), "-o", "pid,pcpu,rss,vsz"])
            .output()
            .map_err(|e| ResourceMonitorError::CommandExecutionFailed(e.to_string()))?;

        let output_str = String::from_utf8(output.stdout)
            .map_err(|e| ResourceMonitorError::ParseError(e.to_string()))?;

        // Parse ps output
        let lines: Vec<&str> = output_str.lines().collect();
        if lines.len() < 2 {
            return Err(ResourceMonitorError::ParseError(
                "Invalid ps output".to_string(),
            ));
        }

        let fields: Vec<&str> = lines[1].split_whitespace().collect();
        if fields.len() < 4 {
            return Err(ResourceMonitorError::ParseError(
                "Invalid ps fields".to_string(),
            ));
        }

        let cpu_percent: f64 = fields[1]
            .parse()
            .map_err(|e| ResourceMonitorError::ParseError(e.to_string()))?;
        let rss_kb: u64 = fields[2]
            .parse()
            .map_err(|e| ResourceMonitorError::ParseError(e.to_string()))?;

        // Network monitoring for macOS (simplified)
        let network_metrics = if config.enable_network_monitoring {
            Self::measure_macos_network_stats(pid)
                .await
                .unwrap_or_default()
        } else {
            toadstool::resources::NetworkMetrics::default()
        };

        Ok(RuntimeMetrics {
            cpu: toadstool::resources::CpuMetrics {
                usage_percent: cpu_percent,
                cores_used: 1.0,
                cpu_time_seconds: 0.0,
            },
            memory: toadstool::resources::MemoryMetrics {
                used_bytes: rss_kb * 1024,
                peak_bytes: rss_kb * 1024,
                usage_percent: (rss_kb * 1024 * 100) as f64 / (4.0 * 1024.0 * 1024.0 * 1024.0), // Assume 4GB total
            },
            storage: toadstool::resources::StorageMetrics::default(),
            network: network_metrics,
            gpu: None,
            timing: toadstool::resources::TimingMetrics::default(),
        })
    }

    #[cfg(target_os = "macos")]
    async fn measure_macos_network_stats(
        _pid: u32,
    ) -> Result<toadstool::resources::NetworkMetrics, ResourceMonitorError> {
        // Use netstat for macOS network statistics
        let output = Command::new("netstat")
            .args(&["-ib"])
            .output()
            .map_err(|e| ResourceMonitorError::NetworkMonitoringNotAvailable)?;

        let output_str = String::from_utf8(output.stdout)
            .map_err(|e| ResourceMonitorError::ParseError(e.to_string()))?;

        let mut total_rx_bytes = 0u64;
        let mut total_tx_bytes = 0u64;

        // Parse netstat output (simplified)
        for line in output_str.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 8 {
                if let (Ok(rx_bytes), Ok(tx_bytes)) =
                    (parts[6].parse::<u64>(), parts[9].parse::<u64>())
                {
                    total_rx_bytes += rx_bytes;
                    total_tx_bytes += tx_bytes;
                }
            }
        }

        Ok(toadstool::resources::NetworkMetrics {
            bytes_received: total_rx_bytes,
            bytes_sent: total_tx_bytes,
            packets_received: 0, // Would need additional parsing
            packets_sent: 0,
        })
    }

    #[cfg(target_os = "windows")]
    async fn measure_windows_resources(
        pid: u32,
        config: &MonitoringConfig,
    ) -> Result<RuntimeMetrics, ResourceMonitorError> {
        // Use PowerShell for Windows
        let ps_command = format!(
            "Get-Process -Id {} | Select-Object CPU,WorkingSet,VirtualMemorySize",
            pid
        );

        let output = Command::new("powershell")
            .args(&["-Command", &ps_command])
            .output()
            .map_err(|e| ResourceMonitorError::CommandExecutionFailed(e.to_string()))?;

        let output_str = String::from_utf8(output.stdout)
            .map_err(|e| ResourceMonitorError::ParseError(e.to_string()))?;

        // Parse PowerShell output (simplified)
        let cpu_percent = Self::parse_powershell_value(&output_str, "CPU").unwrap_or(0.0);
        let working_set =
            Self::parse_powershell_value(&output_str, "WorkingSet").unwrap_or(0.0) as u64;

        // Network monitoring for Windows
        let network_metrics = if config.enable_network_monitoring {
            Self::measure_windows_network_stats(pid)
                .await
                .unwrap_or_default()
        } else {
            toadstool::resources::NetworkMetrics::default()
        };

        Ok(RuntimeMetrics {
            cpu: toadstool::resources::CpuMetrics {
                usage_percent: cpu_percent,
                cores_used: 1.0,
                cpu_time_seconds: 0.0,
            },
            memory: toadstool::resources::MemoryMetrics {
                used_bytes: working_set,
                peak_bytes: working_set,
                usage_percent: (working_set * 100) as f64 / (4.0 * 1024.0 * 1024.0 * 1024.0), // Assume 4GB total
            },
            storage: toadstool::resources::StorageMetrics::default(),
            network: network_metrics,
            gpu: None,
            timing: toadstool::resources::TimingMetrics::default(),
        })
    }

    #[cfg(target_os = "windows")]
    async fn measure_windows_network_stats(
        _pid: u32,
    ) -> Result<toadstool::resources::NetworkMetrics, ResourceMonitorError> {
        // Use PowerShell to get network statistics
        let ps_command = "Get-Counter '\\Network Interface(*)\\Bytes Received/sec', '\\Network Interface(*)\\Bytes Sent/sec' | ForEach-Object {$_.CounterSamples}";

        let output = Command::new("powershell")
            .args(&["-Command", ps_command])
            .output()
            .map_err(|e| ResourceMonitorError::NetworkMonitoringNotAvailable)?;

        let output_str = String::from_utf8(output.stdout)
            .map_err(|e| ResourceMonitorError::ParseError(e.to_string()))?;

        // Simplified parsing - would need more robust implementation
        let mut total_rx_bytes = 0u64;
        let mut total_tx_bytes = 0u64;

        // This is a simplified implementation
        // In practice, you'd need more sophisticated PowerShell parsing

        Ok(toadstool::resources::NetworkMetrics {
            bytes_received: total_rx_bytes,
            bytes_sent: total_tx_bytes,
            packets_received: 0,
            packets_sent: 0,
        })
    }

    #[cfg(target_os = "windows")]
    fn parse_powershell_value(output: &str, field: &str) -> Result<f64, ResourceMonitorError> {
        for line in output.lines() {
            if line.contains(field) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(value_str) = parts.last() {
                    return value_str
                        .parse::<f64>()
                        .map_err(|e| ResourceMonitorError::ParseError(e.to_string()));
                }
            }
        }
        Err(ResourceMonitorError::ParseError(format!(
            "Field {} not found",
            field
        )))
    }

    /// Check thresholds and take action if exceeded
    fn check_thresholds(
        workload_id: &str,
        metrics: &RuntimeMetrics,
        requirements: &ResourceRequirements,
        action: &ThresholdAction,
    ) -> Result<(), ResourceMonitorError> {
        let mut violations = Vec::new();

        // Check CPU threshold
        if let Some(max_cores) = requirements.cpu.max_cores {
            let cpu_cores_used = metrics.cpu.usage_percent / 100.0;
            if cpu_cores_used > max_cores {
                violations.push(ResourceMonitorError::ThresholdViolation {
                    workload_id: workload_id.to_string(),
                    resource_type: "CPU".to_string(),
                    current_value: cpu_cores_used,
                    threshold: max_cores,
                });
            }
        }

        // Check memory threshold
        if let Some(max_memory) = requirements.memory.max_bytes {
            if metrics.memory.used_bytes > max_memory {
                violations.push(ResourceMonitorError::ThresholdViolation {
                    workload_id: workload_id.to_string(),
                    resource_type: "Memory".to_string(),
                    current_value: metrics.memory.used_bytes as f64,
                    threshold: max_memory as f64,
                });
            }
        }

        // Check storage threshold
        if let Some(max_storage) = requirements.storage.max_bytes {
            let storage_used = metrics.storage.bytes_read + metrics.storage.bytes_written;
            if storage_used > max_storage {
                violations.push(ResourceMonitorError::ThresholdViolation {
                    workload_id: workload_id.to_string(),
                    resource_type: "Storage".to_string(),
                    current_value: storage_used as f64,
                    threshold: max_storage as f64,
                });
            }
        }

        // Handle violations based on action
        if !violations.is_empty() {
            for violation in &violations {
                match action {
                    ThresholdAction::Log => {
                        warn!("Threshold violation: {}", violation);
                    }
                    ThresholdAction::Alert => {
                        error!("ALERT: Threshold violation: {}", violation);
                        // In a real implementation, this would send alerts to monitoring systems
                    }
                    ThresholdAction::Terminate => {
                        error!("TERMINATING: Threshold violation: {}", violation);
                        // In a real implementation, this would terminate the process
                        return Err(violation.clone());
                    }
                }
            }
        }

        Ok(())
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
            let mut process_map = process_map.write().await;
            let mut usage_data = usage_data.write().await;
            let mut threshold_data = threshold_data.write().await;
            process_map.remove(&workload_id);
            usage_data.remove(&workload_id);
            threshold_data.remove(&workload_id);
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
                            if let Some(value) = line.split_whitespace().nth(1) {
                                if let Ok(mem_kb) = value.parse::<u64>() {
                                    total_memory_bytes = mem_kb * 1024;
                                }
                            }
                        } else if line.starts_with("MemAvailable:") {
                            if let Some(value) = line.split_whitespace().nth(1) {
                                if let Ok(mem_kb) = value.parse::<u64>() {
                                    available_memory_bytes = mem_kb * 1024;
                                }
                            }
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
