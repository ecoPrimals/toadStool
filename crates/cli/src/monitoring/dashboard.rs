// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dashboard data collection and aggregation

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

use crate::Result;
use crate::monitoring::collectors::{
    MetricsCollector, NetworkMetricsCollector, SystemMetricsCollector,
};
use crate::monitoring::types::{
    BiomeStatusSummary, HealthStatus, MetricValue, MonitoringSession, PerformanceMetrics,
    SessionStatus, SystemHealth, SystemResourceUsage,
};
use std::collections::HashMap;
use toadstool_common::platform_paths;

/// Collects system health status from metrics
pub fn collect_system_health() -> Result<SystemHealth> {
    let sys_collector = SystemMetricsCollector::new();
    let batch = sys_collector.collect()?;

    let health_from_metric = |name: &str, warn: f64, crit: f64| -> HealthStatus {
        batch
            .metrics
            .iter()
            .find(|m| m.name == name)
            .map_or(HealthStatus::Unknown, |m| match &m.value {
                MetricValue::Gauge(v) if *v >= crit => HealthStatus::Critical,
                MetricValue::Gauge(v) if *v >= warn => HealthStatus::Warning,
                MetricValue::Gauge(_) => HealthStatus::Healthy,
                _ => HealthStatus::Unknown,
            })
    };

    let cpu = health_from_metric("cpu_usage_percent", 80.0, 95.0);
    let memory = health_from_metric("memory_usage_percent", 85.0, 95.0);
    let storage = health_from_metric("storage_usage_percent", 85.0, 95.0);

    let net_collector = NetworkMetricsCollector::new();
    let network = if net_collector.collect().is_ok() {
        HealthStatus::Healthy
    } else {
        HealthStatus::Warning
    };

    let overall = match (&cpu, &memory, &storage, &network) {
        _ if matches!(cpu, HealthStatus::Critical)
            || matches!(memory, HealthStatus::Critical)
            || matches!(storage, HealthStatus::Critical) =>
        {
            HealthStatus::Critical
        }
        _ if matches!(cpu, HealthStatus::Warning)
            || matches!(memory, HealthStatus::Warning)
            || matches!(storage, HealthStatus::Warning)
            || matches!(network, HealthStatus::Warning) =>
        {
            HealthStatus::Warning
        }
        _ => HealthStatus::Healthy,
    };

    Ok(SystemHealth {
        overall_status: overall,
        cpu_health: cpu,
        memory_health: memory,
        storage_health: storage,
        network_health: network,
    })
}

/// Scans runtime directories for running biomes and returns status summaries
pub fn collect_biome_status() -> Result<Vec<BiomeStatusSummary>> {
    let mut biomes = Vec::new();

    let primary = platform_paths::biomeos_runtime_dir();
    let fallback = platform_paths::toadstool_temp_dir();
    let fallback_biomeos = fallback.join("biomeos");

    let dirs_to_scan: Vec<_> = [primary, fallback, fallback_biomeos]
        .into_iter()
        .filter(|d| d.exists() && d.is_dir())
        .collect();

    for dir in dirs_to_scan {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let is_socket = path.extension().is_some_and(|e| e == "sock")
                && path.metadata().is_ok_and(|m| {
                    #[cfg(unix)]
                    {
                        m.file_type().is_socket()
                    }
                    #[cfg(not(unix))]
                    {
                        m.file_type().is_file()
                    }
                });
            let is_pid = path.extension().is_some_and(|e| e == "pid");

            if !is_socket && !is_pid {
                continue;
            }

            let (services_running, services_total, cpu_usage, memory_usage, uptime) = if is_pid {
                std::fs::read_to_string(&path).map_or(
                    (1, 1, 0.0, 0.0, std::time::Duration::ZERO),
                    |contents| {
                        contents.trim().parse::<u32>().map_or(
                            (1, 1, 0.0, 0.0, std::time::Duration::ZERO),
                            |pid| {
                                toadstool_sysmon::process_info(pid).ok().flatten().map_or(
                                    (0, 1, 0.0, 0.0, std::time::Duration::ZERO),
                                    |p| {
                                        let cpu = f64::from(p.cpu_usage);
                                        #[expect(
                                            clippy::cast_precision_loss,
                                            reason = "precision loss acceptable for this conversion"
                                        )]
                                        let mem = p.memory as f64 / 1_073_741_824.0;
                                        let uptime_secs = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs()
                                            .saturating_sub(p.start_time);
                                        (
                                            1,
                                            1,
                                            cpu,
                                            mem,
                                            std::time::Duration::from_secs(uptime_secs),
                                        )
                                    },
                                )
                            },
                        )
                    },
                )
            } else {
                (1, 1, 0.0, 0.0, std::time::Duration::ZERO)
            };

            if !biomes.iter().any(|b: &BiomeStatusSummary| b.name == name) {
                biomes.push(BiomeStatusSummary {
                    name: name.to_string(),
                    status: "running".to_string(),
                    services_running,
                    services_total,
                    cpu_usage,
                    memory_usage,
                    uptime,
                });
            }
        }
    }

    Ok(biomes)
}

/// Collects system resource usage metrics
#[expect(
    clippy::cast_precision_loss,
    reason = "precision loss acceptable for this conversion"
)]
pub fn collect_resource_usage() -> Result<SystemResourceUsage> {
    let cpu_percent = f64::from(
        toadstool_sysmon::cpu_usage(std::time::Duration::from_millis(100)).unwrap_or(0.0),
    );

    let mem = toadstool_sysmon::memory_info().map_err(|e| crate::CliError::Other(e.to_string()))?;
    let memory_total_gb = mem.total as f64 / 1_073_741_824.0;
    let memory_used_gb = mem.used as f64 / 1_073_741_824.0;

    let disks = toadstool_sysmon::disk_usage().unwrap_or_default();
    let (total_disk, used_disk) = disks.iter().fold((0u64, 0u64), |(t, u), d| {
        (t + d.total_space, u + d.total_space - d.available_space)
    });
    let storage_total_gb = total_disk as f64 / 1_073_741_824.0;
    let storage_used_gb = used_disk as f64 / 1_073_741_824.0;

    let interfaces = toadstool_sysmon::network_stats().unwrap_or_default();
    let total_rx: u64 = interfaces.iter().map(|i| i.received).sum();
    let total_tx: u64 = interfaces.iter().map(|i| i.transmitted).sum();
    let network_rx_mbps = (total_rx as f64 * 8.0) / 1_000_000.0;
    let network_tx_mbps = (total_tx as f64 * 8.0) / 1_000_000.0;

    let la = toadstool_sysmon::load_average()
        .map(|l| vec![l.one, l.five, l.fifteen])
        .unwrap_or_else(|_| vec![0.0, 0.0, 0.0]);

    Ok(SystemResourceUsage {
        cpu_percent,
        memory_used_gb,
        memory_total_gb,
        storage_used_gb,
        storage_total_gb,
        network_rx_mbps,
        network_tx_mbps,
        load_average: la,
    })
}

/// Aggregates performance metrics from active monitoring sessions
pub fn collect_performance_metrics(
    sessions: &HashMap<String, MonitoringSession>,
) -> PerformanceMetrics {
    let active = sessions
        .values()
        .filter(|s| matches!(s.status, SessionStatus::Active))
        .count();
    PerformanceMetrics {
        execution_latency_ms: 0.0,
        throughput_ops_sec: 0.0,
        error_rate: 0.0,
        success_rate: if active > 0 { 100.0 } else { 0.0 },
        queue_depth: active as u32,
    }
}
