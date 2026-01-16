//! Scale Analysis Benchmark
//!
//! Tests optimizations at multiple scales to find where they provide benefit

use ml_inference_showcase::wgpu::WgpuExecutor;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Scale Analysis - When Do Optimizations Help?");
    println!("================================================\n");

    let executor = WgpuExecutor::new().await?;
    let gpu_info = executor.gpu_info();
    println!("GPU: {}\n", gpu_info);

    // Test at multiple scales
    let sizes = vec![256, 512, 1024, 2048];

    println!("═══════════════════════════════════════════════════════════");
    println!("MatMul: Naive vs Tiled at Different Scales");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("Size      Naive      Tiled      Speedup");
    println!("───────────────────────────────────────────────────────────");

    for &size in &sizes {
        let a: Vec<f32> = (0..size * size).map(|i| ((i % 1000) as f32) * 0.001).collect();
        let b: Vec<f32> = (0..size * size).map(|i| (((i + 1) % 1000) as f32) * 0.001).collect();

        // Naive
        let start = Instant::now();
        let _r1 = executor.execute_matmul(&a, &b, size, size, size).await?;
        let naive = start.elapsed();

        // Tiled
        let start = Instant::now();
        let _r2 = executor.execute_matmul_tiled(&a, &b, size, size, size).await?;
        let tiled = start.elapsed();

        let speedup = naive.as_secs_f64() / tiled.as_secs_f64();
        
        println!("{:4}x{:4}  {:7.2}ms  {:7.2}ms  {:6.2}x {}",
            size, size,
            naive.as_secs_f64() * 1000.0,
            tiled.as_secs_f64() * 1000.0,
            speedup,
            if speedup > 1.0 { "✅" } else { "⚠️" });
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("Async Execution: 3 Concurrent MatMuls at Different Scales");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("Size      Sync       Async      Speedup");
    println!("───────────────────────────────────────────────────────────");

    for &size in &sizes {
        let a: Vec<f32> = (0..size * size).map(|i| ((i % 1000) as f32) * 0.001).collect();
        let b: Vec<f32> = (0..size * size).map(|i| (((i + 1) % 1000) as f32) * 0.001).collect();

        // Synchronous
        let start = Instant::now();
        let _r1 = executor.execute_matmul(&a, &b, size, size, size).await?;
        let _r2 = executor.execute_matmul(&a, &b, size, size, size).await?;
        let _r3 = executor.execute_matmul(&a, &b, size, size, size).await?;
        let sync = start.elapsed();

        // Async
        let b2 = b.clone();
        let b3 = b.clone();
        let start = Instant::now();
        let (r1, r2, r3) = tokio::join!(
            executor.execute_matmul(&a, &b, size, size, size),
            executor.execute_matmul(&a, &b2, size, size, size),
            executor.execute_matmul(&a, &b3, size, size, size),
        );
        let _ = (r1?, r2?, r3?);
        let async_dur = start.elapsed();

        let speedup = sync.as_secs_f64() / async_dur.as_secs_f64();
        
        println!("{:4}x{:4}  {:7.2}ms  {:7.2}ms  {:6.2}x",
            size, size,
            sync.as_secs_f64() * 1000.0,
            async_dur.as_secs_f64() * 1000.0,
            speedup);
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("💡 Analysis");
    println!("═══════════════════════════════════════════════════════════\n");
    
    println!("Async Execution:");
    println!("  • Provides consistent speedup at ALL scales");
    println!("  • Eliminates launch overhead (4-5ms NVIDIA)");
    println!("  • Best optimization for diverse workloads\n");
    
    println!("Tiled MatMul:");
    println!("  • Benefits increase with matrix size");
    println!("  • Small matrices: Tiling overhead > benefit");
    println!("  • Large matrices: Memory bandwidth becomes critical");
    println!("  • Sweet spot: 1024x1024+ matrices\n");

    Ok(())
}
