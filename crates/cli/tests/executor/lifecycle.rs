//! Executor lifecycle tests
//!
//! Tests for executor creation, initialization, and basic lifecycle operations.

use super::*;

// ============================================================================
// BASIC LIFECYCLE TESTS (Event-Driven, Concurrent-Safe)
// ============================================================================

#[tokio::test]
async fn test_executor_creation_succeeds() {
    // ✅ CONCURRENT-SAFE: Each test gets isolated executor
    let result = create_test_executor().await;
    assert!(result.is_ok(), "Executor creation should succeed");
}

#[tokio::test]
async fn test_executor_creation_initializes_components() {
    let executor = create_test_executor().await.unwrap();

    // Executor should be ready to use (internal state initialized)
    // We can verify this by checking it doesn't panic on operations
    let result = executor
        .list_biomes(
            false,               // all
            "table".to_string(), // format
            false,               // quiet
            None,                // filter
        )
        .await;

    // Should return empty list (no running biomes yet)
    assert!(result.is_ok(), "list_biomes() should work on new executor");
}

#[tokio::test]
async fn test_concurrent_executor_creation() {
    // ✅ MODERN: Test concurrent creation (should be safe)
    let barrier = Arc::new(Barrier::new(10));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let b = barrier.clone();
            tokio::spawn(async move {
                b.wait().await; // All start simultaneously
                let result = create_test_executor().await;
                (i, result)
            })
        })
        .collect();

    // All should succeed concurrently
    for handle in handles {
        let (i, result) = handle.await.unwrap();
        assert!(result.is_ok(), "Executor {} should create successfully", i);
    }
}

#[tokio::test]
async fn test_list_biomes_succeeds_on_new_executor() {
    let executor = create_test_executor().await.unwrap();

    // list_biomes prints output and returns Result<()>
    let result = executor
        .list_biomes(
            false,               // all
            "table".to_string(), // format
            false,               // resources
            None,                // filter
        )
        .await;

    assert!(result.is_ok(), "list_biomes should succeed on new executor");
}

