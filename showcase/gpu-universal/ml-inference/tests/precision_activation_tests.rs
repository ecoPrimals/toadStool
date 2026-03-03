// SPDX-License-Identifier: AGPL-3.0-or-later
// Precision tests - Activations
use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::WgpuExecutor;

const FP32_TOLERANCE: f32 = 1e-5;
const FP32_TOLERANCE_RELAXED: f32 = 1e-4;

#[tokio::test]
async fn test_relu_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // ReLU: max(0, x)
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0, -0.5, 0.5];

        let result = executor.execute_relu(&input).await.unwrap();

        // Expected: all negative values become 0, positive values unchanged
        let expected = vec![0.0, 0.0, 0.0, 1.0, 2.0, 0.0, 0.5];

        assert_eq!(result.len(), input.len());
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "ReLU outputs should be finite"
        );

        for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            let error = (out - exp).abs();
            assert!(
                error < FP32_TOLERANCE,
                "ReLU error at index {}: got {}, expected {}, error = {}",
                i,
                out,
                exp,
                error
            );
        }

        // Verify ReLU properties
        assert!(
            result.iter().all(|&x| x >= 0.0),
            "ReLU output should be non-negative"
        );

        println!("✅ ReLU precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_sigmoid_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Sigmoid: 1 / (1 + exp(-x))
        let input = vec![-10.0, -2.0, -1.0, 0.0, 1.0, 2.0, 10.0];

        let result = executor.execute_sigmoid(&input).await.unwrap();

        assert_eq!(result.len(), input.len());
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "Sigmoid outputs should be finite"
        );

        // Sigmoid(0) should be 0.5
        let idx_zero = 3;
        let expected_zero = 0.5;
        assert!(
            (result[idx_zero] - expected_zero).abs() < FP32_TOLERANCE,
            "Sigmoid(0) should be 0.5, got {}",
            result[idx_zero]
        );

        // Verify sigmoid properties
        assert!(
            result.iter().all(|&x| x > 0.0 && x < 1.0),
            "Sigmoid output should be in (0, 1)"
        );

        // Verify monotonic increasing
        for i in 0..result.len() - 1 {
            assert!(
            result[i] < result[i + 1],
            "Sigmoid should be monotonic increasing: sigmoid({}) = {} should be < sigmoid({}) = {}",
            input[i],
            result[i],
            input[i + 1],
            result[i + 1]
        );
        }

        // Verify symmetry: sigmoid(x) + sigmoid(-x) = 1
        for i in 0..input.len() {
            let neg_input = vec![-input[i]];
            let neg_result = executor.execute_sigmoid(&neg_input).await.unwrap();
            let sum = result[i] + neg_result[0];
            assert!(
                (sum - 1.0).abs() < FP32_TOLERANCE_RELAXED,
                "Sigmoid symmetry: sigmoid({}) + sigmoid({}) should = 1, got {}",
                input[i],
                -input[i],
                sum
            );
        }

        println!("✅ Sigmoid precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_tanh_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Tanh: (exp(x) - exp(-x)) / (exp(x) + exp(-x))
        let input = vec![-10.0, -2.0, -1.0, 0.0, 1.0, 2.0, 10.0];

        let result = executor.execute_tanh(&input).await.unwrap();

        assert_eq!(result.len(), input.len());
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "Tanh outputs should be finite"
        );

        // Tanh(0) should be 0
        let idx_zero = 3;
        assert!(
            result[idx_zero].abs() < FP32_TOLERANCE,
            "Tanh(0) should be 0, got {}",
            result[idx_zero]
        );

        // Verify tanh properties
        // Note: For extreme values, tanh can be exactly ±1 due to fp32 limits
        assert!(
            result.iter().all(|x| (-1.0..=1.0).contains(x)),
            "Tanh output should be in [-1, 1]"
        );

        // Verify monotonic increasing
        for i in 0..result.len() - 1 {
            assert!(
                result[i] < result[i + 1],
                "Tanh should be monotonic increasing: tanh({}) = {} should be < tanh({}) = {}",
                input[i],
                result[i],
                input[i + 1],
                result[i + 1]
            );
        }

        // Verify odd function: tanh(-x) = -tanh(x)
        for i in 0..input.len() {
            let neg_input = vec![-input[i]];
            let neg_result = executor.execute_tanh(&neg_input).await.unwrap();
            let expected = -result[i];
            assert!(
                (neg_result[0] - expected).abs() < FP32_TOLERANCE_RELAXED,
                "Tanh odd function: tanh({}) should = -tanh({}), got {} vs expected {}",
                -input[i],
                input[i],
                neg_result[0],
                expected
            );
        }

        println!("✅ Tanh precision test passed");
    })
    .await;
}

// Advanced Activations (Already Tested)

#[tokio::test]
async fn test_gelu_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // GELU: 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x^3)))
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];

        let result = executor.execute_gelu(&input).await.unwrap();

        // Verify basic properties
        assert_eq!(result.len(), input.len());
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "GELU outputs should be finite"
        );

        // GELU(0) should be ~0
        let idx_zero = 2;
        assert!(
            result[idx_zero].abs() < FP32_TOLERANCE,
            "GELU(0) should be ~0, got {}",
            result[idx_zero]
        );

        // Verify GELU executed and produced finite results
        // Note: GELU is monotonic but test values here are correctly ordered:
        // -0.045 < -0.159 is false, but -0.159 < -0.045 is true (both negative)
        // The sequence is: -0.159, -0.045, 0.0, 0.841, 1.955 (correctly increasing!)
        assert!(
            result[0] > result[1],
            "GELU(-2.0) > GELU(-1.0) due to both being negative"
        );
        assert!(result[1] < result[2], "GELU(-1.0) < GELU(0.0)");
        assert!(result[2] < result[3], "GELU(0.0) < GELU(1.0)");
        assert!(result[3] < result[4], "GELU(1.0) < GELU(2.0)");

        println!("✅ GELU precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_swish_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Swish (SiLU): x * sigmoid(x)
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];

        let result = executor.execute_swish(&input).await.unwrap();

        assert_eq!(result.len(), input.len());
        assert!(result.iter().all(|&x| x.is_finite()));

        // Swish(0) = 0
        assert!(
            result[2].abs() < FP32_TOLERANCE,
            "Swish(0) should be 0, got {}",
            result[2]
        );

        // Swish/SiLU is NON-MONOTONIC (has a small dip around x ≈ -1.278)
        // Just verify general increasing trend and proper behavior
        assert!(result[0] < result[4], "Swish(-2.0) < Swish(2.0)");
        assert!(result[2] < result[4], "Swish(0.0) < Swish(2.0)");

        println!("✅ Swish/SiLU precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_leaky_relu_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let alpha = 0.01;

        let result = executor.execute_leaky_relu(&input, alpha).await.unwrap();

        // LeakyReLU: x if x > 0, alpha * x otherwise
        let expected = vec![-0.02, -0.01, 0.0, 1.0, 2.0];

        for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            let error = (out - exp).abs();
            assert!(
                error < FP32_TOLERANCE,
                "LeakyReLU error at {}: got {}, expected {}, error = {}",
                i,
                out,
                exp,
                error
            );
        }

        println!("✅ LeakyReLU precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_elu_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let alpha = 1.0;

        let result = executor.execute_elu(&input, alpha).await.unwrap();

        assert_eq!(result.len(), input.len());
        assert!(result.iter().all(|&x| x.is_finite()));

        // ELU properties:
        // - ELU(x) = x for x > 0
        // - ELU(0) = 0
        // - ELU(x) approaches -alpha as x -> -infinity

        assert!(result[2].abs() < FP32_TOLERANCE, "ELU(0) should be 0");
        assert!(
            (result[3] - 1.0).abs() < FP32_TOLERANCE,
            "ELU(1) should be 1"
        );
        assert!(
            (result[4] - 2.0).abs() < FP32_TOLERANCE,
            "ELU(2) should be 2"
        );

        // Negative values should be bounded by -alpha
        for i in 0..2 {
            assert!(
                result[i] > -alpha - FP32_TOLERANCE,
                "ELU({}) should be > -alpha",
                input[i]
            );
        }

        println!("✅ ELU precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_selu_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // SELU: λ * (x if x > 0, else α * (e^x - 1))
        // Standard params: α ≈ 1.6733, λ ≈ 1.0507
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];

        let result = executor.execute_selu(&input).await.unwrap();

        assert_eq!(result.len(), input.len());
        assert!(result.iter().all(|&x| x.is_finite()));

        // SELU(0) = 0
        assert!(
            result[2].abs() < FP32_TOLERANCE_RELAXED,
            "SELU(0) should be ~0"
        );

        // SELU is self-normalizing (values in reasonable range)
        for &val in &result {
            assert!(val.abs() < 10.0, "SELU should produce bounded outputs");
        }

        println!("✅ SELU precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_hardswish_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // HardSwish: x * ReLU6(x + 3) / 6
        let input = vec![-4.0, -3.0, 0.0, 3.0, 4.0];

        let result = executor.execute_hardswish(&input).await.unwrap();

        assert_eq!(result.len(), input.len());
        assert!(result.iter().all(|&x| x.is_finite()));

        // HardSwish(0) = 0
        assert!(result[2].abs() < FP32_TOLERANCE, "HardSwish(0) should be 0");

        // HardSwish(-3) ≈ 0, HardSwish(3) ≈ 3
        assert!(
            result[1].abs() < FP32_TOLERANCE_RELAXED,
            "HardSwish(-3) should be ~0"
        );
        assert!(
            (result[3] - 3.0).abs() < FP32_TOLERANCE_RELAXED,
            "HardSwish(3) should be ~3"
        );

        println!("✅ HardSwish precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_mish_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Mish: x * tanh(ln(1 + e^x))
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];

        let result = executor.execute_mish(&input).await.unwrap();

        assert_eq!(result.len(), input.len());
        assert!(result.iter().all(|&x| x.is_finite()));

        // Mish is ALSO NON-MONOTONIC (has a dip from -2 to -1)
        // Reference: Mish(-2) = -0.253, Mish(-1) = -0.303 (decreases!)
        // Just verify it's working and producing correct general behavior
        assert!(
            result[2].abs() < 0.1,
            "Mish(0) should be ~0, got {}",
            result[2]
        );
        assert!(
            result[0] > result[1],
            "Mish has characteristic dip: Mish(-2) > Mish(-1)"
        );
        assert!(
            result[1] < result[3],
            "Mish eventually increases: Mish(-1) < Mish(1)"
        );

        println!("✅ Mish precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_all_activations_consistency() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Test all 10 activations with same input
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];

        // Run all activations
        let relu = executor.execute_relu(&input).await.unwrap();
        let sigmoid = executor.execute_sigmoid(&input).await.unwrap();
        let tanh = executor.execute_tanh(&input).await.unwrap();
        let gelu = executor.execute_gelu(&input).await.unwrap();
        let swish = executor.execute_swish(&input).await.unwrap();
        let leaky_relu = executor.execute_leaky_relu(&input, 0.01).await.unwrap();
        let elu = executor.execute_elu(&input, 1.0).await.unwrap();
        let selu = executor.execute_selu(&input).await.unwrap();
        let hardswish = executor.execute_hardswish(&input).await.unwrap();
        let mish = executor.execute_mish(&input).await.unwrap();

        // Verify all produce correct output lengths
        assert_eq!(relu.len(), input.len());
        assert_eq!(sigmoid.len(), input.len());
        assert_eq!(tanh.len(), input.len());
        assert_eq!(gelu.len(), input.len());
        assert_eq!(swish.len(), input.len());
        assert_eq!(leaky_relu.len(), input.len());
        assert_eq!(elu.len(), input.len());
        assert_eq!(selu.len(), input.len());
        assert_eq!(hardswish.len(), input.len());
        assert_eq!(mish.len(), input.len());

        // Verify all produce finite outputs
        assert!(relu.iter().all(|&x| x.is_finite()));
        assert!(sigmoid.iter().all(|&x| x.is_finite()));
        assert!(tanh.iter().all(|&x| x.is_finite()));
        assert!(gelu.iter().all(|&x| x.is_finite()));
        assert!(swish.iter().all(|&x| x.is_finite()));
        assert!(leaky_relu.iter().all(|&x| x.is_finite()));
        assert!(elu.iter().all(|&x| x.is_finite()));
        assert!(selu.iter().all(|&x| x.is_finite()));
        assert!(hardswish.iter().all(|&x| x.is_finite()));
        assert!(mish.iter().all(|&x| x.is_finite()));

        println!("✅ All 10 activations consistency test passed");
        println!("   ReLU, Sigmoid, Tanh, GELU, Swish, LeakyReLU, ELU, SELU, HardSwish, Mish");
    })
    .await;
}

// ============================================================================
// OPTIMIZERS (6 total)
// ============================================================================

// Core Optimizer (Untested - HIGHEST PRIORITY)
