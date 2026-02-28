//! Cache Validation Benchmark
//!
//! Tests that pipeline caching is actually working in Tensor::add()

use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use barracuda::prelude::*;
use std::time::Instant;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     CACHE VALIDATION BENCHMARK                                                ║");
    println!("║     Testing that Tensor::add() uses pipeline caching                          ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let config = WorkloadConfig {
        exclude_software: true,
        min_gflops: 100.0,
        ..Default::default()
    };

    let pool = GpuPool::with_config(config).await?;

    let size = 1_000_000usize; // 1M elements
    let iterations = 20;

    // Test all GPUs with fingerprint-based caching
    for idx in 0..pool.devices().len() {
        let wgpu_device =
            pool.device(idx)
                .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                    std::io::Error::other("No device").into()
                })?;
        // Use the device directly from the pool - don't clone/rewrap
        let device_arc = wgpu_device.clone();
        let name = wgpu_device.name();

        println!(
            "\n══════════════════════════════════════════════════════════════════════════════"
        );
        println!("  {name}");
        println!(
            "══════════════════════════════════════════════════════════════════════════════\n"
        );

        // Create test data
        let data: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
        let a = Tensor::from_data(&data, vec![size], device_arc.clone())?;
        let b = Tensor::from_data(&data, vec![size], device_arc.clone())?;

        // Warmup (should compile/cache on first call)
        println!("  Warmup (compiling and caching)...");
        let warmup_start = Instant::now();
        let _ = a.add(&b)?;
        let warmup_time = warmup_start.elapsed().as_micros();
        println!("  First call (cold cache): {warmup_time} μs\n");

        // Measure subsequent calls (should hit cache)
        let mut times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            let _ = a.add(&b)?;
            times.push(start.elapsed().as_micros() as f64);
        }

        let avg = times.iter().sum::<f64>() / iterations as f64;
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = times.iter().cloned().fold(0.0f64, f64::max);

        println!("  Subsequent calls (warm cache):");
        println!("     Min:     {min:>8.1} μs");
        println!("     Max:     {max:>8.1} μs");
        println!("     Average: {avg:>8.1} μs\n");

        // Speedup
        let speedup = warmup_time as f64 / avg;
        println!("  Cache effectiveness: {speedup:.1}x faster after caching");

        // Compare to targets
        println!("\n  ──────────────────────────────────────────────────────────────");
        println!("  Comparison:");
        println!("    Before caching (expected): ~1200 μs");
        println!("    After caching (expected):  ~200-300 μs");
        println!("    CUDA reference:            ~15-50 μs");
        println!("    ROCm reference:            ~20-60 μs");
        println!("  ──────────────────────────────────────────────────────────────");

        if avg < 400.0 {
            println!("  ✅ CACHING IS WORKING! ({avg:.0}μs is much less than ~1200μs)");
        } else {
            println!("  ❌ CACHING MAY NOT BE WORKING ({avg:.0}μs is too high)");
        }
    }

    // Print cache stats
    println!("\n═══ PIPELINE CACHE STATS ═══\n");
    let stats = barracuda::device::pipeline_cache::GLOBAL_CACHE.stats();
    println!("  Shaders cached:   {}", stats.shaders);
    println!("  Layouts cached:   {}", stats.layouts);
    println!("  Pipelines cached: {}", stats.pipelines);

    Ok(())
}
