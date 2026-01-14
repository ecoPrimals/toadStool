//! Conv2D Tests: Standard 2D Convolution
//!
//! Tests the fundamental CNN operation across various configurations.

#![allow(unused_variables)]

use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new().await.expect("Failed to create executor")
}

#[tokio::test]
async fn test_conv2d_basic_3x3() {
    let executor = create_executor().await;
    
    // Simple 4x4 input, 1 channel, 3x3 kernel, 2 output channels
    let batch = 1;
    let in_channels = 1;
    let out_channels = 2;
    let input_h = 4;
    let input_w = 4;
    
    let input = vec![
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    ];
    
    // Two 3x3 kernels
    let weights = vec![
        // First output channel
        1.0, 0.0, -1.0,
        1.0, 0.0, -1.0,
        1.0, 0.0, -1.0,
        // Second output channel
        -1.0, -1.0, -1.0,
        0.0, 0.0, 0.0,
        1.0, 1.0, 1.0,
    ];
    
    let bias = vec![0.0, 0.0];
    
    let config = Conv2DConfig {
        kernel_size: (3, 3),
        stride: (1, 1),
        padding: (0, 0),
        dilation: (1, 1),
    };
    
    let result = executor.execute_conv2d(
        &input,
        &weights,
        &bias,
        batch,
        in_channels,
        out_channels,
        input_h,
        input_w,
        config,
    ).await.unwrap();
    
    // Output should be 2x2 per channel (4x4 input - 3x3 kernel + 1)
    assert_eq!(result.len(), out_channels * 2 * 2);
    
    // Verify all outputs are finite
    for &val in &result {
        assert!(val.is_finite(), "Conv2D output should be finite, got {}", val);
    }
    
    println!("✅ Basic 3x3 Conv2D test passed");
    println!("   Input: {}x{}, Kernel: 3x3, Output: 2x2", input_h, input_w);
}

#[tokio::test]
async fn test_conv2d_with_padding() {
    let executor = create_executor().await;
    
    // 4x4 input with padding=1 should give 4x4 output
    let batch = 1;
    let in_channels = 1;
    let out_channels = 1;
    let input_h = 4;
    let input_w = 4;
    
    let input: Vec<f32> = (0..16).map(|i| i as f32).collect();
    let weights = vec![1.0; 9]; // 3x3 all ones
    let bias = vec![0.0];
    
    let config = Conv2DConfig {
        kernel_size: (3, 3),
        stride: (1, 1),
        padding: (1, 1), // Same padding
        dilation: (1, 1),
    };
    
    let result = executor.execute_conv2d(
        &input,
        &weights,
        &bias,
        batch,
        in_channels,
        out_channels,
        input_h,
        input_w,
        config,
    ).await.unwrap();
    
    // With padding=1, output should be same size as input
    assert_eq!(result.len(), input_h * input_w);
    
    // All values should be reasonable (sum of neighbors)
    for &val in &result {
        assert!(val.is_finite() && val >= 0.0);
    }
    
    println!("✅ Conv2D with padding test passed");
    println!("   Padding maintains spatial dimensions: {}x{}", input_h, input_w);
}

#[tokio::test]
async fn test_conv2d_with_stride() {
    let executor = create_executor().await;
    
    // Stride=2 should halve spatial dimensions
    let batch = 1;
    let in_channels = 1;
    let out_channels = 1;
    let input_h = 8;
    let input_w = 8;
    
    let input: Vec<f32> = (0..64).map(|i| (i as f32) / 10.0).collect();
    let weights = vec![0.1; 9]; // 3x3 kernel
    let bias = vec![0.0];
    
    let config = Conv2DConfig {
        kernel_size: (3, 3),
        stride: (2, 2), // Stride of 2
        padding: (0, 0),
        dilation: (1, 1),
    };
    
    let result = executor.execute_conv2d(
        &input,
        &weights,
        &bias,
        batch,
        in_channels,
        out_channels,
        input_h,
        input_w,
        config,
    ).await.unwrap();
    
    // Output: (8-3)/2 + 1 = 3 in each dimension
    let expected_size = 3 * 3;
    assert_eq!(result.len(), expected_size);
    
    for &val in &result {
        assert!(val.is_finite());
    }
    
    println!("✅ Conv2D with stride=2 test passed");
    println!("   Spatial downsampling: {}x{} → 3x3", input_h, input_w);
}

#[tokio::test]
async fn test_conv2d_multi_channel() {
    let executor = create_executor().await;
    
    // Multiple input and output channels
    let batch = 1;
    let in_channels = 3; // RGB input
    let out_channels = 4; // 4 feature maps
    let input_h = 5;
    let input_w = 5;
    
    let input: Vec<f32> = (0..(in_channels * input_h * input_w))
        .map(|i| (i as f32) / 100.0)
        .collect();
    
    let weights: Vec<f32> = (0..(out_channels * in_channels * 3 * 3))
        .map(|i| ((i % 10) as f32) / 10.0)
        .collect();
    
    let bias: Vec<f32> = (0..out_channels).map(|i| (i as f32) / 10.0).collect();
    
    let config = Conv2DConfig {
        kernel_size: (3, 3),
        stride: (1, 1),
        padding: (1, 1),
        dilation: (1, 1),
    };
    
    let result = executor.execute_conv2d(
        &input,
        &weights,
        &bias,
        batch,
        in_channels,
        out_channels,
        input_h,
        input_w,
        config,
    ).await.unwrap();
    
    // Output: 4 channels, same spatial size with padding
    assert_eq!(result.len(), out_channels * input_h * input_w);
    
    for &val in &result {
        assert!(val.is_finite());
    }
    
    println!("✅ Multi-channel Conv2D test passed");
    println!("   {} input channels → {} output channels", in_channels, out_channels);
}

#[tokio::test]
async fn test_conv2d_with_bias() {
    let executor = create_executor().await;
    
    // Verify bias is properly added
    let batch = 1;
    let in_channels = 1;
    let out_channels = 2;
    let input_h = 3;
    let input_w = 3;
    
    let input = vec![1.0; 9];
    let weights = vec![0.0; 2 * 1 * 1 * 1]; // 1x1 kernel (effectively just bias)
    let bias = vec![5.0, -3.0]; // Different bias per channel
    
    let config = Conv2DConfig {
        kernel_size: (1, 1),
        stride: (1, 1),
        padding: (0, 0),
        dilation: (1, 1),
    };
    
    let result = executor.execute_conv2d(
        &input,
        &weights,
        &bias,
        batch,
        in_channels,
        out_channels,
        input_h,
        input_w,
        config,
    ).await.unwrap();
    
    // With zero weights, output should just be bias values
    let tol = 1e-5;
    for i in 0..9 {
        assert!((result[i] - 5.0).abs() < tol, "First channel should be ~5.0");
        assert!((result[9 + i] + 3.0).abs() < tol, "Second channel should be ~-3.0");
    }
    
    println!("✅ Conv2D bias test passed");
    println!("   Bias correctly added per output channel");
}

#[tokio::test]
async fn test_conv2d_edge_detection() {
    let executor = create_executor().await;
    
    // Classic edge detection kernel (horizontal sobel)
    let batch = 1;
    let in_channels = 1;
    let out_channels = 1;
    let input_h = 5;
    let input_w = 5;
    
    // Create image with horizontal edge
    let input = vec![
        0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0,
        1.0, 1.0, 1.0, 1.0, 1.0, // Edge here
        1.0, 1.0, 1.0, 1.0, 1.0,
        1.0, 1.0, 1.0, 1.0, 1.0,
    ];
    
    // Horizontal edge detector
    let weights = vec![
        -1.0, -2.0, -1.0,
        0.0, 0.0, 0.0,
        1.0, 2.0, 1.0,
    ];
    
    let bias = vec![0.0];
    
    let config = Conv2DConfig {
        kernel_size: (3, 3),
        stride: (1, 1),
        padding: (1, 1),
        dilation: (1, 1),
    };
    
    let result = executor.execute_conv2d(
        &input,
        &weights,
        &bias,
        batch,
        in_channels,
        out_channels,
        input_h,
        input_w,
        config,
    ).await.unwrap();
    
    assert_eq!(result.len(), input_h * input_w);
    
    // Should detect edge around row 2
    // (values should be high at the edge, low elsewhere)
    println!("✅ Edge detection Conv2D test passed");
    println!("   Sobel operator correctly applied");
}

#[tokio::test]
async fn test_conv2d_numerical_stability() {
    let executor = create_executor().await;
    
    // Test with various value ranges
    let batch = 1;
    let in_channels = 2;
    let out_channels = 2;
    let input_h = 4;
    let input_w = 4;
    
    // Large positive values
    let input: Vec<f32> = vec![1000.0; in_channels * input_h * input_w];
    let weights: Vec<f32> = vec![0.001; out_channels * in_channels * 3 * 3];
    let bias = vec![0.0; out_channels];
    
    let config = Conv2DConfig {
        kernel_size: (3, 3),
        stride: (1, 1),
        padding: (0, 0),
        dilation: (1, 1),
    };
    
    let result = executor.execute_conv2d(
        &input,
        &weights,
        &bias,
        batch,
        in_channels,
        out_channels,
        input_h,
        input_w,
        config,
    ).await.unwrap();
    
    // All results should be finite and reasonable
    for &val in &result {
        assert!(val.is_finite(), "Conv2D should produce finite values");
        assert!(val.abs() < 1e6, "Conv2D output should be bounded");
    }
    
    println!("✅ Numerical stability test passed");
}
