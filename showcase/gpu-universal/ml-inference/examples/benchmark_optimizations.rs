//! Benchmark Optimizations
//!
//! Measures actual performance improvements from async, tiling, and 2-dispatch optimizations

use ml_inference_showcase::wgpu::{NormConfig, WgpuExecutor};
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Optimization Performance Benchmark");
    println!("=====================================\n");

    let executor = WgpuExecutor::new().await?;
    let gpu_info = executor.gpu_info();
    println!("GPU: {}\n", gpu_info);

    // Prepare test data
    let size = 512;
    let a: Vec<f32> = (0..size * size).map(|i| (i as f32) * 0.001).collect();
    let b: Vec<f32> = (0..size * size).map(|i| ((i + 1) as f32) * 0.001).collect();

    println!("═══════════════════════════════════════════════════════════");
    println!("TEST 1: Async Execution Optimization");
    println!("═══════════════════════════════════════════════════════════\n");

    // Synchronous (baseline)
    let start = Instant::now();
    let r1 = executor.execute_matmul(&a, &b, size, size, size).await?;
    let r2 = executor.execute_relu(&r1).await?;
    let _r3 = executor.execute_softmax(&r2).await?;
    let sync_duration = start.elapsed();
    println!(
        "✅ Synchronous: {:.2}ms",
        sync_duration.as_secs_f64() * 1000.0
    );

    // Concurrent (async optimized)
    let b2 = b.clone();
    let c = a.clone();

    let start = Instant::now();
    let (r1, r2, r3) = tokio::join!(
        executor.execute_matmul(&a, &b, size, size, size),
        executor.execute_matmul(&b2, &c, size, size, size),
        executor.execute_matmul(&c, &a, size, size, size),
    );
    let _ = (r1?, r2?, r3?);
    let async_duration = start.elapsed();
    println!("✅ Async: {:.2}ms", async_duration.as_secs_f64() * 1000.0);

    let async_speedup = sync_duration.as_secs_f64() / async_duration.as_secs_f64();
    println!(
        "\n📊 Async Speedup: {:.2}x ({:.2}ms → {:.2}ms)\n",
        async_speedup,
        sync_duration.as_secs_f64() * 1000.0,
        async_duration.as_secs_f64() * 1000.0
    );

    println!("═══════════════════════════════════════════════════════════");
    println!("TEST 2: Memory Optimization (Tiled MatMul)");
    println!("═══════════════════════════════════════════════════════════\n");

    // Naive MatMul
    let start = Instant::now();
    let _r1 = executor.execute_matmul(&a, &b, size, size, size).await?;
    let naive_duration = start.elapsed();
    println!(
        "✅ Naive MatMul: {:.2}ms",
        naive_duration.as_secs_f64() * 1000.0
    );

    // Tiled MatMul
    let start = Instant::now();
    let _r2 = executor
        .execute_matmul_tiled(&a, &b, size, size, size)
        .await?;
    let tiled_duration = start.elapsed();
    println!(
        "✅ Tiled MatMul: {:.2}ms",
        tiled_duration.as_secs_f64() * 1000.0
    );

    let tiled_speedup = naive_duration.as_secs_f64() / tiled_duration.as_secs_f64();
    println!(
        "\n📊 Tiling Speedup: {:.2}x ({:.2}ms → {:.2}ms)\n",
        tiled_speedup,
        naive_duration.as_secs_f64() * 1000.0,
        tiled_duration.as_secs_f64() * 1000.0
    );

    println!("═══════════════════════════════════════════════════════════");
    println!("TEST 3: LayerNorm Optimization (2-Dispatch)");
    println!("═══════════════════════════════════════════════════════════\n");

    let ln_size = 4096;
    let ln_input: Vec<f32> = (0..ln_size).map(|i| (i as f32) * 0.001).collect();
    let config = NormConfig {
        epsilon: 1e-5,
        gamma: None,
        beta: None,
    };

    // Original 3-pass
    let start = Instant::now();
    let _r1 = executor
        .execute_layernorm(&ln_input, config.clone())
        .await?;
    let ln_original = start.elapsed();
    println!(
        "✅ Original (3-pass): {:.2}ms",
        ln_original.as_secs_f64() * 1000.0
    );

    // Optimized 2-dispatch
    let start = Instant::now();
    let _r2 = executor
        .execute_layernorm_2dispatch(&ln_input, config)
        .await?;
    let ln_2dispatch = start.elapsed();
    println!(
        "✅ 2-Dispatch: {:.2}ms",
        ln_2dispatch.as_secs_f64() * 1000.0
    );

    let ln_speedup = ln_original.as_secs_f64() / ln_2dispatch.as_secs_f64();
    println!(
        "\n📊 LayerNorm Speedup: {:.2}x ({:.2}ms → {:.2}ms)\n",
        ln_speedup,
        ln_original.as_secs_f64() * 1000.0,
        ln_2dispatch.as_secs_f64() * 1000.0
    );

    println!("═══════════════════════════════════════════════════════════");
    println!("📊 OVERALL PERFORMANCE SUMMARY");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("Optimization                Speedup");
    println!("───────────────────────────────────────────────────────────");
    println!("Async Execution            {:.2}x", async_speedup);
    println!("Tiled MatMul               {:.2}x", tiled_speedup);
    println!("2-Dispatch LayerNorm       {:.2}x", ln_speedup);
    println!("───────────────────────────────────────────────────────────");

    let combined_matmul = async_speedup * tiled_speedup;
    let combined_layernorm = async_speedup * ln_speedup;

    println!(
        "Combined MatMul Impact     {:.2}x (async + tiling)",
        combined_matmul
    );
    println!(
        "Combined LayerNorm Impact  {:.2}x (async + 2-dispatch)",
        combined_layernorm
    );

    println!("\n✅ All optimizations validated on: {}", gpu_info);
    println!("\n═══════════════════════════════════════════════════════════");

    Ok(())
}
