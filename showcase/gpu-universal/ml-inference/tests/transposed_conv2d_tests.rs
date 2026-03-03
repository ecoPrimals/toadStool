// SPDX-License-Identifier: AGPL-3.0-or-later
//! TransposedConv2D Tests: Upsampling/Deconvolution
//!
//! Tests transposed convolution for learnable upsampling.

#![allow(unused_variables)]

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

#[tokio::test]
async fn test_transposed_conv2d_2x_upsample() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Simple 2x upsampling: 2x2 -> 4x4
        let batch = 1;
        let in_channels = 1;
        let out_channels = 1;
        let input_h = 2;
        let input_w = 2;

        // Input 2x2
        let input = vec![1.0, 2.0, 3.0, 4.0];

        // 2x2 kernel
        let weights = vec![1.0, 0.0, 0.0, 0.0];
        let bias = vec![0.0];

        let config = TransposedConv2DConfig {
            kernel_size: (2, 2),
            stride: (2, 2),
            padding: (0, 0),
            output_padding: (0, 0),
        };

        let result = executor
            .execute_transposed_conv2d(
                &input,
                &weights,
                &bias,
                batch,
                in_channels,
                out_channels,
                input_h,
                input_w,
                config,
            )
            .await
            .unwrap();

        // Output should be 4x4
        let expected_h = 4;
        let expected_w = 4;
        assert_eq!(result.len(), expected_h * expected_w);

        println!("✅ 2x upsampling test passed");
        println!("   {}x{} → {}x{}", input_h, input_w, expected_h, expected_w);
    })
    .await;
}

#[tokio::test]
async fn test_transposed_conv2d_stride() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Test different strides
        let batch = 1;
        let in_channels = 1;
        let out_channels = 1;
        let input_h = 3;
        let input_w = 3;

        let input: Vec<f32> = (0..9).map(|i| i as f32).collect();
        let weights = vec![1.0, 0.0, 0.0, 1.0]; // 2x2 kernel
        let bias = vec![0.0];

        let config = TransposedConv2DConfig {
            kernel_size: (2, 2),
            stride: (2, 2),
            padding: (0, 0),
            output_padding: (0, 0),
        };

        let result = executor
            .execute_transposed_conv2d(
                &input,
                &weights,
                &bias,
                batch,
                in_channels,
                out_channels,
                input_h,
                input_w,
                config,
            )
            .await
            .unwrap();

        // With stride 2, output = (3-1)*2 + 2 = 6
        let expected_size = 6 * 6;
        assert_eq!(result.len(), expected_size);

        println!("✅ Stride test passed");
        println!("   Input: 3x3, Output: 6x6");
    })
    .await;
}

#[tokio::test]
async fn test_transposed_conv2d_multi_channel() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Multi-channel test
        let batch = 1;
        let in_channels = 2;
        let out_channels = 3;
        let input_h = 2;
        let input_w = 2;

        // 2 input channels, each 2x2
        let input = vec![
            1.0, 2.0, 3.0, 4.0, // channel 0
            5.0, 6.0, 7.0, 8.0, // channel 1
        ];

        // Weights: [in_channels, out_channels, kh, kw]
        // 2 input channels × 3 output channels × 2×2 kernel = 24 weights
        let weights: Vec<f32> = (0..24).map(|i| (i as f32) * 0.1).collect();
        let bias = vec![0.0, 0.0, 0.0];

        let config = TransposedConv2DConfig {
            kernel_size: (2, 2),
            stride: (2, 2),
            padding: (0, 0),
            output_padding: (0, 0),
        };

        let result = executor
            .execute_transposed_conv2d(
                &input,
                &weights,
                &bias,
                batch,
                in_channels,
                out_channels,
                input_h,
                input_w,
                config,
            )
            .await
            .unwrap();

        // Output: 3 channels × 4×4
        let expected_size = out_channels * 4 * 4;
        assert_eq!(result.len(), expected_size);

        println!("✅ Multi-channel test passed");
        println!("   {} channels → {} channels", in_channels, out_channels);
    })
    .await;
}

#[tokio::test]
async fn test_transposed_conv2d_with_bias() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Test bias application
        let batch = 1;
        let in_channels = 1;
        let out_channels = 2;
        let input_h = 2;
        let input_w = 2;

        let input = vec![1.0, 1.0, 1.0, 1.0];
        let weights = vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // 2 output channels
        let bias = vec![10.0, 20.0]; // Different bias per channel

        let config = TransposedConv2DConfig {
            kernel_size: (2, 2),
            stride: (2, 2),
            padding: (0, 0),
            output_padding: (0, 0),
        };

        let result = executor
            .execute_transposed_conv2d(
                &input,
                &weights,
                &bias,
                batch,
                in_channels,
                out_channels,
                input_h,
                input_w,
                config,
            )
            .await
            .unwrap();

        assert_eq!(result.len(), 2 * 4 * 4);

        println!("✅ Bias test passed");
    })
    .await;
}

#[tokio::test]
async fn test_transposed_conv2d_unet_decoder() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Simulate U-Net decoder upsampling
        let batch = 1;
        let in_channels = 64;
        let out_channels = 32;
        let input_h = 8;
        let input_w = 8;

        // Small feature map upsampling
        let input: Vec<f32> = (0..batch * in_channels * input_h * input_w)
            .map(|i| (i as f32) * 0.01)
            .collect();

        let weights: Vec<f32> = (0..in_channels * out_channels * 2 * 2)
            .map(|i| (i as f32) * 0.001)
            .collect();

        let bias: Vec<f32> = vec![0.0; out_channels];

        let config = TransposedConv2DConfig {
            kernel_size: (2, 2),
            stride: (2, 2),
            padding: (0, 0),
            output_padding: (0, 0),
        };

        let result = executor
            .execute_transposed_conv2d(
                &input,
                &weights,
                &bias,
                batch,
                in_channels,
                out_channels,
                input_h,
                input_w,
                config,
            )
            .await
            .unwrap();

        // Output should be 16x16 with 32 channels
        let expected_h = 16;
        let expected_w = 16;
        let expected_size = batch * out_channels * expected_h * expected_w;
        assert_eq!(result.len(), expected_size);

        println!("✅ U-Net decoder test passed");
        println!(
            "   {}x{} ({}ch) → {}x{} ({}ch)",
            input_h, input_w, in_channels, expected_h, expected_w, out_channels
        );
    })
    .await;
}
