//! Comprehensive Cross-Substrate Benchmark Suite
//!
//! Tests various workload types across CPU, GPU (AMD/NVIDIA), and Neuromorphic (Akida)
//! to understand strengths, weaknesses, and use cases.

use std::time::Instant;
use toadstool_runtime_universal::{
    ComputeUnitType, OperationType, UniversalRuntime, WorkloadBuilder,
};

#[derive(Debug, Clone)]
pub struct WorkloadSpec {
    pub name: &'static str,
    pub operation: OperationType,
    pub size: usize,
    pub category: WorkloadCategory,
    pub expected_winner: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadCategory {
    ElementWise,   // Simple per-element operations (ReLU, Tanh, etc.)
    Reduction,     // Aggregate operations (sum, max, etc.)
    MemoryBound,   // Gather/scatter, transpose
    ComputeBound,  // MatMul, convolution
    Normalization, // LayerNorm, BatchNorm
    Mixed,         // Complex multi-stage operations
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

#[derive(Debug)]
pub struct BenchmarkResult {
    pub workload: String,
    pub category: WorkloadCategory,
    pub cpu_time_us: Option<f64>,
    pub gpu_amd_time_us: Option<f64>,
    pub gpu_nvidia_time_us: Option<f64>,
    pub npu_time_us: Option<f64>,
    pub winner: String,
    pub speedup: f64,
}

pub async fn run_comprehensive_benchmark(runtime: &UniversalRuntime) -> Vec<BenchmarkResult> {
    let suite = get_benchmark_suite();
    let mut results = Vec::new();

    println!("\n🔬 Running Comprehensive Benchmark Suite...");
    println!(
        "   Testing {} workloads across all substrates\n",
        suite.len()
    );

    for (i, spec) in suite.iter().enumerate() {
        println!("   [{}/{}] Testing: {}", i + 1, suite.len(), spec.name);

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
            .unwrap_or(("None".to_string(), 0.0));

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

pub fn print_results_summary(results: &[BenchmarkResult]) {
    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                              ║");
    println!("║              COMPREHENSIVE BENCHMARK RESULTS                                 ║");
    println!("║                                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    // Group by category
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

        println!("\n━━━ {category:?} Operations ━━━\n");
        println!("   ┌────────────────────────────────┬──────────┬──────────┬──────────┬──────────┬────────────┬──────────┐");
        println!("   │ Workload                       │ CPU (µs) │ AMD (µs) │ NVIDIA   │ NPU (µs) │ Winner     │ Speedup  │");
        println!("   ├────────────────────────────────┼──────────┼──────────┼──────────┼──────────┼────────────┼──────────┤");

        for result in category_results {
            println!(
                "   │ {:30} │ {:8} │ {:8} │ {:8} │ {:8} │ {:10} │ {:7.2}x │",
                truncate(&result.workload, 30),
                format_time(result.cpu_time_us),
                format_time(result.gpu_amd_time_us),
                format_time(result.gpu_nvidia_time_us),
                format_time(result.npu_time_us),
                truncate(&result.winner, 10),
                result.speedup
            );
        }

        println!("   └────────────────────────────────┴──────────┴──────────┴──────────┴──────────┴────────────┴──────────┘");
    }

    // Summary statistics
    println!("\n━━━ Summary Statistics ━━━\n");

    let cpu_wins = results.iter().filter(|r| r.winner.contains("CPU")).count();
    let amd_wins = results.iter().filter(|r| r.winner.contains("AMD")).count();
    let nvidia_wins = results
        .iter()
        .filter(|r| r.winner.contains("NVIDIA"))
        .count();
    let npu_wins = results.iter().filter(|r| r.winner.contains("NPU")).count();

    println!("   Winner Distribution:");
    println!(
        "     CPU:         {} wins ({:.1}%)",
        cpu_wins,
        cpu_wins as f64 / results.len() as f64 * 100.0
    );
    println!(
        "     GPU AMD:     {} wins ({:.1}%)",
        amd_wins,
        amd_wins as f64 / results.len() as f64 * 100.0
    );
    println!(
        "     GPU NVIDIA:  {} wins ({:.1}%)",
        nvidia_wins,
        nvidia_wins as f64 / results.len() as f64 * 100.0
    );
    println!(
        "     NPU:         {} wins ({:.1}%)",
        npu_wins,
        npu_wins as f64 / results.len() as f64 * 100.0
    );

    let avg_speedup: f64 = results.iter().map(|r| r.speedup).sum::<f64>() / results.len() as f64;
    let max_speedup = results.iter().map(|r| r.speedup).fold(0.0f64, f64::max);

    println!("\n   Speedup Statistics:");
    println!("     Average: {avg_speedup:.2}x");
    println!("     Maximum: {max_speedup:.2}x");
}

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
