//! 2-Dispatch LayerNorm Validation Test
//!
//! Validates that the practical 2-dispatch LayerNorm produces correct results
//! matching the original 3-pass implementation.

use ml_inference_showcase::wgpu::{NormConfig, WgpuExecutor};

#[tokio::test]
async fn test_2dispatch_layernorm_correctness() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Test with GPT-2 scale
    let size = 768;
    let input: Vec<f32> = (0..size).map(|i| (i as f32) * 0.001).collect();

    let config = NormConfig {
        epsilon: 1e-5,
        gamma: None,
        beta: None,
    };

    // Execute both implementations
    let result_original = executor
        .execute_layernorm(&input, config.clone())
        .await
        .unwrap();
    let result_2dispatch = executor
        .execute_layernorm_2dispatch(&input, config.clone())
        .await
        .unwrap();

    // They should produce accurate results (within 0.1% tolerance)
    assert_eq!(result_original.len(), result_2dispatch.len());

    let mut max_diff = 0.0f32;
    let mut max_rel_error = 0.0f32;

    for (i, (orig, two_d)) in result_original
        .iter()
        .zip(result_2dispatch.iter())
        .enumerate()
    {
        let diff = (orig - two_d).abs();
        let rel_error = if orig.abs() > 1e-6 {
            diff / orig.abs()
        } else {
            diff
        };

        max_diff = max_diff.max(diff);
        max_rel_error = max_rel_error.max(rel_error);

        assert!(
            rel_error < 0.001, // 0.1% tolerance
            "Mismatch at index {}: original={}, 2dispatch={}, diff={}, rel_error={}",
            i,
            orig,
            two_d,
            diff,
            rel_error
        );
    }

    println!("✅ 2-Dispatch LayerNorm correctness validated (GPT-2 scale: 768)");
    println!("   Max absolute diff: {}", max_diff);
    println!("   Max relative error: {:.4}%", max_rel_error * 100.0);
}

#[tokio::test]
async fn test_2dispatch_layernorm_llama_scale() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Test with LLaMA scale (the critical bottleneck!)
    let size = 4096 * 256;
    let input: Vec<f32> = (0..size).map(|i| ((i % 1000) as f32) * 0.001).collect();

    let config = NormConfig {
        epsilon: 1e-5,
        gamma: None,
        beta: None,
    };

    // Execute both implementations
    let result_original = executor
        .execute_layernorm(&input, config.clone())
        .await
        .unwrap();
    let result_2dispatch = executor
        .execute_layernorm_2dispatch(&input, config.clone())
        .await
        .unwrap();

    assert_eq!(result_original.len(), result_2dispatch.len());

    let mut max_diff = 0.0f32;
    let mut max_rel_error = 0.0f32;

    // Check a sample of elements (checking all 1M would be slow)
    for i in (0..size).step_by(1000) {
        let orig = result_original[i];
        let two_d = result_2dispatch[i];
        let diff = (orig - two_d).abs();
        let rel_error = if orig.abs() > 1e-6 {
            diff / orig.abs()
        } else {
            diff
        };

        max_diff = max_diff.max(diff);
        max_rel_error = max_rel_error.max(rel_error);

        assert!(
            rel_error < 0.001, // 0.1% tolerance
            "Mismatch at index {}: original={}, 2dispatch={}, diff={}, rel_error={}",
            i,
            orig,
            two_d,
            diff,
            rel_error
        );
    }

    println!(
        "✅ 2-Dispatch LayerNorm handles LLaMA-scale correctly (4096 * 256 = 1,048,576 elements)"
    );
    println!("   Max absolute diff (sampled): {}", max_diff);
    println!(
        "   Max relative error (sampled): {:.4}%",
        max_rel_error * 100.0
    );
}

#[tokio::test]
async fn test_2dispatch_layernorm_with_gamma_beta() {
    let executor = WgpuExecutor::new().await.unwrap();

    let size = 1024;
    let input: Vec<f32> = (0..size).map(|i| (i as f32) * 0.001).collect();

    // Custom gamma (scale) and beta (shift)
    let gamma: Vec<f32> = vec![2.0; size]; // Scale by 2
    let beta: Vec<f32> = vec![0.5; size]; // Shift by 0.5

    let config = NormConfig {
        epsilon: 1e-5,
        gamma: Some(gamma.clone()),
        beta: Some(beta.clone()),
    };

    // Execute both implementations
    let result_original = executor
        .execute_layernorm(&input, config.clone())
        .await
        .unwrap();
    let result_2dispatch = executor
        .execute_layernorm_2dispatch(&input, config.clone())
        .await
        .unwrap();

    // They should produce accurate results
    assert_eq!(result_original.len(), result_2dispatch.len());

    let mut max_rel_error = 0.0f32;

    for (i, (orig, two_d)) in result_original
        .iter()
        .zip(result_2dispatch.iter())
        .enumerate()
    {
        let diff = (orig - two_d).abs();
        let rel_error = if orig.abs() > 1e-6 {
            diff / orig.abs()
        } else {
            diff
        };

        max_rel_error = max_rel_error.max(rel_error);

        assert!(
            rel_error < 0.001, // 0.1% tolerance
            "Mismatch at index {}: original={}, 2dispatch={}, diff={}, rel_error={}",
            i,
            orig,
            two_d,
            diff,
            rel_error
        );
    }

    println!("✅ 2-Dispatch LayerNorm correctly applies gamma and beta parameters");
    println!("   Max relative error: {:.4}%", max_rel_error * 100.0);
}

#[tokio::test]
async fn test_2dispatch_normalization_properties() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Verify normalization properties (mean ≈ 0, variance ≈ 1)
    let size = 10000;
    let input: Vec<f32> = (0..size).map(|i| ((i % 100) as f32) * 0.1).collect();

    let config = NormConfig {
        epsilon: 1e-5,
        gamma: None, // No scaling
        beta: None,  // No shifting
    };

    let result = executor
        .execute_layernorm_2dispatch(&input, config)
        .await
        .unwrap();

    // Compute mean and variance of output
    let mean: f32 = result.iter().sum::<f32>() / result.len() as f32;
    let variance: f32 =
        result.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / result.len() as f32;

    assert!(mean.abs() < 0.01, "Mean should be close to 0, got {}", mean);
    assert!(
        (variance - 1.0).abs() < 0.01,
        "Variance should be close to 1, got {}",
        variance
    );

    println!("✅ 2-Dispatch LayerNorm normalization properties verified");
    println!("   Mean: {} (expected: 0)", mean);
    println!("   Variance: {} (expected: 1)", variance);
}
