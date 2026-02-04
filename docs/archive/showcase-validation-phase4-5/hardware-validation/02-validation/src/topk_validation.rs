//! Cross-substrate validation for TopK operation
//!
//! **Deep Debt**: Validates identical TopK behavior on all hardware

use barracuda::device::substrate::Substrate;
use barracuda::tensor::Tensor;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════════");
    println!("  TOPK - Cross-Substrate Validation");
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
    let input_data = vec![5.0, 1.0, 9.0, 3.0, 7.0, 2.0, 8.0, 4.0];
    let k = 3;

    // Create reference data on first device
    println!("Creating reference data on {:?}...", substrates[0].substrate_type);
    let ref_device = Arc::new(substrates[0].create_device().await?);
    
    let ref_input = Tensor::from_vec_on(
        input_data.clone(),
        vec![input_data.len()],
        ref_device,
    )
    .await?;

    let ref_result = ref_input.topk(k)?;
    let ref_indices = ref_result.to_vec()?;

    println!("✓ Reference computed: {:?}\n", 
             ref_indices.iter().map(|&x| x as u32).collect::<Vec<_>>());

    // Validate on each substrate
    println!("Validating on {} substrates:", substrates.len());
    let mut all_passed = true;

    for (i, substrate) in substrates.iter().enumerate() {
        print!("  {}. {:?}...", i + 1, substrate.substrate_type);

        let device = Arc::new(substrate.create_device().await?);

        // Create input tensor on this substrate
        let input = Tensor::from_vec_on(input_data.clone(), vec![input_data.len()], device)
            .await?;

        // Execute TopK
        let result = input.topk(k)?;
        let indices = result.to_vec()?;

        // Compare with reference
        let mut matches = true;
        for j in 0..k {
            if (ref_indices[j] - indices[j]).abs() > 1e-5 {
                matches = false;
                break;
            }
        }

        if matches {
            println!(" ✓ PASS (indices: {:?})", 
                     indices.iter().map(|&x| x as u32).collect::<Vec<_>>());
        } else {
            println!(" ✗ FAIL (expected: {:?}, got: {:?})",
                     ref_indices.iter().map(|&x| x as u32).collect::<Vec<_>>(),
                     indices.iter().map(|&x| x as u32).collect::<Vec<_>>());
            all_passed = false;
        }
    }

    println!();
    if all_passed {
        println!("✓ ALL SUBSTRATES PASSED - Identical TopK behavior!");
    } else {
        println!("✗ Some substrates failed validation");
    }

    println!("═══════════════════════════════════════════════════════════");
    Ok(())
}
