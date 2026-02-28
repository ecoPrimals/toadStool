//! Warmup Demo - ToadStool "Mise en Place"
//!
//! Demonstrates the difference between cold and warm shader cache
//! and how ToadStool can intelligently pre-warm based on workload hints.

use barracuda::device::pipeline_cache::GLOBAL_CACHE;
use barracuda::device::{warmup_pool, WarmupConfig, WarmupWorkloadHint};
use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use barracuda::prelude::*;
use std::sync::Arc;
use std::time::Instant;

async fn benchmark_cold_vs_warm(
    device: Arc<WgpuDevice>,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let name = device.name().to_string();
    let size = 1_000_000usize;

    // Create test data
    let data: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
    let a = Tensor::from_data(&data, vec![size], device.clone())?;
    let b = Tensor::from_data(&data, vec![size], device.clone())?;

    println!("\n  ═══════════════════════════════════════════════════════════════");
    println!("  {name} - Cold vs Warm Comparison");
    println!("  ═══════════════════════════════════════════════════════════════\n");

    // Clear cache to simulate cold start
    GLOBAL_CACHE.clear();

    // Cold run
    let cold_start = Instant::now();
    let _ = a.add(&b)?;
    let cold_time = cold_start.elapsed();

    // Warm run (cache now populated)
    let mut warm_times = Vec::with_capacity(10);
    for _ in 0..10 {
        let start = Instant::now();
        let _ = a.add(&b)?;
        warm_times.push(start.elapsed().as_micros() as f64);
    }
    let warm_avg = warm_times.iter().sum::<f64>() / warm_times.len() as f64;

    println!("  Cold (first call):     {:>8.0} μs", cold_time.as_micros());
    println!("  Warm (cached, avg):    {warm_avg:>8.0} μs");
    println!(
        "  Speedup:               {:>8.1}x",
        cold_time.as_micros() as f64 / warm_avg
    );

    Ok(())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  ToadStool Mise en Place Demo                                                 ║");
    println!("║  Pre-warming shader cache before workload execution                           ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");

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
        println!("\nNo GPUs found!");
        return Ok(());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Part 1: Show cold vs warm difference
    // ═══════════════════════════════════════════════════════════════════════════

    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  Part 1: Cold vs Warm Cache Comparison                                        ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");

    for device in &devices {
        benchmark_cold_vs_warm(device.clone()).await?;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Part 2: Demonstrate proactive warmup
    // ═══════════════════════════════════════════════════════════════════════════

    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  Part 2: Proactive Warmup (Mise en Place)                                     ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    // Clear cache
    GLOBAL_CACHE.clear();
    println!("  Cache cleared. Starting fresh.\n");

    // Warm up based on workload hint
    println!("  Scenario: Scientific computing workload incoming...\n");

    let warmup_start = Instant::now();
    let mut warmup_config = WarmupWorkloadHint::Scientific.to_config();
    warmup_config.verbose = true;
    let _results = warmup_pool(&devices, &warmup_config)?;
    let warmup_time = warmup_start.elapsed();

    println!(
        "  Warmup complete in {:.1}ms\n",
        warmup_time.as_secs_f64() * 1000.0
    );

    // Now run the same benchmark - should be warm from the start
    println!("  Running benchmark after warmup:");

    for device in &devices {
        let size = 1_000_000usize;
        let data: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
        let a = Tensor::from_data(&data, vec![size], device.clone())?;
        let b = Tensor::from_data(&data, vec![size], device.clone())?;

        // All calls should now be warm
        let mut times = Vec::with_capacity(10);
        for _ in 0..10 {
            let start = Instant::now();
            let _ = a.add(&b)?;
            times.push(start.elapsed().as_micros() as f64);
        }
        let avg = times.iter().sum::<f64>() / times.len() as f64;
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);

        println!(
            "    {} - All warm: min={:.0}μs, avg={:.0}μs",
            device.name(),
            min,
            avg
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Part 3: Different workload presets
    // ═══════════════════════════════════════════════════════════════════════════

    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  Part 3: Workload-Specific Warmup Presets                                     ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let presets = [
        ("Minimal", WarmupConfig::minimal()),
        ("Scientific", WarmupConfig::scientific()),
        ("ML Inference", WarmupConfig::ml()),
        ("Full", WarmupConfig::full()),
    ];

    for (name, mut config) in presets {
        GLOBAL_CACHE.clear();
        config.verbose = false;

        let start = Instant::now();
        let _ = warmup_pool(&devices, &config)?;
        let elapsed = start.elapsed();

        let stats = GLOBAL_CACHE.stats();
        println!(
            "  {:15} {:>3} ops × {:>2} wg_sizes = {:>3} pipelines in {:>6.1}ms",
            name,
            config.ops.len(),
            config.workgroup_sizes.len(),
            stats.pipelines,
            elapsed.as_secs_f64() * 1000.0
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Summary
    // ═══════════════════════════════════════════════════════════════════════════

    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  Summary: ToadStool Intelligence                                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    println!("  ┌─────────────────────────────────────────────────────────────────────────┐");
    println!("  │  Without Warmup:                                                         │");
    println!("  │    - First operation: 5,000-10,000 μs (cold compilation)                │");
    println!("  │    - Latency spike at start of every workload                           │");
    println!("  │                                                                          │");
    println!("  │  With ToadStool Mise en Place:                                          │");
    println!("  │    - Upfront warmup: ~100-500ms (amortized once)                        │");
    println!("  │    - All operations start warm: 300-500 μs                              │");
    println!("  │    - Predictable latency throughout workload                            │");
    println!("  │                                                                          │");
    println!("  │  Next Steps:                                                            │");
    println!("  │    - Learn workload patterns from task history                          │");
    println!("  │    - Auto-detect required ops from task graph                           │");
    println!("  │    - Persist warm cache across sessions                                 │");
    println!("  └─────────────────────────────────────────────────────────────────────────┘");

    Ok(())
}
