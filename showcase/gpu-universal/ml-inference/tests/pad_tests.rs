//! Pad Tests: Tensor Padding
//!
//! Tests padding operation for expanding tensors.

#![allow(unused_variables)]

use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

#[tokio::test]
async fn test_pad_zero_simple() {
    let executor = create_executor().await;

    // 2x2 input, pad by 1 on all sides
    let input = vec![1.0, 2.0, 3.0, 4.0];
    let result = executor
        .execute_pad(&input, 2, 2, 1, 1, 1, 1, 0.0)
        .await
        .unwrap();

    // Output should be 4x4
    assert_eq!(result.len(), 16);

    println!("✅ Simple zero padding test passed");
}

#[tokio::test]
async fn test_pad_asymmetric() {
    let executor = create_executor().await;

    // 3x3 input, different padding on each side
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let result = executor
        .execute_pad(&input, 3, 3, 1, 2, 0, 1, 0.0)
        .await
        .unwrap();

    // Output: height = 3+1+2=6, width = 3+0+1=4
    assert_eq!(result.len(), 24);

    println!("✅ Asymmetric padding test passed");
}

#[tokio::test]
async fn test_pad_custom_value() {
    let executor = create_executor().await;

    // 2x2 input, pad with -1.0
    let input = vec![1.0, 2.0, 3.0, 4.0];
    let result = executor
        .execute_pad(&input, 2, 2, 1, 1, 1, 1, -1.0)
        .await
        .unwrap();

    assert_eq!(result.len(), 16);

    // Check corners should be -1.0
    assert_eq!(result[0], -1.0);
    assert_eq!(result[3], -1.0);

    println!("✅ Custom pad value test passed");
}

#[tokio::test]
async fn test_pad_same_padding() {
    let executor = create_executor().await;

    // Simulate "SAME" padding for 3x3 kernel
    let input_h = 5;
    let input_w = 5;
    let input: Vec<f32> = (0..25).map(|i| i as f32).collect();

    // For 3x3 kernel with stride=1, pad=1 gives "same" output size
    let result = executor
        .execute_pad(&input, input_h, input_w, 1, 1, 1, 1, 0.0)
        .await
        .unwrap();

    // Output: 7x7
    assert_eq!(result.len(), 49);

    println!("✅ SAME padding test passed");
    println!("   {}x{} → 7x7 (ready for 3x3 conv)", input_h, input_w);
}
