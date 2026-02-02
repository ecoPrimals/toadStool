//! NVIDIA vs AMD Performance Benchmark
//!
//! Measures actual performance on both GPUs with all optimizations

use ml_inference_showcase::wgpu::{NormConfig, WgpuExecutor};
use std::time::Instant;

async fn benchmark_gpu(executor: &WgpuExecutor, gpu_name: &str) -> anyhow::Result<()> {
    println!("═══════════════════════════════════════════════════════════");
    println!("📊 Benchmarking: {}", gpu_name);
    println!("═══════════════════════════════════════════════════════════\n");

    // Prepare test data
    let size = 512;
    let a: Vec<f32> = (0..size * size).map(|i| (i as f32) * 0.001).collect();
    let b: Vec<f32> = (0..size * size).map(|i| ((i + 1) as f32) * 0.001).collect();

    // Test 1: Async Execution
    println!("TEST 1: Async Execution (3 concurrent MatMuls)");
    println!("───────────────────────────────────────────────\n");

    let start = Instant::now();
    let r1 = executor.execute_matmul(&a, &b, size, size, size).await?;
    let r2 = executor.execute_relu(&r1).await?;
    let _r3 = executor.execute_softmax(&r2).await?;
    let sync = start.elapsed();
    println!("  Synchronous: {:.2}ms", sync.as_secs_f64() * 1000.0);

    let b2 = b.clone();
    let c = a.clone();
    let start = Instant::now();
    let (r1, r2, r3) = tokio::join!(
        executor.execute_matmul(&a, &b, size, size, size),
        executor.execute_matmul(&b2, &c, size, size, size),
        executor.execute_matmul(&c, &a, size, size, size),
    );
    let _ = (r1?, r2?, r3?);
    let async_dur = start.elapsed();
    println!("  Async: {:.2}ms", async_dur.as_secs_f64() * 1000.0);
    println!(
        "  ✅ Speedup: {:.2}x\n",
        sync.as_secs_f64() / async_dur.as_secs_f64()
    );

    // Test 2: Tiled MatMul (larger size for better results)
    println!("TEST 2: Tiled MatMul (1024x1024 - large enough for tiling)");
    println!("───────────────────────────────────────────────────────────\n");

    let large_size = 1024;
    let la: Vec<f32> = (0..large_size * large_size)
        .map(|i| ((i % 1000) as f32) * 0.001)
        .collect();
    let lb: Vec<f32> = (0..large_size * large_size)
        .map(|i| (((i + 1) % 1000) as f32) * 0.001)
        .collect();

    let start = Instant::now();
    let _r1 = executor
        .execute_matmul(&la, &lb, large_size, large_size, large_size)
        .await?;
    let naive = start.elapsed();
    println!("  Naive: {:.2}ms", naive.as_secs_f64() * 1000.0);

    let start = Instant::now();
    let _r2 = executor
        .execute_matmul_tiled(&la, &lb, large_size, large_size, large_size)
        .await?;
    let tiled = start.elapsed();
    println!("  Tiled: {:.2}ms", tiled.as_secs_f64() * 1000.0);
    println!(
        "  ✅ Speedup: {:.2}x\n",
        naive.as_secs_f64() / tiled.as_secs_f64()
    );

    // Test 3: LayerNorm 2-Dispatch
    println!("TEST 3: LayerNorm (4096 elements - typical transformer)");
    println!("────────────────────────────────────────────────────────\n");

    let ln_size = 4096;
    let ln_input: Vec<f32> = (0..ln_size).map(|i| (i as f32) * 0.001).collect();
    let config = NormConfig {
        epsilon: 1e-5,
        gamma: None,
        beta: None,
    };

    let start = Instant::now();
    let _r1 = executor
        .execute_layernorm(&ln_input, config.clone())
        .await?;
    let original = start.elapsed();
    println!(
        "  Original (3-pass): {:.2}ms",
        original.as_secs_f64() * 1000.0
    );

    let start = Instant::now();
    let _r2 = executor
        .execute_layernorm_2dispatch(&ln_input, config)
        .await?;
    let optimized = start.elapsed();
    println!("  2-Dispatch: {:.2}ms", optimized.as_secs_f64() * 1000.0);
    println!(
        "  ✅ Speedup: {:.2}x\n",
        original.as_secs_f64() / optimized.as_secs_f64()
    );

    // Summary
    let async_speedup = sync.as_secs_f64() / async_dur.as_secs_f64();
    let tiled_speedup = naive.as_secs_f64() / tiled.as_secs_f64();
    let ln_speedup = original.as_secs_f64() / optimized.as_secs_f64();

    println!("═══════════════════════════════════════════════════════════");
    println!("📊 {} - Summary", gpu_name);
    println!("═══════════════════════════════════════════════════════════\n");

    println!("Optimization              Speedup");
    println!("───────────────────────────────────────────────────────────");
    println!("Async Execution          {:6.2}x", async_speedup);
    println!("Tiled MatMul (1024x1024) {:6.2}x", tiled_speedup);
    println!("2-Dispatch LayerNorm     {:6.2}x", ln_speedup);
    println!("───────────────────────────────────────────────────────────");
    println!(
        "Combined MatMul          {:6.2}x (async × tiling)",
        async_speedup * tiled_speedup
    );
    println!(
        "Combined LayerNorm       {:6.2}x (async × 2-dispatch)",
        async_speedup * ln_speedup
    );
    println!("\n\n");

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n🚀 NVIDIA vs AMD Performance Comparison");
    println!("========================================\n");

    // Benchmark NVIDIA
    println!("Creating NVIDIA executor...");
    match WgpuExecutor::new_nvidia().await {
        Ok(nvidia_executor) => {
            let gpu_info = nvidia_executor.gpu_info();
            benchmark_gpu(&nvidia_executor, &gpu_info).await?;
        }
        Err(e) => {
            println!("⚠️  NVIDIA GPU not available: {}\n\n", e);
        }
    }

    // Benchmark AMD
    println!("Creating AMD executor...");
    match WgpuExecutor::new_amd().await {
        Ok(amd_executor) => {
            let gpu_info = amd_executor.gpu_info();
            benchmark_gpu(&amd_executor, &gpu_info).await?;
        }
        Err(e) => {
            println!("⚠️  AMD GPU not available: {}\n\n", e);
        }
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("✅ Benchmark Complete!");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
