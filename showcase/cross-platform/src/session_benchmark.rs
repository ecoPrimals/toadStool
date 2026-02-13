//! Session Benchmark - Comparing batched vs unbatched tensor operations
//!
//! Demonstrates the performance improvement from using TensorSession
//! for operation batching vs individual tensor operations.

use anyhow::Result;
use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use barracuda::prelude::*;
use std::time::Instant;

/// Run individual tensor operations (unbatched)
async fn benchmark_unbatched(
    device: &WgpuDevice,
    size: usize,
    num_ops: usize,
) -> Result<(f64, f64)> {
    let data: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();

    let device_arc = std::sync::Arc::new(device.clone());
    let a = Tensor::from_data(&data, vec![size], device_arc.clone())?;
    let b = Tensor::from_data(&data, vec![size], device_arc.clone())?;

    // Warmup
    for _ in 0..3 {
        let _ = a.add(&b)?;
    }

    let start = Instant::now();

    let mut result = a.clone();
    for _ in 0..num_ops {
        result = result.add(&b)?;
    }
    // Force completion
    let _ = result.to_vec()?;

    let elapsed = start.elapsed();
    let total_us = elapsed.as_secs_f64() * 1e6;
    let per_op_us = total_us / num_ops as f64;

    Ok((total_us, per_op_us))
}

/// Run batched tensor operations using TensorSession
fn benchmark_batched(device: &WgpuDevice, size: usize, num_ops: usize) -> Result<(f64, f64)> {
    let data: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();

    // Warmup
    {
        let mut session = TensorSession::new(device);
        let a = session.tensor(&data)?;
        let b = session.tensor(&data)?;
        let _ = session.add(&a, &b)?;
        session.run()?;
    }

    let start = Instant::now();

    let mut session = TensorSession::new(device);
    let mut current = session.tensor(&data)?;
    let b = session.tensor(&data)?;

    for _ in 0..num_ops {
        current = session.add(&current, &b)?;
    }

    session.run()?;

    // Force completion
    let _ = current.to_vec()?;

    let elapsed = start.elapsed();
    let total_us = elapsed.as_secs_f64() * 1e6;
    let per_op_us = total_us / num_ops as f64;

    Ok((total_us, per_op_us))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     TENSOR SESSION BENCHMARK                                                  ║");
    println!("║     Comparing batched vs unbatched performance                                ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let config = WorkloadConfig {
        exclude_software: true,
        min_gflops: 100.0,
        ..Default::default()
    };

    let pool = GpuPool::with_config(config).await?;

    let test_configs = [
        (100_000, 10),   // 100K elements, 10 ops
        (100_000, 50),   // 100K elements, 50 ops
        (100_000, 100),  // 100K elements, 100 ops
        (1_000_000, 10), // 1M elements, 10 ops
        (1_000_000, 50), // 1M elements, 50 ops
    ];

    for idx in 0..pool.devices().len() {
        let wgpu_device = pool
            .device(idx)
            .ok_or_else(|| anyhow::anyhow!("No device"))?;
        let name = wgpu_device.name();
        let wg_size = wgpu_device.optimal_workgroup_size();

        println!(
            "\n══════════════════════════════════════════════════════════════════════════════"
        );
        println!("  {} (WG={})", name, wg_size);
        println!(
            "══════════════════════════════════════════════════════════════════════════════\n"
        );

        println!("┌────────────┬────────┬──────────────┬──────────────┬──────────────┬─────────┐");
        println!("│ Size       │ Ops    │ Unbatched    │ Batched      │ Per-Op       │ Speedup │");
        println!("├────────────┼────────┼──────────────┼──────────────┼──────────────┼─────────┤");

        for (size, num_ops) in &test_configs {
            // Run unbatched
            let (unbatched_total, unbatched_per_op) =
                benchmark_unbatched(&wgpu_device, *size, *num_ops).await?;

            // Run batched
            let (batched_total, batched_per_op) = benchmark_batched(&wgpu_device, *size, *num_ops)?;

            let speedup = unbatched_total / batched_total;

            let size_str = if *size >= 1_000_000 {
                format!("{}M", size / 1_000_000)
            } else {
                format!("{}K", size / 1_000)
            };

            println!(
                "│ {:>10} │ {:>6} │ {:>9.1} ms │ {:>9.1} ms │ {:>7.1}/{:>3.1}μs │ {:>5.1}x │",
                size_str,
                num_ops,
                unbatched_total / 1000.0,
                batched_total / 1000.0,
                unbatched_per_op,
                batched_per_op,
                speedup
            );
        }

        println!("└────────────┴────────┴──────────────┴──────────────┴──────────────┴─────────┘");
    }

    println!("\n═══ ANALYSIS ═══\n");
    println!(
        "  Unbatched: Each operation submits its own command buffer (~200-300μs overhead each)"
    );
    println!("  Batched:   All operations in single submission (overhead amortized)");
    println!("  Speedup:   Higher with more operations (amortization effect)");
    println!("\n  Key insight: The overhead is in wgpu command submission, not GPU execution.");
    println!("  Batching eliminates per-op overhead, approaching theoretical GPU performance.");

    Ok(())
}
