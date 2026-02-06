//! Real Benchmark Suite with Actual Timing
//!
//! Runs REAL benchmarks with ACTUAL data:
//! - MatMul (various sizes)
//! - Element-wise operations
//! - Real timing measurements
//!
//! Uses BarraCUDA's existing API for actual GPU execution.

use barracuda::device::WgpuDevice;
use barracuda::error::Result;
use barracuda::tensor::Tensor;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🦈 BarraCUDA Real Benchmark Suite");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    println!("🔍 Initializing GPU device...\n");
    
    let device = WgpuDevice::new().await?;
    let device = Arc::new(device);
    
    println!("  ✅ Device: {}", device.name());
    println!("  ✅ Type: {:?}\n", device.device_type());
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 MatMul Benchmark (Real Timing)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    let sizes = vec![
        (64, 64, "Tiny"),
        (128, 128, "Small"),
        (256, 256, "Medium"),
        (512, 512, "Large"),
        (1024, 1024, "XLarge"),
    ];
    
    println!("Running {} matrix sizes with REAL data and timing...\n", sizes.len());
    
    println!("┌─────────────┬─────────────┬─────────────┬──────────────┐");
    println!("│    Size     │  Time (ms)  │   TFLOPS    │   GB/s       │");
    println!("├─────────────┼─────────────┼─────────────┼──────────────┤");
    
    for (m, n, _label) in &sizes {
        let (time_ms, tflops, bandwidth) = benchmark_matmul(&device, *m, *n).await?;
        
        println!("│ {:^11} │ {:>9.2}ms │ {:>9.2}   │ {:>10.1}   │", 
            format!("{}×{}", m, n), time_ms, tflops, bandwidth);
    }
    
    println!("└─────────────┴─────────────┴─────────────┴──────────────┘");
    println!();
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Element-Wise Operations (Real Timing)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    let elem_sizes = vec![
        (100_000, "100K elements"),
        (1_000_000, "1M elements"),
        (10_000_000, "10M elements"),
    ];
    
    println!("┌──────────────────┬─────────────┬──────────────┐");
    println!("│   Size           │  Time (ms)  │  Throughput  │");
    println!("├──────────────────┼─────────────┼──────────────┤");
    
    for (size, label) in &elem_sizes {
        let (time_ms, throughput) = benchmark_relu(&device, *size).await?;
        
        println!("│ {:^16} │ {:>9.2}ms │ {:>10.1} M/s │", 
            label, time_ms, throughput / 1e6);
    }
    
    println!("└──────────────────┴─────────────┴──────────────┘");
    println!();
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎯 Results:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("✅ Real Performance Measured:");
    println!("   • Actual GPU execution (not simulated)");
    println!("   • Real data and timing");
    println!("   • Production-grade measurements");
    println!();
    println!("💡 Key Insight:");
    println!("   • THIS CODE runs on AMD, NVIDIA, Intel, Apple");
    println!("   • Same binary, different hardware = different speed");
    println!("   • CUDA would only run on ONE GPU vendor");
    println!();
    println!("🏆 BarraCUDA provides true hardware portability!");
    println!();
    
    Ok(())
}

async fn benchmark_matmul(device: &Arc<WgpuDevice>, m: usize, n: usize) -> Result<(f64, f64, f64)> {
    // Create random matrices
    let data_a: Vec<f32> = (0..(m * n)).map(|_| rand::random::<f32>()).collect();
    let data_b: Vec<f32> = (0..(n * m)).map(|_| rand::random::<f32>()).collect();
    
    let a = Tensor::from_data(&data_a, vec![m, n], device.clone())?;
    let b = Tensor::from_data(&data_b, vec![n, m], device.clone())?;
    
    // Warmup (3 iterations)
    for _ in 0..3 {
        let _ = a.clone().matmul(&b)?;
    }
    
    // Wait for GPU
    device.queue().submit(std::iter::empty());
    device.device().poll(wgpu::Maintain::Wait);
    
    // Benchmark (10 iterations)
    let start = Instant::now();
    
    for _ in 0..10 {
        let _ = a.clone().matmul(&b)?;
    }
    
    // Wait for completion
    device.queue().submit(std::iter::empty());
    device.device().poll(wgpu::Maintain::Wait);
    
    let elapsed = start.elapsed();
    let time_ms = elapsed.as_secs_f64() * 1000.0 / 10.0;
    
    // Calculate TFLOPS
    // MatMul FLOPs: 2 * m * n * k
    let flops = 2.0 * (m * n * m) as f64;
    let tflops = (flops / (time_ms / 1000.0)) / 1e12;
    
    // Calculate bandwidth (GB/s)
    // Read A (m*n), Read B (n*m), Write C (m*m)
    let bytes = ((m * n) + (n * m) + (m * m)) * 4; // f32 = 4 bytes
    let bandwidth_gbs = (bytes as f64 / (time_ms / 1000.0)) / 1e9;
    
    Ok((time_ms, tflops, bandwidth_gbs))
}

async fn benchmark_relu(device: &Arc<WgpuDevice>, size: usize) -> Result<(f64, f64)> {
    // Create random data
    let data: Vec<f32> = (0..size).map(|_| rand::random::<f32>() - 0.5).collect();
    let input = Tensor::from_data(&data, vec![size], device.clone())?;
    
    // Warmup
    for _ in 0..3 {
        let _ = input.clone().relu()?;
    }
    
    device.queue().submit(std::iter::empty());
    device.device().poll(wgpu::Maintain::Wait);
    
    // Benchmark
    let start = Instant::now();
    
    for _ in 0..10 {
        let _ = input.clone().relu()?;
    }
    
    device.queue().submit(std::iter::empty());
    device.device().poll(wgpu::Maintain::Wait);
    
    let elapsed = start.elapsed();
    let time_ms = elapsed.as_secs_f64() * 1000.0 / 10.0;
    
    // Throughput (elements/sec)
    let throughput = size as f64 / (time_ms / 1000.0);
    
    Ok((time_ms, throughput))
}
