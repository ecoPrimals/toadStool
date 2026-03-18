// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metrics storage, cleanup, and model updates.
//!
//! Handles metrics history retention, runtime statistics aggregation,
//! and baseline model updates.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime};

use tracing::{debug, info};

use toadstool::execution::RuntimeType;

use super::internal::BaselineMetrics;
use crate::types::{PerformanceMetrics, RuntimeStats};

/// Remove metrics older than the retention period.
pub(super) fn cleanup_old_metrics(
    history: &mut VecDeque<PerformanceMetrics>,
    retention_hours: u64,
) {
    let retention_duration = Duration::from_secs(retention_hours * 3600);
    let cutoff_time = SystemTime::now() - retention_duration;

    while let Some(front) = history.front() {
        if front.start_time < cutoff_time {
            history.pop_front();
        } else {
            break;
        }
    }
}

/// Update runtime statistics with new metrics.
pub(super) fn update_runtime_stats(
    stats: &mut HashMap<RuntimeType, RuntimeStats>,
    metrics: &PerformanceMetrics,
) {
    let runtime_stats = stats
        .entry(metrics.runtime_type.clone())
        .or_insert_with(|| RuntimeStats {
            runtime_type: metrics.runtime_type.clone(),
            total_executions: 0,
            successful_executions: 0,
            avg_execution_time: Duration::ZERO,
            p95_execution_time: Duration::ZERO,
            avg_memory_usage: 0.0,
            avg_cpu_usage: 0.0,
            success_rate: 0.0,
            efficiency_score: 0.0,
            current_load: 0.0,
        });

    runtime_stats.total_executions += 1;
    if metrics.success {
        runtime_stats.successful_executions += 1;
    }

    runtime_stats.success_rate = if runtime_stats.total_executions > 0 {
        (runtime_stats.successful_executions as f64 / runtime_stats.total_executions as f64) * 100.0
    } else {
        0.0
    };

    if let Some(duration) = metrics.execution_duration {
        runtime_stats.avg_execution_time = Duration::from_secs_f64(
            runtime_stats.avg_execution_time.as_secs_f64().mul_add(
                (runtime_stats.total_executions - 1) as f64,
                duration.as_secs_f64(),
            ) / runtime_stats.total_executions as f64,
        );
    }

    runtime_stats.avg_memory_usage = runtime_stats.avg_memory_usage.mul_add(
        (runtime_stats.total_executions - 1) as f64,
        metrics.resource_metrics.memory.used_bytes as f64 / 1024.0 / 1024.0,
    ) / runtime_stats.total_executions as f64;

    runtime_stats.avg_cpu_usage = runtime_stats.avg_cpu_usage.mul_add(
        (runtime_stats.total_executions - 1) as f64,
        metrics.resource_metrics.cpu.usage_percent,
    ) / runtime_stats.total_executions as f64;

    runtime_stats.efficiency_score = metrics.efficiency_score;
}

/// Update P95 execution times and baseline metrics from history.
pub(super) fn update_model_from_history(
    history: &std::collections::VecDeque<PerformanceMetrics>,
    stats: &mut HashMap<RuntimeType, RuntimeStats>,
    baselines: &mut HashMap<String, BaselineMetrics>,
    min_samples: usize,
) {
    if history.len() < min_samples {
        debug!(
            "Not enough samples ({}) for model update, need {}",
            history.len(),
            min_samples
        );
        return;
    }

    let mut by_runtime: HashMap<RuntimeType, Vec<&PerformanceMetrics>> = HashMap::new();
    for m in history {
        by_runtime
            .entry(m.runtime_type.clone())
            .or_default()
            .push(m);
    }

    for (rt, metrics) in &by_runtime {
        let mut durations: Vec<f64> = metrics
            .iter()
            .filter_map(|m| m.execution_duration.map(|d| d.as_secs_f64()))
            .collect();

        if durations.is_empty() {
            continue;
        }
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p95_idx = ((durations.len() as f64) * 0.95).ceil() as usize;
        let p95_val = durations[p95_idx.min(durations.len() - 1)];

        if let Some(rs) = stats.get_mut(rt) {
            rs.p95_execution_time = Duration::from_secs_f64(p95_val);
        }
    }

    for (rt, metrics) in &by_runtime {
        let (sum_time, sum_mem, sum_cpu, count) =
            metrics
                .iter()
                .fold((0.0f64, 0.0f64, 0.0f64, 0u64), |(t, m, c, n), met| {
                    let dur = met.execution_duration.map_or(0.0, |d| d.as_secs_f64());
                    let mem = met.resource_metrics.memory.used_bytes as f64 / 1024.0 / 1024.0;
                    let cpu = met.resource_metrics.cpu.usage_percent;
                    (t + dur, m + mem, c + cpu, n + 1)
                });
        if count > 0 {
            let c = count as f64;
            baselines.insert(
                format!("{rt:?}"),
                BaselineMetrics {
                    avg_execution_time: Duration::from_secs_f64(sum_time / c),
                    avg_memory_mb: sum_mem / c,
                    avg_cpu_percent: sum_cpu / c,
                },
            );
        }
    }

    info!(
        "Performance model updated with {} samples across {} runtimes",
        history.len(),
        by_runtime.len()
    );
}
