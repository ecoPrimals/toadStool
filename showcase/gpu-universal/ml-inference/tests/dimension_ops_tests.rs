//! Dimension Operations Tests: Split, Squeeze, Unsqueeze
//!
//! Tests dimension manipulation operations.

#![allow(unused_variables)]

use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

// Split Tests
#[tokio::test]
async fn test_split_simple() {
    let executor = create_executor().await;

    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let (output1, output2) = executor.execute_split(&input, 3).await.unwrap();

    assert_eq!(output1, vec![1.0, 2.0, 3.0]);
    assert_eq!(output2, vec![4.0, 5.0, 6.0]);

    println!("✅ Simple split test passed");
}

#[tokio::test]
async fn test_split_uneven() {
    let executor = create_executor().await;

    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let (output1, output2) = executor.execute_split(&input, 2).await.unwrap();

    assert_eq!(output1, vec![1.0, 2.0]);
    assert_eq!(output2, vec![3.0, 4.0, 5.0, 6.0, 7.0]);

    println!("✅ Uneven split test passed");
}

#[tokio::test]
async fn test_split_multi_path() {
    let executor = create_executor().await;

    // Simulate splitting features for multi-path network
    let features = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let (path1, path2) = executor.execute_split(&features, 4).await.unwrap();

    assert_eq!(path1.len(), 4);
    assert_eq!(path2.len(), 4);

    println!("✅ Multi-path split test passed");
}

// Squeeze Tests
#[tokio::test]
async fn test_squeeze_preserves_data() {
    let executor = create_executor().await;

    let input = vec![1.0, 2.0, 3.0, 4.0];
    let output = executor.execute_squeeze(&input).await.unwrap();

    assert_eq!(input, output);

    println!("✅ Squeeze preserves data test passed");
}

#[tokio::test]
async fn test_squeeze_large_tensor() {
    let executor = create_executor().await;

    let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
    let output = executor.execute_squeeze(&input).await.unwrap();

    assert_eq!(input, output);

    println!("✅ Squeeze large tensor test passed");
}

// Unsqueeze Tests
#[tokio::test]
async fn test_unsqueeze_preserves_data() {
    let executor = create_executor().await;

    let input = vec![1.0, 2.0, 3.0, 4.0];
    let output = executor.execute_unsqueeze(&input).await.unwrap();

    assert_eq!(input, output);

    println!("✅ Unsqueeze preserves data test passed");
}

#[tokio::test]
async fn test_unsqueeze_broadcasting_prep() {
    let executor = create_executor().await;

    // Prepare tensor for broadcasting
    let input = vec![1.0, 2.0, 3.0];
    let output = executor.execute_unsqueeze(&input).await.unwrap();

    assert_eq!(input, output);

    println!("✅ Unsqueeze broadcasting prep test passed");
}

#[tokio::test]
async fn test_squeeze_unsqueeze_roundtrip() {
    let executor = create_executor().await;

    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let squeezed = executor.execute_squeeze(&input).await.unwrap();
    let unsqueezed = executor.execute_unsqueeze(&squeezed).await.unwrap();

    assert_eq!(input, unsqueezed);

    println!("✅ Squeeze/Unsqueeze roundtrip test passed");
}
