// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform-specific resource measurement
//!
//! Linux, macOS, and Windows implementations for process metrics.

#[cfg(target_os = "macos")]
use std::process::Command;

use crate::types::{MonitoringConfig, ResourceMonitorError};

#[cfg(target_os = "linux")]
use toadstool_common::constants::platform_paths::procfs;

/// Total system memory in bytes, discovered at runtime via `/proc/meminfo`.
/// Falls back to 4 `GiB` if `/proc` is unavailable (macOS, Windows).
fn total_system_memory_bytes() -> f64 {
    toadstool_sysmon::memory_info()
        .map(|m| m.total as f64)
        .unwrap_or(4.0 * 1024.0 * 1024.0 * 1024.0)
}

/// Gets platform-specific metrics for a process
pub async fn get_platform_metrics(
    pid: u32,
    config: &MonitoringConfig,
) -> Result<toadstool::resources::RuntimeMetrics, ResourceMonitorError> {
    #[cfg(target_os = "linux")]
    {
        measure_linux_resources(pid, config).await
    }
    #[cfg(target_os = "macos")]
    {
        measure_macos_resources(pid, config).await
    }
    #[cfg(target_os = "windows")]
    {
        measure_windows_resources(pid, config).await
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
) -> Result<toadstool::resources::RuntimeMetrics, ResourceMonitorError> {
    use std::fs;

    // Read from /proc/[pid]/stat for CPU info
    let stat_path = procfs::proc_pid_stat(pid);
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
        .map_err(|e: std::num::ParseIntError| ResourceMonitorError::ParseError(e.to_string()))?;
    let stime: u64 = stat_fields[14]
        .parse()
        .map_err(|e: std::num::ParseIntError| ResourceMonitorError::ParseError(e.to_string()))?;

    // Read memory info from /proc/[pid]/status
    let status_path = procfs::proc_pid_status(pid);
    let status_content = fs::read_to_string(&status_path)
        .map_err(|e| ResourceMonitorError::CommandExecutionFailed(e.to_string()))?;

    let vm_rss = parse_proc_status_value(&status_content, "VmRSS")?;
    let _vm_size = parse_proc_status_value(&status_content, "VmSize")?;

    // Read IO stats from /proc/[pid]/io
    let io_path = procfs::proc_pid_io(pid);
    let io_content = fs::read_to_string(&io_path).unwrap_or_else(|e| {
        tracing::warn!(
            path = %io_path,
            error = %e,
            "failed to read process IO stats; storage read/write counters will be zero"
        );
        String::new()
    });

    let read_bytes = parse_proc_io_value(&io_content, "read_bytes").unwrap_or(0);
    let write_bytes = parse_proc_io_value(&io_content, "write_bytes").unwrap_or(0);

    // Network monitoring if enabled
    let network_metrics = if config.enable_network_monitoring {
        match measure_linux_network_stats(pid).await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    pid,
                    error = %e,
                    "linux network stats unavailable; using default network metrics"
                );
                toadstool::resources::NetworkMetrics::default()
            }
        }
    } else {
        toadstool::resources::NetworkMetrics::default()
    };

    Ok(toadstool::resources::RuntimeMetrics {
        cpu: toadstool::resources::CpuMetrics {
            usage_percent: (utime + stime) as f64 / 100.0, // Simplified CPU calculation
            cores_used: 1.0,
            cpu_time_seconds: (utime + stime) as f64 / 100.0,
        },
        memory: toadstool::resources::MemoryMetrics {
            used_bytes: (vm_rss * 1024.0) as u64, // VmRSS is in KB
            peak_bytes: (vm_rss * 1024.0) as u64,
            usage_percent: (vm_rss * 1024.0 * 100.0) / total_system_memory_bytes(),
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
#[allow(
    clippy::similar_names,
    reason = "tx_bytes/rx_bytes are domain-standard names"
)]
#[expect(
    clippy::unused_async,
    reason = "kept for API consistency with async resource monitoring"
)]
async fn measure_linux_network_stats(
    pid: u32,
) -> Result<toadstool::resources::NetworkMetrics, ResourceMonitorError> {
    use std::fs;

    // Read network stats from /proc/[pid]/net/dev
    let net_dev_path = procfs::proc_pid_net_dev(pid);
    let net_content = fs::read_to_string(&net_dev_path)
        .or_else(|_| fs::read_to_string(procfs::PROC_NET_DEV)) // Fallback to system-wide stats
        .map_err(|_e| ResourceMonitorError::NetworkMonitoringNotAvailable)?;

    let mut rx_bytes_total = 0u64;
    let mut tx_bytes_total = 0u64;
    let mut rx_packets_total = 0u64;
    let mut tx_packets_total = 0u64;

    // Parse network interface statistics
    for line in net_content.lines().skip(2) {
        // Skip header lines
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 17 {
            // RX bytes, packets, TX bytes, packets
            if let (Ok(rb), Ok(rp), Ok(tb), Ok(tp)) = (
                parts[1].parse::<u64>(),
                parts[2].parse::<u64>(),
                parts[9].parse::<u64>(),
                parts[10].parse::<u64>(),
            ) {
                rx_bytes_total += rb;
                rx_packets_total += rp;
                tx_bytes_total += tb;
                tx_packets_total += tp;
            }
        }
    }

    Ok(toadstool::resources::NetworkMetrics {
        bytes_received: rx_bytes_total,
        bytes_sent: tx_bytes_total,
        packets_received: rx_packets_total,
        packets_sent: tx_packets_total,
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
) -> Result<toadstool::resources::RuntimeMetrics, ResourceMonitorError> {
    // Use ps command for macOS
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid,pcpu,rss,vsz"])
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
        measure_macos_network_stats(pid).await.unwrap_or_default()
    } else {
        toadstool::resources::NetworkMetrics::default()
    };

    Ok(toadstool::resources::RuntimeMetrics {
        cpu: toadstool::resources::CpuMetrics {
            usage_percent: cpu_percent,
            cores_used: 1.0,
            cpu_time_seconds: 0.0,
        },
        memory: toadstool::resources::MemoryMetrics {
            used_bytes: rss_kb * 1024,
            peak_bytes: rss_kb * 1024,
            usage_percent: (rss_kb * 1024 * 100) as f64 / total_system_memory_bytes(),
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
        .args(["-ib"])
        .output()
        .map_err(|_| ResourceMonitorError::NetworkMonitoringNotAvailable)?;

    let output_str = String::from_utf8(output.stdout)
        .map_err(|e| ResourceMonitorError::ParseError(e.to_string()))?;

    let mut total_rx_bytes = 0u64;
    let mut total_tx_bytes = 0u64;

    // Parse netstat output (simplified)
    for line in output_str.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 8 {
            if let (Ok(rx_bytes), Ok(tx_bytes)) = (parts[6].parse::<u64>(), parts[9].parse::<u64>())
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
) -> Result<toadstool::resources::RuntimeMetrics, ResourceMonitorError> {
    use std::process::Command;

    // Use PowerShell for Windows
    let ps_command = format!(
        "Get-Process -Id {} | Select-Object CPU,WorkingSet,VirtualMemorySize",
        pid
    );

    let output = Command::new("powershell")
        .args(["-Command", &ps_command])
        .output()
        .map_err(|e| ResourceMonitorError::CommandExecutionFailed(e.to_string()))?;

    let output_str = String::from_utf8(output.stdout)
        .map_err(|e| ResourceMonitorError::ParseError(e.to_string()))?;

    // Parse PowerShell output (simplified)
    let cpu_percent = parse_powershell_value(&output_str, "CPU").unwrap_or(0.0);
    let working_set = parse_powershell_value(&output_str, "WorkingSet").unwrap_or(0.0) as u64;

    // Network monitoring for Windows
    let network_metrics = if config.enable_network_monitoring {
        measure_windows_network_stats(pid).await.unwrap_or_default()
    } else {
        toadstool::resources::NetworkMetrics::default()
    };

    Ok(toadstool::resources::RuntimeMetrics {
        cpu: toadstool::resources::CpuMetrics {
            usage_percent: cpu_percent,
            cores_used: 1.0,
            cpu_time_seconds: 0.0,
        },
        memory: toadstool::resources::MemoryMetrics {
            used_bytes: working_set,
            peak_bytes: working_set,
            usage_percent: (working_set * 100) as f64 / total_system_memory_bytes(),
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
    use std::process::Command;

    // Use PowerShell to get network statistics
    let ps_command = "Get-Counter '\\Network Interface(*)\\Bytes Received/sec', '\\Network Interface(*)\\Bytes Sent/sec' | ForEach-Object {$_.CounterSamples}";

    let output = Command::new("powershell")
        .args(["-Command", ps_command])
        .output()
        .map_err(|_| ResourceMonitorError::NetworkMonitoringNotAvailable)?;

    let output_str = String::from_utf8(output.stdout)
        .map_err(|e| ResourceMonitorError::ParseError(e.to_string()))?;

    // Simplified parsing - would need more robust implementation
    let _total_rx_bytes = 0u64;
    let _total_tx_bytes = 0u64;

    // This is a simplified implementation
    // In practice, you'd need more sophisticated PowerShell parsing

    Ok(toadstool::resources::NetworkMetrics {
        bytes_received: _total_rx_bytes,
        bytes_sent: _total_tx_bytes,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_system_memory_is_positive() {
        let mem = total_system_memory_bytes();
        assert!(mem > 0.0);
    }

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::*;

        #[test]
        fn parse_proc_status_vm_rss() {
            let status = "Name:\ttest\nVmRSS:\t 12345 kB\nVmSize:\t 99999 kB\n";
            let val = parse_proc_status_value(status, "VmRSS").unwrap();
            assert!((val - 12345.0).abs() < f64::EPSILON);
        }

        #[test]
        fn parse_proc_status_missing_field() {
            let status = "Name:\ttest\n";
            let err = parse_proc_status_value(status, "VmRSS");
            assert!(err.is_err());
        }

        #[test]
        fn parse_proc_io_read_bytes() {
            let io = "rchar: 123\nwchar: 456\nread_bytes: 789\nwrite_bytes: 101112\n";
            let val = parse_proc_io_value(io, "read_bytes").unwrap();
            assert_eq!(val, 789);
        }

        #[test]
        fn parse_proc_io_write_bytes() {
            let io = "rchar: 123\nwchar: 456\nread_bytes: 789\nwrite_bytes: 101112\n";
            let val = parse_proc_io_value(io, "write_bytes").unwrap();
            assert_eq!(val, 101_112);
        }

        #[test]
        fn parse_proc_io_missing_field() {
            let io = "rchar: 123\n";
            let err = parse_proc_io_value(io, "write_bytes");
            assert!(err.is_err());
        }

        #[tokio::test]
        async fn get_platform_metrics_for_current_process() {
            let pid = std::process::id();
            let config = MonitoringConfig::default();
            let result = get_platform_metrics(pid, &config).await;
            assert!(result.is_ok(), "metrics for own pid should succeed");
            let m = result.unwrap();
            assert!(m.memory.used_bytes > 0);
        }

        #[tokio::test]
        async fn get_platform_metrics_invalid_pid_fails() {
            let config = MonitoringConfig::default();
            let result = get_platform_metrics(u32::MAX, &config).await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn get_platform_metrics_with_network() {
            let pid = std::process::id();
            let config = MonitoringConfig {
                enable_network_monitoring: true,
                ..Default::default()
            };
            let result = get_platform_metrics(pid, &config).await;
            assert!(result.is_ok());
        }
    }
}
