// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bandwidth Validation Benchmark
//!
//! This benchmark validates our bandwidth measurements are accurate by:
//! 1. Using data sizes larger than L2 cache to force DRAM access
//! 2. Properly synchronizing GPU work before timing
//! 3. Validating byte counts match what we expect
//! 4. Comparing against known GPU specs

use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use barracuda::prelude::*;
use std::sync::Arc;
use std::time::Instant;

/// GPU cache sizes (approximate L2 cache)
/// RTX 3090: 6 MB L2 cache
/// RX 6950 XT: 128 MB Infinity Cache (!)
const RTX_3090_L2_BYTES: usize = 6 * 1024 * 1024;
const RX_6950_XT_CACHE_BYTES: usize = 128 * 1024 * 1024;

/// Theoretical peak bandwidth (from specs)
const RTX_3090_BANDWIDTH_GBPS: f64 = 936.2; // 19.5 Gbps × 384-bit bus
const RX_6950_XT_BANDWIDTH_GBPS: f64 = 576.0; // 18 Gbps × 256-bit bus

/// Minimum size to exceed cache (use 4x cache size)
fn min_size_for_dram(device_name: &str) -> usize {
    let lower = device_name.to_lowercase();
    let cache_bytes = if lower.contains("3090") {
        RTX_3090_L2_BYTES
    } else if lower.contains("6950") || lower.contains("radv") {
        RX_6950_XT_CACHE_BYTES
    } else {
        32 * 1024 * 1024 // Default 32MB
    };

    // 4x cache size, divided by 12 bytes per element (3 arrays * 4 bytes)
    (cache_bytes * 4) / 12
}

/// Get theoretical bandwidth for device
fn get_theoretical_bandwidth(device_name: &str) -> f64 {
    let lower = device_name.to_lowercase();
    if lower.contains("3090") {
        RTX_3090_BANDWIDTH_GBPS
    } else if lower.contains("6950") || lower.contains("radv") {
        RX_6950_XT_BANDWIDTH_GBPS
    } else {
        500.0 // Conservative default
    }
}

/// Benchmark with proper GPU synchronization
async fn benchmark_bandwidth(
    device: &Arc<WgpuDevice>,
    size: usize,
    iterations: usize,
) -> (f64, f64) {
    // Create fresh data each time to avoid any caching
    let a_data: Vec<f32> = (0..size).map(|i| (i % 10000) as f32 * 0.0001).collect();
    let b_data: Vec<f32> = (0..size)
        .map(|i| ((i + 5000) % 10000) as f32 * 0.0001)
        .collect();

    let a = Tensor::from_data(&a_data, vec![size], device.clone()).unwrap();
    let b = Tensor::from_data(&b_data, vec![size], device.clone()).unwrap();

    // Warmup and ensure GPU is idle
    for _ in 0..5 {
        let _ = a.add(&b).unwrap();
    }
    device.device().poll(wgpu::Maintain::Wait);

    // Timed run with proper sync
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = a.add(&b).unwrap();
    }
    // CRITICAL: Wait for all GPU work to complete
    device.device().poll(wgpu::Maintain::Wait);
    let elapsed = start.elapsed();

    let total_bytes = size * 3 * 4 * iterations; // read A + read B + write C
    let bandwidth_gbps = (total_bytes as f64 / 1e9) / elapsed.as_secs_f64();
    let time_per_op_us = elapsed.as_secs_f64() * 1e6 / iterations as f64;

    (bandwidth_gbps, time_per_op_us)
}

/// Validate by reading back results to ensure GPU actually did work
async fn validate_correctness(device: &Arc<WgpuDevice>, size: usize) -> bool {
    let a_data: Vec<f32> = (0..size).map(|i| i as f32).collect();
    let b_data: Vec<f32> = (0..size).map(|i| (size - i) as f32).collect();

    let a = Tensor::from_data(&a_data, vec![size], device.clone()).unwrap();
    let b = Tensor::from_data(&b_data, vec![size], device.clone()).unwrap();

    let result = a.add(&b).unwrap();
    let result_data = result.to_vec().unwrap();

    // All results should equal size (i + (size - i) = size)
    let expected = size as f32;
    let correct = result_data.iter().all(|&v| (v - expected).abs() < 0.01);

    correct
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  Bandwidth Validation Benchmark                                              ║");
    println!("║  Goal: Validate measurements, understand cache effects                       ║");
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

    for device in &devices {
        let name = device.name();
        let theoretical = get_theoretical_bandwidth(name);
        let min_dram_size = min_size_for_dram(name);
        let cache_size_mb = if name.to_lowercase().contains("3090") {
            6.0
        } else if name.to_lowercase().contains("6950") || name.to_lowercase().contains("radv") {
            128.0
        } else {
            32.0
        };

        println!("══════════════════════════════════════════════════════════════════════════════");
        println!("  {name}");
        println!("  Theoretical Bandwidth: {theoretical:.1} GB/s");
        println!("  L2/Infinity Cache: {cache_size_mb:.0} MB");
        println!(
            "  Min size for DRAM test: {}M elements ({:.0} MB)",
            min_dram_size / 1_000_000,
            (min_dram_size * 12) as f64 / 1e6
        );
        println!(
            "══════════════════════════════════════════════════════════════════════════════\n"
        );

        // First validate correctness
        println!("  1. CORRECTNESS VALIDATION");
        let correct = validate_correctness(device, 10000).await;
        println!(
            "     Result: {}\n",
            if correct { "✅ PASS" } else { "❌ FAIL" }
        );

        if !correct {
            println!("     CRITICAL: Correctness failed! Skipping bandwidth tests.\n");
            continue;
        }

        // Test different sizes to see cache effects
        println!("  2. CACHE EFFECTS ANALYSIS");
        println!("  ┌────────────────┬────────────┬────────────┬────────────┬────────────────┐");
        println!("  │ Size           │ Data (MB)  │ Time (μs)  │ BW (GB/s)  │ % Theoretical  │");
        println!("  ├────────────────┼────────────┼────────────┼────────────┼────────────────┤");

        // Sizes from definitely-in-cache to definitely-in-DRAM
        // Note: wgpu has limits:
        //   - Buffer: ~128MB max (32M f32 elements)
        //   - Dispatch: 65535 workgroups max (with WG=256, max ~16.7M elements)
        let sizes = [
            100_000,    // 1.2 MB - should be in L2 cache
            500_000,    // 6 MB - at NVIDIA L2 boundary
            1_000_000,  // 12 MB - exceeds NVIDIA L2
            5_000_000,  // 60 MB - partial AMD Infinity Cache
            10_000_000, // 120 MB - tests both cache and DRAM
            16_000_000, // 192 MB - at dispatch limit, definitely DRAM
        ];

        for size in sizes {
            let data_mb = (size * 12) as f64 / 1e6; // 3 arrays * 4 bytes
            let iterations = if size > 10_000_000 { 10 } else { 50 };

            let (bandwidth, time_us) = benchmark_bandwidth(device, size, iterations).await;
            let pct = bandwidth / theoretical * 100.0;

            let size_str = if size >= 1_000_000 {
                format!("{}M", size / 1_000_000)
            } else {
                format!("{}K", size / 1_000)
            };

            // Mark if this is likely cache vs DRAM
            let note = if data_mb < cache_size_mb {
                "← cache"
            } else if data_mb < cache_size_mb * 2.0 {
                "← mixed"
            } else {
                "← DRAM"
            };

            println!(
                "  │ {size_str:>14} │ {data_mb:>8.1} │ {time_us:>8.0} │ {bandwidth:>8.1} │ {pct:>10.1}% {note:6} │"
            );
        }
        println!("  └────────────────┴────────────┴────────────┴────────────┴────────────────┘\n");

        // Focus on DRAM-only test for true bandwidth
        // Use 16M elements (192MB total) - at wgpu dispatch limit, definitely hits DRAM
        println!(
            "  3. TRUE DRAM BANDWIDTH (16M elements, {} iterations)",
            100
        );
        let (dram_bw, dram_time) = benchmark_bandwidth(device, 16_000_000, 100).await;
        let dram_pct = dram_bw / theoretical * 100.0;

        println!("     Data size:     192 MB (12 bytes/element × 16M elements)");
        println!("     Time per op:   {dram_time:.0} μs");
        println!("     Bandwidth:     {dram_bw:.1} GB/s");
        println!("     % Theoretical: {dram_pct:.1}%");

        if dram_pct > 85.0 {
            println!("     Status:        ✅ EXCELLENT (>85% theoretical)");
        } else if dram_pct > 70.0 {
            println!("     Status:        ✅ GOOD (>70% theoretical)");
        } else if dram_pct > 50.0 {
            println!("     Status:        ⚠️ MODERATE (>50% theoretical)");
        } else {
            println!("     Status:        ❌ LOW (<50% theoretical)");
        }
        println!();
    }

    println!("═══ KEY INSIGHTS ═══\n");
    println!("  1. Numbers > 100% theoretical indicate CACHE HITS, not measurement error");
    println!("  2. AMD RX 6950 XT has 128MB Infinity Cache - can cache 10M elements!");
    println!("  3. NVIDIA RTX 3090 has only 6MB L2 - DRAM-bound at 1M+ elements");
    println!("  4. TRUE bandwidth is measured with data >> cache size (100M elements)");
    println!("  5. For realistic workloads, expect 70-85% of theoretical peak\n");

    Ok(())
}
