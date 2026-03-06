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
mod tests {
    use super::*;
    use crate::types::SelectionWeights;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;
    use toadstool::workload::{ExecutableSource, WasmModuleSource};

    fn make_runtime_stats(
        total: u64,
        avg_time: Duration,
        avg_mem: f64,
        avg_cpu: f64,
        load: f64,
        success_rate: f64,
        efficiency: f64,
    ) -> RuntimeStats {
        RuntimeStats {
            runtime_type: RuntimeType::Native,
            total_executions: total,
            successful_executions: total,
            avg_execution_time: avg_time,
            p95_execution_time: avg_time,
            avg_memory_usage: avg_mem,
            avg_cpu_usage: avg_cpu,
            success_rate,
            efficiency_score: efficiency,
            current_load: load,
        }
    }

    #[test]
    fn test_select_fastest_runtime() {
        let mut stats = HashMap::new();
        stats.insert(
            RuntimeType::Native,
            make_runtime_stats(10, Duration::from_millis(100), 100.0, 50.0, 10.0, 1.0, 80.0),
        );
        stats.insert(
            RuntimeType::Wasm,
            make_runtime_stats(10, Duration::from_millis(50), 80.0, 40.0, 5.0, 1.0, 90.0),
        );
        let available = vec![RuntimeType::Native, RuntimeType::Wasm];
        let request = ExecutionRequest {
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("/bin/true"),
                },
                args: None,
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            ..Default::default()
        };
        let selected = select_runtime_by_strategy(
            &stats,
            &RuntimeSelectionStrategy::FastestExecution,
            &request,
            &available,
        );
        assert_eq!(selected, RuntimeType::Wasm);
    }

    #[test]
    fn test_select_fallback_when_no_stats() {
        let stats: HashMap<RuntimeType, RuntimeStats> = HashMap::new();
        let available = vec![RuntimeType::Native];
        let request = ExecutionRequest {
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("/bin/true"),
                },
                args: None,
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            ..Default::default()
        };
        let selected = select_runtime_by_strategy(
            &stats,
            &RuntimeSelectionStrategy::FastestExecution,
            &request,
            &available,
        );
        assert_eq!(selected, RuntimeType::Native);
    }

    #[test]
    fn test_select_workload_optimized_native() {
        let mut stats = HashMap::new();
        stats.insert(
            RuntimeType::Native,
            make_runtime_stats(10, Duration::from_secs(1), 100.0, 50.0, 10.0, 1.0, 80.0),
        );
        let available = vec![RuntimeType::Native, RuntimeType::Wasm];
        let request = ExecutionRequest {
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("/bin/echo"),
                },
                args: Some(vec!["hello".to_string()]),
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            ..Default::default()
        };
        let selected = select_runtime_by_strategy(
            &stats,
            &RuntimeSelectionStrategy::WorkloadOptimized,
            &request,
            &available,
        );
        assert_eq!(selected, RuntimeType::Native);
    }

    #[test]
    fn test_select_load_balance() {
        let mut stats = HashMap::new();
        let mut native_stats =
            make_runtime_stats(10, Duration::from_secs(1), 100.0, 50.0, 80.0, 1.0, 80.0);
        native_stats.runtime_type = RuntimeType::Native;
        let mut wasm_stats =
            make_runtime_stats(10, Duration::from_secs(1), 100.0, 50.0, 20.0, 1.0, 80.0);
        wasm_stats.runtime_type = RuntimeType::Wasm;
        stats.insert(RuntimeType::Native, native_stats);
        stats.insert(RuntimeType::Wasm, wasm_stats);
        let available = vec![RuntimeType::Native, RuntimeType::Wasm];
        let request = ExecutionRequest {
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("/bin/true"),
                },
                args: None,
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            ..Default::default()
        };
        let selected = select_runtime_by_strategy(
            &stats,
            &RuntimeSelectionStrategy::LoadBalance,
            &request,
            &available,
        );
        assert_eq!(selected, RuntimeType::Wasm);
    }

    #[test]
    fn test_select_lowest_resource_runtime() {
        let mut stats = HashMap::new();
        stats.insert(
            RuntimeType::Native,
            make_runtime_stats(10, Duration::from_secs(1), 200.0, 60.0, 10.0, 1.0, 70.0),
        );
        stats.insert(
            RuntimeType::Wasm,
            make_runtime_stats(10, Duration::from_secs(1), 80.0, 30.0, 5.0, 1.0, 85.0),
        );
        let available = vec![RuntimeType::Native, RuntimeType::Wasm];
        let request = ExecutionRequest {
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("/bin/true"),
                },
                args: None,
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            ..Default::default()
        };
        let selected = select_runtime_by_strategy(
            &stats,
            &RuntimeSelectionStrategy::LowestResourceUsage,
            &request,
            &available,
        );
        assert_eq!(selected, RuntimeType::Wasm);
    }

    #[test]
    fn test_select_most_efficient_runtime() {
        let mut stats = HashMap::new();
        stats.insert(
            RuntimeType::Native,
            make_runtime_stats(10, Duration::from_secs(1), 100.0, 50.0, 10.0, 1.0, 70.0),
        );
        stats.insert(
            RuntimeType::Wasm,
            make_runtime_stats(10, Duration::from_secs(1), 80.0, 40.0, 5.0, 1.0, 95.0),
        );
        let available = vec![RuntimeType::Native, RuntimeType::Wasm];
        let request = ExecutionRequest {
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("/bin/true"),
                },
                args: None,
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            ..Default::default()
        };
        let selected = select_runtime_by_strategy(
            &stats,
            &RuntimeSelectionStrategy::BestEfficiency,
            &request,
            &available,
        );
        assert_eq!(selected, RuntimeType::Wasm);
    }

    #[test]
    fn test_select_custom_weighted_runtime() {
        let mut stats = HashMap::new();
        stats.insert(
            RuntimeType::Native,
            make_runtime_stats(
                10,
                Duration::from_millis(50),
                100.0,
                30.0,
                10.0,
                100.0,
                90.0,
            ),
        );
        stats.insert(
            RuntimeType::Wasm,
            make_runtime_stats(10, Duration::from_millis(100), 80.0, 20.0, 5.0, 100.0, 85.0),
        );
        let available = vec![RuntimeType::Native, RuntimeType::Wasm];
        let weights = SelectionWeights {
            execution_time: 0.5,
            memory_usage: 0.2,
            cpu_usage: 0.2,
            resource_availability: 0.05,
            historical_success_rate: 0.05,
        };
        let request = ExecutionRequest {
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("/bin/true"),
                },
                args: None,
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            ..Default::default()
        };
        let selected = select_runtime_by_strategy(
            &stats,
            &RuntimeSelectionStrategy::Custom { weights },
            &request,
            &available,
        );
        assert!(selected == RuntimeType::Native || selected == RuntimeType::Wasm);
    }

    #[test]
    fn test_select_workload_optimized_wasm_fallback_when_native_unavailable() {
        let mut stats = HashMap::new();
        stats.insert(
            RuntimeType::Wasm,
            make_runtime_stats(10, Duration::from_millis(50), 80.0, 30.0, 10.0, 1.0, 90.0),
        );
        let available = vec![RuntimeType::Wasm];
        let request = ExecutionRequest {
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("/bin/true"),
                },
                args: None,
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            ..Default::default()
        };
        let selected = select_runtime_by_strategy(
            &stats,
            &RuntimeSelectionStrategy::WorkloadOptimized,
            &request,
            &available,
        );
        assert_eq!(selected, RuntimeType::Wasm);
    }

    #[test]
    fn test_select_workload_optimized_wasm() {
        let mut stats = HashMap::new();
        stats.insert(
            RuntimeType::Native,
            make_runtime_stats(10, Duration::from_secs(1), 100.0, 50.0, 10.0, 1.0, 80.0),
        );
        stats.insert(
            RuntimeType::Wasm,
            make_runtime_stats(10, Duration::from_millis(50), 80.0, 30.0, 5.0, 1.0, 90.0),
        );
        let available = vec![RuntimeType::Native, RuntimeType::Wasm];
        let request = ExecutionRequest {
            workload: WorkloadSpec::Wasm {
                module: WasmModuleSource::File {
                    path: PathBuf::from("/tmp/test.wasm"),
                },
                args: None,
                wasi_config: None,
                env_vars: Default::default(),
            },
            ..Default::default()
        };
        let selected = select_runtime_by_strategy(
            &stats,
            &RuntimeSelectionStrategy::WorkloadOptimized,
            &request,
            &available,
        );
        assert_eq!(selected, RuntimeType::Wasm);
    }
}
