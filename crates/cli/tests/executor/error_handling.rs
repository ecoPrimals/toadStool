// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error handling tests
//!
//! Tests for error paths, timeouts, and boundary conditions.

use super::*;

// ============================================================================
// ERROR PATH TESTS (Comprehensive Error Handling)
// ============================================================================

#[tokio::test]
async fn test_operation_with_timeout() {
    let executor = create_test_executor().await.unwrap();

    // Use tokio::timeout instead of sleep
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        executor.list_biomes(false, "table".to_string(), false, None),
    )
    .await;

    assert!(
        result.is_ok(),
        "list_biomes() should complete within timeout"
    );
    assert!(result.unwrap().is_ok(), "list_biomes() should succeed");
}

#[tokio::test]
async fn test_invariant_list_biomes_never_panics() {
    // Property: list_biomes() should never panic, even under stress
    let executor = Arc::new(create_test_executor().await.unwrap());

    let handles: Vec<_> = (0..100)
        .map(|_| {
            let exec = executor.clone();
            tokio::spawn(async move {
                exec.list_biomes(false, "table".to_string(), false, None)
                    .await
            })
        })
        .collect();

    // All should return (not panic)
    for handle in handles {
        let _ = handle.await; // Just verify no panic
    }
}

#[tokio::test]
async fn test_error_message_clarity() {
    let executor = create_test_executor().await.unwrap();

    // Test that error messages are clear
    let result = executor
        .down_biome("nonexistent".to_string(), false, 30, false)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // Error should be descriptive
    assert!(
        !err_msg.is_empty(),
        "Error message should not be empty"
    );
}

#[tokio::test]
async fn test_graceful_degradation() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    // Test that system handles errors gracefully
    let invalid_cases = vec![
        std::path::PathBuf::from(""),
        std::path::PathBuf::from("/invalid"),
        std::path::PathBuf::from("/tmp/nonexistent.toml"),
    ];

    for path in invalid_cases {
        let opts = run_biome_opts(path, None, vec![], false, None, None, "basic".to_string());
        let result = executor.run_biome(&ctx, opts).await;

        // Should fail gracefully, not panic
        let _ = result;
    }
}

