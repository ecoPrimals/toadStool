// Precision tests - Reduction
use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::WgpuExecutor;
use ml_inference_showcase::wgpu::*;

const FP32_TOLERANCE: f32 = 1e-5;

#[tokio::test]
async fn test_reduce_sum_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = executor
            .execute_reduce(&input, ReduceOp::Sum)
            .await
            .unwrap();

        // Expected: 1+2+3+4+5+6+7+8+9+10 = 55
        let expected = 55.0;

        assert!(
            (result - expected).abs() < FP32_TOLERANCE,
            "Reduce Sum error: got {}, expected {}",
            result,
            expected
        );

        // Test empty behavior is safe
        assert!(result.is_finite(), "Result should be finite");

        println!("✅ Reduce Sum precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_reduce_max_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![3.5, 1.2, 9.8, 2.1, 5.5, 7.3, 4.9, 8.1];
        let result = executor
            .execute_reduce(&input, ReduceOp::Max)
            .await
            .unwrap();

        // Expected: 9.8 (maximum value)
        let expected = 9.8;

        assert!(
            (result - expected).abs() < FP32_TOLERANCE,
            "Reduce Max error: got {}, expected {}",
            result,
            expected
        );

        // Test with negative values
        let input_neg = vec![-5.0, -1.0, -10.0, -3.0];
        let result_neg = executor
            .execute_reduce(&input_neg, ReduceOp::Max)
            .await
            .unwrap();
        assert!(
            (result_neg - (-1.0)).abs() < FP32_TOLERANCE,
            "Reduce Max with negatives: got {}, expected -1.0",
            result_neg
        );

        println!("✅ Reduce Max precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_reduce_min_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![3.5, 1.2, 9.8, 2.1, 5.5, 7.3, 4.9, 8.1];
        let result = executor
            .execute_reduce(&input, ReduceOp::Min)
            .await
            .unwrap();

        // Expected: 1.2 (minimum value)
        let expected = 1.2;

        assert!(
            (result - expected).abs() < FP32_TOLERANCE,
            "Reduce Min error: got {}, expected {}",
            result,
            expected
        );

        // Test with negative values
        let input_neg = vec![-5.0, -1.0, -10.0, -3.0];
        let result_neg = executor
            .execute_reduce(&input_neg, ReduceOp::Min)
            .await
            .unwrap();
        assert!(
            (result_neg - (-10.0)).abs() < FP32_TOLERANCE,
            "Reduce Min with negatives: got {}, expected -10.0",
            result_neg
        );

        println!("✅ Reduce Min precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_reduce_mean_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let input = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let result = executor
            .execute_reduce(&input, ReduceOp::Mean)
            .await
            .unwrap();

        // Expected: (2+4+6+8+10)/5 = 30/5 = 6.0
        let expected = 6.0;

        assert!(
            (result - expected).abs() < FP32_TOLERANCE,
            "Reduce Mean error: got {}, expected {}",
            result,
            expected
        );

        // Test with non-trivial mean
        let input2 = vec![1.5, 2.5, 3.5, 4.5];
        let result2 = executor
            .execute_reduce(&input2, ReduceOp::Mean)
            .await
            .unwrap();
        let expected2 = 3.0; // (1.5+2.5+3.5+4.5)/4 = 12/4 = 3.0
        assert!(
            (result2 - expected2).abs() < FP32_TOLERANCE,
            "Reduce Mean error: got {}, expected {}",
            result2,
            expected2
        );

        println!("✅ Reduce Mean precision test passed");
    })
    .await;
}

// ============================================================================
// DOT PRODUCT (1 operation) - HIGH PRIORITY
// ============================================================================
