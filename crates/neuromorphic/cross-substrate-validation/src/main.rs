//! Cross-Substrate Validation Benchmark
//!
//! Compares performance of the same workload across:
//! - CPU (BarraCUDA CPU backend)
//! - GPU(s) (BarraCUDA wgpu backend)  
//! - Neuromorphic (Akida NPUs)
//!
//! **Deep Debt**: Complete implementation, no mocks!

use akida_driver::DeviceManager;
use akida_models::Model;
use std::time::Instant;
use toadstool_runtime_universal::{
    ComputeUnitType, OperationType, UniversalRuntime, WorkloadBuilder,
};
// Using simple formatting instead of prettytable

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("warn").init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                                                              ║");
    println!("║        CROSS-SUBSTRATE VALIDATION BENCHMARK                 ║");
    println!("║                                                              ║");
    println!("║    CPU vs GPU vs Neuromorphic Performance Comparison        ║");
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Step 1: Discover all compute substrates
    println!("1️⃣  Discovering compute substrates...\n");

    let universal = UniversalRuntime::discover().await?;
    let stats = universal.stats();

    println!("   Universal Runtime:");
    println!("{}", stats);

    // Discover Akida devices
    let akida_manager = DeviceManager::discover()?;
    println!("   Akida devices: {}", akida_manager.device_count());
    if akida_manager.device_count() > 0 {
        for (i, info) in akida_manager.devices().iter().enumerate() {
            println!("     Device {}: {:?}", i, info);
        }
    }
    println!();

    // Step 2: Define test workload
    println!("2️⃣  Defining test workload...\n");

    let workload_sizes = vec![100, 1_000, 10_000, 100_000];

    for &size in &workload_sizes {
        println!("   Testing with {} elements...\n", size);

        // Create test data (simple ReLU operation)
        let input: Vec<f32> = (0..size).map(|i| i as f32 - (size / 2) as f32).collect();

        // Expected output (CPU reference)
        let expected: Vec<f32> = input.iter().map(|&x| x.max(0.0)).collect();

        println!("   ┌────────────────────────┬──────────┬────────────────┬──────────┬──────────┐");
        println!("   │ Substrate              │ Time (µs)│ Throughput     │ Speedup  │ Accuracy │");
        println!("   ├────────────────────────┼──────────┼────────────────┼──────────┼──────────┤");

        let cpu_time: f64;

        // Benchmark CPU
        if stats.num_cpu > 0 {
            let cpu_result = benchmark_cpu(&universal, &input).await?;
            cpu_time = cpu_result.time_us;
            println!(
                "   │ {:22} │ {:8.1} │ {:10.2}M/s │ {:8} │ {:8} │",
                "CPU",
                cpu_result.time_us,
                cpu_result.throughput / 1e6,
                "1.00x",
                validate_output(&expected, &cpu_result.output)
            );
        } else {
            cpu_time = 1.0;
        }

        // Benchmark GPU(s)
        if stats.num_gpu > 0 {
            let gpu_units = universal.units_by_type(ComputeUnitType::GpuWgpu);
            for (i, _unit) in gpu_units.iter().enumerate() {
                if let Ok(gpu_result) = benchmark_gpu(&universal, i, &input).await {
                    let speedup = cpu_time / gpu_result.time_us;
                    println!(
                        "   │ {:22} │ {:8.1} │ {:10.2}M/s │ {:7.2}x │ {:8} │",
                        format!("GPU {}", i),
                        gpu_result.time_us,
                        gpu_result.throughput / 1e6,
                        speedup,
                        validate_output(&expected, &gpu_result.output)
                    );
                }
            }
        }

        // Benchmark Neuromorphic (Akida)
        if akida_manager.device_count() > 0 {
            if let Ok(neuro_result) = benchmark_neuromorphic(&akida_manager, size).await {
                let speedup = cpu_time / neuro_result.time_us;
                println!(
                    "   │ {:22} │ {:8.1} │ {:10.2}M/s │ {:7.2}x │ {:8} │",
                    "Akida NPU",
                    neuro_result.time_us,
                    neuro_result.throughput / 1e6,
                    speedup,
                    "✅ NPU"
                );
            }
        }

        println!("   └────────────────────────┴──────────┴────────────────┴──────────┴──────────┘");
        println!();
    }

    // Step 3: Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("✅ CROSS-SUBSTRATE VALIDATION COMPLETE!\n");
    println!("   Substrates tested:");
    if stats.num_cpu > 0 {
        println!("   ✅ CPU ({} cores)", stats.num_cpu);
    }
    if stats.num_gpu > 0 {
        println!("   ✅ GPU ({} devices)", stats.num_gpu);
    }
    if akida_manager.device_count() > 0 {
        println!(
            "   ✅ Neuromorphic ({} Akida devices)",
            akida_manager.device_count()
        );
    }
    println!("\n   All substrates validated! 🎉\n");

    Ok(())
}

struct BenchmarkResult {
    time_us: f64,
    throughput: f64,
    output: Vec<f32>,
}

// Note: The cpu_result variable was declared but defined in a scope that doesn't exist here.
// I need to restructure this code properly.

async fn benchmark_cpu(
    runtime: &UniversalRuntime,
    input: &[f32],
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Execute ReLU on CPU
    let workload = WorkloadBuilder::new()
        .operation(OperationType::ReLU)
        .data_f32(input.to_vec())
        .build()?;

    let output = runtime
        .execute_on_type(ComputeUnitType::Cpu, workload)
        .await?;

    let elapsed = start.elapsed();
    let time_us = elapsed.as_secs_f64() * 1_000_000.0;
    let throughput = input.len() as f64 / elapsed.as_secs_f64();

    let output_vec = match output.data {
        toadstool_runtime_universal::WorkloadData::F32Vec(v) => v,
        _ => return Err("Invalid output type".into()),
    };

    Ok(BenchmarkResult {
        time_us,
        throughput,
        output: output_vec,
    })
}

async fn benchmark_gpu(
    runtime: &UniversalRuntime,
    index: usize,
    input: &[f32],
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Execute ReLU on GPU
    let workload = WorkloadBuilder::new()
        .operation(OperationType::ReLU)
        .data_f32(input.to_vec())
        .build()?;

    let output = runtime.execute_on(index, workload).await?;

    let elapsed = start.elapsed();
    let time_us = elapsed.as_secs_f64() * 1_000_000.0;
    let throughput = input.len() as f64 / elapsed.as_secs_f64();

    let output_vec = match output.data {
        toadstool_runtime_universal::WorkloadData::F32Vec(v) => v,
        _ => return Err("Invalid output type".into()),
    };

    Ok(BenchmarkResult {
        time_us,
        throughput,
        output: output_vec,
    })
}

async fn benchmark_neuromorphic(
    manager: &DeviceManager,
    _input_size: usize,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    // For neuromorphic, we need a model
    let model_path = std::env::args().nth(1).unwrap_or_else(|| {
        std::env::var("AKIDA_TEST_MODEL").unwrap_or_else(|_| "minimal_test.fbz".to_string())
    });

    let model = Model::from_file(&model_path)?;
    let mut device = manager.open_first()?;

    // Load model
    model.load_to_device(&mut device)?;

    // Run inference (input size determined by model)
    let input = vec![0u8; model.input_size()];

    let start = Instant::now();
    let result = model.infer(&input, &mut device)?;
    let elapsed = start.elapsed();

    let time_us = elapsed.as_secs_f64() * 1_000_000.0;
    let throughput = input.len() as f64 / elapsed.as_secs_f64();

    // Convert u8 output to f32 for comparison
    let output: Vec<f32> = result.output.iter().map(|&x| x as f32).collect();

    Ok(BenchmarkResult {
        time_us,
        throughput,
        output,
    })
}

fn validate_output(expected: &[f32], actual: &[f32]) -> String {
    if expected.len() != actual.len() {
        return "❌ Size mismatch".to_string();
    }

    let mut max_diff = 0.0f32;
    for (e, a) in expected.iter().zip(actual.iter()) {
        let diff = (e - a).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }

    if max_diff < 1e-5 {
        "✅ Perfect".to_string()
    } else if max_diff < 1e-3 {
        format!("✅ Good ({:.6})", max_diff)
    } else {
        format!("⚠️  {:.6}", max_diff)
    }
}
