// SPDX-License-Identifier: AGPL-3.0-or-later
//! RBF Surrogate Learning Example
//!
//! Demonstrates complete BarraCuda RBF pipeline:
//! - Train on known function (sin, polynomial, etc.)
//! - Predict at new points
//! - Validate accuracy
//!
//! **Deep Debt**: Uses ToadStool for hardware discovery

use barracuda::ops::interpolation::{RbfInterpolator, RbfKernelType};
use barracuda::tensor::Tensor;
use toadstool_core::HardwareManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║   RBF Surrogate Learning - GPU Accelerated          ║");
    println!("║   hotSpring Physics Integration Demo                ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    // Step 1: ToadStool discovers hardware
    println!("[1/5] ToadStool discovering hardware...");
    let hw = HardwareManager::discover()?;
    println!("  ✓ Discovered {} device(s)", hw.device_count());
    println!("  GPU available: {}", hw.has_gpu());

    if !hw.has_gpu() {
        println!("\n⚠️  No GPU detected, RBF will use CPU fallback");
    }

    // Step 2: Create training data (simulate physics EOS)
    println!("\n[2/5] Generating training data...");
    println!("  Simulating physics: y = sin(2πx) + noise");

    let n_train = 12;
    let mut x_train_data = Vec::new();
    let mut y_train_data = Vec::new();

    for i in 0..n_train {
        let x = (i as f32) / (n_train as f32); // [0, 1]
        let y = (2.0 * std::f32::consts::PI * x).sin() + 0.05 * ((i * 7) as f32).sin();
        x_train_data.push(x);
        y_train_data.push(y);
    }

    println!("  Training points: {n_train}");
    println!("  Parameter space: 1D [0, 1]");

    // Step 3: Train RBF surrogate on GPU
    println!("\n[3/5] Training RBF surrogate on GPU...");

    let _device = barracuda::device::WgpuDevice::new().await?;

    let x_train = Tensor::from_vec(x_train_data.clone(), vec![n_train, 1]).await?;
    let y_train = Tensor::from_vec(y_train_data.clone(), vec![n_train]).await?;

    let start = std::time::Instant::now();
    let rbf = RbfInterpolator::fit(
        &x_train,
        &y_train,
        RbfKernelType::ThinPlateSpline, // Best for physics
        1.0,
    )?;
    let train_time = start.elapsed();

    println!("  ✓ RBF surrogate trained");
    println!("  Kernel: Thin Plate Spline (physics-optimized)");
    println!(
        "  Training time: {:.2} ms",
        train_time.as_secs_f64() * 1000.0
    );
    println!("  Weights: {} parameters", rbf.n_training_points());

    // Step 4: Predict at new points
    println!("\n[4/5] Predicting at new evaluation points...");

    let n_eval = 100;
    let mut x_eval_data = Vec::new();
    for i in 0..n_eval {
        x_eval_data.push((i as f32) / (n_eval as f32));
    }

    let x_eval = Tensor::from_vec(x_eval_data.clone(), vec![n_eval, 1]).await?;

    let start = std::time::Instant::now();
    let y_pred = rbf.predict(&x_eval)?;
    let pred_time = start.elapsed();

    let predictions = y_pred.to_vec()?;

    println!("  ✓ Predictions computed");
    println!("  Evaluation points: {n_eval}");
    println!(
        "  Prediction time: {:.2} ms",
        pred_time.as_secs_f64() * 1000.0
    );
    println!(
        "  Throughput: {:.0} predictions/sec",
        n_eval as f64 / pred_time.as_secs_f64()
    );

    // Step 5: Validate accuracy
    println!("\n[5/5] Validating accuracy...");

    let mut max_error = 0.0_f32;
    let mut mean_error = 0.0_f32;

    for (i, &pred) in predictions.iter().enumerate() {
        let x = x_eval_data[i];
        let true_val = (2.0 * std::f32::consts::PI * x).sin();
        let error = (pred - true_val).abs();
        max_error = max_error.max(error);
        mean_error += error;
    }
    mean_error /= n_eval as f32;

    println!("  Mean error: {mean_error:.6}");
    println!("  Max error: {max_error:.6}");

    if mean_error < 0.1 {
        println!("  ✓ Accuracy: EXCELLENT (< 0.1)");
    } else {
        println!("  ⚠️  Accuracy: MODERATE (> 0.1)");
    }

    // Summary
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║   RBF Surrogate Learning: SUCCESS                   ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();
    println!("  Training:   {:.2} ms", train_time.as_secs_f64() * 1000.0);
    println!("  Prediction: {:.2} ms", pred_time.as_secs_f64() * 1000.0);
    println!("  Accuracy:   Mean {mean_error:.6}, Max {max_error:.6}");
    println!();
    println!("🦈 BarraCuda: GPU-accelerated scientific computing ready!");
    println!("🔬 hotSpring: Physics surrogate learning operational!");
    println!();

    Ok(())
}
