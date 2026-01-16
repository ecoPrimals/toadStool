//! Benchmark Both GPUs - NVIDIA and AMD
//!
//! Runs comprehensive benchmarks on both GPUs to measure actual performance
//! improvements from optimizations.

use ml_inference_showcase::wgpu::{WgpuExecutor, NormConfig};
use ml_inference_showcase::substrate::GpuSelection;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Comprehensive GPU Benchmark - NVIDIA vs AMD");
    println!("================================================\n");

    // Benchmark both GPUs
    for (vendor_name, selection) in [
        ("NVIDIA", GpuSelection::nvidia()),
        ("AMD", GpuSelection::amd()),
    ] {
        println!("═══════════════════════════════════════════════════════════");
        println!("📊 Benchmarking {} GPU", vendor_name);
        println!("═══════════════════════════════════════════════════════════\n");

        // Create executor with specific GPU
        let executor = match WgpuExecutor::new_with_selection(selection).await {
            Ok(exec) => exec,
            Err(e) => {
                println!("⚠️  Failed to create executor for {}: {}\n\n", vendor_name, e);
                continue;
            }
        };

        let gpu_info = executor.gpu_info();
        println!("GPU: {}\n", gpu_info);

        // Prepare test data
        let size = 512;
        let a: Vec<f32> = (0..size * size).map(|i| (i as f32) * 0.001).collect();
        let b: Vec<f32> = (0..size * size).map(|i| ((i + 1) as f32) * 0.001).collect();

        // Test 1: Synchronous Execution (baseline)
        println!("TEST 1: Synchronous Execution");
        println!("─────────────────────────────\n");

        let start = Instant::now();
        let r1 = executor.execute_matmul(&a, &b, size, size, size).await?;
        let r2 = executor.execute_relu(&r1).await?;
        let _r3 = executor.execute_softmax(&r2).await?;
        let sync_duration = start.elapsed();

        println!("✅ Synchronous: {:.2}ms\n", sync_duration.as_secs_f64() * 1000.0);

        // Test 2: Concurrent Execution (async optimization)
        println!("TEST 2: Concurrent Execution (Async)");
        println!("────────────────────────────────────\n");

        let a2 = a.clone();
        let b2 = b.clone();
        let c = a.clone();
        let a3 = a.clone();

        let start = Instant::now();
        let (r1, r2, r3) = tokio::join!(
            executor.execute_matmul(&a, &b, size, size, size),
            executor.execute_matmul(&b2, &c, size, size, size),
            executor.execute_matmul(&c, &a3, size, size, size),
        );
        let _ = (r1?, r2?, r3?);
        let async_duration = start.elapsed();

        println!("✅ Async: {:.2}ms\n", async_duration.as_secs_f64() * 1000.0);

        // Test 3: Tiled MatMul (memory optimization)
        println!("TEST 3: Tiled MatMul (Memory Optimization)");
        println!("──────────────────────────────────────────\n");

        let start = Instant::now();
        let _result = executor.execute_matmul_tiled(&a, &b, size, size, size).await?;
        let tiled_duration = start.elapsed();

        println!("✅ Tiled MatMul: {:.2}ms\n", tiled_duration.as_secs_f64() * 1000.0);

        // Test 4: LayerNorm Comparison
        println!("TEST 4: LayerNorm (Original vs 2-Dispatch)");
        println!("───────────────────────────────────────────\n");

        let ln_size = 4096;
        let ln_input: Vec<f32> = (0..ln_size).map(|i| (i as f32) * 0.001).collect();
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: None,
            beta: None,
        };

        let start = Instant::now();
        let _r1 = executor.execute_layernorm(&ln_input, config.clone()).await?;
        let ln_original = start.elapsed();

        let start = Instant::now();
        let _r2 = executor.execute_layernorm_2dispatch(&ln_input, config.clone()).await?;
        let ln_2dispatch = start.elapsed();

        println!("✅ Original LayerNorm: {:.2}ms", ln_original.as_secs_f64() * 1000.0);
        println!("✅ 2-Dispatch LayerNorm: {:.2}ms\n", ln_2dispatch.as_secs_f64() * 1000.0);

        // Summary
        println!("─────────────────────────────────────────────────────────");
        println!("📊 {} GPU Performance Summary", vendor_name);
        println!("─────────────────────────────────────────────────────────\n");

        let async_speedup = sync_duration.as_secs_f64() / async_duration.as_secs_f64();
        let ln_speedup = ln_original.as_secs_f64() / ln_2dispatch.as_secs_f64();

        println!("Async Speedup:     {:.2}x ({:.2}ms → {:.2}ms)", 
            async_speedup,
            sync_duration.as_secs_f64() * 1000.0,
            async_duration.as_secs_f64() * 1000.0);

        println!("LayerNorm Speedup: {:.2}x ({:.2}ms → {:.2}ms)",
            ln_speedup,
            ln_original.as_secs_f64() * 1000.0,
            ln_2dispatch.as_secs_f64() * 1000.0);

        println!("Tiled MatMul:      {:.2}ms (vs {:.2}ms naive)\n",
            tiled_duration.as_secs_f64() * 1000.0,
            sync_duration.as_secs_f64() * 1000.0 / 3.0);

        println!("\n");
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("✅ Benchmark Complete!");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
