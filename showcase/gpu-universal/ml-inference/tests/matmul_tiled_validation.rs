//! Tiled MatMul Validation Test
//!
//! Validates that the memory-optimized tiled MatMul produces correct results.

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::WgpuExecutor;

#[tokio::test]
async fn test_tiled_matmul_small() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Small test (64x64 x 64x64)
        let m = 64;
        let k = 64;
        let n = 64;

        let a: Vec<f32> = (0..m * k).map(|i| (i as f32) * 0.01).collect();
        let b: Vec<f32> = (0..k * n).map(|i| ((i + 1) as f32) * 0.01).collect();

        // Execute both implementations
        let result_original = executor.execute_matmul(&a, &b, m, k, n).await.unwrap();
        let result_tiled = executor
            .execute_matmul_tiled(&a, &b, m, k, n)
            .await
            .unwrap();

        // Verify correctness
        assert_eq!(result_original.len(), result_tiled.len());

        let mut max_diff = 0.0f32;

        for (i, (orig, tiled)) in result_original.iter().zip(result_tiled.iter()).enumerate() {
            let diff = (orig - tiled).abs();
            max_diff = max_diff.max(diff);

            assert!(
                diff < 0.01,
                "Mismatch at index {}: original={}, tiled={}, diff={}",
                i,
                orig,
                tiled,
                diff
            );
        }

        println!("✅ Tiled MatMul correctness validated (64x64)");
        println!("   Max difference: {}", max_diff);
    })
    .await;
}

#[tokio::test]
async fn test_tiled_matmul_medium() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Medium test (512x512 x 512x512) - where tiling matters!
        let m = 512;
        let k = 512;
        let n = 512;

        let a: Vec<f32> = (0..m * k).map(|i| ((i % 1000) as f32) * 0.001).collect();
        let b: Vec<f32> = (0..k * n)
            .map(|i| (((i + 1) % 1000) as f32) * 0.001)
            .collect();

        // Execute both implementations
        let result_original = executor.execute_matmul(&a, &b, m, k, n).await.unwrap();
        let result_tiled = executor
            .execute_matmul_tiled(&a, &b, m, k, n)
            .await
            .unwrap();

        // Verify correctness
        assert_eq!(result_original.len(), result_tiled.len());

        let mut max_diff = 0.0f32;
        let mut max_rel_error = 0.0f32;

        // Sample validation (checking all 262K elements is slow)
        for i in (0..result_original.len()).step_by(1000) {
            let orig = result_original[i];
            let tiled = result_tiled[i];
            let diff = (orig - tiled).abs();
            let rel_error = if orig.abs() > 1e-3 {
                diff / orig.abs()
            } else {
                diff
            };

            max_diff = max_diff.max(diff);
            max_rel_error = max_rel_error.max(rel_error);

            assert!(
                rel_error < 0.01, // 1% tolerance for accumulated floating-point errors
                "Mismatch at index {}: original={}, tiled={}, diff={}, rel_error={}",
                i,
                orig,
                tiled,
                diff,
                rel_error
            );
        }

        println!("✅ Tiled MatMul correctness validated (512x512)");
        println!("   Max difference (sampled): {}", max_diff);
        println!("   Max relative error: {:.4}%", max_rel_error * 100.0);
    })
    .await;
}

#[tokio::test]
async fn test_tiled_matmul_large() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Large test (2048x2048 x 2048x2048) - ultimate memory optimization test
        let m = 2048;
        let k = 2048;
        let n = 2048;

        let a: Vec<f32> = (0..m * k).map(|i| ((i % 10000) as f32) * 0.0001).collect();
        let b: Vec<f32> = (0..k * n)
            .map(|i| (((i + 1) % 10000) as f32) * 0.0001)
            .collect();

        // Execute tiled implementation (original would be too slow to compare)
        let result_tiled = executor
            .execute_matmul_tiled(&a, &b, m, k, n)
            .await
            .unwrap();

        // Verify result size
        assert_eq!(result_tiled.len(), m * n);

        // Verify results are finite and reasonable
        for &val in result_tiled.iter().take(1000) {
            assert!(val.is_finite(), "Result should be finite");
            assert!(val.abs() < 1000.0, "Result should be in reasonable range");
        }

        println!("✅ Tiled MatMul handles large matrices (2048x2048)");
        println!("   Output size: {} elements", result_tiled.len());
    })
    .await;
}
