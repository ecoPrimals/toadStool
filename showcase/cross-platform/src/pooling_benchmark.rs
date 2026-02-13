//! Pooling Benchmark - Validates buffer pooling reduces allocation overhead
//!
//! Tests the TensorContext buffer pooling system:
//! - First ops allocate buffers
//! - Subsequent ops reuse from pool
//! - Steady-state should show zero allocations

use anyhow::Result;
use barracuda::device::{clear_global_contexts, get_device_context, warmup_pool, WarmupConfig};
use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use barracuda::prelude::*;
use std::sync::Arc;
use std::time::Instant;

async fn benchmark_tensor_ops(device: &Arc<WgpuDevice>, size: usize, iterations: usize) -> (f64, f64) {
    let data: Vec<f32> = (0..size).map(|i| (i % 10000) as f32 * 0.0001).collect();
    let a = Tensor::from_data(&data, vec![size], device.clone()).unwrap();
    let b = Tensor::from_data(&data, vec![size], device.clone()).unwrap();
    
    // Warmup (populate pool)
    for _ in 0..5 {
        let _ = a.add(&b).unwrap();
    }
    device.device().poll(wgpu::Maintain::Wait);
    
    // Benchmark add
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = a.add(&b).unwrap();
    }
    device.device().poll(wgpu::Maintain::Wait);
    let add_time = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    
    // Benchmark mul
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = a.mul(&b).unwrap();
    }
    device.device().poll(wgpu::Maintain::Wait);
    let mul_time = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    
    (add_time, mul_time)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  Buffer Pooling Benchmark - Zero-Allocation Tensor Operations                ║");
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

    // Warmup shaders
    println!("Warming up shaders...\n");
    warmup_pool(&devices, &WarmupConfig::default())?;

    for device in &devices {
        let name = device.name();
        println!("══════════════════════════════════════════════════════════════════════════════");
        println!("  {}", name);
        println!("══════════════════════════════════════════════════════════════════════════════\n");

        // Clear context to start fresh
        clear_global_contexts();
        
        // Get fresh context
        let ctx = get_device_context(device);
        
        // Test 1: Cold start (first operations)
        println!("  Test 1: Cold Start (allocating buffers)");
        println!("  ┌────────────────┬────────────┬────────────┬────────────┐");
        println!("  │ Size           │ Add (μs)   │ Mul (μs)   │ Status     │");
        println!("  ├────────────────┼────────────┼────────────┼────────────┤");

        let sizes = [
            (10_000, "10K"),
            (100_000, "100K"),
            (1_000_000, "1M"),
            (10_000_000, "10M"),
        ];

        let mut cold_times = Vec::new();
        for (size, label) in &sizes {
            // Clear pool to force allocation
            clear_global_contexts();
            
            let (add_time, mul_time) = benchmark_tensor_ops(device, *size, 10).await;
            cold_times.push((add_time, mul_time));
            
            println!("  │ {:>14} │ {:>7.0} μs │ {:>7.0} μs │ allocating │",
                label, add_time * 1000.0, mul_time * 1000.0);
        }
        println!("  └────────────────┴────────────┴────────────┴────────────┘");
        
        // Get stats after cold runs
        let ctx = get_device_context(device);
        let stats = ctx.stats();
        println!("\n  After cold start: {}", stats);

        // Test 2: Warm (reusing pooled buffers)
        println!("\n  Test 2: Warm Steady-State (reusing pooled buffers)");
        println!("  ┌────────────────┬────────────┬────────────┬────────────┐");
        println!("  │ Size           │ Add (μs)   │ Mul (μs)   │ Speedup    │");
        println!("  ├────────────────┼────────────┼────────────┼────────────┤");

        for (i, (size, label)) in sizes.iter().enumerate() {
            // Don't clear - use existing pool
            let (add_time, mul_time) = benchmark_tensor_ops(device, *size, 50).await;
            
            let (cold_add, cold_mul) = cold_times[i];
            let add_speedup = cold_add / add_time;
            let mul_speedup = cold_mul / mul_time;
            let avg_speedup = (add_speedup + mul_speedup) / 2.0;
            
            println!("  │ {:>14} │ {:>7.0} μs │ {:>7.0} μs │ {:>7.1}x   │",
                label, add_time * 1000.0, mul_time * 1000.0, avg_speedup);
        }
        println!("  └────────────────┴────────────┴────────────┴────────────┘");

        // Get stats after warm runs
        let stats = ctx.stats();
        println!("\n  After warm runs: {}", stats);

        // Test 3: Sustained throughput with pooling
        println!("\n  Test 3: Sustained Throughput (1 second)");
        
        let size = 1_000_000;
        let data: Vec<f32> = (0..size).map(|i| (i % 10000) as f32 * 0.0001).collect();
        let a = Tensor::from_data(&data, vec![size], device.clone())?;
        let b = Tensor::from_data(&data, vec![size], device.clone())?;
        
        // Warmup pool
        for _ in 0..10 { let _ = a.add(&b)?; }
        device.device().poll(wgpu::Maintain::Wait);
        
        let ctx = get_device_context(device);
        let stats_before = ctx.stats();
        
        // Run for 1 second
        let target = std::time::Duration::from_secs(1);
        let start = Instant::now();
        let mut ops = 0;
        
        while start.elapsed() < target {
            let _ = a.add(&b)?;
            ops += 1;
        }
        device.device().poll(wgpu::Maintain::Wait);
        
        let elapsed = start.elapsed();
        let stats_after = ctx.stats();
        
        let ops_per_sec = ops as f64 / elapsed.as_secs_f64();
        let new_allocs = stats_after.buffer_allocations - stats_before.buffer_allocations;
        let reuses = stats_after.buffer_reuses - stats_before.buffer_reuses;
        
        println!("    Duration:       {:.2}s", elapsed.as_secs_f64());
        println!("    Operations:     {}", ops);
        println!("    Ops/second:     {:.0}", ops_per_sec);
        println!("    New allocations: {} (should be 0 in steady state)", new_allocs);
        println!("    Buffer reuses:   {}", reuses);
        
        if new_allocs == 0 && reuses > 0 {
            println!("    ✅ Zero-allocation steady state achieved!");
        } else {
            println!("    ⚠️ Still allocating - pool needs tuning");
        }

        println!("\n");
    }

    // Summary
    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("  SUMMARY: Buffer Pooling Results");
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    println!("  ┌─────────────────────────────────────────────────────────────────────────┐");
    println!("  │ Buffer pooling eliminates per-operation allocation overhead:            │");
    println!("  ├─────────────────────────────────────────────────────────────────────────┤");
    println!("  │                                                                          │");
    println!("  │   Without pooling (cold start):                                         │");
    println!("  │     - Each operation allocates new GPU buffer (~20μs)                   │");
    println!("  │     - Buffers are never reused                                          │");
    println!("  │                                                                          │");
    println!("  │   With pooling (steady state):                                          │");
    println!("  │     - First few operations populate the pool                            │");
    println!("  │     - Subsequent operations reuse pooled buffers (0μs allocation)       │");
    println!("  │     - Pool buckets by power-of-2 sizes for efficient reuse              │");
    println!("  │                                                                          │");
    println!("  │   Combined with pipeline caching:                                       │");
    println!("  │     ✅ Shaders compiled once, reused forever                            │");
    println!("  │     ✅ Buffers allocated once, reused from pool                         │");
    println!("  │     ✅ Only per-call overhead: bind group + encoder + submit            │");
    println!("  └─────────────────────────────────────────────────────────────────────────┘");

    Ok(())
}
