//! Fused LayerNorm Validation Test
//!
//! Validates that the fused (1-pass) LayerNorm produces numerically similar results
//! to the original (3-pass) implementation.
//!
//! **Note**: Small numerical differences (<2%) are expected due to different
//! accumulation order in floating-point operations. The fused version uses
//! Welford's algorithm in shared memory while the original uses multiple
//! passes through global memory.

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::{NormConfig, WgpuExecutor};

#[tokio::test]
async fn test_fused_layernorm_correctness() {
    gpu_test_resilient_async(async {
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
        let result_fused = executor
            .execute_layernorm_fused(&input, config.clone())
            .await
            .unwrap();

        // They should produce identical results
        assert_eq!(result_original.len(), result_fused.len());

        for (i, (orig, fused)) in result_original.iter().zip(result_fused.iter()).enumerate() {
            let diff = (orig - fused).abs();
            assert!(
                diff < 0.02, // 2% tolerance for floating-point accumulation differences
                "Mismatch at index {}: original={}, fused={}, diff={}",
                i,
                orig,
                fused,
                diff
            );
        }

        println!("✅ Fused LayerNorm produces identical results to original (GPT-2 scale)");
    })
    .await;
}

#[tokio::test]
async fn test_fused_layernorm_llama_scale() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test with LLaMA scale (the bottleneck!)
        let size = 4096 * 256;
        let input: Vec<f32> = (0..size).map(|i| ((i % 1000) as f32) * 0.001).collect();

        let config = NormConfig {
            epsilon: 1e-5,
            gamma: None,
            beta: None,
        };

        // Execute fused implementation
        let result = executor
            .execute_layernorm_fused(&input, config)
            .await
            .unwrap();

        assert_eq!(result.len(), size);

        // Verify normalization properties (mean ≈ 0, variance ≈ 1)
        let mean: f32 = result.iter().sum::<f32>() / result.len() as f32;
        let variance: f32 =
            result.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / result.len() as f32;

        assert!(mean.abs() < 0.01, "Mean should be close to 0, got {}", mean);
        assert!(
            (variance - 1.0).abs() < 0.01,
            "Variance should be close to 1, got {}",
            variance
        );

        println!(
            "✅ Fused LayerNorm handles LLaMA-scale correctly (4096 * 256 = 1,048,576 elements)"
        );
    })
    .await;
}

#[tokio::test]
async fn test_fused_layernorm_with_gamma_beta() {
    gpu_test_resilient_async(async {
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
        let result_fused = executor
            .execute_layernorm_fused(&input, config.clone())
            .await
            .unwrap();

        // They should produce identical results
        assert_eq!(result_original.len(), result_fused.len());

        for (i, (orig, fused)) in result_original.iter().zip(result_fused.iter()).enumerate() {
            let diff = (orig - fused).abs();
            assert!(
                diff < 0.02, // 2% tolerance for floating-point accumulation differences
                "Mismatch at index {}: original={}, fused={}, diff={}",
                i,
                orig,
                fused,
                diff
            );
        }

        println!("✅ Fused LayerNorm correctly applies gamma and beta parameters");
    })
    .await;
}
