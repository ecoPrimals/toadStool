//! Cross-substrate validation for NAdam optimizer
//!
//! **Deep Debt**: Validates identical NAdam behavior on all hardware

use barracuda::device::substrate::Substrate;
use barracuda::tensor::Tensor;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════════");
    println!("  NADAM OPTIMIZER - Cross-Substrate Validation");
    println!("═══════════════════════════════════════════════════════════\n");

    // Discover all substrates
    let substrates = Substrate::discover_all()?;
    println!("Found {} substrates:", substrates.len());
    for (i, sub) in substrates.iter().enumerate() {
        println!("  {}. {:?} ({}) - {}", i + 1, sub.substrate_type, sub.backend, sub.name);
    }
    println!();

    if substrates.is_empty() {
        println!("❌ No substrates found!");
        return Ok(());
    }

    // Test parameters
    let size = 1000;
    let learning_rate = 0.001;
    let beta1 = 0.9;
    let beta2 = 0.999;
    let epsilon = 1e-8;
    let weight_decay = 0.0;
    let step = 1;

    // Create reference data on first device
    println!("Creating reference data on {:?}...", substrates[0].substrate_type);
    let ref_device = Arc::new(substrates[0].create_device().await?);
    
    let ref_weights = Tensor::from_vec_on(
        vec![1.0; size],
        vec![size],
        ref_device.clone(),
    )
    .await?;

    let ref_gradients = Tensor::from_vec_on(
        vec![0.1; size],
        vec![size],
        ref_device.clone(),
    )
    .await?;

    let ref_m = Tensor::from_vec_on(vec![0.0; size], vec![size], ref_device.clone())
        .await?;

    let ref_v = Tensor::from_vec_on(vec![0.0; size], vec![size], ref_device.clone())
        .await?;

    let (ref_w_out, ref_m_out, ref_v_out) = ref_weights
        .nadam(&ref_gradients, &ref_m, &ref_v, learning_rate, beta1, beta2, epsilon, weight_decay, step)?;

    let ref_w_data = ref_w_out.to_vec()?;
    let ref_m_data = ref_m_out.to_vec()?;
    let ref_v_data = ref_v_out.to_vec()?;

    println!("✓ Reference computed\n");

    // Validate on each substrate
    println!("Validating on {} substrates:", substrates.len());
    let mut all_passed = true;

    for (i, substrate) in substrates.iter().enumerate() {
        print!("  {}. {:?}...", i + 1, substrate.substrate_type);

        let device = Arc::new(substrate.create_device().await?);

        // Create input tensors on this substrate
        let weights = Tensor::from_vec_on(vec![1.0; size], vec![size], device.clone())
            .await?;
        let gradients = Tensor::from_vec_on(vec![0.1; size], vec![size], device.clone())
            .await?;
        let m = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await?;
        let v = Tensor::from_vec_on(vec![0.0; size], vec![size], device.clone())
            .await?;

        // Execute NAdam
        let (w_out, m_out, v_out) = weights
            .nadam(&gradients, &m, &v, learning_rate, beta1, beta2, epsilon, weight_decay, step)?;

        let w_data = w_out.to_vec()?;
        let m_data = m_out.to_vec()?;
        let v_data = v_out.to_vec()?;

        // Compare with reference
        let mut max_diff_w = 0.0f32;
        let mut max_diff_m = 0.0f32;
        let mut max_diff_v = 0.0f32;

        for j in 0..size {
            max_diff_w = max_diff_w.max((ref_w_data[j] - w_data[j]).abs());
            max_diff_m = max_diff_m.max((ref_m_data[j] - m_data[j]).abs());
            max_diff_v = max_diff_v.max((ref_v_data[j] - v_data[j]).abs());
        }

        let tolerance = 1e-5;
        let passed = max_diff_w < tolerance && max_diff_m < tolerance && max_diff_v < tolerance;

        if passed {
            println!(" ✓ PASS (max diff: w={:.2e}, m={:.2e}, v={:.2e})", 
                     max_diff_w, max_diff_m, max_diff_v);
        } else {
            println!(" ✗ FAIL (max diff: w={:.2e}, m={:.2e}, v={:.2e})", 
                     max_diff_w, max_diff_m, max_diff_v);
            all_passed = false;
        }
    }

    println!();
    if all_passed {
        println!("✓ ALL SUBSTRATES PASSED - Identical NAdam behavior!");
    } else {
        println!("✗ Some substrates failed validation");
    }

    println!("═══════════════════════════════════════════════════════════");
    Ok(())
}
