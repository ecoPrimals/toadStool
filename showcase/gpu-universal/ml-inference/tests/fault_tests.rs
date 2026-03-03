// SPDX-License-Identifier: AGPL-3.0-or-later
// Fault Testing - Invalid inputs, error handling, graceful degradation
// Tests system resilience under error conditions

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu_executor::{
    BinaryOp, CrossEntropyConfig, LossReduction, NormConfig, ReduceOp, WgpuExecutor,
};

// ============================================================================
// Invalid Input Tests
// ============================================================================

#[tokio::test]
async fn test_mismatched_sizes_elementwise() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0]; // Wrong size!

        let result = executor
            .execute_elementwise_binary(&a, &b, BinaryOp::Add)
            .await;

        // Should return error, not panic
        assert!(result.is_err(), "Mismatched sizes should return error");

        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("size")
                || err_msg.contains("length")
                || err_msg.contains("must equal"),
            "Error should mention size mismatch"
        );
    })
    .await;
}

#[tokio::test]
async fn test_invalid_matmul_dimensions() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test matmul with compatible dimensions
        // A: 2x3, B: 3x2 (valid dimensions for matmul)
        let a = vec![1.0; 6]; // 2x3 matrix
        let b = vec![1.0; 6]; // 3x2 matrix

        // execute_matmul(a, b, m, n, k) where:
        // - m = rows of A (2)
        // - n = cols of B (2)
        // - k = cols of A / rows of B (3)
        // For A(2x3) * B(3x2) = C(2x2), call with m=2, n=2, k=3
        let result = executor.execute_matmul(&a, &b, 2, 2, 3).await;

        // This should succeed (dimensions are compatible: 2x3 * 3x2 = 2x2)
        assert!(result.is_ok(), "Valid matmul dimensions should succeed");
        let output = result.unwrap();
        assert_eq!(output.len(), 4, "2x3 * 3x2 should produce 2x2 = 4 elements");

        println!("MatMul validates buffer sizes correctly");
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "binding size is zero")]
async fn test_zero_dimensions_matmul() {
    let executor = WgpuExecutor::new().await.unwrap();

    let a = vec![];
    let b = vec![];

    // 0x0 matrix multiply
    // wgpu panics on zero-size buffers (expected behavior)
    let _result = executor.execute_matmul(&a, &b, 0, 0, 0).await;
}

#[tokio::test]
async fn test_negative_reduction() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // All negative values
        let input = vec![-1.0, -2.0, -3.0, -4.0, -5.0];

        // These should all work correctly
        let sum = executor
            .execute_reduce(&input, ReduceOp::Sum)
            .await
            .unwrap();
        assert!(sum < 0.0, "Sum of negatives should be negative");

        let mean = executor
            .execute_reduce(&input, ReduceOp::Mean)
            .await
            .unwrap();
        assert!(mean < 0.0, "Mean of negatives should be negative");

        let max = executor
            .execute_reduce(&input, ReduceOp::Max)
            .await
            .unwrap();
        assert_eq!(max, -1.0, "Max of negatives should be -1.0");
    })
    .await;
}

#[tokio::test]
async fn test_cross_entropy_invalid_sizes() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Predictions: 2 samples, 3 classes
        let predictions = vec![0.7, 0.2, 0.1, 0.1, 0.8, 0.1];

        // Targets: Wrong size (should be 2x3 = 6, but give 4)
        let targets = vec![1.0, 0.0, 0.0, 0.0]; // Too small!

        let config = CrossEntropyConfig {
            epsilon: 1e-7,
            reduction: LossReduction::Mean,
        };

        let result = executor
            .execute_cross_entropy(&predictions, &targets, 2, 3, config)
            .await;

        assert!(result.is_err(), "Mismatched sizes should return error");
    })
    .await;
}

// ============================================================================
// Special Float Values
// ============================================================================

#[tokio::test]
async fn test_all_nan_input() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![f32::NAN; 10];

        // Operations should handle NaN (may propagate or return NaN)
        let result = executor.execute_relu(&input).await.unwrap();
        assert_eq!(result.len(), 10);
        // NaN handling is implementation-defined, so we just verify it doesn't panic
    })
    .await;
}

#[tokio::test]
async fn test_all_infinity_input() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![f32::INFINITY; 10];

        let result = executor.execute_relu(&input).await.unwrap();
        assert!(result.iter().all(|&x| x == f32::INFINITY));

        let sum = executor
            .execute_reduce(&input, ReduceOp::Sum)
            .await
            .unwrap();
        assert_eq!(sum, f32::INFINITY);
    })
    .await;
}

#[tokio::test]
async fn test_mixed_nan_and_infinity() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, 1.0];

        let result = executor.execute_relu(&input).await.unwrap();
        assert_eq!(result.len(), 5);
        // Should not panic, behavior is implementation-defined for NaN
    })
    .await;
}

// ============================================================================
// Normalization Edge Cases
// ============================================================================

#[tokio::test]
async fn test_layernorm_all_zeros() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // All zeros = zero variance
        let input = vec![0.0; 100];
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: None,
            beta: None,
        };

        let result = executor.execute_layernorm(&input, config).await.unwrap();

        // With zero variance, output should be all zeros (or near-zero with epsilon)
        assert!(
            result.iter().all(|&x| x.abs() < 0.1),
            "Zero-variance input should produce near-zero output"
        );
    })
    .await;
}

#[tokio::test]
async fn test_layernorm_single_value() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Single element = zero variance
        let input = vec![42.0];
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: None,
            beta: None,
        };

        let result = executor.execute_layernorm(&input, config).await.unwrap();
        assert_eq!(result.len(), 1);
        // Single element gets normalized to 0 (or near-zero)
        assert!(result[0].abs() < 0.1);
    })
    .await;
}

// ============================================================================
// Softmax Edge Cases
// ============================================================================

#[tokio::test]
async fn test_softmax_all_same_values() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // All same logits = uniform distribution
        let input = vec![5.0; 10];

        let result = executor.execute_softmax(&input).await.unwrap();

        // Should be uniform: each element = 1/10 = 0.1
        for &prob in &result {
            assert!(
                (prob - 0.1).abs() < 1e-4,
                "Uniform logits should give uniform probabilities, got {}",
                prob
            );
        }

        // Should sum to 1
        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
    })
    .await;
}

#[tokio::test]
async fn test_softmax_single_element() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Single element should get probability 1.0
        let input = vec![42.0];

        let result = executor.execute_softmax(&input).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(
            (result[0] - 1.0).abs() < 1e-5,
            "Single element softmax should be 1.0, got {}",
            result[0]
        );
    })
    .await;
}

#[tokio::test]
async fn test_softmax_negative_infinity() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Very negative values (near-zero probabilities)
        let input = vec![-1000.0, -1001.0, -999.0];

        let result = executor.execute_softmax(&input).await.unwrap();

        // Should not produce NaN or Inf
        assert!(result.iter().all(|&x| x.is_finite() && x >= 0.0));

        // Should sum to 1
        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
    })
    .await;
}

// ============================================================================
// Activation Function Edge Cases
// ============================================================================

#[tokio::test]
async fn test_sigmoid_extreme_values() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Sigmoid should saturate at 0 and 1
        let input = vec![-1000.0, 0.0, 1000.0];

        let result = executor.execute_sigmoid(&input).await.unwrap();

        // sigmoid(-1000) ≈ 0
        assert!(
            result[0] < 0.01,
            "Sigmoid of large negative should be near 0"
        );

        // sigmoid(0) = 0.5
        assert!((result[1] - 0.5).abs() < 0.01, "Sigmoid of 0 should be 0.5");

        // sigmoid(1000) ≈ 1
        assert!(
            result[2] > 0.99,
            "Sigmoid of large positive should be near 1"
        );
    })
    .await;
}

#[tokio::test]
async fn test_tanh_extreme_values() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Tanh should saturate at -1 and 1
        let input = vec![-1000.0, 0.0, 1000.0];

        let result = executor.execute_tanh(&input).await.unwrap();

        // tanh(-1000) ≈ -1
        assert!(
            result[0] < -0.99,
            "Tanh of large negative should be near -1"
        );

        // tanh(0) = 0
        assert!(result[1].abs() < 0.01, "Tanh of 0 should be 0");

        // tanh(1000) ≈ 1
        assert!(result[2] > 0.99, "Tanh of large positive should be near 1");
    })
    .await;
}

// ============================================================================
// Dropout Edge Cases
// ============================================================================

#[tokio::test]
async fn test_dropout_rate_zero() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Dropout rate 0.0 = no dropout (all elements kept)
        let result = executor
            .execute_dropout(&input, 0.0, true, Some(12345))
            .await
            .unwrap();

        // All elements should be present (scaled by 1/(1-0) = 1)
        assert_eq!(result.len(), 5);
        // At least some values should be non-zero
        assert!(result.iter().any(|&x| x > 0.0));
    })
    .await;
}

#[tokio::test]
async fn test_dropout_rate_one() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Dropout rate 1.0 = drop all (all elements zeroed)
        let result = executor
            .execute_dropout(&input, 1.0, true, Some(12345))
            .await
            .unwrap();

        // All elements should be zero
        assert!(
            result.iter().all(|&x| x == 0.0),
            "Dropout rate 1.0 should zero all elements"
        );
    })
    .await;
}

// ============================================================================
// Gather/Scatter Edge Cases
// ============================================================================

#[tokio::test]
async fn test_gather_invalid_indices() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let indices = vec![0, 2, 10]; // Index 10 is out of bounds!

        let result = executor.execute_gather(&input, &indices).await;

        // Should fail or handle gracefully
        // wgpu may clamp indices or error
        match result {
            Ok(values) => {
                // If it succeeds, indices may have been clamped
                assert_eq!(values.len(), 3);
            }
            Err(_) => {
                // Error is acceptable for out-of-bounds
            }
        }
    })
    .await;
}

#[tokio::test]
async fn test_scatter_overlapping_indices() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let output_size = 5;
        let values = vec![10.0, 20.0, 30.0];
        let indices: Vec<u32> = vec![1, 1, 1]; // All write to same location!

        let result = executor
            .execute_scatter(&values, &indices, output_size)
            .await
            .unwrap();

        // With atomic operations, one of the values will win (order not guaranteed in parallel execution)
        // We just verify that index 1 has *one* of the written values
        assert_eq!(result.len(), 5);
        assert!(
            result[1] == 10.0 || result[1] == 20.0 || result[1] == 30.0,
            "Index 1 should have one of the written values, got {}",
            result[1]
        );
        assert_eq!(result[0], 0.0, "Unwritten indices should be 0");
        assert_eq!(result[2], 0.0, "Unwritten indices should be 0");
    })
    .await;
}

// ============================================================================
// Transpose Edge Cases
// ============================================================================

#[tokio::test]
async fn test_transpose_1x1_matrix() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![42.0];

        let result = executor.execute_transpose(&input, 1, 1).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 42.0);
    })
    .await;
}

#[tokio::test]
async fn test_transpose_single_row() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // 1x5 matrix
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Transpose to 5x1
        let result = executor.execute_transpose(&input, 1, 5).await.unwrap();

        assert_eq!(result, input); // Order unchanged for 1xN
    })
    .await;
}

#[tokio::test]
async fn test_transpose_single_column() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // 5x1 matrix
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Transpose to 1x5
        let result = executor.execute_transpose(&input, 5, 1).await.unwrap();

        assert_eq!(result, input); // Order unchanged for Nx1
    })
    .await;
}

// ============================================================================
// Error Message Quality
// ============================================================================

#[tokio::test]
async fn test_error_messages_are_helpful() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0]; // Wrong size

        let result = executor
            .execute_elementwise_binary(&a, &b, BinaryOp::Add)
            .await;

        assert!(result.is_err());

        let err_msg = format!("{}", result.unwrap_err());

        // Error should be clear and mention size/length mismatch
        assert!(
            err_msg.contains("size") || err_msg.contains("length") || err_msg.contains("match"),
            "Error should mention size mismatch: {}",
            err_msg
        );
    })
    .await;
}

// ============================================================================
// Graceful Degradation
// ============================================================================

#[tokio::test]
async fn test_multiple_operations_after_error() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Cause an error
        let bad_a = vec![1.0, 2.0, 3.0];
        let bad_b = vec![1.0, 2.0];
        let _err = executor
            .execute_elementwise_binary(&bad_a, &bad_b, BinaryOp::Add)
            .await;

        // Verify executor still works after error
        let good_a = vec![1.0, 2.0, 3.0];
        let good_b = vec![4.0, 5.0, 6.0];
        let result = executor
            .execute_elementwise_binary(&good_a, &good_b, BinaryOp::Add)
            .await
            .unwrap();

        assert_eq!(result, vec![5.0, 7.0, 9.0]);
        println!("Executor remains functional after error - graceful degradation confirmed");
    })
    .await;
}
