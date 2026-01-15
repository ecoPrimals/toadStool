//! Conv3D Tests: 3D Convolution
//!
//! Tests 3D convolution for video/medical imaging.

#![allow(unused_variables)]

use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

#[tokio::test]
async fn test_conv3d_basic_3x3x3() {
    let executor = create_executor().await;

    // Simple 3x3x3 convolution
    let batch = 1;
    let in_channels = 1;
    let out_channels = 1;
    let input_d = 4;
    let input_h = 4;
    let input_w = 4;

    // Input: 4x4x4 volume
    let input: Vec<f32> = (0..64).map(|i| i as f32).collect();

    // 3x3x3 kernel
    let weights: Vec<f32> = vec![1.0; 27];
    let bias = vec![0.0];

    let config = Conv3DConfig {
        kernel_size: (3, 3, 3),
        stride: (1, 1, 1),
        padding: (0, 0, 0),
        dilation: (1, 1, 1),
    };

    let result = executor
        .execute_conv3d(
            &input,
            &weights,
            &bias,
            batch,
            in_channels,
            out_channels,
            input_d,
            input_h,
            input_w,
            config,
        )
        .await
        .unwrap();

    // Output should be 2x2x2
    let expected_size = 2 * 2 * 2;
    assert_eq!(result.len(), expected_size);

    println!("✅ Basic 3x3x3 conv3d test passed");
    println!(
        "   Input: {}x{}x{} → Output: 2x2x2",
        input_d, input_h, input_w
    );
}

#[tokio::test]
async fn test_conv3d_with_padding() {
    let executor = create_executor().await;

    // Conv3D with padding
    let batch = 1;
    let in_channels = 1;
    let out_channels = 1;
    let input_d = 4;
    let input_h = 4;
    let input_w = 4;

    let input: Vec<f32> = vec![1.0; 64];
    let weights: Vec<f32> = vec![0.1; 27]; // 3x3x3
    let bias = vec![0.0];

    let config = Conv3DConfig {
        kernel_size: (3, 3, 3),
        stride: (1, 1, 1),
        padding: (1, 1, 1),
        dilation: (1, 1, 1),
    };

    let result = executor
        .execute_conv3d(
            &input,
            &weights,
            &bias,
            batch,
            in_channels,
            out_channels,
            input_d,
            input_h,
            input_w,
            config,
        )
        .await
        .unwrap();

    // With padding=1, output should be same size as input
    let expected_size = 4 * 4 * 4;
    assert_eq!(result.len(), expected_size);

    println!("✅ Conv3D with padding test passed");
}

#[tokio::test]
async fn test_conv3d_multi_channel() {
    let executor = create_executor().await;

    // Multi-channel 3D convolution
    let batch = 1;
    let in_channels = 2;
    let out_channels = 3;
    let input_d = 4;
    let input_h = 4;
    let input_w = 4;

    let input: Vec<f32> = (0..2 * 64).map(|i| (i as f32) * 0.1).collect();
    let weights: Vec<f32> = (0..2 * 3 * 27).map(|i| (i as f32) * 0.01).collect();
    let bias = vec![0.0, 0.0, 0.0];

    let config = Conv3DConfig {
        kernel_size: (3, 3, 3),
        stride: (1, 1, 1),
        padding: (0, 0, 0),
        dilation: (1, 1, 1),
    };

    let result = executor
        .execute_conv3d(
            &input,
            &weights,
            &bias,
            batch,
            in_channels,
            out_channels,
            input_d,
            input_h,
            input_w,
            config,
        )
        .await
        .unwrap();

    // Output: 3 channels × 2×2×2
    let expected_size = 3 * 2 * 2 * 2;
    assert_eq!(result.len(), expected_size);

    println!("✅ Multi-channel conv3d test passed");
    println!("   {} channels → {} channels", in_channels, out_channels);
}

#[tokio::test]
async fn test_conv3d_video_classification() {
    let executor = create_executor().await;

    // Simulate video classification (temporal + spatial)
    let batch = 1;
    let in_channels = 3; // RGB
    let out_channels = 16;
    let input_d = 8; // 8 frames
    let input_h = 16; // 16x16 spatial
    let input_w = 16;

    let input: Vec<f32> = (0..3 * 8 * 16 * 16).map(|i| (i as f32) * 0.001).collect();
    let weights: Vec<f32> = (0..3 * 16 * 3 * 3 * 3)
        .map(|i| (i as f32) * 0.0001)
        .collect();
    let bias: Vec<f32> = vec![0.0; 16];

    let config = Conv3DConfig {
        kernel_size: (3, 3, 3),
        stride: (1, 1, 1),
        padding: (1, 1, 1),
        dilation: (1, 1, 1),
    };

    let result = executor
        .execute_conv3d(
            &input,
            &weights,
            &bias,
            batch,
            in_channels,
            out_channels,
            input_d,
            input_h,
            input_w,
            config,
        )
        .await
        .unwrap();

    // Output: 16 channels × 8×16×16 (same size with padding)
    let expected_size = 16 * 8 * 16 * 16;
    assert_eq!(result.len(), expected_size);

    println!("✅ Video classification conv3d test passed");
    println!(
        "   Video: {}x{}x{} ({}ch) → {}x{}x{} ({}ch)",
        input_d, input_h, input_w, in_channels, 8, 16, 16, out_channels
    );
}

#[tokio::test]
async fn test_conv3d_medical_imaging() {
    let executor = create_executor().await;

    // Simulate medical imaging (CT/MRI volume processing)
    let batch = 1;
    let in_channels = 1; // Grayscale medical scan
    let out_channels = 8;
    let input_d = 16; // 16 slices
    let input_h = 32; // 32x32 spatial
    let input_w = 32;

    let input: Vec<f32> = (0..16 * 32 * 32).map(|i| (i as f32) * 0.01).collect();
    let weights: Vec<f32> = (0..8 * 3 * 3 * 3).map(|i| (i as f32) * 0.1).collect();
    let bias: Vec<f32> = vec![0.0; 8];

    let config = Conv3DConfig {
        kernel_size: (3, 3, 3),
        stride: (2, 2, 2), // Downsample
        padding: (1, 1, 1),
        dilation: (1, 1, 1),
    };

    let result = executor
        .execute_conv3d(
            &input,
            &weights,
            &bias,
            batch,
            in_channels,
            out_channels,
            input_d,
            input_h,
            input_w,
            config,
        )
        .await
        .unwrap();

    // Output: 8 channels × 8×16×16 (downsampled by 2)
    let expected_size = 8 * 8 * 16 * 16;
    assert_eq!(result.len(), expected_size);

    println!("✅ Medical imaging conv3d test passed");
    println!(
        "   CT/MRI: {}x{}x{} → {}x{}x{} ({}ch)",
        input_d, input_h, input_w, 8, 16, 16, out_channels
    );
}
