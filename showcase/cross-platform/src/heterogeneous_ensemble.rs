//! Heterogeneous Ensemble — Uses ALL Hardware Simultaneously
//!
//! Pipeline:
//! - 3 GPUs (2x RTX 3090, 1x RX 6950 XT): Parallel matrix operations
//! - 2 NPUs (Akida AKD1000): Neuromorphic inference
//! - CPUs (Dual EPYC): Coordination, aggregation, sparse ops
//!
//! This demonstrates true heterogeneous computing across all available silicon.

use akida_driver::{select_backend, BackendSelection};
use barracuda::linalg::sparse::{cg_solve, CsrMatrix};
use barracuda::multi_gpu::{GpuPool, GpuVendor, WorkloadConfig};
use barracuda::tensor::Tensor;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

/// Results from each compute tier
#[derive(Debug)]
struct TierResult {
    tier: &'static str,
    device: String,
    latency_ms: f64,
    output_sum: f64,
}

/// Aggregated ensemble result (for future multi-device aggregation)
#[derive(Debug)]
#[allow(dead_code)]
struct EnsembleResult {
    gpu_results: Vec<TierResult>,
    npu_results: Vec<TierResult>,
    cpu_result: TierResult,
    total_latency_ms: f64,
    combined_output: f64,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║     HETEROGENEOUS ENSEMBLE — All Silicon Active                      ║");
    println!("║     3 GPUs + 2 NPUs + Dual EPYC CPUs                                  ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();

    let total_start = Instant::now();

    // ════════════════════════════════════════════════════════════════════════
    // Phase 1: Initialize all hardware
    // ════════════════════════════════════════════════════════════════════════

    println!("═══ Phase 1: Hardware Initialization ═══");
    println!();

    // GPU Pool
    let gpu_config = WorkloadConfig {
        max_parallel: 4,
        prefer_discrete: true,
        exclude_software: true,
        min_gflops: 50.0,
    };
    let gpu_pool = GpuPool::with_config(gpu_config).await?;
    println!("✓ GPU Pool: {}", gpu_pool.summary());

    // NPUs
    let npu1 = select_backend(BackendSelection::Vfio, "0000:a1:00.0");
    let npu2 = select_backend(BackendSelection::Vfio, "0000:e2:00.0");

    let npu1_ok = npu1.is_ok();
    let npu2_ok = npu2.is_ok();
    println!(
        "✓ NPU #1 (0000:a1:00.0): {}",
        if npu1_ok { "Ready" } else { "Unavailable" }
    );
    println!(
        "✓ NPU #2 (0000:e2:00.0): {}",
        if npu2_ok { "Ready" } else { "Unavailable" }
    );

    // CPU info
    let cpu_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    println!("✓ CPU Threads: {cpu_threads}");
    println!();

    // ════════════════════════════════════════════════════════════════════════
    // Phase 2: Prepare workloads
    // ════════════════════════════════════════════════════════════════════════

    println!("═══ Phase 2: Workload Distribution ═══");
    println!();

    // Shared input data (simulating a batch of samples)
    let batch_size = 1024;
    let feature_dim = 512;
    let input_data: Vec<f32> = (0..batch_size * feature_dim)
        .map(|i| ((i as f32 * 0.001).sin() + 0.5).abs())
        .collect();

    println!(
        "Input: {} samples × {} features = {} values",
        batch_size,
        feature_dim,
        input_data.len()
    );
    println!();

    // ════════════════════════════════════════════════════════════════════════
    // Phase 3: Parallel execution on ALL hardware
    // ════════════════════════════════════════════════════════════════════════

    println!("═══ Phase 3: Parallel Execution (All Hardware) ═══");
    println!();

    let (tx, mut rx) = mpsc::channel::<TierResult>(16);
    let mut join_set = JoinSet::new();

    // ────────────────────────────────────────────────────────────────────────
    // GPU Tasks: Each GPU processes a partition of the data
    // ────────────────────────────────────────────────────────────────────────

    let num_gpus = gpu_pool.device_count();
    let chunk_size = batch_size / num_gpus.max(1);

    for gpu_idx in 0..num_gpus {
        let device = gpu_pool.device(gpu_idx).unwrap();
        let device_name = gpu_pool.devices()[gpu_idx].name.clone();
        let vendor = gpu_pool.devices()[gpu_idx].vendor;
        let tx = tx.clone();

        // Partition data for this GPU
        let start_idx = gpu_idx * chunk_size * feature_dim;
        let end_idx = ((gpu_idx + 1) * chunk_size * feature_dim).min(input_data.len());
        let gpu_data: Vec<f32> = input_data[start_idx..end_idx].to_vec();
        let gpu_chunk_size = (end_idx - start_idx) / feature_dim;

        join_set.spawn(async move {
            let start = Instant::now();

            // Create tensor and perform GPU operations
            let tensor =
                Tensor::from_data(&gpu_data, vec![gpu_chunk_size, feature_dim], device.clone())?;

            // Simulate neural network layer: ReLU(x) + x (residual connection)
            let relu_result = tensor.add(&tensor)?; // Simplified: 2x instead of ReLU for now
            let output = relu_result.to_vec()?;

            let output_sum: f64 = output.iter().map(|&x| x as f64).sum();
            let latency = start.elapsed();

            let vendor_str = match vendor {
                GpuVendor::Nvidia => "NVIDIA",
                GpuVendor::Amd => "AMD",
                _ => "Other",
            };

            tx.send(TierResult {
                tier: "GPU",
                device: format!("{device_name} ({vendor_str})"),
                latency_ms: latency.as_secs_f64() * 1000.0,
                output_sum,
            })
            .await
            .ok();

            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
    }

    // ────────────────────────────────────────────────────────────────────────
    // NPU Tasks: Event-driven neuromorphic processing
    // ────────────────────────────────────────────────────────────────────────

    if let Ok(mut npu) = npu1 {
        let tx = tx.clone();
        // Convert to spike-like input (normalized to 0-1 range for NPU)
        let npu_input: Vec<f32> = input_data.iter()
            .take(1000) // NPU processes first 1000 values
            .map(|&x| x.clamp(0.0, 1.0))
            .collect();

        join_set.spawn(async move {
            let start = Instant::now();

            // Run neuromorphic inference
            let output: Vec<f32> = npu.infer(&npu_input)?;
            let output_sum: f64 = output.iter().map(|&x| x as f64).sum();
            let latency = start.elapsed();

            tx.send(TierResult {
                tier: "NPU",
                device: "Akida #1 (0000:a1:00.0)".to_string(),
                latency_ms: latency.as_secs_f64() * 1000.0,
                output_sum,
            })
            .await
            .ok();

            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
    }

    if let Ok(mut npu) = npu2 {
        let tx = tx.clone();
        let npu_input: Vec<f32> = input_data
            .iter()
            .skip(1000)
            .take(1000)
            .map(|&x| x.clamp(0.0, 1.0))
            .collect();

        join_set.spawn(async move {
            let start = Instant::now();

            let output: Vec<f32> = npu.infer(&npu_input)?;
            let output_sum: f64 = output.iter().map(|&x| x as f64).sum();
            let latency = start.elapsed();

            tx.send(TierResult {
                tier: "NPU",
                device: "Akida #2 (0000:e2:00.0)".to_string(),
                latency_ms: latency.as_secs_f64() * 1000.0,
                output_sum,
            })
            .await
            .ok();

            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
    }

    // ────────────────────────────────────────────────────────────────────────
    // CPU Task: Sparse linear algebra (what CPUs excel at)
    // ────────────────────────────────────────────────────────────────────────

    {
        let tx = tx.clone();
        let cpu_data = input_data.clone();

        join_set.spawn(async move {
            let start = Instant::now();

            // Solve sparse system: Create a sparse matrix and solve Ax = b
            // This is where CPUs shine - sparse operations with good cache utilization
            let n = 256;

            // Create a sparse tridiagonal matrix (common in PDE solvers)
            let mut triplets = Vec::new();

            for i in 0..n {
                if i > 0 {
                    triplets.push((i, i - 1, -1.0f64));
                }
                triplets.push((i, i, 4.0f64)); // Diagonal dominance for convergence
                if i < n - 1 {
                    triplets.push((i, i + 1, -1.0f64));
                }
            }

            let sparse_a = CsrMatrix::from_triplets(n, n, &triplets);

            // RHS vector from input data
            let b: Vec<f64> = cpu_data.iter().take(n).map(|&x| x as f64).collect();

            // Solve using Conjugate Gradient
            let result = cg_solve(&sparse_a, &b, 1e-10, 1000)?;
            let output_sum: f64 = result.x.iter().sum();
            let latency = start.elapsed();

            tx.send(TierResult {
                tier: "CPU",
                device: format!(
                    "EPYC ({} threads, sparse CG)",
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(1)
                ),
                latency_ms: latency.as_secs_f64() * 1000.0,
                output_sum,
            })
            .await
            .ok();

            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
    }

    // Drop the original sender so the channel closes when all tasks complete
    drop(tx);

    // ════════════════════════════════════════════════════════════════════════
    // Phase 4: Collect results
    // ════════════════════════════════════════════════════════════════════════

    // Wait for all tasks
    while let Some(result) = join_set.join_next().await {
        if let Err(e) = result {
            eprintln!("Task failed: {e}");
        }
    }

    // Collect results from channel
    let mut gpu_results = Vec::new();
    let mut npu_results = Vec::new();
    let mut cpu_result = None;

    while let Ok(result) = rx.try_recv() {
        match result.tier {
            "GPU" => gpu_results.push(result),
            "NPU" => npu_results.push(result),
            "CPU" => cpu_result = Some(result),
            _ => {}
        }
    }

    let total_latency = total_start.elapsed();

    // ════════════════════════════════════════════════════════════════════════
    // Phase 5: Display results
    // ════════════════════════════════════════════════════════════════════════

    println!("═══ Results ═══");
    println!();

    println!("┌─────────────────────────────────────────────────────────────────────┐");
    println!("│ TIER  │ DEVICE                                    │ LATENCY │ OUTPUT│");
    println!("├─────────────────────────────────────────────────────────────────────┤");

    for r in &gpu_results {
        println!(
            "│ GPU   │ {:41} │ {:6.2}ms │ {:.2e} │",
            truncate(&r.device, 41),
            r.latency_ms,
            r.output_sum
        );
    }

    for r in &npu_results {
        println!(
            "│ NPU   │ {:41} │ {:6.2}ms │ {:.2e} │",
            truncate(&r.device, 41),
            r.latency_ms,
            r.output_sum
        );
    }

    if let Some(ref r) = cpu_result {
        println!(
            "│ CPU   │ {:41} │ {:6.2}ms │ {:.2e} │",
            truncate(&r.device, 41),
            r.latency_ms,
            r.output_sum
        );
    }

    println!("└─────────────────────────────────────────────────────────────────────┘");
    println!();

    // Aggregate statistics
    let total_gpu_output: f64 = gpu_results.iter().map(|r| r.output_sum).sum();
    let total_npu_output: f64 = npu_results.iter().map(|r| r.output_sum).sum();
    let total_cpu_output: f64 = cpu_result.as_ref().map(|r| r.output_sum).unwrap_or(0.0);

    let gpu_count = gpu_results.len();
    let npu_count = npu_results.len();

    println!("═══ Ensemble Summary ═══");
    println!();
    println!("Hardware Utilized:");
    println!("  • {gpu_count} GPUs (NVIDIA + AMD via same WGSL shaders)");
    println!("  • {npu_count} NPUs (Akida AKD1000 via pure Rust VFIO)");
    println!("  • 1 CPU tier (sparse linear algebra)");
    println!();
    println!("Aggregate Output:");
    println!("  • GPU tier:  {total_gpu_output:.6e}");
    println!("  • NPU tier:  {total_npu_output:.6e}");
    println!("  • CPU tier:  {total_cpu_output:.6e}");
    println!(
        "  • Combined:  {:.6e}",
        total_gpu_output + total_npu_output + total_cpu_output
    );
    println!();
    println!(
        "Total wall-clock time: {:.2}ms",
        total_latency.as_secs_f64() * 1000.0
    );
    println!();

    // Power estimate
    let estimated_power = (gpu_count as f64 * 350.0) + (npu_count as f64 * 1.5) + 200.0; // rough TDP
    println!(
        "Estimated peak power: ~{estimated_power:.0}W ({gpu_count} GPU × 350W + {npu_count} NPU × 1.5W + CPU)"
    );

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║     HETEROGENEOUS ENSEMBLE COMPLETE — ALL SILICON ACTIVE             ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{s:max_len$}")
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
