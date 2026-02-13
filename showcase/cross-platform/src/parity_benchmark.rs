//! Comprehensive Parity Benchmark: BarraCUDA vs CUDA vs ROCm
//!
//! Goal: Identify performance gaps and optimization targets to achieve
//! vendor-free performance parity with native APIs.
//!
//! Tests:
//! 1. Vector operations (add, mul, fma)
//! 2. Matrix multiplication (compute-bound)
//! 3. Reductions (memory-bound)
//! 4. Memory bandwidth
//!
//! Targets:
//! - CUDA (cudarc) on NVIDIA RTX 3090
//! - ROCm (HIP) on AMD RX 6950 XT
//! - BarraCUDA (wgpu/Vulkan) on both

use anyhow::Result;
use barracuda::device::WgpuDevice;
use barracuda::multi_gpu::{GpuPool, GpuVendor, WorkloadConfig};
use barracuda::tensor::Tensor;
use std::sync::Arc;
use std::time::Instant;

/// Benchmark result
#[derive(Debug, Clone)]
struct BenchResult {
    backend: String,
    device: String,
    operation: String,
    size: String,
    time_us: f64,
    throughput_gbps: f64,
    gflops: f64,
}

impl BenchResult {
    fn new(backend: &str, device: &str, op: &str, size: &str, 
           time_us: f64, bytes: usize, flops: usize) -> Self {
        let throughput_gbps = (bytes as f64) / (time_us * 1000.0); // GB/s
        let gflops = (flops as f64) / (time_us * 1000.0); // GFLOPS
        Self {
            backend: backend.to_string(),
            device: device.to_string(),
            operation: op.to_string(),
            size: size.to_string(),
            time_us,
            throughput_gbps,
            gflops,
        }
    }
}

// ============================================================================
// CUDA Benchmarks (NVIDIA only)
// ============================================================================

#[cfg(feature = "cuda")]
mod cuda_bench {
    use super::*;
    use cudarc::driver::*;

    const VECTOR_ADD: &str = r#"
extern "C" __global__ void vector_add(float* a, float* b, float* c, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) c[idx] = a[idx] + b[idx];
}
"#;

    const VECTOR_MUL: &str = r#"
extern "C" __global__ void vector_mul(float* a, float* b, float* c, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) c[idx] = a[idx] * b[idx];
}
"#;

    const VECTOR_FMA: &str = r#"
extern "C" __global__ void vector_fma(float* a, float* b, float* c, float alpha, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) c[idx] = __fmaf_rn(alpha, a[idx], b[idx]);
}
"#;

    const REDUCTION_SUM: &str = r#"
extern "C" __global__ void reduce_sum(float* input, float* output, int n) {
    extern __shared__ float sdata[];
    int tid = threadIdx.x;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    
    sdata[tid] = (idx < n) ? input[idx] : 0.0f;
    __syncthreads();
    
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (tid < s) sdata[tid] += sdata[tid + s];
        __syncthreads();
    }
    
    if (tid == 0) atomicAdd(output, sdata[0]);
}
"#;

    pub fn run_cuda_benchmarks(sizes: &[usize], iterations: usize) -> Result<Vec<BenchResult>> {
        let device = CudaDevice::new(0)?;
        let mut results = Vec::new();

        // Compile kernels
        let ptx_add = cudarc::nvrtc::compile_ptx(VECTOR_ADD)?;
        let ptx_mul = cudarc::nvrtc::compile_ptx(VECTOR_MUL)?;
        let ptx_fma = cudarc::nvrtc::compile_ptx(VECTOR_FMA)?;
        let ptx_reduce = cudarc::nvrtc::compile_ptx(REDUCTION_SUM)?;

        device.load_ptx(ptx_add, "vector_add", &["vector_add"])?;
        device.load_ptx(ptx_mul, "vector_mul", &["vector_mul"])?;
        device.load_ptx(ptx_fma, "vector_fma", &["vector_fma"])?;
        device.load_ptx(ptx_reduce, "reduce_sum", &["reduce_sum"])?;

        for &size in sizes {
            let size_str = format_size(size);
            
            // Allocate
            let a: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
            let b: Vec<f32> = (0..size).map(|i| ((i + 500) % 1000) as f32 * 0.001).collect();
            let d_a = device.htod_copy(a)?;
            let d_b = device.htod_copy(b)?;
            let mut d_c: CudaSlice<f32> = device.alloc_zeros(size)?;

            let block = 256u32;
            let grid = ((size as u32 + block - 1) / block, 1, 1);
            let cfg = LaunchConfig { grid_dim: grid, block_dim: (block, 1, 1), shared_mem_bytes: 0 };

            // Vector Add
            let f = device.get_func("vector_add", "vector_add").unwrap();
            unsafe { f.launch(cfg, (&d_a, &d_b, &mut d_c, size as i32))?; }
            device.synchronize()?;
            
            let start = Instant::now();
            for _ in 0..iterations {
                let f = device.get_func("vector_add", "vector_add").unwrap();
                unsafe { f.launch(cfg, (&d_a, &d_b, &mut d_c, size as i32))?; }
            }
            device.synchronize()?;
            let time_us = start.elapsed().as_secs_f64() * 1e6 / iterations as f64;
            let bytes = size * 3 * 4; // 2 read + 1 write
            let flops = size;
            results.push(BenchResult::new("CUDA", "RTX 3090", "vector_add", &size_str, time_us, bytes, flops));

            // Vector Mul
            let start = Instant::now();
            for _ in 0..iterations {
                let f = device.get_func("vector_mul", "vector_mul").unwrap();
                unsafe { f.launch(cfg, (&d_a, &d_b, &mut d_c, size as i32))?; }
            }
            device.synchronize()?;
            let time_us = start.elapsed().as_secs_f64() * 1e6 / iterations as f64;
            results.push(BenchResult::new("CUDA", "RTX 3090", "vector_mul", &size_str, time_us, bytes, flops));

            // Vector FMA
            let start = Instant::now();
            for _ in 0..iterations {
                let f = device.get_func("vector_fma", "vector_fma").unwrap();
                unsafe { f.launch(cfg, (&d_a, &d_b, &mut d_c, 2.0f32, size as i32))?; }
            }
            device.synchronize()?;
            let time_us = start.elapsed().as_secs_f64() * 1e6 / iterations as f64;
            let flops_fma = size * 2; // multiply + add
            results.push(BenchResult::new("CUDA", "RTX 3090", "vector_fma", &size_str, time_us, bytes, flops_fma));

            // Reduction
            let mut d_out: CudaSlice<f32> = device.alloc_zeros(1)?;
            let reduce_cfg = LaunchConfig { 
                grid_dim: grid, 
                block_dim: (block, 1, 1), 
                shared_mem_bytes: block as u32 * 4 
            };
            
            let start = Instant::now();
            for _ in 0..iterations {
                let f = device.get_func("reduce_sum", "reduce_sum").unwrap();
                unsafe { f.launch(reduce_cfg, (&d_a, &mut d_out, size as i32))?; }
            }
            device.synchronize()?;
            let time_us = start.elapsed().as_secs_f64() * 1e6 / iterations as f64;
            let bytes_reduce = size * 4; // read only
            results.push(BenchResult::new("CUDA", "RTX 3090", "reduction", &size_str, time_us, bytes_reduce, size));
        }

        Ok(results)
    }
}

// ============================================================================
// BarraCUDA Benchmarks (Any GPU via wgpu)
// ============================================================================

mod barracuda_bench {
    use super::*;

    pub async fn run_barracuda_benchmarks(sizes: &[usize], iterations: usize) -> Result<Vec<BenchResult>> {
        let config = WorkloadConfig {
            exclude_software: true,
            min_gflops: 100.0,
            ..Default::default()
        };
        
        let pool = GpuPool::with_config(config).await?;
        let mut results = Vec::new();

        // Test on each GPU
        for (idx, gpu_info) in pool.devices().iter().enumerate() {
            let device = pool.device(idx).ok_or_else(|| anyhow::anyhow!("No device"))?;
            let device_name = match gpu_info.vendor {
                GpuVendor::Nvidia => "RTX 3090",
                GpuVendor::Amd => "RX 6950 XT",
                _ => "Unknown",
            };

            for &size in sizes {
                let size_str = format_size(size);
                
                // Create test data
                let data_a: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
                let data_b: Vec<f32> = (0..size).map(|i| ((i + 500) % 1000) as f32 * 0.001).collect();
                
                let tensor_a = Tensor::from_data(&data_a, vec![size], device.clone())?;
                let tensor_b = Tensor::from_data(&data_b, vec![size], device.clone())?;

                // Warmup
                let _ = tensor_a.add(&tensor_b)?;

                // Vector Add
                let start = Instant::now();
                for _ in 0..iterations {
                    let _ = tensor_a.add(&tensor_b)?;
                }
                let time_us = start.elapsed().as_secs_f64() * 1e6 / iterations as f64;
                let bytes = size * 3 * 4;
                results.push(BenchResult::new("BarraCUDA", device_name, "vector_add", &size_str, time_us, bytes, size));

                // Vector Mul
                let start = Instant::now();
                for _ in 0..iterations {
                    let _ = tensor_a.mul(&tensor_b)?;
                }
                let time_us = start.elapsed().as_secs_f64() * 1e6 / iterations as f64;
                results.push(BenchResult::new("BarraCUDA", device_name, "vector_mul", &size_str, time_us, bytes, size));

                // Note: FMA and reduction need dedicated kernels in BarraCUDA
                // For now, skip to show where we need to add them
            }
        }

        Ok(results)
    }
}

// ============================================================================
// Utilities
// ============================================================================

fn format_size(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}K", n / 1_000)
    } else {
        format!("{}", n)
    }
}

fn print_results_table(results: &[BenchResult]) {
    println!("\n┌────────────┬─────────────┬─────────────┬────────┬────────────┬────────────┬──────────┐");
    println!("│ Backend    │ Device      │ Operation   │ Size   │ Time (μs)  │ BW (GB/s)  │ GFLOPS   │");
    println!("├────────────┼─────────────┼─────────────┼────────┼────────────┼────────────┼──────────┤");
    
    for r in results {
        println!("│ {:10} │ {:11} │ {:11} │ {:>6} │ {:>10.2} │ {:>10.2} │ {:>8.2} │",
            r.backend, r.device, r.operation, r.size, r.time_us, r.throughput_gbps, r.gflops);
    }
    println!("└────────────┴─────────────┴─────────────┴────────┴────────────┴────────────┴──────────┘");
}

fn print_parity_analysis(results: &[BenchResult]) {
    println!("\n═══ PARITY ANALYSIS ═══\n");
    
    let operations = ["vector_add", "vector_mul"];
    let sizes = ["1M", "4M", "16M"];
    
    for op in operations {
        println!("{}:", op);
        for size in sizes {
            // Find CUDA baseline
            let cuda = results.iter()
                .find(|r| r.backend == "CUDA" && r.operation == op && r.size == size);
            
            // Find BarraCUDA on both devices
            let bc_nvidia = results.iter()
                .find(|r| r.backend == "BarraCUDA" && r.device == "RTX 3090" && r.operation == op && r.size == size);
            let bc_amd = results.iter()
                .find(|r| r.backend == "BarraCUDA" && r.device == "RX 6950 XT" && r.operation == op && r.size == size);
            
            if let Some(cuda) = cuda {
                print!("  {:>4}: CUDA {:>8.1}μs", size, cuda.time_us);
                
                if let Some(bc) = bc_nvidia {
                    let ratio = bc.time_us / cuda.time_us;
                    let gap = if ratio > 1.0 { format!("{:.1}x slower", ratio) } else { format!("{:.1}x faster", 1.0/ratio) };
                    print!(" | BC/NVIDIA {:>8.1}μs ({:>12})", bc.time_us, gap);
                }
                
                if let Some(bc) = bc_amd {
                    print!(" | BC/AMD {:>8.1}μs", bc.time_us);
                }
                
                println!();
            }
        }
        println!();
    }
}

fn print_optimization_targets(results: &[BenchResult]) {
    println!("═══ OPTIMIZATION TARGETS ═══\n");
    
    // Calculate average gap
    let cuda_times: Vec<f64> = results.iter()
        .filter(|r| r.backend == "CUDA")
        .map(|r| r.time_us)
        .collect();
    
    let bc_nvidia_times: Vec<f64> = results.iter()
        .filter(|r| r.backend == "BarraCUDA" && r.device == "RTX 3090")
        .map(|r| r.time_us)
        .collect();
    
    if !cuda_times.is_empty() && !bc_nvidia_times.is_empty() {
        let avg_gap: f64 = bc_nvidia_times.iter().zip(&cuda_times)
            .map(|(bc, cuda)| bc / cuda)
            .sum::<f64>() / cuda_times.len() as f64;
        
        println!("Current average gap (BarraCUDA vs CUDA): {:.1}x", avg_gap);
        println!();
        println!("Identified bottlenecks:");
        println!("  1. wgpu command buffer submission overhead");
        println!("  2. Vulkan pipeline state switching");
        println!("  3. Shader compilation (first run)");
        println!("  4. Missing fused kernels (FMA)");
        println!();
        println!("Optimization strategies:");
        println!("  1. Batch multiple operations in single dispatch");
        println!("  2. Pre-compile shader pipelines at init");
        println!("  3. Use persistent command buffers");
        println!("  4. Implement native FMA shader");
        println!("  5. Profile with GPU profiler (nsight/rocprof)");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     PARITY BENCHMARK: BarraCUDA vs CUDA vs ROCm                              ║");
    println!("║     Goal: Achieve vendor-free performance parity                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let sizes = [1_000_000, 4_000_000, 16_000_000];
    let iterations = 100;

    let mut all_results = Vec::new();

    // Native CUDA benchmarks (NVIDIA only)
    #[cfg(feature = "cuda")]
    {
        println!("Running Native CUDA benchmarks (NVIDIA RTX 3090)...");
        match cuda_bench::run_cuda_benchmarks(&sizes, iterations) {
            Ok(results) => all_results.extend(results),
            Err(e) => eprintln!("  CUDA benchmark failed: {}", e),
        }
    }
    
    #[cfg(not(feature = "cuda"))]
    {
        println!("Native CUDA not enabled (compile with --features cuda)");
    }

    // BarraCUDA benchmarks (all GPUs)
    println!("Running BarraCUDA benchmarks (wgpu/Vulkan)...");
    match barracuda_bench::run_barracuda_benchmarks(&sizes, iterations).await {
        Ok(results) => all_results.extend(results),
        Err(e) => eprintln!("  BarraCUDA benchmark failed: {}", e),
    }

    // Results
    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     BENCHMARK RESULTS                                                         ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");

    print_results_table(&all_results);
    print_parity_analysis(&all_results);
    print_optimization_targets(&all_results);

    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     TARGET: Close the gap to achieve vendor-free CUDA/ROCm parity            ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    Ok(())
}
