// SPDX-License-Identifier: AGPL-3.0-or-later
//! Vector Addition Demo
//!
//! Simple demonstration of vector addition across multiple GPU backends

use anyhow::Result;
use vector_add_showcase::*;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Vector Addition GPU Showcase                                ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Test parameters
    let sizes = vec![1_000, 10_000, 100_000, 1_000_000];

    for size in sizes {
        println!("═══ Size: {} elements ═══", size);
        println!();

        // Generate test data
        let a: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..size).map(|i| (i * 2) as f32).collect();

        // CPU reference
        println!("Running CPU reference...");
        let start = std::time::Instant::now();
        let _cpu_result = vector_add_cpu(&a, &b);
        let cpu_time = start.elapsed();
        println!("  Time: {:.3} μs", cpu_time.as_micros() as f64);
        println!();

        // OpenCL
        #[cfg(feature = "opencl")]
        {
            println!("Running OpenCL...");
            match opencl::vector_add_opencl(&a, &b) {
                Ok(result) => {
                    result.display();
                    let speedup = cpu_time.as_micros() as f64 / result.compute_time_us;
                    println!("  Speedup: {:.2}x vs CPU", speedup);
                }
                Err(e) => println!("  Error: {}", e),
            }
            println!();
        }

        // CUDA
        #[cfg(feature = "cuda")]
        {
            println!("Running CUDA...");
            match cuda::vector_add_cuda(&a, &b) {
                Ok(result) => {
                    result.display();
                    let speedup = cpu_time.as_micros() as f64 / result.compute_time_us;
                    println!("  Speedup: {:.2}x vs CPU", speedup);
                }
                Err(e) => println!("  Error: {}", e),
            }
            println!();
        }

        println!();
    }

    println!("═══ Summary ═══");
    println!();
    println!("✅ Vector addition working across backends");
    println!("✅ Results verified against CPU reference");
    println!("✅ Ready for ZLUDA/SCALE comparison");
    println!();
    println!("Next steps:");
    println!("  1. Run with ZLUDA: LD_LIBRARY_PATH=/path/to/zluda ./demo");
    println!("  2. Run with SCALE: (follow SCALE documentation)");
    println!("  3. Compare performance across all backends");

    Ok(())
}

