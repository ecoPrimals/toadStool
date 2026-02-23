// Precision tests - Pool
use ml_inference_showcase::wgpu::WgpuExecutor;
use ml_inference_showcase::wgpu::*;

const FP32_TOLERANCE: f32 = 1e-5;

#[tokio::test]
async fn test_maxpool2d_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // MaxPool2D: 2D max pooling with kernel, stride, padding
    // Input: 1 batch, 1 channel, 4x4 spatial
    let input = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ];

    let config = Pool2DConfig {
        kernel_size: (2, 2),
        stride: (2, 2),
        padding: (0, 0),
    };

    // Output should be 2x2 (4x4 -> 2x2 with kernel=2, stride=2)
    let result = executor
        .execute_max_pool_2d(&input, 1, 1, 4, 4, config)
        .await
        .unwrap();

    // Expected: max of each 2x2 window
    // Top-left: max(1,2,5,6) = 6
    // Top-right: max(3,4,7,8) = 8
    // Bottom-left: max(9,10,13,14) = 14
    // Bottom-right: max(11,12,15,16) = 16
    let expected = vec![6.0, 8.0, 14.0, 16.0];

    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "MaxPool2D error at index {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ MaxPool2D precision test passed");
}

// Note: AvgPool2D not yet implemented (would be Gap #29)

// Advanced Pooling (Already Tested)

#[tokio::test]
async fn test_global_avg_pool_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Batch=1, Channels=2, H=2, W=2
    let input = vec![
        // Channel 0
        1.0, 2.0, 3.0, 4.0, // Channel 1
        5.0, 6.0, 7.0, 8.0,
    ];

    let result = executor
        .execute_global_avg_pool(&input, 1, 2, 2, 2)
        .await
        .unwrap();

    // Expected: avg of each channel
    // Channel 0: (1+2+3+4)/4 = 2.5
    // Channel 1: (5+6+7+8)/4 = 6.5
    let expected = vec![2.5, 6.5];

    assert_eq!(result.len(), 2);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "GlobalAvgPool error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ GlobalAvgPool precision test passed");
}

#[tokio::test]
async fn test_global_max_pool_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Batch=1, Channels=2, H=2, W=2
    let input = vec![
        // Channel 0
        1.0, 2.0, 3.0, 4.0, // Channel 1
        5.0, 6.0, 7.0, 8.0,
    ];

    let result = executor
        .execute_global_max_pool(&input, 1, 2, 2, 2)
        .await
        .unwrap();

    // Expected: max of each channel
    // Channel 0: max(1,2,3,4) = 4.0
    // Channel 1: max(5,6,7,8) = 8.0
    let expected = vec![4.0, 8.0];

    assert_eq!(result.len(), 2);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "GlobalMaxPool error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ GlobalMaxPool precision test passed");
}

#[tokio::test]
async fn test_adaptive_avg_pool_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Input: 1x1x4x4, Output: 1x1x2x2
    let input = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ];

    let result = executor
        .execute_adaptive_avg_pool_2d(&input, 1, 1, 4, 4, 2, 2)
        .await
        .unwrap();

    // Expected: average of 2x2 regions
    // Top-left: (1+2+5+6)/4 = 3.5
    // Top-right: (3+4+7+8)/4 = 5.5
    // Bottom-left: (9+10+13+14)/4 = 11.5
    // Bottom-right: (11+12+15+16)/4 = 13.5
    let expected = vec![3.5, 5.5, 11.5, 13.5];

    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "AdaptiveAvgPool error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ AdaptiveAvgPool2D precision test passed");
}

#[tokio::test]
async fn test_adaptive_max_pool_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Input: 1x1x4x4, Output: 1x1x2x2
    let input = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ];

    let result = executor
        .execute_adaptive_max_pool_2d(&input, 1, 1, 4, 4, 2, 2)
        .await
        .unwrap();

    // Expected: max of 2x2 regions
    // Top-left: max(1,2,5,6) = 6
    // Top-right: max(3,4,7,8) = 8
    // Bottom-left: max(9,10,13,14) = 14
    // Bottom-right: max(11,12,15,16) = 16
    let expected = vec![6.0, 8.0, 14.0, 16.0];

    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "AdaptiveMaxPool error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ AdaptiveMaxPool2D precision test passed");
}

// ============================================================================
// NORMALIZATIONS (6 total)
// ============================================================================

// Core Normalizations (Untested - HIGH PRIORITY)
