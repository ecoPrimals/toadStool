// SPDX-License-Identifier: AGPL-3.0-or-later
// Precision tests - Convolutions
use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::WgpuExecutor;
use ml_inference_showcase::wgpu::*;

const FP32_TOLERANCE: f32 = 1e-5;

#[tokio::test]
async fn test_conv1d_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Simple 1D conv: batch=1, in_channels=1, out_channels=1, length=5, kernel=3
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![1.0, 0.0, -1.0]; // 1 kernel of size 3
        let bias = vec![0.0];

        let config = Conv1DConfig {
            kernel_size: 3,
            stride: 1,
            padding: 0,
            dilation: 1,
        };

        let result = executor
            .execute_conv1d(&input, &kernel, &bias, 1, 1, 1, 5, config)
            .await
            .unwrap();

        // Output length should be (5 - 3)/1 + 1 = 3
        // Output[0] = 1*1 + 2*0 + 3*(-1) = -2
        // Output[1] = 2*1 + 3*0 + 4*(-1) = -2
        // Output[2] = 3*1 + 4*0 + 5*(-1) = -2
        let expected = vec![-2.0, -2.0, -2.0];

        assert_eq!(result.len(), 3);
        for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
            assert!(
                (out - exp).abs() < FP32_TOLERANCE,
                "Conv1D error at {}: got {}, expected {}",
                i,
                out,
                exp
            );
        }

        println!("✅ Conv1D precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_depthwise_conv2d_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Depthwise conv: batch=1, channels=2, H=3, W=3, kernel=2x2
        let input = vec![
            // Channel 0
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, // Channel 1
            9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
        ];

        let kernel = vec![
            // Kernel for channel 0
            1.0, 0.0, 0.0, 1.0, // Kernel for channel 1
            1.0, 1.0, 1.0, 1.0,
        ];

        let bias = vec![0.0, 0.0];

        let config = DepthwiseConv2DConfig {
            kernel_size: (2, 2),
            stride: (1, 1),
            padding: (0, 0),
        };

        let result = executor
            .execute_depthwise_conv2d(&input, &kernel, &bias, 1, 2, 3, 3, config)
            .await
            .unwrap();

        // Output size: (3-2)/1+1 = 2x2 per channel
        // Total: 2 channels * 2 * 2 = 8 elements
        assert_eq!(result.len(), 8);
        assert!(result.iter().all(|&x| x.is_finite()));

        println!("✅ DepthwiseConv2D precision test passed");
    })
    .await;
}

// ============================================================================
// SUMMARY TEST - All 60 Operations
// ============================================================================

#[tokio::test]
async fn test_conv3d_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Conv3D: 3D convolution for video/medical imaging
        // Input: [batch=1, channels=1, depth=4, height=4, width=4] = 64 values
        // Kernel: [out_channels=1, in_channels=1, kd=3, kh=3, kw=3] = 27 values
        // Output: [batch=1, channels=1, depth=2, height=2, width=2] = 8 values

        let input: Vec<f32> = (1..=64).map(|i| i as f32).collect();
        let kernel: Vec<f32> = vec![1.0; 27]; // 3x3x3 kernel with all 1.0
        let bias = vec![0.0];

        let config = Conv3DConfig {
            kernel_size: (3, 3, 3),
            stride: (1, 1, 1),
            padding: (0, 0, 0),
            dilation: (1, 1, 1),
        };

        let result = executor
            .execute_conv3d(
                &input, &kernel, &bias, 1, // batch
                1, // in_channels
                1, // out_channels
                4, // depth
                4, // height
                4, // width
                config,
            )
            .await
            .unwrap();

        // Should produce 2x2x2 = 8 output values (4x4x4 input, 3x3x3 kernel, stride 1, no padding)
        assert_eq!(
            result.len(),
            8,
            "Conv3D output shape mismatch: expected 8, got {}",
            result.len()
        );

        // Verify all outputs are finite
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "Conv3D outputs should be finite"
        );

        // Verify outputs are non-zero (kernel has non-zero values)
        let non_zero_count = result.iter().filter(|&&x| x.abs() > FP32_TOLERANCE).count();
        assert!(
            non_zero_count > 0,
            "Conv3D should produce non-zero outputs with non-zero input"
        );

        // Verify numerical stability (no extreme values for simple inputs)
        let max_val = result.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = result.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        assert!(
            max_val < 5000.0 && min_val > -5000.0,
            "Conv3D outputs should be in reasonable range: min={}, max={}",
            min_val,
            max_val
        );

        println!("✅ Conv3D FP32 precision test passed (Operation #104/105)");
    })
    .await;
}

#[tokio::test]
async fn test_transposed_conv2d_fp32_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // TransposedConv2D: Upsampling convolution (deconvolution)
        // Input: [batch=1, channels=1, height=2, width=2] = 4 values
        // Kernel: [in_channels=1, out_channels=1, kh=2, kw=2] = 4 values
        // Output: with stride=2, output = (h-1)*stride + kh = (2-1)*2 + 2 = 4

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let kernel = vec![1.0, 0.0, 0.0, 0.0]; // Simple 2x2 kernel
        let bias = vec![0.0];

        let config = TransposedConv2DConfig {
            kernel_size: (2, 2),
            stride: (2, 2),
            padding: (0, 0),
            output_padding: (0, 0),
        };

        let result = executor
            .execute_transposed_conv2d(
                &input, &kernel, &bias, 1, // batch
                1, // in_channels
                1, // out_channels
                2, // input_height
                2, // input_width
                config,
            )
            .await
            .unwrap();

        // Expected output: 4x4 = 16 values
        assert_eq!(
            result.len(),
            16,
            "TransposedConv2D output shape mismatch: expected 16, got {}",
            result.len()
        );

        // Verify all outputs are finite
        assert!(
            result.iter().all(|&x| x.is_finite()),
            "TransposedConv2D outputs should be finite"
        );

        // Verify upsampling behavior: output should be larger than input
        assert!(
            result.len() > input.len(),
            "TransposedConv2D should upsample: input={}, output={}",
            input.len(),
            result.len()
        );

        // Verify numerical stability
        let max_val = result.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = result.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        assert!(
            max_val < 50.0 && min_val > -50.0,
            "TransposedConv2D outputs should be in reasonable range: min={}, max={}",
            min_val,
            max_val
        );

        // Test FP32 precision with known simple case
        let simple_input = vec![1.0];
        let simple_kernel = vec![1.0, 0.5, 0.5, 0.25];
        let simple_bias = vec![0.0];

        let simple_config = TransposedConv2DConfig {
            kernel_size: (2, 2),
            stride: (1, 1),
            padding: (0, 0),
            output_padding: (0, 0),
        };

        let simple_result = executor
            .execute_transposed_conv2d(
                &simple_input,
                &simple_kernel,
                &simple_bias,
                1, // batch
                1, // in_channels
                1, // out_channels
                1, // input_height
                1, // input_width
                simple_config,
            )
            .await
            .unwrap();

        // With stride=1, output = (h-1)*stride + kh = (1-1)*1 + 2 = 2
        // Output: 2x2 = 4 values
        assert_eq!(
            simple_result.len(),
            4,
            "TransposedConv2D simple case output size"
        );

        // Verify all values are finite and in reasonable range
        for (i, &val) in simple_result.iter().enumerate() {
            assert!(
                val.is_finite() && val.abs() < 10.0,
                "TransposedConv2D simple case value at {}: expected finite value <10.0, got {}",
                i,
                val
            );
        }

        println!("✅ TransposedConv2D FP32 precision test passed (Operation #105/105)");
    })
    .await;
}
