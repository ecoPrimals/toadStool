//! Multi-GPU Benchmark
//!
//! Tests workload distribution across NVIDIA and AMD GPUs.

use barracuda::multi_gpu::{GpuPool, GpuVendor, WorkloadConfig};
use barracuda::tensor::Tensor;
use std::time::Instant;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           Multi-GPU Workload Distribution Benchmark          ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Create GPU pool
    let config = WorkloadConfig {
        max_parallel: 4,
        prefer_discrete: true,
        exclude_software: true,
        min_gflops: 50.0,
    };

    let pool = GpuPool::with_config(config).await?;

    println!("Pool Summary: {}", pool.summary());
    println!();

    println!("Available GPUs:");
    for (i, device) in pool.devices().iter().enumerate() {
        let vendor_str = match device.vendor {
            GpuVendor::Nvidia => "NVIDIA",
            GpuVendor::Amd => "AMD",
            GpuVendor::Intel => "Intel",
            GpuVendor::Software => "Software",
            GpuVendor::Unknown => "Unknown",
        };
        println!(
            "  [{i}] {} ({vendor_str}, ~{:.0} GFLOPS)",
            device.name, device.gflops
        );
    }
    println!();

    if pool.device_count() == 0 {
        println!("No GPUs available for benchmarking.");
        return Ok(());
    }

    // Test 1: Single GPU tensor operation
    println!("═══ Test 1: Single GPU Tensor Operations ═══");

    if let Some(device) = pool.device(0) {
        let start = Instant::now();

        // Create test tensor
        let data: Vec<f32> = (0..1024 * 1024).map(|i| i as f32 * 0.001).collect();
        let tensor = Tensor::from_data(&data, vec![1024, 1024], device.clone())?;

        // Perform addition (round-trip test)
        let result = tensor.add(&tensor)?;
        let output = result.to_vec()?;

        let elapsed = start.elapsed();
        println!("  Matrix size: 1024x1024");
        println!("  Operation: tensor + tensor");
        println!("  Time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        println!(
            "  Result[0]: {:.6} (expected: {:.6})",
            output[0],
            data[0] * 2.0
        );

        // Verify correctness
        let expected = data[0] * 2.0;
        if (output[0] - expected).abs() < 1e-5 {
            println!("  ✓ Correct result");
        } else {
            println!("  ✗ Result mismatch!");
        }
    }
    println!();

    // Test 2: Multi-GPU parallel execution (if multiple GPUs)
    if pool.device_count() >= 2 {
        println!("═══ Test 2: Multi-GPU Parallel Execution ═══");

        let workloads: Vec<usize> = (0..4).collect();

        let start = Instant::now();

        let results = pool
            .parallel_map(workloads, |device, work_id| {
                // Each GPU processes a workload
                let data: Vec<f32> = (0..512 * 512)
                    .map(|i| (i + work_id * 1000) as f32 * 0.001)
                    .collect();
                let tensor = Tensor::from_data(&data, vec![512, 512], device)?;
                let result = tensor.add(&tensor)?;
                let sum: f32 = result.to_vec()?.iter().sum();
                Ok(sum)
            })
            .await?;

        let elapsed = start.elapsed();
        println!("  Parallel workloads: {}", results.len());
        println!("  Total time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        println!(
            "  Average per workload: {:.2}ms",
            elapsed.as_secs_f64() * 1000.0 / results.len() as f64
        );

        for (i, sum) in results.iter().enumerate() {
            println!("    Workload {i}: sum = {sum:.2}");
        }
        println!("  ✓ All workloads completed");
    } else {
        println!("═══ Test 2: Skipped (need ≥2 GPUs) ═══");
    }
    println!();

    // Test 3: Cross-vendor parity (if we have both NVIDIA and AMD)
    let has_nvidia = pool.devices().iter().any(|d| d.vendor == GpuVendor::Nvidia);
    let has_amd = pool.devices().iter().any(|d| d.vendor == GpuVendor::Amd);

    if has_nvidia && has_amd {
        println!("═══ Test 3: Cross-Vendor Parity (NVIDIA vs AMD) ═══");

        // Find indices
        let nvidia_idx = pool
            .devices()
            .iter()
            .position(|d| d.vendor == GpuVendor::Nvidia)
            .unwrap();
        let amd_idx = pool
            .devices()
            .iter()
            .position(|d| d.vendor == GpuVendor::Amd)
            .unwrap();

        let nvidia_device = pool.device(nvidia_idx).unwrap();
        let amd_device = pool.device(amd_idx).unwrap();

        // Same input data
        let data: Vec<f32> = (0..1000).map(|i| i as f32 * 0.001).collect();

        // Compute on NVIDIA
        let nvidia_tensor = Tensor::from_data(&data, vec![10, 100], nvidia_device)?;
        let nvidia_result = nvidia_tensor.add(&nvidia_tensor)?;
        let nvidia_output = nvidia_result.to_vec()?;

        // Compute on AMD
        let amd_tensor = Tensor::from_data(&data, vec![10, 100], amd_device)?;
        let amd_result = amd_tensor.add(&amd_tensor)?;
        let amd_output = amd_result.to_vec()?;

        // Compare
        let mut max_diff: f32 = 0.0;
        for (nv, amd) in nvidia_output.iter().zip(amd_output.iter()) {
            let diff = (nv - amd).abs();
            if diff > max_diff {
                max_diff = diff;
            }
        }

        println!("  Input: 10x100 matrix");
        println!("  NVIDIA result[0]: {:.8}", nvidia_output[0]);
        println!("  AMD result[0]:    {:.8}", amd_output[0]);
        println!("  Max difference:   {:.2e}", max_diff);

        if max_diff < 1e-5 {
            println!("  ✓ Cross-vendor parity achieved (<1e-5 difference)");
        } else {
            println!("  ⚠ Difference exceeds threshold");
        }
    } else {
        println!("═══ Test 3: Skipped (need both NVIDIA and AMD) ═══");
    }
    println!();

    println!("════════════════════════════════════════════════════════════════");
    println!("                        Benchmark Complete                        ");
    println!("════════════════════════════════════════════════════════════════");

    Ok(())
}
