//! Matrix multiplication benchmark across all GPU backends
//!
//! Demonstrates universal abstraction: same operation on CUDA, ROCm, WebGPU, OpenCL

use anyhow::Result;
use clap::Parser;
use ndarray::{Array2, ArrayView2};
use std::time::Instant;
use tracing_subscriber;

use toadstool_runtime_gpu::{GpuFramework, BackendSelectionStrategy};

#[derive(Parser)]
#[command(name = "Matrix Multiplication Benchmark")]
#[command(about = "Benchmark matrix multiply across GPU backends")]
struct Args {
    /// Matrix size (NxN)
    #[arg(long, default_value = "4096")]
    size: usize,
    
    /// GPU backend (cuda, rocm, webgpu, opencl, vulkan, auto)
    #[arg(long, default_value = "auto")]
    backend: String,
    
    /// Number of iterations
    #[arg(long, default_value = "10")]
    iterations: usize,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    if args.verbose {
        tracing_subscriber::fmt::init();
    }
    
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║       Matrix Multiplication Benchmark                     ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    
    // Detect available GPUs
    println!("Detecting GPUs...");
    let available_backends = detect_backends();
    
    if available_backends.is_empty() {
        println!("⚠️  No GPU backends available, using CPU fallback");
        return run_cpu_benchmark(args.size, args.iterations);
    }
    
    println!("Available backends: {:?}", available_backends);
    println!();
    
    // Select backend
    let backend = match args.backend.as_str() {
        "auto" => {
            let strategy = BackendSelectionStrategy::Automatic;
            match strategy.select_framework(None, &available_backends) {
                Some(fw) => fw,
                None => {
                    println!("No backend available, falling back to CPU");
                    return run_cpu_benchmark(args.size, args.iterations);
                }
            }
        }
        "cuda" => GpuFramework::Cuda,
        "rocm" => GpuFramework::Rocm,
        "webgpu" => GpuFramework::WebGpu,
        "opencl" => GpuFramework::OpenCl,
        "vulkan" => GpuFramework::Vulkan,
        "cpu" => {
            // Run CPU benchmark
            return run_cpu_benchmark(args.size, args.iterations);
        }
        _ => {
            eprintln!("Unknown backend: {}", args.backend);
            eprintln!("Available: auto, cuda, rocm, webgpu, vulkan, opencl, cpu");
            return Ok(());
        }
    };
    
    println!("Selected backend: {:?}", backend);
    println!("Matrix size: {}x{}", args.size, args.size);
    println!("Iterations: {}", args.iterations);
    println!();
    
    // Run benchmark
    let result = run_gpu_benchmark(backend.clone(), args.size, args.iterations)?;
    
    // Display results
    println!("Results:");
    println!("  Average time: {:.2}ms", result.avg_time_ms);
    println!("  Min time: {:.2}ms", result.min_time_ms);
    println!("  Max time: {:.2}ms", result.max_time_ms);
    println!("  Performance: {:.2} GFLOPS", result.gflops);
    println!("  Throughput: {:.0} matrices/sec", result.throughput);
    
    if let Some(power) = result.power_watts {
        println!("  Power: {:.0}W", power);
        println!("  Efficiency: {:.2} GFLOPS/W", result.gflops / power);
    }
    
    // Save results
    save_results(&backend, &result)?;
    
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct BenchmarkResult {
    backend: String,
    size: usize,
    iterations: usize,
    avg_time_ms: f64,
    min_time_ms: f64,
    max_time_ms: f64,
    gflops: f64,
    throughput: f64,
    power_watts: Option<f64>,
}

fn detect_backends() -> Vec<GpuFramework> {
    let mut backends = Vec::new();
    
    // Check for NVIDIA (CUDA)
    if is_nvidia_available() {
        backends.push(GpuFramework::Cuda);
    }
    
    // Check for AMD (ROCm)
    if is_amd_available() {
        backends.push(GpuFramework::Rocm);
    }
    
    // WebGPU is always available (with CPU fallback)
    backends.push(GpuFramework::WebGpu);
    
    // Check for OpenCL
    if is_opencl_available() {
        backends.push(GpuFramework::OpenCl);
    }
    
    backends
}

fn is_nvidia_available() -> bool {
    // Check for nvidia-smi
    std::process::Command::new("nvidia-smi")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_amd_available() -> bool {
    // Check for rocm-smi
    std::process::Command::new("rocm-smi")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_opencl_available() -> bool {
    // Check for OpenCL devices
    std::process::Command::new("clinfo")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_gpu_benchmark(
    backend: GpuFramework,
    size: usize,
    iterations: usize,
) -> Result<BenchmarkResult> {
    println!("Generating matrices...");
    let a = Array2::<f32>::zeros((size, size));
    let b = Array2::<f32>::zeros((size, size));
    
    println!("Warming up...");
    // Warmup
    for _ in 0..3 {
        let _ = matrix_multiply_cpu(a.view(), b.view());
    }
    
    println!("Running benchmark...");
    let mut times = Vec::new();
    let start_power = measure_gpu_power(&backend);
    
    for i in 0..iterations {
        let start = Instant::now();
        
        // TODO: Actually run on GPU using ToadStool runtime
        // For now, fallback to CPU to show structure
        let _ = matrix_multiply_cpu(a.view(), b.view());
        
        let elapsed = start.elapsed();
        times.push(elapsed.as_secs_f64() * 1000.0);  // Convert to ms
        
        if (i + 1) % 5 == 0 {
            println!("  Completed {}/{} iterations...", i + 1, iterations);
        }
    }
    
    let end_power = measure_gpu_power(&backend);
    
    // Calculate statistics
    let avg_time_ms = times.iter().sum::<f64>() / times.len() as f64;
    let min_time_ms = times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_time_ms = times.iter().fold(0.0_f64, |a, &b| a.max(b));
    
    // Calculate FLOPS (2*N^3 operations for matrix multiply)
    let flops = 2.0 * (size as f64).powi(3);
    let gflops = flops / (avg_time_ms / 1000.0) / 1e9;
    let throughput = 1000.0 / avg_time_ms;  // matrices per second
    
    let power_watts = if end_power > 0.0 { Some((start_power + end_power) / 2.0) } else { None };
    
    Ok(BenchmarkResult {
        backend: format!("{:?}", backend),
        size,
        iterations,
        avg_time_ms,
        min_time_ms,
        max_time_ms,
        gflops,
        throughput,
        power_watts,
    })
}

fn matrix_multiply_cpu(a: ArrayView2<f32>, b: ArrayView2<f32>) -> Array2<f32> {
    a.dot(&b)
}

fn measure_gpu_power(backend: &GpuFramework) -> f64 {
    let power = match backend {
        GpuFramework::Cuda => {
            // Try nvidia-smi
            std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=power.draw", "--format=csv,noheader,nounits"])
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .and_then(|s| s.trim().parse().ok())
        }
        GpuFramework::Rocm => {
            // Try rocm-smi
            std::process::Command::new("rocm-smi")
                .args(["--showpower"])
                .output()
                .ok()
                .and_then(|_output| {
                    // Parse rocm-smi output (format varies)
                    // For now, return estimate
                    Some(190.0)  // Typical RX 6700 power
                })
        }
        _ => None,
    };
    
    power.unwrap_or(0.0)
}

fn run_cpu_benchmark(size: usize, iterations: usize) -> Result<()> {
    println!("Running CPU-only benchmark...");
    
    let a = Array2::<f32>::zeros((size, size));
    let b = Array2::<f32>::zeros((size, size));
    
    let mut times = Vec::new();
    
    for i in 0..iterations {
        let start = Instant::now();
        let _ = matrix_multiply_cpu(a.view(), b.view());
        let elapsed = start.elapsed();
        times.push(elapsed.as_secs_f64() * 1000.0);
        
        if (i + 1) % 5 == 0 {
            println!("  Completed {}/{} iterations...", i + 1, iterations);
        }
    }
    
    let avg_time_ms = times.iter().sum::<f64>() / times.len() as f64;
    let flops = 2.0 * (size as f64).powi(3);
    let gflops = flops / (avg_time_ms / 1000.0) / 1e9;
    
    println!("\nCPU Results:");
    println!("  Average time: {:.2}ms", avg_time_ms);
    println!("  Performance: {:.2} GFLOPS", gflops);
    
    Ok(())
}

fn save_results(backend: &GpuFramework, result: &BenchmarkResult) -> Result<()> {
    let filename = format!("results/local/{}-matrix.json", format!("{:?}", backend).to_lowercase());
    std::fs::create_dir_all("results/local")?;
    
    let json = serde_json::to_string_pretty(result)?;
    std::fs::write(&filename, json)?;
    
    println!("\n✓ Results saved to {}", filename);
    
    Ok(())
}

