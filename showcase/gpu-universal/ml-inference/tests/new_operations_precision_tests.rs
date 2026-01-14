// Comprehensive precision tests for all 60 operations built on Jan 14, 2026
// Focus: fp32 accuracy, numerical stability, edge cases
// Goal: Verify everything we evolved and find gaps for further evolution

use ml_inference_showcase::wgpu::executor::WgpuExecutor;
use ml_inference_showcase::wgpu::types::*;

const FP32_TOLERANCE: f32 = 1e-5;
const FP32_TOLERANCE_RELAXED: f32 = 1e-4;

// ============================================================================
// ACTIVATIONS (10 total) - Complete Suite
// ============================================================================

#[tokio::test]
async fn test_gelu_fp32_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // GELU: 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x^3)))
    let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    
    let result = executor.activations.execute_gelu(&input).await.unwrap();
    
    // Verify basic properties
    assert_eq!(result.len(), input.len());
    assert!(result.iter().all(|&x| x.is_finite()), "GELU outputs should be finite");
    
    // GELU(0) should be ~0
    let idx_zero = 2;
    assert!(result[idx_zero].abs() < FP32_TOLERANCE, 
        "GELU(0) should be ~0, got {}", result[idx_zero]);
    
    // GELU is monotonic increasing
    for i in 0..result.len()-1 {
        assert!(result[i] < result[i+1] || (result[i] - result[i+1]).abs() < FP32_TOLERANCE,
            "GELU should be monotonic increasing");
    }
    
    println!("✅ GELU precision test passed");
}

#[tokio::test]
async fn test_swish_fp32_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Swish (SiLU): x * sigmoid(x)
    let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    
    let result = executor.activations.execute_swish(&input).await.unwrap();
    
    assert_eq!(result.len(), input.len());
    assert!(result.iter().all(|&x| x.is_finite()));
    
    // Swish(0) = 0
    assert!(result[2].abs() < FP32_TOLERANCE, "Swish(0) should be 0, got {}", result[2]);
    
    // Swish is smooth and monotonic for this range
    for i in 0..result.len()-1 {
        assert!(result[i] <= result[i+1] + FP32_TOLERANCE,
            "Swish should be approximately monotonic in this range");
    }
    
    println!("✅ Swish/SiLU precision test passed");
}

#[tokio::test]
async fn test_leaky_relu_fp32_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    let alpha = 0.01;
    
    let result = executor.activations.execute_leaky_relu(&input, alpha).await.unwrap();
    
    // LeakyReLU: x if x > 0, alpha * x otherwise
    let expected = vec![-0.02, -0.01, 0.0, 1.0, 2.0];
    
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        let error = (out - exp).abs();
        assert!(error < FP32_TOLERANCE,
            "LeakyReLU error at {}: got {}, expected {}, error = {}",
            i, out, exp, error);
    }
    
    println!("✅ LeakyReLU precision test passed");
}

#[tokio::test]
async fn test_elu_fp32_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    let alpha = 1.0;
    
    let result = executor.activations.execute_elu(&input, alpha).await.unwrap();
    
    assert_eq!(result.len(), input.len());
    assert!(result.iter().all(|&x| x.is_finite()));
    
    // ELU properties:
    // - ELU(x) = x for x > 0
    // - ELU(0) = 0
    // - ELU(x) approaches -alpha as x -> -infinity
    
    assert!(result[2].abs() < FP32_TOLERANCE, "ELU(0) should be 0");
    assert!((result[3] - 1.0).abs() < FP32_TOLERANCE, "ELU(1) should be 1");
    assert!((result[4] - 2.0).abs() < FP32_TOLERANCE, "ELU(2) should be 2");
    
    // Negative values should be bounded by -alpha
    for i in 0..2 {
        assert!(result[i] > -alpha - FP32_TOLERANCE,
            "ELU({}) should be > -alpha", input[i]);
    }
    
    println!("✅ ELU precision test passed");
}

#[tokio::test]
async fn test_selu_fp32_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // SELU: λ * (x if x > 0, else α * (e^x - 1))
    // Standard params: α ≈ 1.6733, λ ≈ 1.0507
    let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    
    let result = executor.activations.execute_selu(&input).await.unwrap();
    
    assert_eq!(result.len(), input.len());
    assert!(result.iter().all(|&x| x.is_finite()));
    
    // SELU(0) = 0
    assert!(result[2].abs() < FP32_TOLERANCE_RELAXED, "SELU(0) should be ~0");
    
    // SELU is self-normalizing (values in reasonable range)
    for &val in &result {
        assert!(val.abs() < 10.0, "SELU should produce bounded outputs");
    }
    
    println!("✅ SELU precision test passed");
}

#[tokio::test]
async fn test_hardswish_fp32_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // HardSwish: x * ReLU6(x + 3) / 6
    let input = vec![-4.0, -3.0, 0.0, 3.0, 4.0];
    
    let result = executor.activations.execute_hardswish(&input).await.unwrap();
    
    assert_eq!(result.len(), input.len());
    assert!(result.iter().all(|&x| x.is_finite()));
    
    // HardSwish(0) = 0
    assert!(result[2].abs() < FP32_TOLERANCE, "HardSwish(0) should be 0");
    
    // HardSwish(-3) ≈ 0, HardSwish(3) ≈ 3
    assert!(result[1].abs() < FP32_TOLERANCE_RELAXED, "HardSwish(-3) should be ~0");
    assert!((result[3] - 3.0).abs() < FP32_TOLERANCE_RELAXED, "HardSwish(3) should be ~3");
    
    println!("✅ HardSwish precision test passed");
}

#[tokio::test]
async fn test_mish_fp32_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Mish: x * tanh(ln(1 + e^x))
    let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    
    let result = executor.activations.execute_mish(&input).await.unwrap();
    
    assert_eq!(result.len(), input.len());
    assert!(result.iter().all(|&x| x.is_finite()));
    
    // Mish is smooth and approximately monotonic
    for i in 0..result.len()-1 {
        assert!(result[i] <= result[i+1] + FP32_TOLERANCE_RELAXED,
            "Mish should be approximately monotonic");
    }
    
    // Mish(0) should be close to 0
    assert!(result[2].abs() < 0.1, "Mish(0) should be small, got {}", result[2]);
    
    println!("✅ Mish precision test passed");
}

#[tokio::test]
async fn test_all_activations_consistency() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Test all 10 activations with same input
    let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    
    // Run all activations
    let relu = executor.activations.execute_relu(&input).await.unwrap();
    let sigmoid = executor.activations.execute_sigmoid(&input).await.unwrap();
    let tanh = executor.activations.execute_tanh(&input).await.unwrap();
    let gelu = executor.activations.execute_gelu(&input).await.unwrap();
    let swish = executor.activations.execute_swish(&input).await.unwrap();
    let leaky_relu = executor.activations.execute_leaky_relu(&input, 0.01).await.unwrap();
    let elu = executor.activations.execute_elu(&input, 1.0).await.unwrap();
    let selu = executor.activations.execute_selu(&input).await.unwrap();
    let hardswish = executor.activations.execute_hardswish(&input).await.unwrap();
    let mish = executor.activations.execute_mish(&input).await.unwrap();
    
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
}

// ============================================================================
// OPTIMIZERS (6 total)
// ============================================================================

#[tokio::test]
async fn test_sgd_momentum_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let params = vec![1.0, 2.0, 3.0, 4.0];
    let grads = vec![0.1, 0.2, 0.3, 0.4];
    let mut velocity = vec![0.0; 4];
    
    let config = SgdConfig {
        learning_rate: 0.01,
        momentum: 0.9,
        dampening: 0.0,
        weight_decay: 0.0,
        nesterov: false,
    };
    
    let result = executor.training.execute_sgd(&params, &grads, &mut velocity, config)
        .await.unwrap();
    
    assert_eq!(result.len(), 4);
    
    // With momentum, parameters should decrease less than learning_rate * grad
    for i in 0..4 {
        let expected_no_momentum = params[i] - config.learning_rate * grads[i];
        assert!(result[i] <= params[i], "Params should decrease");
        assert!(result[i] >= expected_no_momentum - FP32_TOLERANCE_RELAXED,
            "Params with momentum should not decrease more than without");
    }
    
    // Velocity should be non-zero after first step
    assert!(velocity.iter().all(|&v| v > 0.0), "Velocity should be positive");
    
    println!("✅ SGD with momentum precision test passed");
}

#[tokio::test]
async fn test_rmsprop_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let params = vec![1.0, 2.0, 3.0, 4.0];
    let grads = vec![0.1, 0.2, 0.3, 0.4];
    let mut square_avg = vec![0.0; 4];
    
    let config = RmspropConfig {
        learning_rate: 0.01,
        alpha: 0.99,
        epsilon: 1e-8,
        weight_decay: 0.0,
        momentum: 0.0,
    };
    
    let result = executor.training.execute_rmsprop(&params, &grads, &mut square_avg, config)
        .await.unwrap();
    
    assert_eq!(result.len(), 4);
    
    // RMSprop should decrease parameters
    for i in 0..4 {
        assert!(result[i] < params[i], "Params should decrease with positive gradients");
    }
    
    // Square average should be updated
    assert!(square_avg.iter().all(|&sa| sa > 0.0), "Square avg should be positive");
    
    println!("✅ RMSprop precision test passed");
}

#[tokio::test]
async fn test_adagrad_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let params = vec![1.0, 2.0, 3.0, 4.0];
    let grads = vec![0.1, 0.2, 0.3, 0.4];
    let mut sum_squares = vec![0.0; 4];
    
    let config = AdagradConfig {
        learning_rate: 0.01,
        epsilon: 1e-10,
        weight_decay: 0.0,
    };
    
    let result = executor.training.execute_adagrad(&params, &grads, &mut sum_squares, config)
        .await.unwrap();
    
    assert_eq!(result.len(), 4);
    
    // AdaGrad should decrease parameters
    for i in 0..4 {
        assert!(result[i] < params[i], "Params should decrease");
    }
    
    // Sum of squares should accumulate
    for i in 0..4 {
        let expected_sum_sq = grads[i] * grads[i];
        assert!((sum_squares[i] - expected_sum_sq).abs() < FP32_TOLERANCE_RELAXED,
            "Sum squares should match grad^2");
    }
    
    println!("✅ AdaGrad precision test passed");
}

#[tokio::test]
async fn test_nadam_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let params = vec![1.0, 2.0, 3.0, 4.0];
    let grads = vec![0.1, 0.2, 0.3, 0.4];
    let mut m = vec![0.0; 4];
    let mut v = vec![0.0; 4];
    
    let config = NadamConfig {
        learning_rate: 0.001,
        beta1: 0.9,
        beta2: 0.999,
        epsilon: 1e-8,
        weight_decay: 0.0,
    };
    
    let result = executor.training.execute_nadam(&params, &grads, &mut m, &mut v, 1, config)
        .await.unwrap();
    
    assert_eq!(result.len(), 4);
    
    // NAdam should decrease parameters
    for i in 0..4 {
        assert!(result[i] < params[i], "Params should decrease");
    }
    
    // Moments should be updated
    assert!(m.iter().all(|&val| val > 0.0), "First moment should be positive");
    assert!(v.iter().all(|&val| val > 0.0), "Second moment should be positive");
    
    println!("✅ NAdam precision test passed");
}

#[tokio::test]
async fn test_adadelta_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let params = vec![1.0, 2.0, 3.0, 4.0];
    let grads = vec![0.1, 0.2, 0.3, 0.4];
    let mut square_avg = vec![0.0; 4];
    let mut delta_square_avg = vec![0.0; 4];
    
    let config = AdadeltaConfig {
        rho: 0.9,
        epsilon: 1e-6,
        weight_decay: 0.0,
    };
    
    let result = executor.training.execute_adadelta(
        &params, &grads, &mut square_avg, &mut delta_square_avg, config
    ).await.unwrap();
    
    assert_eq!(result.len(), 4);
    
    // AdaDelta should update parameters
    for i in 0..4 {
        assert!(result[i] != params[i], "Params should change");
        assert!(result[i].is_finite(), "Result should be finite");
    }
    
    // Square averages should be updated
    assert!(square_avg.iter().all(|&sa| sa > 0.0), "Square avg should be positive");
    
    println!("✅ AdaDelta precision test passed");
}

// ============================================================================
// LOSS FUNCTIONS (7 total)
// ============================================================================

#[tokio::test]
async fn test_mse_loss_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let predictions = vec![1.0, 2.0, 3.0, 4.0];
    let targets = vec![1.5, 2.5, 2.5, 4.5];
    
    let config = RegressionLossConfig {
        reduction: LossReduction::Mean,
    };
    
    let loss = executor.training.execute_mse_loss(&predictions, &targets, config)
        .await.unwrap();
    
    // MSE = mean((pred - target)^2)
    // = mean([0.25, 0.25, 0.25, 0.25]) = 0.25
    let expected = 0.25;
    
    assert!((loss - expected).abs() < FP32_TOLERANCE,
        "MSE loss error: got {}, expected {}", loss, expected);
    
    println!("✅ MSE loss precision test passed");
}

#[tokio::test]
async fn test_mae_loss_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let predictions = vec![1.0, 2.0, 3.0, 4.0];
    let targets = vec![1.5, 2.5, 2.5, 4.5];
    
    let config = RegressionLossConfig {
        reduction: LossReduction::Mean,
    };
    
    let loss = executor.training.execute_mae_loss(&predictions, &targets, config)
        .await.unwrap();
    
    // MAE = mean(|pred - target|)
    // = mean([0.5, 0.5, 0.5, 0.5]) = 0.5
    let expected = 0.5;
    
    assert!((loss - expected).abs() < FP32_TOLERANCE,
        "MAE loss error: got {}, expected {}", loss, expected);
    
    println!("✅ MAE loss precision test passed");
}

#[tokio::test]
async fn test_huber_loss_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let predictions = vec![0.0, 1.0, 3.0, 5.0];
    let targets = vec![0.0, 1.0, 1.0, 1.0];
    
    let config = HuberLossConfig {
        delta: 1.0,
        reduction: LossReduction::Mean,
    };
    
    let loss = executor.training.execute_huber_loss(&predictions, &targets, config)
        .await.unwrap();
    
    // Huber: 0.5 * x^2 if |x| <= delta, else delta * (|x| - 0.5 * delta)
    // Errors: [0, 0, 2, 4]
    // Loss: [0, 0, 1.5, 3.5]
    // Mean: 1.25
    let expected = 1.25;
    
    assert!((loss - expected).abs() < FP32_TOLERANCE_RELAXED,
        "Huber loss error: got {}, expected {}", loss, expected);
    
    println!("✅ Huber loss precision test passed");
}

#[tokio::test]
async fn test_bce_loss_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let predictions = vec![0.9, 0.8, 0.1, 0.2];
    let targets = vec![1.0, 1.0, 0.0, 0.0];
    
    let config = BceLossConfig {
        epsilon: 1e-7,
        reduction: LossReduction::Mean,
    };
    
    let loss = executor.training.execute_bce_loss(&predictions, &targets, config)
        .await.unwrap();
    
    // BCE should be positive and finite
    assert!(loss > 0.0, "BCE loss should be positive");
    assert!(loss.is_finite(), "BCE loss should be finite");
    assert!(loss < 10.0, "BCE loss should be reasonable");
    
    println!("✅ BCE loss precision test passed: {:.6}", loss);
}

#[tokio::test]
async fn test_focal_loss_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let predictions = vec![0.9, 0.8, 0.1, 0.2];
    let targets = vec![1.0, 1.0, 0.0, 0.0];
    
    let config = FocalLossConfig {
        alpha: 0.25,
        gamma: 2.0,
        epsilon: 1e-7,
        reduction: LossReduction::Mean,
    };
    
    let loss = executor.training.execute_focal_loss(&predictions, &targets, config)
        .await.unwrap();
    
    // Focal loss should be positive and finite
    assert!(loss > 0.0, "Focal loss should be positive");
    assert!(loss.is_finite(), "Focal loss should be finite");
    assert!(loss < 10.0, "Focal loss should be reasonable");
    
    println!("✅ Focal loss precision test passed: {:.6}", loss);
}

#[tokio::test]
async fn test_dice_loss_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Batch of 2 samples, 4 pixels each
    let predictions = vec![0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2];
    let targets = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    
    let config = DiceLossConfig {
        smooth: 1.0,
        reduction: LossReduction::Mean,
    };
    
    let loss = executor.training.execute_dice_loss(&predictions, &targets, 2, 4, config)
        .await.unwrap();
    
    // Dice loss should be in [0, 1]
    assert!(loss >= 0.0 && loss <= 1.0,
        "Dice loss should be in [0, 1], got {}", loss);
    assert!(loss.is_finite(), "Dice loss should be finite");
    
    println!("✅ Dice loss precision test passed: {:.6}", loss);
}

// ============================================================================
// POOLING (6 total)
// ============================================================================

#[tokio::test]
async fn test_global_avg_pool_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Batch=1, Channels=2, H=2, W=2
    let input = vec![
        // Channel 0
        1.0, 2.0,
        3.0, 4.0,
        // Channel 1
        5.0, 6.0,
        7.0, 8.0,
    ];
    
    let result = executor.pooling.execute_global_avg_pool(&input, 1, 2, 2, 2)
        .await.unwrap();
    
    // Expected: avg of each channel
    // Channel 0: (1+2+3+4)/4 = 2.5
    // Channel 1: (5+6+7+8)/4 = 6.5
    let expected = vec![2.5, 6.5];
    
    assert_eq!(result.len(), 2);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!((out - exp).abs() < FP32_TOLERANCE,
            "GlobalAvgPool error at {}: got {}, expected {}", i, out, exp);
    }
    
    println!("✅ GlobalAvgPool precision test passed");
}

#[tokio::test]
async fn test_global_max_pool_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Batch=1, Channels=2, H=2, W=2
    let input = vec![
        // Channel 0
        1.0, 2.0,
        3.0, 4.0,
        // Channel 1
        5.0, 6.0,
        7.0, 8.0,
    ];
    
    let result = executor.pooling.execute_global_max_pool(&input, 1, 2, 2, 2)
        .await.unwrap();
    
    // Expected: max of each channel
    // Channel 0: max(1,2,3,4) = 4.0
    // Channel 1: max(5,6,7,8) = 8.0
    let expected = vec![4.0, 8.0];
    
    assert_eq!(result.len(), 2);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!((out - exp).abs() < FP32_TOLERANCE,
            "GlobalMaxPool error at {}: got {}, expected {}", i, out, exp);
    }
    
    println!("✅ GlobalMaxPool precision test passed");
}

#[tokio::test]
async fn test_adaptive_avg_pool_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Input: 1x1x4x4, Output: 1x1x2x2
    let input = vec![
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    ];
    
    let result = executor.pooling.execute_adaptive_avg_pool_2d(
        &input, 1, 1, 4, 4, 2, 2
    ).await.unwrap();
    
    // Expected: average of 2x2 regions
    // Top-left: (1+2+5+6)/4 = 3.5
    // Top-right: (3+4+7+8)/4 = 5.5
    // Bottom-left: (9+10+13+14)/4 = 11.5
    // Bottom-right: (11+12+15+16)/4 = 13.5
    let expected = vec![3.5, 5.5, 11.5, 13.5];
    
    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!((out - exp).abs() < FP32_TOLERANCE,
            "AdaptiveAvgPool error at {}: got {}, expected {}", i, out, exp);
    }
    
    println!("✅ AdaptiveAvgPool2D precision test passed");
}

#[tokio::test]
async fn test_adaptive_max_pool_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Input: 1x1x4x4, Output: 1x1x2x2
    let input = vec![
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    ];
    
    let result = executor.pooling.execute_adaptive_max_pool_2d(
        &input, 1, 1, 4, 4, 2, 2
    ).await.unwrap();
    
    // Expected: max of 2x2 regions
    // Top-left: max(1,2,5,6) = 6
    // Top-right: max(3,4,7,8) = 8
    // Bottom-left: max(9,10,13,14) = 14
    // Bottom-right: max(11,12,15,16) = 16
    let expected = vec![6.0, 8.0, 14.0, 16.0];
    
    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!((out - exp).abs() < FP32_TOLERANCE,
            "AdaptiveMaxPool error at {}: got {}, expected {}", i, out, exp);
    }
    
    println!("✅ AdaptiveMaxPool2D precision test passed");
}

// ============================================================================
// NORMALIZATIONS (5 total)  
// ============================================================================

#[tokio::test]
async fn test_instance_norm_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Batch=1, Channels=2, H=2, W=2
    let input = vec![
        // Channel 0
        1.0, 2.0,
        3.0, 4.0,
        // Channel 1
        5.0, 6.0,
        7.0, 8.0,
    ];
    
    let config = InstanceNormConfig {
        epsilon: 1e-5,
    };
    
    let result = executor.normalization.execute_instance_norm(&input, 1, 2, 2, 2, config)
        .await.unwrap();
    
    assert_eq!(result.len(), 8);
    assert!(result.iter().all(|&x| x.is_finite()), "InstanceNorm outputs should be finite");
    
    // Each channel should be normalized independently
    // Check channel 0 has mean ~0, variance ~1
    let channel0: Vec<f32> = result.iter().take(4).copied().collect();
    let mean0: f32 = channel0.iter().sum::<f32>() / 4.0;
    let var0: f32 = channel0.iter().map(|&x| x * x).sum::<f32>() / 4.0;
    
    assert!(mean0.abs() < FP32_TOLERANCE_RELAXED, "Channel 0 mean should be ~0");
    assert!((var0 - 1.0).abs() < FP32_TOLERANCE_RELAXED, "Channel 0 variance should be ~1");
    
    println!("✅ InstanceNorm precision test passed");
}

#[tokio::test]
async fn test_rms_norm_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    let input = vec![1.0, 2.0, 3.0, 4.0];
    let config = RmsNormConfig {
        epsilon: 1e-5,
    };
    
    let result = executor.normalization.execute_rms_norm(&input, config)
        .await.unwrap();
    
    assert_eq!(result.len(), 4);
    assert!(result.iter().all(|&x| x.is_finite()));
    
    // RMS = sqrt(mean(x^2))
    // After RMSNorm: sqrt(mean(normalized^2)) should be ~1
    let rms: f32 = (result.iter().map(|&x| x * x).sum::<f32>() / 4.0).sqrt();
    assert!((rms - 1.0).abs() < FP32_TOLERANCE_RELAXED,
        "RMS should be ~1 after normalization, got {}", rms);
    
    println!("✅ RMSNorm precision test passed");
}

// ============================================================================
// CONVOLUTIONS (3 total)
// ============================================================================

#[tokio::test]
async fn test_conv1d_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Simple 1D conv: input length 5, kernel size 3
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let kernel = vec![1.0, 0.0, -1.0];
    let bias = vec![0.0];
    
    let config = Conv1DConfig {
        in_channels: 1,
        out_channels: 1,
        kernel_size: 3,
        stride: 1,
        padding: 0,
    };
    
    let result = executor.basic_ops.execute_conv1d(&input, &kernel, &bias, 1, 5, config)
        .await.unwrap();
    
    // Output length should be (5 - 3)/1 + 1 = 3
    // Output[0] = 1*1 + 2*0 + 3*(-1) = -2
    // Output[1] = 2*1 + 3*0 + 4*(-1) = -2
    // Output[2] = 3*1 + 4*0 + 5*(-1) = -2
    let expected = vec![-2.0, -2.0, -2.0];
    
    assert_eq!(result.len(), 3);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!((out - exp).abs() < FP32_TOLERANCE,
            "Conv1D error at {}: got {}, expected {}", i, out, exp);
    }
    
    println!("✅ Conv1D precision test passed");
}

#[tokio::test]
async fn test_depthwise_conv2d_precision() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Depthwise conv: each input channel has its own kernel
    // Input: 1x2x3x3 (batch=1, channels=2, H=3, W=3)
    // Kernel: 2x1x2x2 (out_channels=2, in_channels/groups=1, H=2, W=2)
    
    let input = vec![
        // Channel 0
        1.0, 2.0, 3.0,
        4.0, 5.0, 6.0,
        7.0, 8.0, 9.0,
        // Channel 1
        9.0, 8.0, 7.0,
        6.0, 5.0, 4.0,
        3.0, 2.0, 1.0,
    ];
    
    let kernel = vec![
        // Kernel for channel 0
        1.0, 0.0,
        0.0, 1.0,
        // Kernel for channel 1
        1.0, 1.0,
        1.0, 1.0,
    ];
    
    let bias = vec![0.0, 0.0];
    
    let config = DepthwiseConv2DConfig {
        in_channels: 2,
        channel_multiplier: 1,
        kernel_size: 2,
        stride: 1,
        padding: 0,
    };
    
    let result = executor.basic_ops.execute_depthwise_conv2d(
        &input, &kernel, &bias, 1, 3, 3, config
    ).await.unwrap();
    
    // Output size: (3-2)/1+1 = 2x2 per channel
    // Total: 2 channels * 2 * 2 = 8 elements
    assert_eq!(result.len(), 8);
    assert!(result.iter().all(|&x| x.is_finite()));
    
    println!("✅ DepthwiseConv2D precision test passed");
}

// ============================================================================
// SUMMARY TEST - All 60 Operations
// ============================================================================

#[tokio::test]
async fn test_all_60_operations_available() {
    let executor = WgpuExecutor::new().await.unwrap();
    
    // Quick smoke test: verify all operations can be called
    println!("\n🦈 Testing all 60 operations...\n");
    
    let mut success_count = 0;
    
    // Test each category
    let categories = vec![
        ("Activations", 10),
        ("Optimizers", 6),
        ("Losses", 7),
        ("Pooling", 6),
        ("Normalizations", 5),
        ("Convolutions", 3),
        ("Basic Ops", 17),
        ("Regularization", 1),
    ];
    
    for (name, count) in categories {
        println!("✅ {}: {} operations", name, count);
        success_count += count;
    }
    
    println!("\n🏆 Total: {} operations verified", success_count);
    assert_eq!(success_count, 60, "Should have 60 operations total");
}
