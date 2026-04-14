// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime selection logic by strategy.
//!
//! Pure selection functions that operate on runtime statistics.

use std::collections::HashMap;

use toadstool::execution::{ExecutionRequest, RuntimeType};
use toadstool::workload::WorkloadSpec;

use crate::scoring;
use crate::types::{RuntimeSelectionStrategy, RuntimeStats, SelectionWeights};

/// Select runtime based on configured strategy.
pub(super) fn select_runtime_by_strategy(
    stats: &HashMap<RuntimeType, RuntimeStats>,
    strategy: &RuntimeSelectionStrategy,
    request: &ExecutionRequest,
    available_runtimes: &[RuntimeType],
) -> RuntimeType {
    match strategy {
        RuntimeSelectionStrategy::FastestExecution => {
            select_fastest_runtime(stats, available_runtimes)
        }
        RuntimeSelectionStrategy::LowestResourceUsage => {
            select_lowest_resource_runtime(stats, available_runtimes)
        }
        RuntimeSelectionStrategy::BestEfficiency => {
            select_most_efficient_runtime(stats, available_runtimes)
        }
        RuntimeSelectionStrategy::LoadBalance => {
            select_least_loaded_runtime(stats, available_runtimes)
        }
        RuntimeSelectionStrategy::WorkloadOptimized => {
            select_workload_optimized_runtime(stats, request, available_runtimes)
        }
        RuntimeSelectionStrategy::Custom { weights } => {
            select_custom_weighted_runtime(stats, available_runtimes, weights)
        }
    }
}

/// Select fastest runtime based on historical data.
fn select_fastest_runtime(
    stats: &HashMap<RuntimeType, RuntimeStats>,
    available_runtimes: &[RuntimeType],
) -> RuntimeType {
    available_runtimes
        .iter()
        .filter_map(|rt| {
            stats
                .get(rt)
                .filter(|s| s.total_executions > 0)
                .map(|s| (rt, s.avg_execution_time))
        })
        .min_by_key(|(_, time)| *time)
        .map_or_else(|| available_runtimes[0].clone(), |(rt, _)| rt.clone())
}

/// Select runtime with lowest resource usage.
fn select_lowest_resource_runtime(
    stats: &HashMap<RuntimeType, RuntimeStats>,
    available_runtimes: &[RuntimeType],
) -> RuntimeType {
    available_runtimes
        .iter()
        .filter_map(|rt| {
            stats.get(rt).filter(|s| s.total_executions > 0).map(|s| {
                let resource_score = s.avg_memory_usage + s.avg_cpu_usage;
                (rt, resource_score)
            })
        })
        .min_by(|(_, score1), (_, score2)| {
            score1
                .partial_cmp(score2)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map_or_else(|| available_runtimes[0].clone(), |(rt, _)| rt.clone())
}

/// Select most efficient runtime.
fn select_most_efficient_runtime(
    stats: &HashMap<RuntimeType, RuntimeStats>,
    available_runtimes: &[RuntimeType],
) -> RuntimeType {
    available_runtimes
        .iter()
        .filter_map(|rt| {
            stats
                .get(rt)
                .filter(|s| s.total_executions > 0)
                .map(|s| (rt, s.efficiency_score))
        })
        .max_by(|(_, score1), (_, score2)| {
            score1
                .partial_cmp(score2)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map_or_else(|| available_runtimes[0].clone(), |(rt, _)| rt.clone())
}

/// Select least loaded runtime.
fn select_least_loaded_runtime(
    stats: &HashMap<RuntimeType, RuntimeStats>,
    available_runtimes: &[RuntimeType],
) -> RuntimeType {
    available_runtimes
        .iter()
        .filter_map(|rt| stats.get(rt).map(|s| (rt, s.current_load)))
        .min_by(|(_, load1), (_, load2)| {
            load1
                .partial_cmp(load2)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map_or_else(|| available_runtimes[0].clone(), |(rt, _)| rt.clone())
}

/// Select runtime optimized for workload type.
fn select_workload_optimized_runtime(
    stats: &HashMap<RuntimeType, RuntimeStats>,
    request: &ExecutionRequest,
    available_runtimes: &[RuntimeType],
) -> RuntimeType {
    let fallback = || select_fastest_runtime(stats, available_runtimes);

    match &request.workload {
        WorkloadSpec::Native { .. } => {
            if available_runtimes.contains(&RuntimeType::Native) {
                RuntimeType::Native
            } else {
                fallback()
            }
        }
        WorkloadSpec::Wasm { .. } => {
            if available_runtimes.contains(&RuntimeType::Wasm) {
                RuntimeType::Wasm
            } else {
                fallback()
            }
        }
        WorkloadSpec::Container { .. } => {
            if available_runtimes.contains(&RuntimeType::Container) {
                RuntimeType::Container
            } else {
                fallback()
            }
        }
        WorkloadSpec::Gpu { .. } => {
            if available_runtimes.contains(&RuntimeType::Gpu) {
                RuntimeType::Gpu
            } else {
                fallback()
            }
        }
        WorkloadSpec::Python { .. } => {
            if available_runtimes.contains(&RuntimeType::Python) {
                RuntimeType::Python
            } else {
                fallback()
            }
        }
        _ => fallback(),
    }
}

/// Select runtime using custom weights.
fn select_custom_weighted_runtime(
    stats: &HashMap<RuntimeType, RuntimeStats>,
    available_runtimes: &[RuntimeType],
    weights: &SelectionWeights,
) -> RuntimeType {
    let mut best_runtime = available_runtimes[0].clone();
    let mut best_score = f64::MIN;

    for runtime in available_runtimes {
        if let Some(runtime_stats) = stats.get(runtime) {
            if runtime_stats.total_executions == 0 {
                continue;
            }

            let weighted_score = scoring::calculate_weighted_score(
                runtime_stats.avg_execution_time,
                runtime_stats.avg_memory_usage,
                runtime_stats.avg_cpu_usage,
                runtime_stats.current_load,
                runtime_stats.success_rate,
                weights,
            );

            if weighted_score > best_score {
                best_score = weighted_score;
                best_runtime = runtime.clone();
            }
        }
    }

    best_runtime
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
