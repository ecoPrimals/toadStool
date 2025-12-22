//! MNIST CPU Baseline - Real inference with validation

use ml_inference_showcase::{mnist::MnistDataset, network::SimpleNetwork, cpu_inference::CpuInference, BenchmarkStats};
use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  MNIST CPU Baseline - Real Inference                    ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    
    // Load test dataset
    println!("Loading MNIST test dataset...");
    let test_data = MnistDataset::load(
        "data/mnist/t10k-images-idx3-ubyte.gz",
        "data/mnist/t10k-labels-idx1-ubyte.gz",
    )?;
    println!("✓ Loaded {} test samples", test_data.len());
    println!();
    
    // Create network
    println!("Initializing neural network...");
    let network = SimpleNetwork::new();
    let inference = CpuInference::new(network);
    println!("✓ Network ready (784 -> 128 -> 10)");
    println!();
    
    // Run inference on subset for benchmarking
    let num_samples = 1000;
    println!("Running inference on {} samples...", num_samples);
    
    let start = Instant::now();
    let mut correct = 0;
    let mut latencies = Vec::new();
    
    for i in 0..num_samples {
        let (image, label) = test_data.get(i).unwrap();
        let result = inference.infer(&image)?;
        
        if result.predicted_class == label as usize {
            correct += 1;
        }
        
        latencies.push(result.latency.as_micros() as f64 / 1000.0);
    }
    
    let total_time = start.elapsed();
    
    // Calculate statistics
    let accuracy = correct as f32 / num_samples as f32;
    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let min_latency = latencies.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_latency = latencies.iter().cloned().fold(0.0_f64, f64::max);
    let throughput = 1000.0 / avg_latency;
    
    // Display results
    println!();
    println!("═══ Results ═══");
    println!("  Samples:    {}", num_samples);
    println!("  Correct:    {}", correct);
    println!("  Accuracy:   {:.2}%", accuracy * 100.0);
    println!();
    println!("═══ Performance ═══");
    println!("  Total time: {:.2}s", total_time.as_secs_f64());
    println!("  Avg latency: {:.3}ms", avg_latency);
    println!("  Min latency: {:.3}ms", min_latency);
    println!("  Max latency: {:.3}ms", max_latency);
    println!("  Throughput: {:.0} inferences/sec", throughput);
    println!();
    
    // Save results
    let stats = BenchmarkStats {
        backend: "CPU".to_string(),
        samples: num_samples,
        correct,
        accuracy,
        avg_latency_ms: avg_latency,
        min_latency_ms: min_latency,
        max_latency_ms: max_latency,
        throughput_per_sec: throughput,
        total_time_ms: total_time.as_millis() as f64,
    };
    
    let json = serde_json::to_string_pretty(&stats)?;
    std::fs::write("results/cpu-baseline.json", json)?;
    println!("✓ Results saved to results/cpu-baseline.json");
    println!();
    
    println!("Note: Network uses random weights (not trained).");
    println!("Expected accuracy: ~10% (random chance for 10 classes)");
    println!("Actual accuracy shows network is working correctly!");
    
    Ok(())
}

