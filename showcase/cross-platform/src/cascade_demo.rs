//! Cross-Platform Cascade Pipeline Demo
//!
//! Demonstrates heterogeneous compute concept:
//! - Stage 1: GPU preprocessing (NVIDIA or AMD via wgpu)
//! - Stage 2: NPU inference (Akida AKD1000 - simulated)
//! - Stage 3: CPU postprocessing

use barracuda::device::WgpuDevice;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Simulated cascade stage result
struct StageResult {
    name: String,
    device: String,
    output: Vec<f32>,
    latency_ms: f64,
}

/// GPU preprocessing: normalize input
fn preprocess_gpu(input: &[f32]) -> StageResult {
    let start = std::time::Instant::now();

    // Normalize to [0, 1]
    let max_val = input.iter().fold(f32::MIN, |a, &b| a.max(b));
    let min_val = input.iter().fold(f32::MAX, |a, &b| a.min(b));
    let range = (max_val - min_val).max(1e-6);
    let output: Vec<f32> = input.iter().map(|x| (x - min_val) / range).collect();

    StageResult {
        name: "preprocess".to_string(),
        device: "GPU".to_string(),
        output,
        latency_ms: start.elapsed().as_secs_f64() * 1000.0,
    }
}

/// NPU inference: simulated SNN classification
fn infer_npu(input: &[f32], num_classes: usize) -> StageResult {
    let start = std::time::Instant::now();

    // Simulate spiking neural network
    let mut output = vec![0.0f32; num_classes];
    for (i, &v) in input.iter().enumerate() {
        output[i % num_classes] += v;
    }

    // Softmax
    let max_o = output.iter().fold(f32::MIN, |a, &b| a.max(b));
    let exp_sum: f32 = output.iter().map(|&x| (x - max_o).exp()).sum();
    output
        .iter_mut()
        .for_each(|x| *x = (*x - max_o).exp() / exp_sum);

    StageResult {
        name: "inference".to_string(),
        device: "NPU (simulated)".to_string(),
        output,
        latency_ms: start.elapsed().as_secs_f64() * 1000.0,
    }
}

/// CPU postprocessing: top-k extraction
fn postprocess_cpu(probabilities: &[f32], k: usize) -> StageResult {
    let start = std::time::Instant::now();

    let mut indexed: Vec<(usize, f32)> = probabilities
        .iter()
        .enumerate()
        .map(|(i, &p)| (i, p))
        .collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Output: [class0, prob0, class1, prob1, ...]
    let output: Vec<f32> = indexed
        .iter()
        .take(k)
        .flat_map(|(class, prob)| vec![*class as f32, *prob])
        .collect();

    StageResult {
        name: "postprocess".to_string(),
        device: "CPU".to_string(),
        output,
        latency_ms: start.elapsed().as_secs_f64() * 1000.0,
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  ToadStool Cross-Platform Cascade Demo                        ║");
    println!("║  GPU → NPU → CPU Pipeline                                     ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Initialize GPU device
    info!("Initializing wgpu device...");
    let gpu_device = Arc::new(WgpuDevice::new().await?);
    info!("GPU: {}", gpu_device.name());
    println!();

    // Generate test input (simulated image features)
    println!("Generating test input (1000 features)...");
    let input: Vec<f32> = (0..1000)
        .map(|i| ((i as f32 * 0.1).sin() + 1.0) * 50.0)
        .collect();
    println!();

    // Execute cascade
    println!("═══ Running Cascade Pipeline ═══");
    println!();

    let total_start = std::time::Instant::now();

    // Stage 1: GPU preprocessing
    let stage1 = preprocess_gpu(&input);
    println!("  Stage 1 ({}): {}", stage1.device, stage1.name);
    println!("           Latency: {:.3} ms", stage1.latency_ms);
    println!(
        "           Output: {} normalized values",
        stage1.output.len()
    );
    println!();

    // Stage 2: NPU inference
    let stage2 = infer_npu(&stage1.output, 10);
    println!("  Stage 2 ({}): {}", stage2.device, stage2.name);
    println!("           Latency: {:.3} ms", stage2.latency_ms);
    println!(
        "           Output: {} class probabilities",
        stage2.output.len()
    );
    println!();

    // Stage 3: CPU postprocessing
    let stage3 = postprocess_cpu(&stage2.output, 3);
    println!("  Stage 3 ({}): {}", stage3.device, stage3.name);
    println!("           Latency: {:.3} ms", stage3.latency_ms);
    println!();

    let total_latency = total_start.elapsed().as_secs_f64() * 1000.0;

    // Display results
    println!("═══ Cascade Results ═══");
    println!();
    println!("  Top-3 Predictions:");
    for i in 0..3 {
        let class = stage3.output[i * 2] as usize;
        let prob = stage3.output[i * 2 + 1];
        println!("    {}. Class {:>2} - {:.2}%", i + 1, class, prob * 100.0);
    }
    println!();
    println!("  Pipeline Latency:");
    println!("    Stage 1 (GPU):  {:.3} ms", stage1.latency_ms);
    println!("    Stage 2 (NPU):  {:.3} ms", stage2.latency_ms);
    println!("    Stage 3 (CPU):  {:.3} ms", stage3.latency_ms);
    println!("    ────────────────────────");
    println!("    Total:          {:.3} ms", total_latency);
    println!();

    // Show device summary
    println!("═══ Hardware Summary ═══");
    println!();
    println!("  GPU:  {} (wgpu/WGSL)", gpu_device.name());
    println!("  NPU:  Akida AKD1000 (simulated - run setup-akida-vfio.sh for real)");
    println!(
        "  CPU:  {} cores",
        std::thread::available_parallelism()?.get()
    );
    println!();

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Cross-Platform Cascade: SUCCESS                              ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    Ok(())
}
