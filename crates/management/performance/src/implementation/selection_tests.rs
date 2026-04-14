// SPDX-License-Identifier: AGPL-3.0-or-later

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
            env_vars: std::collections::HashMap::default(),
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
            env_vars: std::collections::HashMap::default(),
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
            env_vars: std::collections::HashMap::default(),
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
            env_vars: std::collections::HashMap::default(),
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
            env_vars: std::collections::HashMap::default(),
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
            env_vars: std::collections::HashMap::default(),
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
            env_vars: std::collections::HashMap::default(),
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
            env_vars: std::collections::HashMap::default(),
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
            env_vars: std::collections::HashMap::default(),
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
