//! Logging tests
//!
//! Tests for log management and output handling.

use super::*;

// ============================================================================
// LOG MANAGEMENT TESTS
// ============================================================================

#[tokio::test]
async fn test_show_logs_basic_call() {
    let executor = create_test_executor().await.unwrap();

    let result = executor
        .show_logs(
            "test-biome".to_string(),
            false, // follow
            None,  // tail
            false, // timestamps
        )
        .await;

    // Will fail for nonexistent biome, but shouldn't panic
    assert!(result.is_err());
}

#[tokio::test]
async fn test_show_logs_with_tail_option() {
    let executor = create_test_executor().await.unwrap();

    // Test various tail values
    let tail_values = vec![10, 50, 100, 1000];

    for tail in tail_values {
        let result = executor
            .show_logs("test-biome".to_string(), false, Some(tail), false)
            .await;

        // Should handle gracefully
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_show_logs_with_timestamps() {
    let executor = create_test_executor().await.unwrap();

    let result = executor
        .show_logs("test-biome".to_string(), false, None, true)
        .await;

    // Should handle gracefully
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_log_access() {
    let executor = Arc::new(create_test_executor().await.unwrap());
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let exec = executor.clone();
            tokio::spawn(async move {
                exec.show_logs(format!("biome-{}", i), false, Some(10), false)
                    .await
            })
        })
        .collect();

    // All should complete without panicking
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

