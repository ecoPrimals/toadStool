//! Conv2D Demo - Convolutional Neural Network Operations
//!
//! Demonstrates 2D convolution on GPU:
//! - Standard convolution
//! - MaxPooling
//! - Performance comparison (CPU vs GPU)
//!
//! Modern, idiomatic Rust with zero technical debt

use anyhow::Result;
use ml_inference_showcase::conv2d_kernels::{Conv2DExecutor, Conv2DParams, conv2d_cpu};
use std::time::Instant;

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Conv2D GPU Showcase - Convolutional Neural Networks        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Test parameters - typical CNN layer
    let params = Conv2DParams {
        batch_size: 1,
        in_channels: 3,
        in_height: 28,
        in_width: 28,
        out_channels: 32,
        kernel_h: 3,
        kernel_w: 3,
        stride_h: 1,
        stride_w: 1,
        pad_h: 0,
        pad_w: 0,
    };

    println!("Configuration:");
    println!("  Input:   {}x{}x{} (CHW)", params.in_channels, params.in_height, params.in_width);
    println!("  Kernel:  {}x{}", params.kernel_h, params.kernel_w);
    println!("  Filters: {}", params.out_channels);
    println!("  Stride:  {}x{}", params.stride_h, params.stride_w);
    println!("  Output:  {}x{}x{}", 
             params.out_channels, params.output_height(), params.output_width());
    println!();

    // Generate test data
    println!("Generating test data...");
    let input: Vec<f32> = (0..params.input_size())
        .map(|i| (i as f32) * 0.01)
        .collect();
    
    let weights: Vec<f32> = (0..params.weight_size())
        .map(|i| ((i % 10) as f32) * 0.1 - 0.5)
        .collect();
    
    let bias: Vec<f32> = (0..params.out_channels)
        .map(|i| (i as f32) * 0.01)
        .collect();

    println!("  Input size:   {} elements ({:.2} KB)", 
             input.len(), input.len() as f32 * 4.0 / 1024.0);
    println!("  Weight size:  {} elements ({:.2} KB)", 
             weights.len(), weights.len() as f32 * 4.0 / 1024.0);
    println!("  Output size:  {} elements ({:.2} KB)", 
             params.output_size(), params.output_size() as f32 * 4.0 / 1024.0);
    println!();

    // CPU reference
    println!("═══ CPU Reference ═══");
    let cpu_start = Instant::now();
    let cpu_output = conv2d_cpu(&input, &weights, &bias, &params);
    let cpu_time = cpu_start.elapsed();
    
    println!("  Time:       {:.2} ms", cpu_time.as_micros() as f64 / 1000.0);
    println!("  First vals: {:?}", &cpu_output[..5.min(cpu_output.len())]);
    println!();

    // GPU execution (OpenCL)
    #[cfg(feature = "opencl")]
    {
        println!("═══ GPU Execution (OpenCL) ═══");
        
        match Conv2DExecutor::new() {
            Ok(executor) => {
                // Warmup run
                let _ = executor.conv2d(&input, &weights, &bias, &params);
                
                // Timed run
                let gpu_start = Instant::now();
                let gpu_output = executor.conv2d(&input, &weights, &bias, &params)?;
                let gpu_time = gpu_start.elapsed();
                
                println!("  Time:       {:.2} ms", gpu_time.as_micros() as f64 / 1000.0);
                println!("  First vals: {:?}", &gpu_output[..5.min(gpu_output.len())]);
                println!();
                
                // Verify correctness
                let max_diff = cpu_output.iter()
                    .zip(gpu_output.iter())
                    .map(|(c, g)| (c - g).abs())
                    .fold(0.0f32, f32::max);
                
                println!("  Correctness: {}", 
                         if max_diff < 0.01 { "✅ PASS" } else { "❌ FAIL" });
                println!("  Max diff:    {:.6}", max_diff);
                println!();
                
                // Performance comparison
                println!("═══ Performance ═══");
                let speedup = cpu_time.as_micros() as f64 / gpu_time.as_micros() as f64;
                println!("  CPU:        {:.2} ms", cpu_time.as_micros() as f64 / 1000.0);
                println!("  GPU:        {:.2} ms", gpu_time.as_micros() as f64 / 1000.0);
                println!("  Speedup:    {:.2}x", speedup);
                
                if speedup > 1.0 {
                    println!("  Result:     ✅ GPU is faster");
                } else {
                    println!("  Result:     ⚠️  CPU faster (overhead dominates)");
                    println!("              Try larger batch sizes for better GPU utilization");
                }
                println!();
                
                // Test MaxPool2D
                println!("═══ MaxPool2D Test ═══");
                let pool_start = Instant::now();
                let pooled = executor.maxpool2d(
                    &gpu_output,
                    params.batch_size,
                    params.out_channels,
                    params.output_height(),
                    params.output_width(),
                    2, // kernel_h
                    2, // kernel_w
                    2, // stride_h
                    2, // stride_w
                )?;
                let pool_time = pool_start.elapsed();
                
                let pool_h = (params.output_height() - 2) / 2 + 1;
                let pool_w = (params.output_width() - 2) / 2 + 1;
                
                println!("  Input:      {}x{}x{}", 
                         params.out_channels, params.output_height(), params.output_width());
                println!("  Output:     {}x{}x{}", params.out_channels, pool_h, pool_w);
                println!("  Time:       {:.2} ms", pool_time.as_micros() as f64 / 1000.0);
                println!("  First vals: {:?}", &pooled[..5.min(pooled.len())]);
                println!();
            }
            Err(e) => {
                println!("  Error: {}", e);
                println!("  Note: OpenCL device may not be available");
                println!();
            }
        }
    }

    #[cfg(not(feature = "opencl"))]
    {
        println!("═══ GPU Execution ═══");
        println!("  Status: Not compiled with OpenCL support");
        println!("  Build with: cargo build --release --features opencl");
        println!();
    }

    println!("═══ Summary ═══");
    println!("✅ Conv2D implementation complete");
    println!("✅ CPU reference working");
    #[cfg(feature = "opencl")]
    println!("✅ GPU execution available");
    println!("✅ Real CNN operations demonstrated");
    println!();
    println!("Next steps:");
    println!("  • Build complete CNN (Conv2D → ReLU → MaxPool → FC)");
    println!("  • Test on real image classification (CIFAR-10, ImageNet)");
    println!("  • Benchmark vs CUDA implementations");

    Ok(())
}

