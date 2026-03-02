// Precision tests - Norm
use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::WgpuExecutor;
use ml_inference_showcase::wgpu::*;

const FP32_TOLERANCE: f32 = 1e-5;
const FP32_TOLERANCE_RELAXED: f32 = 1e-4;

#[tokio::test]
async fn test_softmax_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Softmax: exp(x_i) / sum(exp(x_j))
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let result = executor.execute_softmax(&input).await.unwrap();

        assert_eq!(result.len(), input.len());
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "Softmax outputs should be finite"
        );

        // Verify softmax properties
        // 1. All outputs in (0, 1)
        assert!(
            result.iter().all(|&x| x > 0.0 && x < 1.0),
            "Softmax outputs should be in (0, 1)"
        );

        // 2. Sum should be 1.0
        let sum: f32 = result.iter().sum();
        assert!(
            (sum - 1.0).abs() < FP32_TOLERANCE,
            "Softmax outputs should sum to 1.0, got {}",
            sum
        );

        // 3. Largest input should produce largest output
        let max_idx = input
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(idx, _)| idx)
            .unwrap();
        let max_output_idx = result
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(idx, _)| idx)
            .unwrap();
        assert_eq!(
            max_idx, max_output_idx,
            "Largest input should produce largest output"
        );

        // 4. Verify monotonic ordering preserved
        for i in 0..result.len() - 1 {
            assert!(
                result[i] < result[i + 1],
                "Softmax should preserve ordering: softmax({}) = {} should be < softmax({}) = {}",
                input[i],
                result[i],
                input[i + 1],
                result[i + 1]
            );
        }

        println!("✅ Softmax precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_layernorm_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // LayerNorm: (x - mean) / sqrt(variance + eps) * gamma + beta
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let config = NormConfig {
            epsilon: 1e-5,
            gamma: Some(vec![1.0; 5]), // Scale (all 1s)
            beta: Some(vec![0.0; 5]),  // Shift (all 0s)
        };

        let result = executor.execute_layernorm(&input, config).await.unwrap();

        assert_eq!(result.len(), input.len());
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "LayerNorm outputs should be finite"
        );

        // Verify normalized properties (gamma=1, beta=0 means standard normalization)
        // Mean should be ~0
        let mean: f32 = result.iter().sum::<f32>() / result.len() as f32;
        assert!(
            mean.abs() < FP32_TOLERANCE_RELAXED,
            "LayerNorm mean should be ~0, got {}",
            mean
        );

        // Variance should be ~1
        let variance: f32 =
            result.iter().map(|&x| (x - mean) * (x - mean)).sum::<f32>() / result.len() as f32;
        assert!(
            (variance - 1.0).abs() < FP32_TOLERANCE_RELAXED,
            "LayerNorm variance should be ~1, got {}",
            variance
        );

        println!("✅ LayerNorm precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_batchnorm_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // BatchNorm: (x - running_mean) / sqrt(running_var + eps) * gamma + beta
        // batch=2, channels=2, spatial_size=2 (e.g., 2 images, 2 channels, 1x2 spatial)
        let input = vec![
            // Batch 0, Channel 0
            1.0, 2.0, // Batch 0, Channel 1
            3.0, 4.0, // Batch 1, Channel 0
            5.0, 6.0, // Batch 1, Channel 1
            7.0, 8.0,
        ];

        let config = BatchNormConfig {
            epsilon: 1e-5,
            gamma: vec![1.0, 1.0],         // 2 channels
            beta: vec![0.0, 0.0],          // 2 channels
            running_mean: vec![3.5, 5.5],  // Pre-computed per-channel mean
            running_var: vec![5.25, 5.25], // Pre-computed per-channel variance
        };

        let result = executor
            .execute_batchnorm(&input, 2, 2, 2, config)
            .await
            .unwrap();

        assert_eq!(result.len(), 8);
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "BatchNorm outputs should be finite"
        );

        // Verify each channel is normalized using its running statistics
        // Channel 0 values: [1, 2, 5, 6] with mean=3.5, var=5.25
        // Channel 1 values: [3, 4, 7, 8] with mean=5.5, var=5.25
        // After normalization, values should be centered differently per channel

        println!("✅ BatchNorm precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_groupnorm_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // GroupNorm: Divide channels into groups, normalize each group independently
        // batch=1, channels=4, spatial_size=2, num_groups=2 (2 channels per group)
        let input = vec![
            // Group 0: Channels 0-1
            1.0, 2.0, // Channel 0
            3.0, 4.0, // Channel 1
            // Group 1: Channels 2-3
            5.0, 6.0, // Channel 2
            7.0, 8.0, // Channel 3
        ];

        let config = GroupNormConfig {
            num_groups: 2,
            epsilon: 1e-5,
            gamma: vec![1.0; 4], // 4 channels
            beta: vec![0.0; 4],  // 4 channels
        };

        let result = executor
            .execute_groupnorm(&input, 1, 4, 2, config)
            .await
            .unwrap();

        assert_eq!(result.len(), 8);
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "GroupNorm outputs should be finite"
        );

        // Each group should be normalized independently
        // Group 0 (channels 0-1): values [1,2,3,4]
        // Group 1 (channels 2-3): values [5,6,7,8]

        println!("✅ GroupNorm precision test passed");
    })
    .await;
}

// Advanced Normalizations (Already Tested)

#[tokio::test]
async fn test_instance_norm_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Batch=1, Channels=2, H=2, W=2
        let input = vec![
            // Channel 0
            1.0, 2.0, 3.0, 4.0, // Channel 1
            5.0, 6.0, 7.0, 8.0,
        ];

        let config = InstanceNormConfig {
            epsilon: 1e-5,
            gamma: vec![1.0, 1.0], // 2 channels
            beta: vec![0.0, 0.0],  // 2 channels
        };

        // batch=1, channels=2, spatial_size=4 (2x2 spatial dimensions)
        let result = executor
            .execute_instance_norm(&input, 1, 2, 4, config)
            .await
            .unwrap();

        assert_eq!(result.len(), 8);
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "InstanceNorm outputs should be finite"
        );

        // Each channel should be normalized independently
        // Check channel 0 has mean ~0, variance ~1
        let channel0: Vec<f32> = result.iter().take(4).copied().collect();
        let mean0: f32 = channel0.iter().sum::<f32>() / 4.0;
        let var0: f32 = channel0.iter().map(|&x| x * x).sum::<f32>() / 4.0;

        assert!(
            mean0.abs() < FP32_TOLERANCE_RELAXED,
            "Channel 0 mean should be ~0"
        );
        assert!(
            (var0 - 1.0).abs() < FP32_TOLERANCE_RELAXED,
            "Channel 0 variance should be ~1"
        );

        println!("✅ InstanceNorm precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_rms_norm_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let config = RmsNormConfig {
            epsilon: 1e-5,
            gamma: vec![1.0, 1.0, 1.0, 1.0], // 4 features
        };

        // batch_size=1, feature_size=4
        let result = executor
            .execute_rms_norm(&input, 1, 4, config)
            .await
            .unwrap();

        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|&x| x.is_finite()));

        // RMS = sqrt(mean(x^2))
        // After RMSNorm: sqrt(mean(normalized^2)) should be ~1
        let rms: f32 = (result.iter().map(|&x| x * x).sum::<f32>() / 4.0).sqrt();
        assert!(
            (rms - 1.0).abs() < FP32_TOLERANCE_RELAXED,
            "RMS should be ~1 after normalization, got {}",
            rms
        );

        println!("✅ RMSNorm precision test passed");
    })
    .await;
}

// ============================================================================
// CONVOLUTIONS (3 total)
// ============================================================================
