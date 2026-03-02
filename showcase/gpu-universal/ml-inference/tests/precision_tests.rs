// Comprehensive fp32 precision validation tests
// Tests numerical accuracy at single-precision (fp32) for all operations

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu_executor::{BinaryOp, NormConfig, ReduceOp, WgpuExecutor};

/// fp32 has ~7 decimal digits of precision
/// We use 1e-5 tolerance (5 decimal places) to account for:
/// - Floating point rounding
/// - GPU computation differences
/// - Accumulation errors
const FP32_TOLERANCE: f32 = 1e-5;

/// For operations with many accumulations, use relaxed tolerance
const FP32_TOLERANCE_RELAXED: f32 = 1e-4;

// ============================================================================
// Core Operations - Precision Tests
// ============================================================================

#[tokio::test]
async fn test_relu_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test with known values
        let input = vec![-2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0];
        let expected = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.5, 2.0];

        let result = executor.execute_relu(&input).await.unwrap();

        for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            let error = (out - exp).abs();
            assert!(
            error < FP32_TOLERANCE,
            "ReLU precision error at index {}: got {}, expected {}, error = {} (tolerance = {})",
            i,
            out,
            exp,
            error,
            FP32_TOLERANCE
        );
        }
    })
    .await;
}

#[tokio::test]
async fn test_matmul_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // 2x3 × 3x2 = 2x2
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0];

        // Expected result (computed with exact arithmetic):
        // [1*7+2*9+3*11, 1*8+2*10+3*12] = [58, 64]
        // [4*7+5*9+6*11, 4*8+5*10+6*12] = [139, 154]
        let expected = vec![58.0, 64.0, 139.0, 154.0];

        // MatMul(m, n, k): A[m, k] @ B[k, n] = C[m, n]
        // A is [2, 3], B is [3, 2], result is [2, 2]
        // So: m=2, n=2, k=3
        let result = executor.execute_matmul(&a, &b, 2, 2, 3).await.unwrap();

        for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            let error = (out - exp).abs();
            assert!(
            error < FP32_TOLERANCE_RELAXED,
            "MatMul precision error at index {}: got {}, expected {}, error = {} (tolerance = {})",
            i,
            out,
            exp,
            error,
            FP32_TOLERANCE_RELAXED
        );
        }
    })
    .await;
}

#[tokio::test]
async fn test_elementwise_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test addition with fractional values
        let a = vec![1.1, 2.2, 3.3, 4.4, 5.5];
        let b = vec![0.9, 1.8, 2.7, 3.6, 4.5];
        let expected = vec![2.0, 4.0, 6.0, 8.0, 10.0];

        let result = executor
            .execute_elementwise_binary(&a, &b, BinaryOp::Add)
            .await
            .unwrap();

        for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            let error = (out - exp).abs();
            assert!(
                error < FP32_TOLERANCE,
                "Add precision error at index {}: got {}, expected {}, error = {}",
                i,
                out,
                exp,
                error
            );
        }

        // Test multiplication
        let a = vec![1.5, 2.5, 3.5, 4.5];
        let b = vec![2.0, 3.0, 4.0, 5.0];
        let expected = vec![3.0, 7.5, 14.0, 22.5];

        let result = executor
            .execute_elementwise_binary(&a, &b, BinaryOp::Mul)
            .await
            .unwrap();

        for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            let error = (out - exp).abs();
            assert!(
                error < FP32_TOLERANCE,
                "Mul precision error at index {}: got {}, expected {}, error = {}",
                i,
                out,
                exp,
                error
            );
        }
    })
    .await;
}

#[tokio::test]
async fn test_reduce_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test sum with known result
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let expected_sum = 55.0;
        let expected_mean = 5.5;
        let expected_max = 10.0;

        let sum = executor
            .execute_reduce(&input, ReduceOp::Sum)
            .await
            .unwrap();
        let mean = executor
            .execute_reduce(&input, ReduceOp::Mean)
            .await
            .unwrap();
        let max = executor
            .execute_reduce(&input, ReduceOp::Max)
            .await
            .unwrap();

        assert!(
            (sum - expected_sum).abs() < FP32_TOLERANCE_RELAXED,
            "Sum precision error: got {}, expected {}",
            sum,
            expected_sum
        );
        assert!(
            (mean - expected_mean).abs() < FP32_TOLERANCE_RELAXED,
            "Mean precision error: got {}, expected {}",
            mean,
            expected_mean
        );
        assert!(
            (max - expected_max).abs() < FP32_TOLERANCE,
            "Max precision error: got {}, expected {}",
            max,
            expected_max
        );
    })
    .await;
}

#[tokio::test]
async fn test_softmax_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Simple case: [1, 2, 3]
        let input = vec![1.0, 2.0, 3.0];

        // Expected (computed with higher precision):
        // exp(1) / (exp(1) + exp(2) + exp(3)) ≈ 0.09003057
        // exp(2) / (exp(1) + exp(2) + exp(3)) ≈ 0.24472847
        // exp(3) / (exp(1) + exp(2) + exp(3)) ≈ 0.66524096
        let expected = vec![0.09003057, 0.24472847, 0.66524096];

        let result = executor.execute_softmax(&input).await.unwrap();

        // Verify probabilities sum to 1
        let sum: f32 = result.iter().sum();
        assert!(
            (sum - 1.0).abs() < FP32_TOLERANCE,
            "Softmax probabilities should sum to 1.0, got {}",
            sum
        );

        // Verify individual values
        for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            let error = (out - exp).abs();
            assert!(
                error < FP32_TOLERANCE_RELAXED,
                "Softmax precision error at index {}: got {}, expected {}, error = {}",
                i,
                out,
                exp,
                error
            );
        }
    })
    .await;
}

#[tokio::test]
async fn test_layernorm_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test with [1, 2, 3, 4, 5]
        // Mean = 3.0, Variance = 2.0, Std = sqrt(2.0) ≈ 1.414
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: None,
            beta: None,
        };

        let result = executor.execute_layernorm(&input, config).await.unwrap();

        // Verify mean is close to 0
        let mean: f32 = result.iter().sum::<f32>() / result.len() as f32;
        assert!(
            mean.abs() < FP32_TOLERANCE_RELAXED,
            "LayerNorm mean should be ~0, got {}",
            mean
        );

        // Verify variance is close to 1
        let variance: f32 = result.iter().map(|&x| x * x).sum::<f32>() / result.len() as f32;
        assert!(
            (variance - 1.0).abs() < FP32_TOLERANCE_RELAXED,
            "LayerNorm variance should be ~1, got {}",
            variance
        );
    })
    .await;
}

// ============================================================================
// Edge Cases - Special Values
// ============================================================================

#[tokio::test]
async fn test_relu_edge_cases() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test special float values
        let input = vec![
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            0.0,
            -0.0,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::MAX,
            -f32::MAX,
        ];

        let result = executor.execute_relu(&input).await.unwrap();

        // ReLU: max(0, x)
        // In WGSL/GPU: max(0.0, NaN) may return 0.0 (implementation-defined NaN handling)
        // This is acceptable per IEEE 754-2008 which allows different NaN propagation in max/min
        // We document this behavior rather than require NaN propagation
        let nan_result = result[0];
        println!(
            "ReLU(NaN) = {} (is_nan: {})",
            nan_result,
            nan_result.is_nan()
        );
        // Accept both NaN propagation OR returning 0.0 for NaN input
        assert!(
            nan_result.is_nan() || nan_result == 0.0,
            "ReLU(NaN) should be either NaN or 0.0 (GPU max behavior)"
        );

        assert_eq!(result[1], f32::INFINITY, "+Inf should remain +Inf");
        assert_eq!(result[2], 0.0, "-Inf should become 0");
        assert_eq!(result[3], 0.0, "0 should remain 0");
        assert_eq!(result[4], 0.0, "-0 should become 0");
        assert_eq!(
            result[5],
            f32::MIN_POSITIVE,
            "MIN_POSITIVE should pass through"
        );
        assert_eq!(result[6], 0.0, "-MIN_POSITIVE should become 0");
        assert_eq!(result[7], f32::MAX, "MAX should pass through");
        assert_eq!(result[8], 0.0, "-MAX should become 0");
    })
    .await;
}

#[tokio::test]
async fn test_elementwise_nan_handling() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // NaN propagation
        let a = vec![1.0, f32::NAN, 3.0];
        let b = vec![2.0, 2.0, 2.0];

        let result = executor
            .execute_elementwise_binary(&a, &b, BinaryOp::Add)
            .await
            .unwrap();

        assert_eq!(result[0], 3.0);
        assert!(result[1].is_nan(), "NaN should propagate");
        assert_eq!(result[2], 5.0);
    })
    .await;
}

#[tokio::test]
async fn test_reduce_with_infinities() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test max with infinity
        let input = vec![1.0, 2.0, f32::INFINITY, 3.0];
        let max = executor
            .execute_reduce(&input, ReduceOp::Max)
            .await
            .unwrap();
        assert_eq!(max, f32::INFINITY);

        // Test sum with infinity
        let sum = executor
            .execute_reduce(&input, ReduceOp::Sum)
            .await
            .unwrap();
        assert_eq!(sum, f32::INFINITY);
    })
    .await;
}

// ============================================================================
// Boundary Conditions - Size Limits
// ============================================================================

#[tokio::test]
#[should_panic(expected = "binding size is zero")]
async fn test_empty_array_handling() {
    let executor = WgpuExecutor::new().await.unwrap();

    let empty: Vec<f32> = vec![];

    // wgpu doesn't allow zero-size buffers (Validation Error: Buffer binding size is zero)
    // This is expected behavior - wgpu panics on zero-size buffer creation
    // In production, validate input sizes before calling GPU operations
    let _result = executor.execute_relu(&empty).await;
}

#[tokio::test]
async fn test_single_element() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![42.0];

        let result = executor.execute_relu(&input).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 42.0);

        let result = executor.execute_softmax(&input).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!((result[0] - 1.0).abs() < FP32_TOLERANCE);
    })
    .await;
}

#[tokio::test]
async fn test_large_arrays() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test with 1 million elements
        let size = 1_000_000;
        let input = vec![1.0; size];

        let result = executor.execute_relu(&input).await.unwrap();
        assert_eq!(result.len(), size);
        assert!(result.iter().all(|&x| x == 1.0));
    })
    .await;
}

#[tokio::test]
async fn test_power_of_2_sizes() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test various power-of-2 sizes
        let sizes = vec![1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

        for size in sizes {
            let input = vec![1.0; size];
            let result = executor.execute_relu(&input).await.unwrap();
            assert_eq!(result.len(), size, "Failed at size {}", size);
        }
    })
    .await;
}

#[tokio::test]
async fn test_non_power_of_2_sizes() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test various non-power-of-2 sizes (common in ML)
        let sizes = vec![3, 5, 7, 10, 13, 17, 100, 127, 255, 1000, 1023, 10000];

        for size in sizes {
            let input = vec![1.0; size];
            let result = executor.execute_relu(&input).await.unwrap();
            assert_eq!(result.len(), size, "Failed at size {}", size);
        }
    })
    .await;
}

// ============================================================================
// Accumulation Error Tests
// ============================================================================

#[tokio::test]
async fn test_reduce_accumulation_error() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Large number of small values - tests accumulation precision
        let size = 10_000;
        let input = vec![0.0001; size];
        let expected_sum = size as f32 * 0.0001;

        let sum = executor
            .execute_reduce(&input, ReduceOp::Sum)
            .await
            .unwrap();

        // Accumulation error should be small
        let relative_error = ((sum - expected_sum) / expected_sum).abs();
        assert!(
            relative_error < 0.01, // 1% relative error
            "Accumulation error too large: got {}, expected {}, relative error = {}",
            sum,
            expected_sum,
            relative_error
        );
    })
    .await;
}

#[tokio::test]
async fn test_matmul_accumulation_error() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Large matrix multiply - tests accumulation
        let n = 128;
        let a = vec![0.01; n * n];
        let b = vec![0.01; n * n];

        // Expected: each element should be n * 0.01 * 0.01 = n * 0.0001
        let expected_value = n as f32 * 0.0001;

        let result = executor.execute_matmul(&a, &b, n, n, n).await.unwrap();

        for (i, &val) in result.iter().enumerate() {
            let relative_error = ((val - expected_value) / expected_value).abs();
            assert!(
                relative_error < 0.05, // 5% relative error
                "MatMul accumulation error at index {}: got {}, expected {}, relative error = {}",
                i,
                val,
                expected_value,
                relative_error
            );
        }
    })
    .await;
}

// ============================================================================
// Numerical Stability Tests
// ============================================================================

#[tokio::test]
async fn test_softmax_numerical_stability() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Large values that could cause overflow without max subtraction
        let input = vec![1000.0, 1001.0, 1002.0];

        let result = executor.execute_softmax(&input).await.unwrap();

        // Should not produce NaN or Inf
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "Softmax should remain finite even with large inputs"
        );

        // Should still sum to 1
        let sum: f32 = result.iter().sum();
        assert!(
            (sum - 1.0).abs() < FP32_TOLERANCE,
            "Softmax probabilities should sum to 1.0, got {}",
            sum
        );
    })
    .await;
}

#[tokio::test]
async fn test_layernorm_numerical_stability() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Large values
        let input = vec![1000.0, 2000.0, 3000.0, 4000.0];
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: None,
            beta: None,
        };

        let result = executor.execute_layernorm(&input, config).await.unwrap();

        // Should normalize correctly without overflow
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "LayerNorm should handle large values without overflow"
        );

        // Mean should still be ~0
        let mean: f32 = result.iter().sum::<f32>() / result.len() as f32;
        assert!(
            mean.abs() < FP32_TOLERANCE_RELAXED,
            "LayerNorm mean should be ~0 even with large inputs, got {}",
            mean
        );
    })
    .await;
}
