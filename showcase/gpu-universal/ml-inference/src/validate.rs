// SPDX-License-Identifier: AGPL-3.0-or-later
//! Validate CPU and GPU produce identical results

use anyhow::Result;
use ml_inference_showcase::{
    cpu_inference::CpuInference, mnist::MnistDataset, network::SimpleNetwork,
};

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  CPU/GPU Correctness Validation                         ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Load test samples
    println!("Loading MNIST test dataset...");
    let test_data = MnistDataset::load(
        "data/mnist/t10k-images-idx3-ubyte.gz",
        "data/mnist/t10k-labels-idx1-ubyte.gz",
    )?;
    println!("✓ Loaded {} samples", test_data.len());
    println!();

    // Create network
    println!("Initializing network...");
    let network = SimpleNetwork::new();
    let cpu_inference = CpuInference::new(network);
    println!("✓ Network ready");
    println!();

    // Validate on subset
    let num_samples = 100;
    println!("Validating {num_samples} samples...");

    let mut matches = 0;
    let tolerance = 1e-5_f32;

    for i in 0..num_samples {
        let (image, _label) = test_data.get(i).unwrap();

        // CPU inference
        let cpu_result = cpu_inference.infer(&image)?;

        // TODO: GPU inference
        // For now, just validate CPU is deterministic
        let cpu_result2 = cpu_inference.infer(&image)?;

        if cpu_result.matches(&cpu_result2, tolerance) {
            matches += 1;
        } else {
            println!("  ⚠ Mismatch at sample {i}");
            println!(
                "    CPU1: class={}, conf={:.4}",
                cpu_result.predicted_class, cpu_result.confidence
            );
            println!(
                "    CPU2: class={}, conf={:.4}",
                cpu_result2.predicted_class, cpu_result2.confidence
            );
        }
    }

    println!();
    println!("═══ Validation Results ═══");
    println!("  Samples validated: {num_samples}");
    println!("  Matches: {matches}");
    println!(
        "  Match rate: {:.2}%",
        (matches as f32 / num_samples as f32) * 100.0
    );

    if matches == num_samples {
        println!();
        println!("✅ CPU inference is deterministic!");
        println!("   Ready to compare with GPU implementation.");
    } else {
        println!();
        println!("⚠ CPU inference has non-determinism!");
        println!("  This should not happen with fixed random seed.");
    }

    Ok(())
}
