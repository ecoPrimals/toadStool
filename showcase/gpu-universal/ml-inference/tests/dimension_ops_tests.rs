//! Dimension Operations Tests: Split, Squeeze, Unsqueeze
//!
//! Tests dimension manipulation operations.
//!
//! **Known NVK limitation**: Under concurrent GPU load, dimension_ops_tests may trigger
//! driver-level SIGSEGV (cannot be caught by catch_unwind). Tests pass individually.

#![allow(unused_variables)]

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

// Split Tests
#[tokio::test]
async fn test_split_simple() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let (output1, output2) = executor.execute_split(&input, 3).await.unwrap();

        assert_eq!(output1, vec![1.0, 2.0, 3.0]);
        assert_eq!(output2, vec![4.0, 5.0, 6.0]);

        println!("✅ Simple split test passed");
    })
    .await;
}

#[tokio::test]
async fn test_split_uneven() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let (output1, output2) = executor.execute_split(&input, 2).await.unwrap();

        assert_eq!(output1, vec![1.0, 2.0]);
        assert_eq!(output2, vec![3.0, 4.0, 5.0, 6.0, 7.0]);

        println!("✅ Uneven split test passed");
    })
    .await;
}

#[tokio::test]
async fn test_split_multi_path() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Simulate splitting features for multi-path network
        let features = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let (path1, path2) = executor.execute_split(&features, 4).await.unwrap();

        assert_eq!(path1.len(), 4);
        assert_eq!(path2.len(), 4);

        println!("✅ Multi-path split test passed");
    })
    .await;
}

// Squeeze Tests
#[tokio::test]
async fn test_squeeze_preserves_data() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = executor.execute_squeeze(&input).await.unwrap();

        assert_eq!(input, output);

        println!("✅ Squeeze preserves data test passed");
    })
    .await;
}

#[tokio::test]
async fn test_squeeze_large_tensor() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let output = executor.execute_squeeze(&input).await.unwrap();

        assert_eq!(input, output);

        println!("✅ Squeeze large tensor test passed");
    })
    .await;
}

// Unsqueeze Tests
#[tokio::test]
async fn test_unsqueeze_preserves_data() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = executor.execute_unsqueeze(&input).await.unwrap();

        assert_eq!(input, output);

        println!("✅ Unsqueeze preserves data test passed");
    })
    .await;
}

#[tokio::test]
async fn test_unsqueeze_broadcasting_prep() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Prepare tensor for broadcasting
        let input = vec![1.0, 2.0, 3.0];
        let output = executor.execute_unsqueeze(&input).await.unwrap();

        assert_eq!(input, output);

        println!("✅ Unsqueeze broadcasting prep test passed");
    })
    .await;
}

#[tokio::test]
async fn test_squeeze_unsqueeze_roundtrip() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let squeezed = executor.execute_squeeze(&input).await.unwrap();
        let unsqueezed = executor.execute_unsqueeze(&squeezed).await.unwrap();

        assert_eq!(input, unsqueezed);

        println!("✅ Squeeze/Unsqueeze roundtrip test passed");
    })
    .await;
}
