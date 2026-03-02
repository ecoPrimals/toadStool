//! Slice Tests: Tensor Slicing
//!
//! Tests slicing operation for extracting tensor sections.

#![allow(unused_variables)]

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

#[tokio::test]
async fn test_slice_simple() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = executor.execute_slice(&input, 2, 5).await.unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result, vec![3.0, 4.0, 5.0]);

        println!("✅ Simple slice test passed");
    })
    .await;
}

#[tokio::test]
async fn test_slice_beginning() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = executor.execute_slice(&input, 0, 3).await.unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result, vec![1.0, 2.0, 3.0]);

        println!("✅ Slice from beginning test passed");
    })
    .await;
}

#[tokio::test]
async fn test_slice_end() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = executor.execute_slice(&input, 3, 5).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result, vec![4.0, 5.0]);

        println!("✅ Slice to end test passed");
    })
    .await;
}

#[tokio::test]
async fn test_slice_single_element() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = executor.execute_slice(&input, 2, 3).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result, vec![3.0]);

        println!("✅ Single element slice test passed");
    })
    .await;
}

#[tokio::test]
async fn test_slice_large_tensor() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let size = 10000;
        let input: Vec<f32> = (0..size).map(|i| i as f32).collect();

        let start = 2000;
        let end = 7000;
        let result = executor.execute_slice(&input, start, end).await.unwrap();

        assert_eq!(result.len(), end - start);

        // Verify contents
        for (i, &val) in result.iter().enumerate() {
            assert_eq!(val, (start + i) as f32);
        }

        println!("✅ Large tensor slice test passed");
        println!("   Extracted {} elements from {} total", result.len(), size);
    })
    .await;
}

#[tokio::test]
async fn test_slice_attention_window() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Simulate extracting attention windows from sequences
        let sequence_length = 128;
        let sequence: Vec<f32> = (0..sequence_length).map(|i| i as f32).collect();

        // Extract window [32..96)
        let window_start = 32;
        let window_end = 96;
        let window = executor
            .execute_slice(&sequence, window_start, window_end)
            .await
            .unwrap();

        assert_eq!(window.len(), 64);

        // Verify window contents
        for (i, &item) in window.iter().enumerate().take(64) {
            assert_eq!(item, (window_start + i) as f32);
        }

        println!("✅ Attention window slice test passed");
    })
    .await;
}
