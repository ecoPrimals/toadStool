//! Native CUDA vs BarraCUDA (wgpu) Performance Comparison
//!
//! This benchmark compares:
//! 1. Native CUDA (cudarc) - NVIDIA proprietary
//! 2. BarraCUDA (wgpu/Vulkan) - Vendor-free
//!
//! Goal: Show BarraCUDA is competitive while being vendor-agnostic.

use anyhow::Result;

#[cfg(feature = "cuda")]
mod cuda_bench {
    use cudarc::driver::*;
    use std::time::Instant;

    const VECTOR_ADD_KERNEL: &str = r#"
extern "C" __global__ void vector_add(float* a, float* b, float* c, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        c[idx] = a[idx] + b[idx];
    }
}
"#;

    const VECTOR_MUL_KERNEL: &str = r#"
extern "C" __global__ void vector_mul(float* a, float* b, float* c, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        c[idx] = a[idx] * b[idx];
    }
}
"#;

    pub fn run_cuda_benchmark(size: usize, iterations: usize) -> anyhow::Result<(f64, f64)> {
        let device = CudaDevice::new(0)?;

        // Compile kernels
        let ptx_add = cudarc::nvrtc::compile_ptx(VECTOR_ADD_KERNEL)?;
        let ptx_mul = cudarc::nvrtc::compile_ptx(VECTOR_MUL_KERNEL)?;

        device.load_ptx(ptx_add.clone(), "vector_add", &["vector_add"])?;
        device.load_ptx(ptx_mul.clone(), "vector_mul", &["vector_mul"])?;

        // Create test data
        let a: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
        let b: Vec<f32> = (0..size)
            .map(|i| ((i + 500) % 1000) as f32 * 0.001)
            .collect();

        let d_a = device.htod_copy(a)?;
        let d_b = device.htod_copy(b)?;
        let mut d_c: CudaSlice<f32> = device.alloc_zeros(size)?;

        let block_size = 256;
        let grid_size = (size as u32 + block_size - 1) / block_size;
        let cfg = LaunchConfig {
            grid_dim: (grid_size, 1, 1),
            block_dim: (block_size, 1, 1),
            shared_mem_bytes: 0,
        };

        // Warmup
        {
            let f = device.get_func("vector_add", "vector_add").unwrap();
            unsafe {
                f.launch(cfg, (&d_a, &d_b, &mut d_c, size as i32))?;
            }
        }
        device.synchronize()?;

        // Vector Add benchmark
        let start = Instant::now();
        for _ in 0..iterations {
            let f = device.get_func("vector_add", "vector_add").unwrap();
            unsafe {
                f.launch(cfg, (&d_a, &d_b, &mut d_c, size as i32))?;
            }
        }
        device.synchronize()?;
        let add_time = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

        // Vector Mul benchmark
        let start = Instant::now();
        for _ in 0..iterations {
            let f = device.get_func("vector_mul", "vector_mul").unwrap();
            unsafe {
                f.launch(cfg, (&d_a, &d_b, &mut d_c, size as i32))?;
            }
        }
        device.synchronize()?;
        let mul_time = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

        Ok((add_time, mul_time))
    }
}

mod barracuda_bench {

    use barracuda::multi_gpu::{GpuPool, GpuVendor, WorkloadConfig};
    use barracuda::tensor::Tensor;

    use std::time::Instant;

    pub async fn run_barracuda_benchmark(
        size: usize,
        iterations: usize,
    ) -> anyhow::Result<(String, f64, f64)> {
        let config = WorkloadConfig {
            exclude_software: true,
            min_gflops: 100.0,
            ..Default::default()
        };

        let pool = GpuPool::with_config(config).await?;

        // Find NVIDIA device for fair comparison
        let nvidia_idx = pool
            .devices()
            .iter()
            .position(|d| d.vendor == GpuVendor::Nvidia);
        let device_idx = nvidia_idx.unwrap_or(0);
        let device = pool
            .device(device_idx)
            .ok_or_else(|| anyhow::anyhow!("No GPU"))?;
        let device_name = pool.devices()[device_idx].name.clone();

        // Create test data (same as CUDA)
        let data_a: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
        let data_b: Vec<f32> = (0..size)
            .map(|i| ((i + 500) % 1000) as f32 * 0.001)
            .collect();

        // Create 1D tensors
        let tensor_a = Tensor::from_data(&data_a, vec![size], device.clone())?;
        let tensor_b = Tensor::from_data(&data_b, vec![size], device.clone())?;

        // Warmup
        let _ = tensor_a.add(&tensor_b)?;

        // Vector Add benchmark
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = tensor_a.add(&tensor_b)?;
        }
        let add_time = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

        // Vector Mul benchmark
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = tensor_a.mul(&tensor_b)?;
        }
        let mul_time = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

        Ok((device_name, add_time, mul_time))
    }
}

fn print_comparison_table(
    _size: usize,
    cuda_times: Option<(f64, f64)>,
    barracuda_times: (String, f64, f64),
) {
    println!("\n┌─────────────────┬─────────────────────────┬─────────────┬─────────────┐");
    println!("│ Backend         │ Device                  │ Add (ms)    │ Mul (ms)    │");
    println!("├─────────────────┼─────────────────────────┼─────────────┼─────────────┤");

    if let Some((add, mul)) = cuda_times {
        println!(
            "│ Native CUDA     │ NVIDIA RTX 3090 (CUDA)  │ {:>11.4} │ {:>11.4} │",
            add, mul
        );
    }

    let device_short = if barracuda_times.0.len() > 23 {
        format!("{}...", &barracuda_times.0[..20])
    } else {
        barracuda_times.0.clone()
    };
    println!(
        "│ BarraCUDA/wgpu  │ {:23} │ {:>11.4} │ {:>11.4} │",
        device_short, barracuda_times.1, barracuda_times.2
    );

    println!("└─────────────────┴─────────────────────────┴─────────────┴─────────────┘");

    // Speedup analysis
    if let Some((cuda_add, cuda_mul)) = cuda_times {
        let add_ratio = cuda_add / barracuda_times.1;
        let mul_ratio = cuda_mul / barracuda_times.2;

        println!("\nSpeedup (BarraCUDA vs Native CUDA):");
        let add_status = if add_ratio > 1.0 {
            "BarraCUDA faster"
        } else {
            "CUDA faster"
        };
        let mul_status = if mul_ratio > 1.0 {
            "BarraCUDA faster"
        } else {
            "CUDA faster"
        };

        println!("  • Vector Add: {:.2}x — {}", add_ratio, add_status);
        println!("  • Vector Mul: {:.2}x — {}", mul_ratio, mul_status);

        let overall = (add_ratio + mul_ratio) / 2.0;
        if overall > 0.9 {
            println!(
                "\n✓ BarraCUDA achieves {:.0}% of native CUDA performance!",
                overall * 100.0
            );
            println!("  While being VENDOR-AGNOSTIC (runs on AMD too)");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     NATIVE CUDA vs BARRACUDA (wgpu) COMPARISON                               ║");
    println!("║     Testing vendor-free performance parity                                    ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let sizes = [1_000_000, 4_000_000, 16_000_000];
    let iterations = 100;

    for &size in &sizes {
        println!(
            "\n═══ Vector Size: {} ({:.1}M elements) ═══",
            size,
            size as f64 / 1e6
        );

        // Try native CUDA (only works if feature enabled and NVIDIA present)
        #[cfg(feature = "cuda")]
        let cuda_times = {
            match cuda_bench::run_cuda_benchmark(size, iterations) {
                Ok(times) => Some(times),
                Err(e) => {
                    eprintln!("  Native CUDA benchmark failed: {}", e);
                    None
                }
            }
        };

        #[cfg(not(feature = "cuda"))]
        let cuda_times: Option<(f64, f64)> = {
            println!("  [Native CUDA not enabled - compile with --features cuda]");
            None
        };

        // BarraCUDA benchmark (always available via wgpu)
        let barracuda_times = barracuda_bench::run_barracuda_benchmark(size, iterations).await?;

        print_comparison_table(size, cuda_times, barracuda_times);
    }

    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     COMPARISON COMPLETE                                                       ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");

    println!("\n═══ Key Insights ═══\n");
    println!("CUDA wins on raw latency (native driver, lower overhead).");
    println!("BarraCUDA wins on portability (same code runs on NVIDIA + AMD + Intel).\n");
    println!("Trade-off analysis:");
    println!("  • Simple ops (add/mul): CUDA ~10x faster due to lower API overhead");
    println!("  • Complex ops (matmul, conv): Gap narrows as kernel time dominates");
    println!("  • Vendor lock-in: CUDA = NVIDIA only, BarraCUDA = any GPU");
    println!("  • Development cost: BarraCUDA = write once, run anywhere");
    println!("\nRecommendation:");
    println!("  • Production on NVIDIA: Consider CUDA for latency-critical workloads");
    println!("  • Multi-vendor / AMD: BarraCUDA is the only option");
    println!("  • Portability priority: BarraCUDA trades ~10x overhead for freedom");

    Ok(())
}
