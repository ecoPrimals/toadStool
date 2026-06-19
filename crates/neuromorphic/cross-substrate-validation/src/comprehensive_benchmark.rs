// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive Cross-Substrate Benchmark Suite
//!
//! Tests various workload types across CPU, GPU (AMD/NVIDIA), and Neuromorphic (Akida)
//! to understand strengths, weaknesses, and use cases.

use std::time::Instant;
use toadstool_runtime_universal::{
    ComputeUnitType, OperationType, UniversalRuntime, WorkloadBuilder,
};
use tracing::info;

/// Specification for a single benchmark workload.
#[derive(Debug, Clone)]
pub struct WorkloadSpec {
    /// Human-readable workload name.
    pub name: &'static str,
    /// Operation type to benchmark.
    pub operation: OperationType,
    /// Input size (element count).
    pub size: usize,
    /// Workload category for grouping results.
    pub category: WorkloadCategory,
    /// Expected best-performing substrate (informational).
    pub expected_winner: &'static str,
}

/// Workload category for grouping benchmark results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadCategory {
    /// Simple per-element operations (`ReLU`, `Tanh`, etc.).
    ElementWise,
    /// Aggregate operations (sum, max, etc.).
    Reduction,
    /// Gather/scatter, transpose.
    MemoryBound,
    /// `MatMul`, convolution.
    ComputeBound,
    /// `LayerNorm`, `BatchNorm`.
    Normalization,
    /// Complex multi-stage operations.
    Mixed,
}

/// Comprehensive workload suite
pub fn get_benchmark_suite() -> Vec<WorkloadSpec> {
    vec![
        // Element-wise operations (memory-bound, should favor GPU/NPU)
        WorkloadSpec {
            name: "ReLU (tiny - 100 elem)",
            operation: OperationType::ReLU,
            size: 100,
            category: WorkloadCategory::ElementWise,
            expected_winner: "NPU (low latency)",
        },
        WorkloadSpec {
            name: "ReLU (small - 1K elem)",
            operation: OperationType::ReLU,
            size: 1_000,
            category: WorkloadCategory::ElementWise,
            expected_winner: "GPU (parallel)",
        },
        WorkloadSpec {
            name: "ReLU (medium - 10K elem)",
            operation: OperationType::ReLU,
            size: 10_000,
            category: WorkloadCategory::ElementWise,
            expected_winner: "GPU (parallel)",
        },
        WorkloadSpec {
            name: "ReLU (large - 100K elem)",
            operation: OperationType::ReLU,
            size: 100_000,
            category: WorkloadCategory::ElementWise,
            expected_winner: "GPU (high throughput)",
        },
        WorkloadSpec {
            name: "ReLU (huge - 1M elem)",
            operation: OperationType::ReLU,
            size: 1_000_000,
            category: WorkloadCategory::ElementWise,
            expected_winner: "GPU (bandwidth)",
        },
        // Activation functions
        WorkloadSpec {
            name: "Tanh (10K elem)",
            operation: OperationType::Tanh,
            size: 10_000,
            category: WorkloadCategory::ElementWise,
            expected_winner: "GPU",
        },
        WorkloadSpec {
            name: "Sigmoid (10K elem)",
            operation: OperationType::Sigmoid,
            size: 10_000,
            category: WorkloadCategory::ElementWise,
            expected_winner: "GPU",
        },
        WorkloadSpec {
            name: "GELU (10K elem)",
            operation: OperationType::GELU,
            size: 10_000,
            category: WorkloadCategory::ElementWise,
            expected_winner: "GPU",
        },
        // Reduction operations (compute-bound)
        WorkloadSpec {
            name: "Reduce (1K elem)",
            operation: OperationType::Reduce,
            size: 1_000,
            category: WorkloadCategory::Reduction,
            expected_winner: "GPU/CPU",
        },
        WorkloadSpec {
            name: "Reduce (100K elem)",
            operation: OperationType::Reduce,
            size: 100_000,
            category: WorkloadCategory::Reduction,
            expected_winner: "GPU",
        },
        // Memory-bound operations
        WorkloadSpec {
            name: "Transpose (10K elem)",
            operation: OperationType::Transpose,
            size: 10_000,
            category: WorkloadCategory::MemoryBound,
            expected_winner: "GPU (coalescing)",
        },
        WorkloadSpec {
            name: "Gather (10K elem)",
            operation: OperationType::Gather,
            size: 10_000,
            category: WorkloadCategory::MemoryBound,
            expected_winner: "GPU",
        },
        WorkloadSpec {
            name: "Scatter (10K elem)",
            operation: OperationType::Scatter,
            size: 10_000,
            category: WorkloadCategory::MemoryBound,
            expected_winner: "GPU",
        },
        // Normalization (reduce-map-reduce pattern)
        WorkloadSpec {
            name: "LayerNorm (10K elem)",
            operation: OperationType::LayerNorm,
            size: 10_000,
            category: WorkloadCategory::Normalization,
            expected_winner: "GPU",
        },
        WorkloadSpec {
            name: "BatchNorm (10K elem)",
            operation: OperationType::BatchNorm,
            size: 10_000,
            category: WorkloadCategory::Normalization,
            expected_winner: "GPU",
        },
        // Compute-bound operations
        WorkloadSpec {
            name: "MatMul (small)",
            operation: OperationType::MatMul,
            size: 1_000,
            category: WorkloadCategory::ComputeBound,
            expected_winner: "GPU (FMA units)",
        },
        WorkloadSpec {
            name: "MatMul (large)",
            operation: OperationType::MatMul,
            size: 100_000,
            category: WorkloadCategory::ComputeBound,
            expected_winner: "GPU (FMA units)",
        },
        // Vector operations
        WorkloadSpec {
            name: "DotProduct (10K elem)",
            operation: OperationType::DotProduct,
            size: 10_000,
            category: WorkloadCategory::Reduction,
            expected_winner: "GPU",
        },
        WorkloadSpec {
            name: "ElementwiseBinary (10K elem)",
            operation: OperationType::ElementwiseBinary,
            size: 10_000,
            category: WorkloadCategory::ElementWise,
            expected_winner: "GPU",
        },
    ]
}

/// Result of a single workload benchmark across substrates.
#[derive(Debug)]
pub struct BenchmarkResult {
    /// Workload name.
    pub workload: String,
    /// Workload category.
    pub category: WorkloadCategory,
    /// CPU execution time in microseconds.
    pub cpu_time_us: Option<f64>,
    /// AMD GPU execution time in microseconds.
    pub gpu_amd_time_us: Option<f64>,
    /// NVIDIA GPU execution time in microseconds.
    pub gpu_nvidia_time_us: Option<f64>,
    /// NPU execution time in microseconds.
    pub npu_time_us: Option<f64>,
    /// Best-performing substrate name.
    pub winner: String,
    /// Speedup vs CPU baseline.
    pub speedup: f64,
}

/// Runs the full benchmark suite across all substrates.
pub async fn run_comprehensive_benchmark(runtime: &UniversalRuntime) -> Vec<BenchmarkResult> {
    let suite = get_benchmark_suite();
    let mut results = Vec::new();

    info!(
        workload_count = suite.len(),
        "starting comprehensive benchmark suite"
    );

    for (i, spec) in suite.iter().enumerate() {
        info!(
            progress = i + 1,
            total = suite.len(),
            workload = spec.name,
            "testing workload"
        );

        // Generate test data
        let input: Vec<f32> = (0..spec.size).map(|i| i as f32 * 0.1).collect();

        // Benchmark CPU
        let cpu_time =
            benchmark_substrate(runtime, ComputeUnitType::Cpu, spec.operation, &input).await;

        // Benchmark GPUs (we'll detect which is which by looking at device info)
        let gpu_units = runtime.units_by_type(ComputeUnitType::GpuWgpu);
        let mut gpu_amd_time = None;
        let mut gpu_nvidia_time = None;

        for (idx, _unit) in gpu_units.iter().enumerate() {
            if let Some(time) =
                benchmark_substrate_by_index(runtime, idx, spec.operation, &input).await
            {
                // For now, assume first GPU is primary (we'd need device info to distinguish AMD/NVIDIA)
                if idx == 0 {
                    gpu_amd_time = Some(time);
                } else if idx == 1 && gpu_nvidia_time.is_none() {
                    gpu_nvidia_time = Some(time);
                }
            }
        }

        // Neuromorphic - note: Akida runs fixed models, so this is latency-only
        // We can't actually run arbitrary operations on NPU, but we can measure its latency
        let npu_time = None; // Would need actual model for each operation

        // Determine winner and speedup
        let times: Vec<(&str, f64)> = vec![
            ("CPU", cpu_time.unwrap_or(f64::MAX)),
            ("GPU AMD", gpu_amd_time.unwrap_or(f64::MAX)),
            ("GPU NVIDIA", gpu_nvidia_time.unwrap_or(f64::MAX)),
            ("NPU", f64::MAX),
        ];

        let (winner, best_time) = times
            .iter()
            .filter(|(_, t)| *t < f64::MAX && !t.is_nan())
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(n, t)| (n.to_string(), *t))
            .unwrap_or_else(|| ("None".to_string(), 0.0));

        let baseline = cpu_time.unwrap_or(best_time);
        let speedup = if best_time > 0.0 {
            baseline / best_time
        } else {
            1.0
        };

        results.push(BenchmarkResult {
            workload: spec.name.to_string(),
            category: spec.category,
            cpu_time_us: cpu_time,
            gpu_amd_time_us: gpu_amd_time,
            gpu_nvidia_time_us: gpu_nvidia_time,
            npu_time_us: npu_time,
            winner,
            speedup,
        });
    }

    results
}

async fn benchmark_substrate(
    runtime: &UniversalRuntime,
    unit_type: ComputeUnitType,
    operation: OperationType,
    input: &[f32],
) -> Option<f64> {
    let workload = WorkloadBuilder::new()
        .operation(operation)
        .data_f32(input.to_vec())
        .build()
        .ok()?;

    let start = Instant::now();
    runtime.execute_on_type(unit_type, workload).await.ok()?;
    let elapsed = start.elapsed();

    Some(elapsed.as_secs_f64() * 1_000_000.0)
}

async fn benchmark_substrate_by_index(
    runtime: &UniversalRuntime,
    index: usize,
    operation: OperationType,
    input: &[f32],
) -> Option<f64> {
    let workload = WorkloadBuilder::new()
        .operation(operation)
        .data_f32(input.to_vec())
        .build()
        .ok()?;

    let start = Instant::now();
    runtime.execute_on(index, workload).await.ok()?;
    let elapsed = start.elapsed();

    Some(elapsed.as_secs_f64() * 1_000_000.0)
}

/// Logs a structured summary of benchmark results via `tracing`.
pub fn print_results_summary(results: &[BenchmarkResult]) {
    let categories = [
        WorkloadCategory::ElementWise,
        WorkloadCategory::Reduction,
        WorkloadCategory::MemoryBound,
        WorkloadCategory::ComputeBound,
        WorkloadCategory::Normalization,
        WorkloadCategory::Mixed,
    ];

    for category in &categories {
        let category_results: Vec<_> = results.iter().filter(|r| r.category == *category).collect();
        if category_results.is_empty() {
            continue;
        }

        for result in &category_results {
            info!(
                category = ?category,
                workload = %result.workload,
                cpu_us = ?result.cpu_time_us,
                amd_us = ?result.gpu_amd_time_us,
                nvidia_us = ?result.gpu_nvidia_time_us,
                npu_us = ?result.npu_time_us,
                winner = %result.winner,
                speedup = result.speedup,
                "benchmark result"
            );
        }
    }

    let cpu_wins = results.iter().filter(|r| r.winner.contains("CPU")).count();
    let amd_wins = results.iter().filter(|r| r.winner.contains("AMD")).count();
    let nvidia_wins = results
        .iter()
        .filter(|r| r.winner.contains("NVIDIA"))
        .count();
    let npu_wins = results.iter().filter(|r| r.winner.contains("NPU")).count();
    let total = results.len();

    let avg_speedup: f64 = results.iter().map(|r| r.speedup).sum::<f64>() / total as f64;
    let max_speedup = results.iter().map(|r| r.speedup).fold(0.0f64, f64::max);

    info!(
        total,
        cpu_wins, amd_wins, nvidia_wins, npu_wins, avg_speedup, max_speedup, "benchmark summary"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format_time(time: Option<f64>) -> String {
        match time {
            Some(t) => format!("{t:8.1}"),
            None => "    -   ".to_string(),
        }
    }

    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            format!("{s:max_len$}")
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }

    #[test]
    fn test_get_benchmark_suite() {
        let suite = get_benchmark_suite();
        assert!(!suite.is_empty());
        assert!(suite.len() >= 15);
        assert!(suite.iter().any(|s| s.name.contains("ReLU")));
        assert!(suite.iter().any(|s| s.name.contains("MatMul")));
    }

    #[test]
    fn test_workload_spec_structure() {
        let spec = &get_benchmark_suite()[0];
        assert!(!spec.name.is_empty());
        assert!(spec.size > 0);
        assert!(!spec.expected_winner.is_empty());
    }

    #[test]
    fn test_workload_category_variants() {
        let _ = WorkloadCategory::ElementWise;
        let _ = WorkloadCategory::Reduction;
        let _ = WorkloadCategory::MemoryBound;
        let _ = WorkloadCategory::ComputeBound;
        let _ = WorkloadCategory::Normalization;
        let _ = WorkloadCategory::Mixed;
    }

    #[test]
    fn test_benchmark_result_structure() {
        let result = BenchmarkResult {
            workload: "Test".to_string(),
            category: WorkloadCategory::ElementWise,
            cpu_time_us: Some(100.0),
            gpu_amd_time_us: None,
            gpu_nvidia_time_us: None,
            npu_time_us: None,
            winner: "CPU".to_string(),
            speedup: 1.5,
        };
        assert_eq!(result.winner, "CPU");
        assert!((result.speedup - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_print_results_summary_no_panic() {
        let results = vec![BenchmarkResult {
            workload: "Test Workload".to_string(),
            category: WorkloadCategory::ElementWise,
            cpu_time_us: Some(50.0),
            gpu_amd_time_us: Some(20.0),
            gpu_nvidia_time_us: None,
            npu_time_us: None,
            winner: "GPU AMD".to_string(),
            speedup: 2.5,
        }];
        print_results_summary(&results);
    }

    #[test]
    fn test_benchmark_suite_categories() {
        let suite = get_benchmark_suite();
        let categories: Vec<_> = suite.iter().map(|s| s.category).collect();
        assert!(categories.contains(&WorkloadCategory::ElementWise));
        assert!(categories.contains(&WorkloadCategory::Reduction));
        assert!(categories.contains(&WorkloadCategory::MemoryBound));
        assert!(categories.contains(&WorkloadCategory::ComputeBound));
        assert!(categories.contains(&WorkloadCategory::Normalization));
    }

    #[test]
    fn test_benchmark_suite_operations() {
        let suite = get_benchmark_suite();
        assert!(suite.iter().any(|s| s.name.contains("ReLU")));
        assert!(suite.iter().any(|s| s.name.contains("Tanh")));
        assert!(suite.iter().any(|s| s.name.contains("Sigmoid")));
        assert!(suite.iter().any(|s| s.name.contains("MatMul")));
        assert!(suite.iter().any(|s| s.name.contains("Reduce")));
        assert!(suite.iter().any(|s| s.name.contains("Transpose")));
    }

    #[test]
    fn test_workload_spec_expected_winner() {
        let suite = get_benchmark_suite();
        for spec in &suite {
            assert!(!spec.expected_winner.is_empty());
            assert!(spec.size > 0);
        }
    }

    #[test]
    fn test_benchmark_result_all_fields() {
        let result = BenchmarkResult {
            workload: "Full Test".to_string(),
            category: WorkloadCategory::ComputeBound,
            cpu_time_us: Some(100.0),
            gpu_amd_time_us: Some(50.0),
            gpu_nvidia_time_us: Some(40.0),
            npu_time_us: Some(10.0),
            winner: "NPU".to_string(),
            speedup: 10.0,
        };
        assert_eq!(result.winner, "NPU");
        assert!((result.speedup - 10.0).abs() < f64::EPSILON);
        assert_eq!(result.category, WorkloadCategory::ComputeBound);
    }

    #[test]
    fn test_format_time_and_truncate() {
        assert_eq!(format_time(Some(123.45)), "   123.5");
        assert_eq!(format_time(None), "    -   ");
        assert_eq!(truncate("short", 10), "short     ");
        assert!(truncate("verylongstring", 5).ends_with("..."));
    }
}
