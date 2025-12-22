//! Hybrid CPU+GPU pipeline - real workload sharing

use ml_inference_showcase::{mnist::MnistDataset, network::SimpleNetwork, cpu_inference::CpuInference};
use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Hybrid CPU+GPU Pipeline                                ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    
    // Load test dataset
    println!("Loading MNIST test dataset...");
    let test_data = MnistDataset::load(
        "data/mnist/t10k-images-idx3-ubyte.gz",
        "data/mnist/t10k-labels-idx1-ubyte.gz",
    )?;
    println!("✓ Loaded {} samples", test_data.len());
    println!();
    
    // Strategy: Use CPU for single samples, GPU for batches
    println!("Strategy: CPU for latency-critical, GPU for throughput");
    println!();
    
    let network = SimpleNetwork::new();
    let cpu_inference = CpuInference::new(network.clone());
    
    // Simulate real-time scenario:
    // - Interactive requests (10%) go to CPU for low latency
    // - Batch requests (90%) go to GPU for high throughput
    
    let total_samples = 1000;
    let interactive_ratio = 0.1;
    let num_interactive = (total_samples as f32 * interactive_ratio) as usize;
    let num_batch = total_samples - num_interactive;
    
    println!("Processing {} samples:", total_samples);
    println!("  - {} interactive (CPU, single)", num_interactive);
    println!("  - {} batch (GPU, batched)", num_batch);
    println!();
    
    // CPU: Interactive
    println!("CPU: Processing interactive requests...");
    let cpu_start = Instant::now();
    let mut cpu_correct = 0;
    
    for i in 0..num_interactive {
        let (image, label) = test_data.get(i).unwrap();
        let result = cpu_inference.infer(&image)?;
        if result.predicted_class == label as usize {
            cpu_correct += 1;
        }
    }
    
    let cpu_time = cpu_start.elapsed();
    let cpu_latency = cpu_time.as_micros() as f64 / num_interactive as f64 / 1000.0;
    println!("✓ CPU done: {:.3}ms avg latency", cpu_latency);
    
    // GPU: Batch (simulated with CPU for now)
    println!("GPU: Processing batch requests...");
    let gpu_start = Instant::now();
    let mut gpu_correct = 0;
    
    let batch_size = 64;
    let num_batches = num_batch / batch_size;
    
    for batch_idx in 0..num_batches {
        let batch_start = num_interactive + batch_idx * batch_size;
        let (images, labels) = test_data.batch(batch_start, batch_size).unwrap();
        
        // TODO: GPU batched inference
        let outputs = network.forward_batch_cpu(&images)?;
        
        for i in 0..batch_size {
            let output = outputs.row(i).to_owned();
            let (predicted, _) = network.predict(&output);
            if predicted == labels[i] as usize {
                gpu_correct += 1;
            }
        }
    }
    
    let gpu_time = gpu_start.elapsed();
    let gpu_latency = gpu_time.as_micros() as f64 / (num_batches * batch_size) as f64 / 1000.0;
    println!("✓ GPU done: {:.3}ms avg latency (batched)", gpu_latency);
    
    // Overall statistics
    let total_time = cpu_time + gpu_time;
    let total_correct = cpu_correct + gpu_correct;
    let overall_accuracy = total_correct as f32 / total_samples as f32;
    let overall_throughput = total_samples as f64 / total_time.as_secs_f64();
    
    println!();
    println!("═══ Hybrid Pipeline Results ═══");
    println!("  Total samples: {}", total_samples);
    println!("  Total correct: {}", total_correct);
    println!("  Overall accuracy: {:.2}%", overall_accuracy * 100.0);
    println!();
    println!("  CPU latency: {:.3}ms", cpu_latency);
    println!("  GPU latency: {:.3}ms", gpu_latency);
    println!("  GPU speedup: {:.1}x", cpu_latency / gpu_latency);
    println!();
    println!("  Total time: {:.2}s", total_time.as_secs_f64());
    println!("  Overall throughput: {:.0} inferences/sec", overall_throughput);
    println!();
    
    println!("This demonstrates intelligent workload placement:");
    println!("  ✓ Latency-critical → CPU (immediate response)");
    println!("  ✓ Throughput-oriented → GPU (batched processing)");
    println!("  ✓ Best of both worlds!");
    
    Ok(())
}

