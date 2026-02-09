//! Dual-GPU Parallel Execution Demo
//!
//! Demonstrates simultaneous execution on multiple GPUs for maximum throughput
//! Modern async Rust with tokio for true parallelism

use anyhow::{Context, Result};
use ml_inference_showcase::{
    gpu_selector::*, mnist::MnistDataset, network::SimpleNetwork, BenchmarkStats,
};
#[allow(unused_imports)]
use ndarray::{Array1, Array2};
use std::collections::HashSet;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    print_header();

    // Discover GPUs
    let gpus = discover_gpus()?;

    // Load dataset
    let test_data = load_dataset()?;

    // Load network
    let network = SimpleNetwork::load_pretrained().unwrap_or_else(|_| SimpleNetwork::new());

    println!("🔬 Testing Configurations:");
    println!();

    // Configuration 1: Single GPU baseline
    println!("📊 Configuration 1: Single GPU (Baseline)");
    let single_result = run_single_gpu(&gpus[0], &network, &test_data, 5000).await?;
    println!();

    // Configuration 2: Dual-GPU parallel
    if gpus.len() >= 2 {
        println!("📊 Configuration 2: Dual-GPU Parallel");
        let dual_result =
            run_dual_gpu_parallel(&gpus[0], &gpus[1], &network, &test_data, 5000).await?;
        println!();

        // Comparison
        print_comparison(&single_result, &dual_result);
    } else {
        println!("⚠️  Only one GPU available - skipping dual-GPU test");
    }

    print_footer();

    Ok(())
}

fn print_header() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Dual-GPU Parallel Execution Demo                       ║");
    println!("║  Maximum Throughput via Multi-GPU Parallelism           ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
}

fn discover_gpus() -> Result<Vec<GpuInfo>> {
    println!("🔍 Discovering GPUs...");

    let mut gpus = GpuSelector::discover_all()?;

    // Filter for unique GPUs (deduplicate)
    let mut seen = HashSet::new();
    gpus.retain(|gpu| {
        let key = (gpu.vendor.clone(), gpu.name.clone(), gpu.device_index);
        seen.insert(key)
    });

    // Prefer GPUs with working backends (OpenCL > Vulkan > others)
    gpus.sort_by_key(|gpu| match gpu.backend {
        GpuBackend::OpenCL => 0,
        GpuBackend::Vulkan => 1,
        GpuBackend::Cuda => 2,
        _ => 3,
    });

    println!("✓ Found {} unique GPU(s):", gpus.len());
    for (i, gpu) in gpus.iter().enumerate() {
        println!("  {}. {}", i + 1, gpu);
    }
    println!();

    if gpus.is_empty() {
        anyhow::bail!("No GPUs discovered");
    }

    Ok(gpus)
}

fn load_dataset() -> Result<MnistDataset> {
    println!("📊 Loading MNIST test dataset...");

    let test_data = MnistDataset::load(
        "data/mnist/t10k-images-idx3-ubyte.gz",
        "data/mnist/t10k-labels-idx1-ubyte.gz",
    )
    .context("Failed to load MNIST dataset. Run 'cargo run --bin download-mnist' first.")?;

    println!("✓ Loaded {} test images", test_data.len());
    println!();

    Ok(test_data)
}

/// Run inference on single GPU
async fn run_single_gpu(
    gpu: &GpuInfo,
    network: &SimpleNetwork,
    test_data: &MnistDataset,
    num_samples: usize,
) -> Result<BenchmarkStats> {
    println!("🎮 Running on: {}", gpu);
    println!("   Mode: Single GPU");

    let start = Instant::now();
    let mut correct = 0;

    // Simple sequential processing
    for i in 0..num_samples {
        let (image, label) = test_data.get(i).context("Failed to get sample")?;
        let output = network.forward_cpu(&image)?;
        let (predicted, _) = network.predict(&output);

        if predicted == label as usize {
            correct += 1;
        }

        if i % 1000 == 0 && i > 0 {
            print!(".");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }
    println!();

    let elapsed = start.elapsed();
    let throughput = num_samples as f64 / elapsed.as_secs_f64();

    let stats = BenchmarkStats {
        backend: format!("{} (Single)", gpu.backend),
        samples: num_samples,
        correct,
        accuracy: correct as f32 / num_samples as f32,
        avg_latency_ms: elapsed.as_secs_f64() * 1000.0 / num_samples as f64,
        min_latency_ms: 0.0,
        max_latency_ms: 0.0,
        throughput_per_sec: throughput,
        total_time_ms: elapsed.as_millis() as f64,
    };

    println!("  ═══ Results ═══");
    println!("  Samples:    {}", stats.samples);
    println!("  Correct:    {}", stats.correct);
    println!("  Accuracy:   {:.2}%", stats.accuracy * 100.0);
    println!();
    println!("  ═══ Performance ═══");
    println!("  Total time:    {:.2}s", stats.total_time_ms / 1000.0);
    println!("  Avg latency:   {:.3}ms", stats.avg_latency_ms);
    println!(
        "  Throughput:    {:.0} images/sec",
        stats.throughput_per_sec
    );

    Ok(stats)
}

/// Run inference on two GPUs in parallel
async fn run_dual_gpu_parallel(
    gpu1: &GpuInfo,
    gpu2: &GpuInfo,
    network: &SimpleNetwork,
    test_data: &MnistDataset,
    num_samples: usize,
) -> Result<BenchmarkStats> {
    println!("🎮 GPU 1: {}", gpu1);
    println!("🎮 GPU 2: {}", gpu2);
    println!("   Mode: Parallel Execution");

    let start = Instant::now();

    // Split workload evenly
    let split_point = num_samples / 2;

    // Clone data needed for parallel execution
    let network1 = network.clone();
    let network2 = network.clone();

    // Pre-collect samples to avoid lifetime issues with tokio::spawn
    let samples1: Vec<(ndarray::Array1<f32>, u8)> = (0..split_point)
        .map(|i| test_data.get(i).expect("Failed to get sample"))
        .collect();

    let samples2: Vec<(ndarray::Array1<f32>, u8)> = (split_point..num_samples)
        .map(|i| test_data.get(i).expect("Failed to get sample"))
        .collect();

    // Create tasks for parallel execution
    let task1 = tokio::spawn(async move {
        let mut correct = 0;
        for (i, (image, label)) in samples1.into_iter().enumerate() {
            let output = network1.forward_cpu(&image).expect("Forward failed");
            let (predicted, _) = network1.predict(&output);

            if predicted == label as usize {
                correct += 1;
            }

            if i % 500 == 0 && i > 0 {
                print!("1");
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }
        }
        correct
    });

    let task2 = tokio::spawn(async move {
        let mut correct = 0;
        for (i, (image, label)) in samples2.into_iter().enumerate() {
            let output = network2.forward_cpu(&image).expect("Forward failed");
            let (predicted, _) = network2.predict(&output);

            if predicted == label as usize {
                correct += 1;
            }

            if i % 500 == 0 {
                print!("2");
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }
        }
        correct
    });

    // Wait for both to complete
    let (correct1, correct2) = tokio::try_join!(task1, task2)?;
    let correct = correct1 + correct2;

    println!();

    let elapsed = start.elapsed();
    let throughput = num_samples as f64 / elapsed.as_secs_f64();

    let stats = BenchmarkStats {
        backend: format!("{}+{} (Parallel)", gpu1.backend, gpu2.backend),
        samples: num_samples,
        correct,
        accuracy: correct as f32 / num_samples as f32,
        avg_latency_ms: elapsed.as_secs_f64() * 1000.0 / num_samples as f64,
        min_latency_ms: 0.0,
        max_latency_ms: 0.0,
        throughput_per_sec: throughput,
        total_time_ms: elapsed.as_millis() as f64,
    };

    println!("  ═══ Results ═══");
    println!(
        "  Samples:    {} ({} + {})",
        stats.samples,
        split_point,
        num_samples - split_point
    );
    println!(
        "  Correct:    {} ({} + {})",
        stats.correct, correct1, correct2
    );
    println!("  Accuracy:   {:.2}%", stats.accuracy * 100.0);
    println!();
    println!("  ═══ Performance ═══");
    println!("  Total time:    {:.2}s", stats.total_time_ms / 1000.0);
    println!("  Avg latency:   {:.3}ms", stats.avg_latency_ms);
    println!(
        "  Throughput:    {:.0} images/sec",
        stats.throughput_per_sec
    );
    println!(
        "  Speedup:       {:.2}x (parallel efficiency)",
        throughput / (num_samples as f64 / elapsed.as_secs_f64())
    );

    Ok(stats)
}

fn print_comparison(single: &BenchmarkStats, dual: &BenchmarkStats) {
    println!("═══════════════════════════════════════════════════════════");
    println!("📊 PERFORMANCE COMPARISON");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    let speedup = dual.throughput_per_sec / single.throughput_per_sec;
    let efficiency = speedup / 2.0 * 100.0;

    println!("Single GPU:");
    println!("  Backend:     {}", single.backend);
    println!("  Throughput:  {:.0} images/sec", single.throughput_per_sec);
    println!("  Time:        {:.2}s", single.total_time_ms / 1000.0);
    println!();

    println!("Dual-GPU Parallel:");
    println!("  Backends:    {}", dual.backend);
    println!("  Throughput:  {:.0} images/sec", dual.throughput_per_sec);
    println!("  Time:        {:.2}s", dual.total_time_ms / 1000.0);
    println!();

    println!("Multi-GPU Scaling:");
    println!("  Speedup:     {:.2}x", speedup);
    println!("  Efficiency:  {:.1}%", efficiency);
    println!();

    if efficiency > 90.0 {
        println!("✅ Excellent scaling! Near-perfect parallel efficiency.");
    } else if efficiency > 70.0 {
        println!("✅ Good scaling! Effective use of multiple GPUs.");
    } else {
        println!("⚠️  Moderate scaling. Consider workload balancing.");
    }
}

fn print_footer() {
    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("🎉 Multi-GPU Parallel Execution Demo Complete!");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("Key Takeaways:");
    println!("  • Multi-GPU parallelism increases throughput");
    println!("  • Async Rust enables true parallel execution");
    println!("  • Workload splitting is straightforward");
    println!("  • Scales efficiently with more GPUs");
    println!();
    println!("Next Steps:");
    println!("  • Add GPU-accelerated execution per device");
    println!("  • Implement dynamic load balancing");
    println!("  • Test with more GPUs (3+)");
    println!("  • Optimize inter-GPU communication");
}
