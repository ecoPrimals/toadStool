// SPDX-License-Identifier: AGPL-3.0-or-later
//! Vector Addition Benchmark
//!
//! Comprehensive benchmarking across backends for comparison with ZLUDA/SCALE

use anyhow::Result;
use std::time::Instant;
use vector_add_showcase::*;

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Vector Addition Benchmark                                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Benchmark parameters
    let sizes = vec![
        1_024,      // 1K
        4_096,      // 4K
        16_384,     // 16K
        65_536,     // 64K
        262_144,    // 256K
        1_048_576,  // 1M
        4_194_304,  // 4M
    ];
    let iterations = 100;

    println!("Configuration:");
    println!("  Iterations: {}", iterations);
    println!("  Sizes: {:?}", sizes);
    println!();

    // Results table
    println!("╔════════════╦════════════╦════════════╦════════════╦════════════╗");
    println!("║ Size       ║ Backend    ║ Avg (μs)   ║ Throughput ║ Speedup    ║");
    println!("╠════════════╬════════════╬════════════╬════════════╬════════════╣");

    for size in sizes {
        // Generate test data
        let a: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..size).map(|i| (i * 2) as f32).collect();

        // CPU baseline
        let mut cpu_times = Vec::new();
        for _ in 0..iterations {
            let start = Instant::now();
            let _result = vector_add_cpu(&a, &b);
            cpu_times.push(start.elapsed().as_micros() as f64);
        }
        let cpu_avg = cpu_times.iter().sum::<f64>() / cpu_times.len() as f64;
        let cpu_throughput = (size * 4 * 3) as f64 / (cpu_avg * 1e-6) / 1e9;

        println!("║ {:10} ║ {:10} ║ {:10.2} ║ {:8.2} GB/s ║ {:10} ║",
                 format!("{}K", size / 1024), "CPU", cpu_avg, cpu_throughput, "1.00x");

        // OpenCL
        #[cfg(feature = "opencl")]
        {
            let mut opencl_times = Vec::new();
            
            for _ in 0..iterations {
                match opencl::vector_add_opencl(&a, &b) {
                    Ok(result) => {
                        opencl_times.push(result.compute_time_us);
                    }
                    Err(e) => {
                        eprintln!("OpenCL error: {}", e);
                        break;
                    }
                }
            }

            if !opencl_times.is_empty() {
                let opencl_avg = opencl_times.iter().sum::<f64>() / opencl_times.len() as f64;
                let opencl_throughput = (size * 4 * 3) as f64 / (opencl_avg * 1e-6) / 1e9;
                let speedup = cpu_avg / opencl_avg;

                println!("║ {:10} ║ {:10} ║ {:10.2} ║ {:8.2} GB/s ║ {:9.2}x ║",
                         "", "OpenCL", opencl_avg, opencl_throughput, speedup);
            }
        }

        // CUDA
        #[cfg(feature = "cuda")]
        {
            let mut cuda_times = Vec::new();
            
            for _ in 0..iterations {
                match cuda::vector_add_cuda(&a, &b) {
                    Ok(result) => {
                        cuda_times.push(result.compute_time_us);
                    }
                    Err(e) => {
                        eprintln!("CUDA error: {}", e);
                        break;
                    }
                }
            }

            if !cuda_times.is_empty() {
                let cuda_avg = cuda_times.iter().sum::<f64>() / cuda_times.len() as f64;
                let cuda_throughput = (size * 4 * 3) as f64 / (cuda_avg * 1e-6) / 1e9;
                let speedup = cpu_avg / cuda_avg;

                println!("║ {:10} ║ {:10} ║ {:10.2} ║ {:8.2} GB/s ║ {:9.2}x ║",
                         "", "CUDA", cuda_avg, cuda_throughput, speedup);
            }
        }

        println!("╠════════════╬════════════╬════════════╬════════════╬════════════╣");
    }

    println!("╚════════════╩════════════╩════════════╩════════════╩════════════╝");
    println!();
    println!("✅ Benchmark complete");
    println!();
    println!("For ZLUDA comparison:");
    println!("  1. Build CUDA version: cargo build --release --features cuda");
    println!("  2. Run with ZLUDA: LD_LIBRARY_PATH=/path/to/zluda ./benchmark");
    println!("  3. Compare CUDA results with/without ZLUDA");

    Ok(())
}

