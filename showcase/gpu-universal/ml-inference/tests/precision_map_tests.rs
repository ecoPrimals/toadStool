// Precision tests - Map
use ml_inference_showcase::wgpu::WgpuExecutor;
use ml_inference_showcase::wgpu::*;

const FP32_TOLERANCE: f32 = 1e-5;

#[tokio::test]
async fn test_dotproduct_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // DotProduct: A · B = sum(a[i] * b[i])
    let a = vec![1.0, 2.0, 3.0, 4.0];
    let b = vec![5.0, 6.0, 7.0, 8.0];

    let result = executor.execute_dot_product(&a, &b).await.unwrap();

    // Expected: 1*5 + 2*6 + 3*7 + 4*8 = 5 + 12 + 21 + 32 = 70
    let expected = 70.0;

    assert!(
        (result - expected).abs() < FP32_TOLERANCE,
        "DotProduct error: got {}, expected {}",
        result,
        expected
    );

    // Test orthogonal vectors (should be 0)
    let c = vec![1.0, 0.0, 0.0, 0.0];
    let d = vec![0.0, 1.0, 0.0, 0.0];
    let result_ortho = executor.execute_dot_product(&c, &d).await.unwrap();
    assert!(
        (result_ortho - 0.0).abs() < FP32_TOLERANCE,
        "Orthogonal vectors should have dot product 0, got {}",
        result_ortho
    );

    // Test parallel vectors (a·a = ||a||²)
    let e = vec![3.0, 4.0];
    let result_parallel = executor.execute_dot_product(&e, &e).await.unwrap();
    let expected_parallel = 3.0 * 3.0 + 4.0 * 4.0; // = 9 + 16 = 25
    assert!(
        (result_parallel - expected_parallel).abs() < FP32_TOLERANCE,
        "Parallel vectors: got {}, expected {}",
        result_parallel,
        expected_parallel
    );

    println!("✅ DotProduct precision test passed");
}

// ============================================================================
// MAP OPERATIONS (5 total) - HIGH PRIORITY
// ============================================================================

#[tokio::test]
async fn test_map_square_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let result = executor.execute_map(&input, MapOp::Square).await.unwrap();

    // Expected: [1, 4, 9, 16, 25]
    let expected = vec![1.0, 4.0, 9.0, 16.0, 25.0];

    assert_eq!(result.len(), expected.len());
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Map Square error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    // Test negative values
    let input_neg = vec![-2.0, -3.0];
    let result_neg = executor
        .execute_map(&input_neg, MapOp::Square)
        .await
        .unwrap();
    assert!((result_neg[0] - 4.0).abs() < FP32_TOLERANCE);
    assert!((result_neg[1] - 9.0).abs() < FP32_TOLERANCE);

    println!("✅ Map Square precision test passed");
}

#[tokio::test]
async fn test_map_sqrt_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    let input = vec![1.0, 4.0, 9.0, 16.0, 25.0];
    let result = executor.execute_map(&input, MapOp::Sqrt).await.unwrap();

    // Expected: [1, 2, 3, 4, 5]
    let expected = vec![1.0, 2.0, 3.0, 4.0, 5.0];

    assert_eq!(result.len(), expected.len());
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Map Sqrt error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    // Test sqrt(0) = 0
    let result_zero = executor.execute_map(&[0.0], MapOp::Sqrt).await.unwrap();
    assert!((result_zero[0] - 0.0).abs() < FP32_TOLERANCE);

    println!("✅ Map Sqrt precision test passed");
}

#[tokio::test]
async fn test_map_abs_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    let input = vec![-5.0, -2.0, 0.0, 3.0, 7.0];
    let result = executor.execute_map(&input, MapOp::Abs).await.unwrap();

    // Expected: [5, 2, 0, 3, 7]
    let expected = vec![5.0, 2.0, 0.0, 3.0, 7.0];

    assert_eq!(result.len(), expected.len());
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Map Abs error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    // Property: abs(x) >= 0
    for &val in &result {
        assert!(val >= 0.0, "Abs should always be non-negative, got {}", val);
    }

    println!("✅ Map Abs precision test passed");
}

#[tokio::test]
async fn test_map_negate_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    let input = vec![1.0, -2.0, 3.0, -4.0, 0.0];
    let result = executor.execute_map(&input, MapOp::Negate).await.unwrap();

    // Expected: [-1, 2, -3, 4, 0]
    let expected = vec![-1.0, 2.0, -3.0, 4.0, 0.0];

    assert_eq!(result.len(), expected.len());
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Map Negate error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    // Property: -(-x) = x (double negation)
    let result2 = executor.execute_map(&result, MapOp::Negate).await.unwrap();
    for (i, (&val, &orig)) in result2.iter().zip(input.iter()).enumerate() {
        assert!(
            (val - orig).abs() < FP32_TOLERANCE,
            "Double negation at {}: got {}, expected {}",
            i,
            val,
            orig
        );
    }

    println!("✅ Map Negate precision test passed");
}

#[tokio::test]
async fn test_map_reciprocal_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    let input = vec![1.0, 2.0, 4.0, 5.0, 10.0];
    let result = executor
        .execute_map(&input, MapOp::Reciprocal)
        .await
        .unwrap();

    // Expected: [1, 0.5, 0.25, 0.2, 0.1]
    let expected = vec![1.0, 0.5, 0.25, 0.2, 0.1];

    assert_eq!(result.len(), expected.len());
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Map Reciprocal error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    // Property: 1/(1/x) = x (double reciprocal)
    let result2 = executor
        .execute_map(&result, MapOp::Reciprocal)
        .await
        .unwrap();
    for (i, (&val, &orig)) in result2.iter().zip(input.iter()).enumerate() {
        assert!(
            (val - orig).abs() < FP32_TOLERANCE * 10.0, // Allow more tolerance for double operation
            "Double reciprocal at {}: got {}, expected {}",
            i,
            val,
            orig
        );
    }

    println!("✅ Map Reciprocal precision test passed");
}

// ============================================================================
// SCAN OPERATION (1 operation) - Prefix Sum
// ============================================================================

#[tokio::test]
async fn test_scan_sum_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Scan (prefix sum): [1, 2, 3, 4] -> [1, 3, 6, 10]
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let exclusive = false; // Inclusive scan
    let result = executor
        .execute_scan(&input, ScanOp::Sum, exclusive)
        .await
        .unwrap();

    // Expected: [1, 1+2, 1+2+3, 1+2+3+4, 1+2+3+4+5] = [1, 3, 6, 10, 15]
    let expected = vec![1.0, 3.0, 6.0, 10.0, 15.0];

    assert_eq!(result.len(), expected.len());
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Scan Sum error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    // Property: Each element should be >= previous element for positive inputs
    for i in 1..result.len() {
        assert!(
            result[i] >= result[i - 1],
            "Scan Sum should be non-decreasing for positive inputs"
        );
    }

    println!("✅ Scan Sum precision test passed");
}

// ============================================================================
// GATHER OPERATION (1 operation) - Index-based selection
// ============================================================================
