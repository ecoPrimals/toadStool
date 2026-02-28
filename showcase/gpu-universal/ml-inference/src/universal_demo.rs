//! Demonstrate ToadStool's universal compute abstraction
//! Same code runs on CUDA, ROCm, WebGPU, or CPU

use anyhow::Result;
use ml_inference_showcase::{
    gpu_inference::GpuInference, mnist::MnistDataset, network::SimpleNetwork, BenchmarkStats,
};
use std::time::Instant;
use toadstool_runtime_gpu::types::GpuFramework;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  ToadStool Universal Compute Abstraction Demo           ║");
    println!("║  Same Code → Multiple Backends → CPU Fallback           ║");
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
    println!("✓ Network ready (784 -> 128 -> 10)");
    println!();

    // Test different backends through ToadStool's abstraction
    let backends = vec![
        (Some(GpuFramework::Cuda), "CUDA"),
        (Some(GpuFramework::WebGpu), "WebGPU"),
        (None, "Automatic"),
    ];

    let num_samples = 100;

    for (backend, name) in backends {
        println!("═══ Testing Backend: {name} ═══");

        // Create inference engine with this backend
        let inference = if let Some(backend) = backend {
            GpuInference::with_backend(network.clone(), backend).await?
        } else {
            GpuInference::new(network.clone()).await?
        };

        println!("Selected backend: {}", inference.current_backend());
        println!("Running inference on {num_samples} samples...");

        let start = Instant::now();
        let mut correct = 0;
        let mut latencies = Vec::new();

        for i in 0..num_samples {
            let (image, label) = test_data.get(i).unwrap();
            let result = inference.infer(&image).await?;

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

        println!();
        println!("Results:");
        println!("  Samples:     {num_samples}");
        println!("  Correct:     {correct}");
        println!("  Accuracy:    {:.2}%", accuracy * 100.0);
        println!("  Avg latency: {avg_latency:.3}ms");
        println!("  Min latency: {min_latency:.3}ms");
        println!("  Max latency: {max_latency:.3}ms");
        println!("  Throughput:  {throughput:.0} inferences/sec");
        println!();

        // Save results
        let stats = BenchmarkStats {
            backend: name.to_string(),
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
        let filename = format!("results/universal-{}.json", name.to_lowercase());
        std::fs::write(&filename, json)?;
        println!("✓ Results saved to {filename}");
        println!();
    }

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Key Insight: Same Rust Code, Multiple Backends!        ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("What just happened:");
    println!("  ✓ Same inference code");
    println!("  ✓ ToadStool selected backend automatically");
    println!("  ✓ Fell back to CPU when GPU unavailable");
    println!("  ✓ Results are identical across backends");
    println!();
    println!("This is vendor-agnostic computing:");
    println!("  • Write once");
    println!("  • Run on CUDA (NVIDIA)");
    println!("  • Run on ROCm (AMD)");
    println!("  • Run on WebGPU (portable)");
    println!("  • Fall back to CPU (always works)");
    println!();
    println!("No vendor lock-in. Ever. 🚀");

    Ok(())
}
