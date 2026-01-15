//! Embedding Tests: Token Embedding Lookups
//!
//! Tests embedding operation for NLP.

#![allow(unused_variables)]

use ml_inference_showcase::wgpu::*;

async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new()
        .await
        .expect("Failed to create executor")
}

#[tokio::test]
async fn test_embedding_simple() {
    let executor = create_executor().await;

    // Simple embedding lookup
    let batch_size = 1;
    let seq_length = 4;
    let embedding_dim = 3;
    let vocab_size = 10;

    // Token indices: [0, 2, 5, 9]
    let indices = vec![0u32, 2, 5, 9];

    // Embedding table: 10 tokens × 3 dimensions
    let weight: Vec<f32> = (0..vocab_size * embedding_dim).map(|i| i as f32).collect();

    let result = executor
        .execute_embedding(
            &indices,
            &weight,
            batch_size,
            seq_length,
            embedding_dim,
            vocab_size,
        )
        .await
        .unwrap();

    assert_eq!(result.len(), batch_size * seq_length * embedding_dim);

    // Verify first embedding (index 0)
    assert_eq!(result[0], 0.0);
    assert_eq!(result[1], 1.0);
    assert_eq!(result[2], 2.0);

    // Verify second embedding (index 2)
    assert_eq!(result[3], 6.0); // 2 * 3 = 6
    assert_eq!(result[4], 7.0);
    assert_eq!(result[5], 8.0);

    println!("✅ Simple embedding test passed");
}

#[tokio::test]
async fn test_embedding_batch() {
    let executor = create_executor().await;

    // Batch of sequences
    let batch_size = 2;
    let seq_length = 3;
    let embedding_dim = 4;
    let vocab_size = 100;

    let indices = vec![0u32, 1, 2, 3, 4, 5]; // 2 sequences of 3 tokens

    let weight: Vec<f32> = (0..vocab_size * embedding_dim)
        .map(|i| (i as f32) * 0.1)
        .collect();

    let result = executor
        .execute_embedding(
            &indices,
            &weight,
            batch_size,
            seq_length,
            embedding_dim,
            vocab_size,
        )
        .await
        .unwrap();

    assert_eq!(result.len(), batch_size * seq_length * embedding_dim);

    println!("✅ Batch embedding test passed");
    println!(
        "   {} sequences × {} tokens = {} total",
        batch_size,
        seq_length,
        batch_size * seq_length
    );
}

#[tokio::test]
async fn test_embedding_bert_size() {
    let executor = create_executor().await;

    // BERT-like embedding dimensions
    let batch_size = 8;
    let seq_length = 128;
    let embedding_dim = 768;
    let vocab_size = 30000;

    // Random token indices
    let indices: Vec<u32> = (0..batch_size * seq_length)
        .map(|i| (i % vocab_size) as u32)
        .collect();

    let weight: Vec<f32> = (0..vocab_size * embedding_dim)
        .map(|i| ((i % 1000) as f32) * 0.001)
        .collect();

    let result = executor
        .execute_embedding(
            &indices,
            &weight,
            batch_size,
            seq_length,
            embedding_dim,
            vocab_size,
        )
        .await
        .unwrap();

    assert_eq!(result.len(), batch_size * seq_length * embedding_dim);

    println!("✅ BERT-scale embedding test passed");
    println!(
        "   Batch: {}, SeqLen: {}, EmbedDim: {}",
        batch_size, seq_length, embedding_dim
    );
}

#[tokio::test]
async fn test_embedding_preserve_order() {
    let executor = create_executor().await;

    // Verify order preservation
    let batch_size = 1;
    let seq_length = 5;
    let embedding_dim = 2;
    let vocab_size = 5;

    // Indices in specific order
    let indices = vec![4u32, 3, 2, 1, 0];

    // Simple weight table
    let weight = vec![
        10.0, 11.0, // token 0
        20.0, 21.0, // token 1
        30.0, 31.0, // token 2
        40.0, 41.0, // token 3
        50.0, 51.0, // token 4
    ];

    let result = executor
        .execute_embedding(
            &indices,
            &weight,
            batch_size,
            seq_length,
            embedding_dim,
            vocab_size,
        )
        .await
        .unwrap();

    // Verify embeddings appear in correct order
    assert_eq!(result[0], 50.0); // token 4
    assert_eq!(result[1], 51.0);
    assert_eq!(result[2], 40.0); // token 3
    assert_eq!(result[3], 41.0);
    assert_eq!(result[8], 10.0); // token 0
    assert_eq!(result[9], 11.0);

    println!("✅ Order preservation test passed");
}

#[tokio::test]
async fn test_embedding_repeated_tokens() {
    let executor = create_executor().await;

    // Test same token appearing multiple times
    let batch_size = 1;
    let seq_length = 4;
    let embedding_dim = 3;
    let vocab_size = 10;

    // Repeated token: [5, 5, 5, 5]
    let indices = vec![5u32, 5, 5, 5];

    let weight: Vec<f32> = (0..vocab_size * embedding_dim).map(|i| i as f32).collect();

    let result = executor
        .execute_embedding(
            &indices,
            &weight,
            batch_size,
            seq_length,
            embedding_dim,
            vocab_size,
        )
        .await
        .unwrap();

    // All embeddings should be identical
    for i in 0..4 {
        let start = i * embedding_dim;
        assert_eq!(result[start], 15.0); // 5 * 3
        assert_eq!(result[start + 1], 16.0);
        assert_eq!(result[start + 2], 17.0);
    }

    println!("✅ Repeated tokens test passed");
}
