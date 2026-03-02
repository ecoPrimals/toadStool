//! BatchMatMul Tests: Batched Matrix Multiplication
//!
//! Tests batched matmul for transformer attention.

#![allow(unused_variables)]

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

#[tokio::test]
async fn test_batch_matmul_simple() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // 2 batches of 2x3 @ 3x2 = 2x2
        let batch_size = 2;
        let m = 2;
        let n = 2;
        let k = 3;

        // Batch 0: [[1,2,3],[4,5,6]] @ [[1,2],[3,4],[5,6]]
        // Batch 1: [[7,8,9],[10,11,12]] @ [[1,2],[3,4],[5,6]]
        let a = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, // batch 0
            7.0, 8.0, 9.0, 10.0, 11.0, 12.0, // batch 1
        ];
        let b = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, // batch 0
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, // batch 1 (same)
        ];

        let result = executor
            .execute_batch_matmul(&a, &b, batch_size, m, n, k)
            .await
            .unwrap();

        assert_eq!(result.len(), batch_size * m * n);

        println!("✅ Simple batch matmul test passed");
    })
    .await;
}

#[tokio::test]
async fn test_batch_matmul_transformer_attention() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Simulate multi-head attention: (batch, heads, seq, seq)
        let batch_size = 4; // 4 attention heads
        let seq_len = 8; // sequence length
        let m = seq_len;
        let n = seq_len;
        let k = seq_len;

        let a: Vec<f32> = (0..batch_size * m * k).map(|i| (i as f32) * 0.01).collect();
        let b: Vec<f32> = (0..batch_size * k * n).map(|i| (i as f32) * 0.01).collect();

        let result = executor
            .execute_batch_matmul(&a, &b, batch_size, m, n, k)
            .await
            .unwrap();

        assert_eq!(result.len(), batch_size * m * n);

        println!("✅ Transformer attention batch matmul test passed");
        println!(
            "   {} heads × {}x{} attention matrices",
            batch_size, seq_len, seq_len
        );
    })
    .await;
}

#[tokio::test]
async fn test_batch_matmul_large_batch() {
    gpu_test_resilient_async(async {
        let executor = create_executor().await;

        // Large batch size (common in training)
        let batch_size = 32;
        let m = 16;
        let n = 16;
        let k = 16;

        let a: Vec<f32> = vec![1.0; batch_size * m * k];
        let b: Vec<f32> = vec![1.0; batch_size * k * n];

        let result = executor
            .execute_batch_matmul(&a, &b, batch_size, m, n, k)
            .await
            .unwrap();

        assert_eq!(result.len(), batch_size * m * n);

        println!("✅ Large batch matmul test passed");
        println!("   Batch size: {}", batch_size);
    })
    .await;
}
