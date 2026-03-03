// SPDX-License-Identifier: AGPL-3.0-or-later
//! FMA vs Separate Operations Benchmark
//!
//! Compares fused multiply-add (d = a * b + c) against separate mul + add.
//! FMA should be ~2x faster due to single memory pass vs two.

use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use barracuda::prelude::*;
use std::sync::Arc;
use std::time::Instant;

async fn benchmark_fma(device: &Arc<WgpuDevice>, size: usize, iterations: usize) -> f64 {
    let a_data: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
    let b_data: Vec<f32> = (0..size)
        .map(|i| ((i + 500) % 1000) as f32 * 0.001)
        .collect();
    let c_data: Vec<f32> = (0..size)
        .map(|i| ((i + 250) % 1000) as f32 * 0.001)
        .collect();

    let a = Tensor::from_data(&a_data, vec![size], device.clone()).unwrap();
    let b = Tensor::from_data(&b_data, vec![size], device.clone()).unwrap();
    let c = Tensor::from_data(&c_data, vec![size], device.clone()).unwrap();

    // Warmup
    for _ in 0..3 {
        let _ = a.fma(&b, &c).unwrap();
    }
    device.device().poll(wgpu::Maintain::Wait);

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = a.fma(&b, &c).unwrap();
    }
    device.device().poll(wgpu::Maintain::Wait);

    start.elapsed().as_secs_f64() * 1000.0 / iterations as f64
}

async fn benchmark_separate(device: &Arc<WgpuDevice>, size: usize, iterations: usize) -> f64 {
    let a_data: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
    let b_data: Vec<f32> = (0..size)
        .map(|i| ((i + 500) % 1000) as f32 * 0.001)
        .collect();
    let c_data: Vec<f32> = (0..size)
        .map(|i| ((i + 250) % 1000) as f32 * 0.001)
        .collect();

    let a = Tensor::from_data(&a_data, vec![size], device.clone()).unwrap();
    let b = Tensor::from_data(&b_data, vec![size], device.clone()).unwrap();
    let c = Tensor::from_data(&c_data, vec![size], device.clone()).unwrap();

    // Warmup
    for _ in 0..3 {
        let mul = a.mul(&b).unwrap();
        let _ = mul.add(&c).unwrap();
    }
    device.device().poll(wgpu::Maintain::Wait);

    let start = Instant::now();
    for _ in 0..iterations {
        let mul = a.mul(&b).unwrap();
        let _ = mul.add(&c).unwrap();
    }
    device.device().poll(wgpu::Maintain::Wait);

    start.elapsed().as_secs_f64() * 1000.0 / iterations as f64
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  FMA vs Separate Operations Benchmark                                        ║");
    println!("║  Comparing: d = a * b + c (fused) vs mul then add (2 ops)                   ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let config = WorkloadConfig {
        exclude_software: true,
        min_gflops: 100.0,
        ..Default::default()
    };

    let pool = GpuPool::with_config(config).await?;
    let devices: Vec<_> = (0..pool.device_count())
        .filter_map(|i| pool.device(i))
        .collect();

    if devices.is_empty() {
        println!("No GPUs found!");
        return Ok(());
    }

    let sizes = [100_000, 1_000_000, 10_000_000];
    let iterations = 100;

    for device in &devices {
        println!("══════════════════════════════════════════════════════════════════════════════");
        println!("  {}", device.name());
        println!(
            "══════════════════════════════════════════════════════════════════════════════\n"
        );

        println!("  ┌────────────────┬──────────────┬──────────────┬──────────────┐");
        println!("  │ Size           │ FMA (μs)     │ Sep (μs)     │ Speedup      │");
        println!("  ├────────────────┼──────────────┼──────────────┼──────────────┤");

        for &size in &sizes {
            let fma_time = benchmark_fma(device, size, iterations).await;
            let sep_time = benchmark_separate(device, size, iterations).await;
            let speedup = sep_time / fma_time;

            let size_str = if size >= 1_000_000 {
                format!("{}M", size / 1_000_000)
            } else {
                format!("{}K", size / 1_000)
            };

            println!(
                "  │ {:>14} │ {:>10.1} │ {:>10.1} │ {:>10.2}x │",
                size_str,
                fma_time * 1000.0, // Convert to μs
                sep_time * 1000.0,
                speedup
            );
        }

        println!("  └────────────────┴──────────────┴──────────────┴──────────────┘");
        println!();
    }

    println!("═══ ANALYSIS ═══\n");
    println!("  FMA advantage: Single dispatch, single memory pass");
    println!("  Separate ops: Two dispatches, intermediate buffer allocation");
    println!();
    println!("  Expected speedup: 1.5-2x (actual depends on memory bandwidth)");
    println!("  At large sizes, memory bandwidth dominates → FMA has bigger advantage");

    Ok(())
}
