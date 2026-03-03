// SPDX-License-Identifier: AGPL-3.0-or-later
// Chaos Testing - Random inputs, extreme values, edge cases
// Tests system resilience under unpredictable conditions

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu_executor::{BinaryOp, NormConfig, ReduceOp, WgpuExecutor};
use rand::Rng;

// ============================================================================
// Random Input Testing
// ============================================================================

#[tokio::test]
async fn test_relu_random_inputs() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();
        let mut rng = rand::thread_rng();

        // 100 random test cases
        for _ in 0..100 {
            let size = rng.gen_range(1..1000);
            let input: Vec<f32> = (0..size).map(|_| rng.gen_range(-100.0..100.0)).collect();

            let result = executor.execute_relu(&input).await.unwrap();

            // Verify all outputs are non-negative
            assert!(
                result.iter().all(|&x| x >= 0.0 || x.is_nan()),
                "ReLU should produce non-negative outputs"
            );

            // Verify size unchanged
            assert_eq!(result.len(), input.len());
        }
    })
    .await;
}

#[tokio::test]
async fn test_elementwise_random_operations() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();
        let mut rng = rand::thread_rng();

        for _ in 0..50 {
            let size = rng.gen_range(1..500);
            let a: Vec<f32> = (0..size).map(|_| rng.gen_range(-10.0..10.0)).collect();
            let b: Vec<f32> = (0..size).map(|_| rng.gen_range(-10.0..10.0)).collect();

            // Test Add
            let result = executor
                .execute_elementwise_binary(&a, &b, BinaryOp::Add)
                .await
                .unwrap();
            assert_eq!(result.len(), size);
            assert!(result.iter().all(|&x| x.is_finite() || x.is_nan()));

            // Test Mul
            let result = executor
                .execute_elementwise_binary(&a, &b, BinaryOp::Mul)
                .await
                .unwrap();
            assert_eq!(result.len(), size);
        }
    })
    .await;
}

#[tokio::test]
async fn test_matmul_random_dimensions() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();
        let mut rng = rand::thread_rng();

        for _ in 0..20 {
            // Random dimensions (keep small for speed)
            let m = rng.gen_range(1..16);
            let k = rng.gen_range(1..16);
            let n = rng.gen_range(1..16);

            let a: Vec<f32> = (0..m * k).map(|_| rng.gen_range(-1.0..1.0)).collect();
            let b: Vec<f32> = (0..k * n).map(|_| rng.gen_range(-1.0..1.0)).collect();

            // MatMul signature: (m, n, k) where A is [m, k], B is [k, n], result is [m, n]
            let result = executor.execute_matmul(&a, &b, m, n, k).await.unwrap();
            assert_eq!(result.len(), m * n);
        }
    })
    .await;
}

// ============================================================================
// Extreme Values
// ============================================================================

#[tokio::test]
async fn test_extreme_positive_values() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Very large positive values (but not so large they overflow when summed)
        let input = vec![1e6; 100]; // 1 million, safe to sum 100 of them

        let result = executor.execute_relu(&input).await.unwrap();
        assert!(result.iter().all(|&x| x > 0.0));

        let mean = executor
            .execute_reduce(&input, ReduceOp::Mean)
            .await
            .unwrap();
        assert!(
            mean > 0.0 && mean.is_finite(),
            "Mean should be finite, got {}",
            mean
        );
        assert!(
            (mean - 1e6).abs() < 1.0,
            "Mean should be close to 1e6, got {}",
            mean
        );
    })
    .await;
}

#[tokio::test]
async fn test_extreme_negative_values() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Very large negative values
        let input = vec![-f32::MAX / 2.0; 100];

        let result = executor.execute_relu(&input).await.unwrap();
        assert!(result.iter().all(|&x| x == 0.0));
    })
    .await;
}

#[tokio::test]
async fn test_tiny_values() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Very small values (near zero)
        let input = vec![f32::MIN_POSITIVE; 100];

        let result = executor.execute_relu(&input).await.unwrap();
        assert_eq!(result.len(), 100);

        let sum = executor
            .execute_reduce(&input, ReduceOp::Sum)
            .await
            .unwrap();
        assert!(sum > 0.0 && sum.is_finite());
    })
    .await;
}

#[tokio::test]
async fn test_mixed_extreme_values() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Mix of extreme values
        let input = vec![
            f32::MAX / 2.0,
            -f32::MAX / 2.0,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            1000.0,
            -1000.0,
            0.0,
            -0.0,
        ];

        let result = executor.execute_relu(&input).await.unwrap();
        assert!(result[0] > 0.0); // Large positive stays
        assert_eq!(result[1], 0.0); // Large negative becomes 0
        assert!(result[2] > 0.0); // Tiny positive stays
        assert_eq!(result[3], 0.0); // Tiny negative becomes 0
    })
    .await;
}

// ============================================================================
// Numerical Stress Tests
// ============================================================================

#[tokio::test]
async fn test_softmax_extreme_logits() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Extreme logits (should handle via max subtraction)
        let input = vec![1000.0, 1001.0, 999.0];

        let result = executor.execute_softmax(&input).await.unwrap();

        // Should not overflow to NaN or Inf
        assert!(result.iter().all(|&x| x.is_finite() && x >= 0.0));

        // Should sum to 1
        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
    })
    .await;
}

#[tokio::test]
async fn test_layernorm_constant_values() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // All same values (zero variance edge case)
        let input = vec![5.0; 100];
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: None,
            beta: None,
        };

        let result = executor.execute_layernorm(&input, config).await.unwrap();

        // With zero variance, should return all zeros (or near-zero with epsilon)
        assert!(
            result.iter().all(|&x| x.abs() < 0.1),
            "Constant input should produce near-zero output"
        );
    })
    .await;
}

#[tokio::test]
async fn test_reduce_alternating_signs() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Alternating large positive/negative (tests cancellation)
        let input: Vec<f32> = (0..1000)
            .map(|i| if i % 2 == 0 { 1000.0 } else { -1000.0 })
            .collect();

        let sum = executor
            .execute_reduce(&input, ReduceOp::Sum)
            .await
            .unwrap();

        // Should be near zero (or exactly zero)
        assert!(
            sum.abs() < 10.0,
            "Alternating sum should be near zero, got {}",
            sum
        );
    })
    .await;
}

// ============================================================================
// Size Chaos
// ============================================================================

#[tokio::test]
async fn test_prime_number_sizes() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Prime numbers are never power-of-2, test edge cases
        let primes = vec![
            3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73,
        ];

        for &prime in &primes {
            let input = vec![1.0; prime];
            let result = executor.execute_relu(&input).await.unwrap();
            assert_eq!(result.len(), prime);
        }
    })
    .await;
}

#[tokio::test]
async fn test_odd_sizes() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();
        let mut rng = rand::thread_rng();

        // Random odd sizes
        for _ in 0..20 {
            let size = rng.gen_range(1..1000) * 2 + 1; // Ensure odd
            let input = vec![1.0; size];
            let result = executor.execute_relu(&input).await.unwrap();
            assert_eq!(result.len(), size);
        }
    })
    .await;
}

// ============================================================================
// Operation Composition Chaos
// ============================================================================

#[tokio::test]
async fn test_random_operation_chain() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();
        let mut rng = rand::thread_rng();

        // Start with random data
        let mut data: Vec<f32> = (0..100).map(|_| rng.gen_range(-10.0..10.0)).collect();

        // Apply 10 random operations
        for _ in 0..10 {
            let op = rng.gen_range(0..4);

            data = match op {
                0 => executor.execute_relu(&data).await.unwrap(),
                1 => executor.execute_sigmoid(&data).await.unwrap(),
                2 => executor.execute_tanh(&data).await.unwrap(),
                3 => {
                    // Normalize if variance exists
                    let mean: f32 = data.iter().sum::<f32>() / data.len() as f32;
                    let variance: f32 =
                        data.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / data.len() as f32;

                    if variance > 1e-6 {
                        let config = NormConfig {
                            epsilon: 1e-5,
                            gamma: None,
                            beta: None,
                        };
                        executor.execute_layernorm(&data, config).await.unwrap()
                    } else {
                        data // Skip if no variance
                    }
                }
                _ => unreachable!(),
            };

            // Verify output is valid
            assert_eq!(data.len(), 100);
            assert!(data.iter().all(|&x| x.is_finite() || x.is_nan()));
        }

        println!("Random operation chain completed successfully");
    })
    .await;
}

// ============================================================================
// Sequential Chaos (simulating concurrent-like workload)
// ============================================================================

#[tokio::test]
async fn test_sequential_varied_operations() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Run multiple varied operations sequentially
        // (true concurrent execution would require Arc<WgpuExecutor> + Mutex or similar)
        for i in 0..10 {
            let size = (i + 1) * 10;
            let input = vec![i as f32; size];
            let result = executor.execute_relu(&input).await.unwrap();
            assert_eq!(result.len(), size);
        }

        println!("Sequential varied operations completed successfully");
    })
    .await;
}
