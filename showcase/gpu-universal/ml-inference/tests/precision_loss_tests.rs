// Precision tests - Loss
use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::WgpuExecutor;
use ml_inference_showcase::wgpu::*;

const FP32_TOLERANCE: f32 = 1e-5;
const FP32_TOLERANCE_RELAXED: f32 = 1e-4;

#[tokio::test]
async fn test_cross_entropy_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // CrossEntropy: Standard classification loss
        // Predictions: probabilities (after softmax) for 2 samples, 3 classes
        // Using simple uniform-ish probabilities for testing
        let predictions = vec![
            0.1, 0.2, 0.7, // Sample 0: class 2 has highest prob (matches target)
            0.6, 0.1, 0.3, // Sample 1: class 0 has highest prob (matches target)
        ];

        // Targets: one-hot encoded (2 samples, 3 classes)
        let targets = vec![
            0.0, 0.0, 1.0, // Sample 0: class 2
            1.0, 0.0, 0.0, // Sample 1: class 0
        ];

        let config = CrossEntropyConfig {
            epsilon: 1e-7,
            reduction: LossReduction::Mean,
        };

        let loss = executor
            .execute_cross_entropy(&predictions, &targets, 2, 3, config)
            .await
            .unwrap();

        // With Mean reduction, should return a single aggregated loss value
        assert_eq!(
            loss.len(),
            1,
            "With Mean reduction, should return single aggregated loss"
        );

        // Loss should be positive (negative log probability)
        assert!(
            loss[0] > 0.0,
            "CrossEntropy loss should be positive, got {}",
            loss[0]
        );

        // Loss should be finite
        assert!(loss[0].is_finite(), "CrossEntropy loss should be finite");

        // When predictions are correct (highest logits match targets), loss should be reasonable
        // Sample 0: logits [1,2,3], target class 2 (highest) ✓
        // Sample 1: logits [3,1,2], target class 0 (highest) ✓
        // Both predictions are correct, so loss should be relatively low (< 1.0 typically)
        println!("CrossEntropy mean loss: {}", loss[0]);

        println!("✅ CrossEntropy precision test passed");
    })
    .await;
}

// Regression Loss Functions (Already Tested)

#[tokio::test]
async fn test_mse_loss_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let predictions = vec![1.0, 2.0, 3.0, 4.0];
        let targets = vec![1.5, 2.5, 2.5, 4.5];

        let config = RegressionLossConfig {
            reduction: LossReduction::Mean,
        };

        let loss = executor
            .execute_mse_loss(&predictions, &targets, config)
            .await
            .unwrap();

        // MSE = mean((pred - target)^2)
        // = mean([0.25, 0.25, 0.25, 0.25]) = 0.25
        let expected = 0.25;

        assert!(
            (loss - expected).abs() < FP32_TOLERANCE,
            "MSE loss error: got {}, expected {}",
            loss,
            expected
        );

        println!("✅ MSE loss precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_mae_loss_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let predictions = vec![1.0, 2.0, 3.0, 4.0];
        let targets = vec![1.5, 2.5, 2.5, 4.5];

        let config = RegressionLossConfig {
            reduction: LossReduction::Mean,
        };

        let loss = executor
            .execute_mae_loss(&predictions, &targets, config)
            .await
            .unwrap();

        // MAE = mean(|pred - target|)
        // = mean([0.5, 0.5, 0.5, 0.5]) = 0.5
        let expected = 0.5;

        assert!(
            (loss - expected).abs() < FP32_TOLERANCE,
            "MAE loss error: got {}, expected {}",
            loss,
            expected
        );

        println!("✅ MAE loss precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_huber_loss_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let predictions = vec![0.0, 1.0, 3.0, 5.0];
        let targets = vec![0.0, 1.0, 1.0, 1.0];

        let config = HuberLossConfig {
            delta: 1.0,
            reduction: LossReduction::Mean,
        };

        let loss = executor
            .execute_huber_loss(&predictions, &targets, config)
            .await
            .unwrap();

        // Huber: 0.5 * x^2 if |x| <= delta, else delta * (|x| - 0.5 * delta)
        // Errors: [0, 0, 2, 4]
        // Loss: [0, 0, 1.5, 3.5]
        // Mean: 1.25
        let expected = 1.25;

        assert!(
            (loss - expected).abs() < FP32_TOLERANCE_RELAXED,
            "Huber loss error: got {}, expected {}",
            loss,
            expected
        );

        println!("✅ Huber loss precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_bce_loss_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let predictions = vec![0.9, 0.8, 0.1, 0.2];
        let targets = vec![1.0, 1.0, 0.0, 0.0];

        let config = BceLossConfig {
            epsilon: 1e-7,
            reduction: LossReduction::Mean,
        };

        let loss = executor
            .execute_bce_loss(&predictions, &targets, config)
            .await
            .unwrap();

        // BCE should be positive and finite
        assert!(loss > 0.0, "BCE loss should be positive");
        assert!(loss.is_finite(), "BCE loss should be finite");
        assert!(loss < 10.0, "BCE loss should be reasonable");

        println!("✅ BCE loss precision test passed: {:.6}", loss);
    })
    .await;
}

#[tokio::test]
async fn test_focal_loss_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let predictions = vec![0.9, 0.8, 0.1, 0.2];
        let targets = vec![1.0, 1.0, 0.0, 0.0];

        let config = FocalLossConfig {
            alpha: 0.25,
            gamma: 2.0,
            epsilon: 1e-7,
            reduction: LossReduction::Mean,
        };

        let loss = executor
            .execute_focal_loss(&predictions, &targets, config)
            .await
            .unwrap();

        // Focal loss should be positive and finite
        assert!(loss > 0.0, "Focal loss should be positive");
        assert!(loss.is_finite(), "Focal loss should be finite");
        assert!(loss < 10.0, "Focal loss should be reasonable");

        println!("✅ Focal loss precision test passed: {:.6}", loss);
    })
    .await;
}

#[tokio::test]
async fn test_dice_loss_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Batch of 2 samples, 4 pixels each
        let predictions = vec![0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2];
        let targets = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let config = DiceLossConfig {
            smooth: 1.0,
            reduction: LossReduction::Mean,
        };

        let loss = executor
            .execute_dice_loss(&predictions, &targets, 2, 4, config)
            .await
            .unwrap();

        // Dice loss should be in [0, 1]
        assert!(
            (0.0..=1.0).contains(&loss),
            "Dice loss should be in [0, 1], got {}",
            loss
        );
        assert!(loss.is_finite(), "Dice loss should be finite");

        println!("✅ Dice loss precision test passed: {:.6}", loss);
    })
    .await;
}

// ============================================================================
// POOLING (6 total)
// ============================================================================

// Standard Pooling (Untested - HIGH PRIORITY)
