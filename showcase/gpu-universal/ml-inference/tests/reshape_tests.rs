// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reshape Tests: Tensor Reshaping
//!
//! Tests reshape operation for dimension manipulation.

#![allow(unused_variables)]

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

#[tokio::test]
async fn test_reshape_1d_to_2d() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // 6 elements: reshape from [6] to [2, 3]
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = executor.execute_reshape(&input, &[2, 3]).await.unwrap();

        assert_eq!(result.len(), 6);
        assert_eq!(result, input); // Data unchanged, just shape

        println!("✅ 1D to 2D reshape test passed");
    })
    .await;
}

#[tokio::test]
async fn test_reshape_2d_to_1d() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Flatten 2x3 to 6
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = executor.execute_reshape(&input, &[6]).await.unwrap();

        assert_eq!(result.len(), 6);
        assert_eq!(result, input);

        println!("✅ 2D to 1D (flatten) reshape test passed");
    })
    .await;
}

#[tokio::test]
async fn test_reshape_3d() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // 24 elements: reshape to [2, 3, 4]
        let input: Vec<f32> = (0..24).map(|i| i as f32).collect();
        let result = executor.execute_reshape(&input, &[2, 3, 4]).await.unwrap();

        assert_eq!(result.len(), 24);
        assert_eq!(result, input);

        println!("✅ 3D reshape test passed");
    })
    .await;
}

#[tokio::test]
async fn test_reshape_batch_dimension() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Common ML use case: [batch * features] to [batch, features]
        let batch_size = 32;
        let features = 128;
        let total = batch_size * features;

        let input: Vec<f32> = (0..total).map(|i| i as f32).collect();
        let result = executor
            .execute_reshape(&input, &[batch_size, features])
            .await
            .unwrap();

        assert_eq!(result.len(), total);

        // Verify data integrity
        for i in 0..total {
            assert_eq!(result[i], input[i]);
        }

        println!("✅ Batch dimension reshape test passed");
        println!("   {} elements → [{}, {}]", total, batch_size, features);
    })
    .await;
}

#[tokio::test]
async fn test_reshape_preserve_data() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Ensure data is preserved exactly
        let input = vec![1.5, 2.7, 3.9, 4.2, 5.1, 6.8, 7.3, 8.6];
        let result = executor.execute_reshape(&input, &[2, 4]).await.unwrap();

        assert_eq!(result.len(), 8);

        // Every value must match exactly
        for (i, (&a, &b)) in input.iter().zip(result.iter()).enumerate() {
            assert_eq!(a, b, "Mismatch at index {}", i);
        }

        println!("✅ Data preservation test passed");
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "input size")]
async fn test_reshape_invalid_size() {
    let executor = create_executor().await;

    // 6 elements cannot reshape to [2, 4] = 8 elements
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let _result = executor.execute_reshape(&input, &[2, 4]).await.unwrap();
}
