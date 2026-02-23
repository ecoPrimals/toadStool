// Precision tests - Arithmetic
use ml_inference_showcase::wgpu::{BinaryOp, WgpuExecutor};

const FP32_TOLERANCE: f32 = 1e-5;

#[tokio::test]
async fn test_matmul_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // MatMul: C = A * B
    // A: 2x3, B: 3x2 -> C: 2x2
    let a = vec![
        1.0, 2.0, 3.0, // Row 0
        4.0, 5.0, 6.0, // Row 1
    ];
    let b = vec![
        7.0, 8.0, // Col 0, Col 1
        9.0, 10.0, 11.0, 12.0,
    ];

    let m = 2; // A rows
    let n = 2; // B cols
    let k = 3; // A cols = B rows

    let result = executor.execute_matmul(&a, &b, m, n, k).await.unwrap();

    // Expected: C[0,0] = 1*7 + 2*9 + 3*11 = 7 + 18 + 33 = 58
    //           C[0,1] = 1*8 + 2*10 + 3*12 = 8 + 20 + 36 = 64
    //           C[1,0] = 4*7 + 5*9 + 6*11 = 28 + 45 + 66 = 139
    //           C[1,1] = 4*8 + 5*10 + 6*12 = 32 + 50 + 72 = 154
    let expected = vec![58.0, 64.0, 139.0, 154.0];

    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "MatMul error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ MatMul precision test passed");
}

#[tokio::test]
async fn test_add_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Add: C = alpha * A + B
    let a = vec![1.0, 2.0, 3.0, 4.0];
    let b = vec![5.0, 6.0, 7.0, 8.0];
    let alpha = 2.0;

    let result = executor.execute_add(&a, &b, alpha).await.unwrap();

    // Expected: C = 2.0 * A + B = [2+5, 4+6, 6+7, 8+8] = [7, 10, 13, 16]
    let expected = vec![7.0, 10.0, 13.0, 16.0];

    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Add error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ Add precision test passed");
}

#[tokio::test]
async fn test_elementwise_sub_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Subtract: C = A - B
    let a = vec![10.0, 20.0, 30.0, 40.0];
    let b = vec![1.0, 2.0, 3.0, 4.0];

    let result = executor
        .execute_elementwise_binary(&a, &b, BinaryOp::Sub)
        .await
        .unwrap();

    // Expected: [9, 18, 27, 36]
    let expected = vec![9.0, 18.0, 27.0, 36.0];

    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Sub error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ Elementwise Sub precision test passed");
}

#[tokio::test]
async fn test_elementwise_mul_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Multiply: C = A * B
    let a = vec![2.0, 3.0, 4.0, 5.0];
    let b = vec![10.0, 10.0, 10.0, 10.0];

    let result = executor
        .execute_elementwise_binary(&a, &b, BinaryOp::Mul)
        .await
        .unwrap();

    // Expected: [20, 30, 40, 50]
    let expected = vec![20.0, 30.0, 40.0, 50.0];

    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Mul error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ Elementwise Mul precision test passed");
}

#[tokio::test]
async fn test_elementwise_div_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Divide: C = A / B
    let a = vec![100.0, 200.0, 300.0, 400.0];
    let b = vec![10.0, 20.0, 30.0, 40.0];

    let result = executor
        .execute_elementwise_binary(&a, &b, BinaryOp::Div)
        .await
        .unwrap();

    // Expected: [10, 10, 10, 10]
    let expected = vec![10.0, 10.0, 10.0, 10.0];

    assert_eq!(result.len(), 4);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Div error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ Elementwise Div precision test passed");
}

#[tokio::test]
async fn test_transpose_precision() {
    let executor = WgpuExecutor::new().await.unwrap();

    // Transpose: 2x3 -> 3x2
    let input = vec![
        1.0, 2.0, 3.0, // Row 0
        4.0, 5.0, 6.0, // Row 1
    ];

    let rows = 2;
    let cols = 3;

    let result = executor
        .execute_transpose(&input, rows, cols)
        .await
        .unwrap();

    // Expected: [[1,4], [2,5], [3,6]] = [1, 4, 2, 5, 3, 6]
    let expected = vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0];

    assert_eq!(result.len(), 6);
    for (i, (&out, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (out - exp).abs() < FP32_TOLERANCE,
            "Transpose error at {}: got {}, expected {}",
            i,
            out,
            exp
        );
    }

    println!("✅ Transpose precision test passed");
}

// ============================================================================
// REDUCE OPERATIONS (4 total) - HIGH PRIORITY
// ============================================================================
