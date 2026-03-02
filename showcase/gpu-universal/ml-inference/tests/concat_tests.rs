//! Concat Tests: Tensor Concatenation
//!
//! Tests concatenation operation for joining tensors.

#![allow(unused_variables)]

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

#[tokio::test]
async fn test_concat_simple() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input1 = vec![1.0, 2.0, 3.0];
        let input2 = vec![4.0, 5.0, 6.0];

        let result = executor.execute_concat(&input1, &input2).await.unwrap();

        assert_eq!(result.len(), 6);
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        println!("✅ Simple concat test passed");
    })
    .await;
}

#[tokio::test]
async fn test_concat_different_sizes() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input1 = vec![1.0, 2.0];
        let input2 = vec![3.0, 4.0, 5.0, 6.0];

        let result = executor.execute_concat(&input1, &input2).await.unwrap();

        assert_eq!(result.len(), 6);
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        println!("✅ Different sizes concat test passed");
    })
    .await;
}

#[tokio::test]
async fn test_concat_single_elements() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let input1 = vec![42.0];
        let input2 = vec![99.0];

        let result = executor.execute_concat(&input1, &input2).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result, vec![42.0, 99.0]);

        println!("✅ Single element concat test passed");
    })
    .await;
}

#[tokio::test]
async fn test_concat_large_tensors() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        let size1 = 1000;
        let size2 = 1500;

        let input1: Vec<f32> = (0..size1).map(|i| i as f32).collect();
        let input2: Vec<f32> = (0..size2).map(|i| (size1 + i) as f32).collect();

        let result = executor.execute_concat(&input1, &input2).await.unwrap();

        assert_eq!(result.len(), size1 + size2);

        // Verify first part matches input1
        for i in 0..size1 {
            assert_eq!(result[i], input1[i]);
        }

        // Verify second part matches input2
        for i in 0..size2 {
            assert_eq!(result[size1 + i], input2[i]);
        }

        println!("✅ Large tensor concat test passed");
        println!(
            "   Concatenated {} + {} = {} elements",
            size1,
            size2,
            result.len()
        );
    })
    .await;
}

#[tokio::test]
async fn test_concat_feature_maps() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Simulate concatenating feature maps (common in U-Net, DenseNet)
        // Two "batches" of features
        let features1 = vec![1.0, 2.0, 3.0, 4.0]; // 2x2 feature map
        let features2 = vec![5.0, 6.0, 7.0, 8.0]; // 2x2 feature map

        let result = executor
            .execute_concat(&features1, &features2)
            .await
            .unwrap();

        assert_eq!(result.len(), 8);

        // Verify concatenation preserves order
        for i in 0..4 {
            assert_eq!(result[i], features1[i]);
            assert_eq!(result[4 + i], features2[i]);
        }

        println!("✅ Feature map concat test passed");
    })
    .await;
}

#[tokio::test]
async fn test_concat_numerical_stability() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Test with various value ranges
        let input1 = vec![1e-10, 1e-5, 1.0, 1e5, 1e10];
        let input2 = vec![-1e10, -1e5, -1.0, -1e-5, -1e-10];

        let result = executor.execute_concat(&input1, &input2).await.unwrap();

        assert_eq!(result.len(), 10);

        // Verify all values preserved accurately
        for i in 0..5 {
            assert_eq!(result[i], input1[i]);
            assert_eq!(result[5 + i], input2[i]);
        }

        println!("✅ Numerical stability test passed");
    })
    .await;
}
