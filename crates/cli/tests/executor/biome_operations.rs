// SPDX-License-Identifier: AGPL-3.0-only
//! Biome operations tests
//!
//! Tests for biome lifecycle operations: up, down, restart, status.

use super::*;

// ============================================================================
// BIOME LIFECYCLE TESTS (up, down operations)
// ============================================================================

#[tokio::test]
async fn test_up_biome_with_nonexistent_manifest_fails() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    let nonexistent = std::path::PathBuf::from("/nonexistent/manifest.toml");

    let opts = up_biome_opts(nonexistent, false, None, vec![], false, 30);
    let result = executor.up_biome(&ctx, opts).await;

    assert!(
        result.is_err(),
        "up_biome should fail with nonexistent manifest"
    );
}

#[tokio::test]
async fn test_down_biome_nonexistent_fails() {
    let executor = create_test_executor().await.unwrap();

    let result = executor
        .down_biome("nonexistent-biome", false, 30, false)
        .await;

    assert!(
        result.is_err(),
        "down_biome should fail for nonexistent biome"
    );
}

#[tokio::test]
async fn test_down_biome_with_different_timeouts() {
    let executor = create_test_executor().await.unwrap();

    let timeouts = vec![10, 30, 60, 120];

    for timeout in timeouts {
        let result = executor
            .down_biome("test-biome", false, timeout, false)
            .await;

        // Should fail (biome doesn't exist) but not panic on timeout value
        assert!(result.is_err(), "Should fail for nonexistent biome");
    }
}

#[tokio::test]
async fn test_concurrent_up_biome_calls_with_different_names() {
    let executor = Arc::new(create_test_executor().await.unwrap());

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let exec = executor.clone();
            let manifest_name = format!("concurrent-up-{}", i);

            tokio::spawn(async move {
                let ctx = create_test_context();
                let manifest_path = create_test_manifest_file(&manifest_name).await?;

                let opts = up_biome_opts(
                    manifest_path.clone(),
                    true,
                    Some(format!("biome-{}", i)),
                    vec![],
                    false,
                    30,
                );
                let result = exec.up_biome(&ctx, opts).await;

                cleanup_test_manifest(&manifest_path).await.ok();
                result
            })
        })
        .collect();

    // All should execute (may fail due to no services)
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_down_biome_with_force_flag() {
    let executor = create_test_executor().await.unwrap();

    // Test force flag variations
    for force in [true, false] {
        let result = executor
            .down_biome("test-biome", force, 30, false)
            .await;

        // Should handle gracefully (will fail for nonexistent biome)
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_down_biome_with_purge_flag() {
    let executor = create_test_executor().await.unwrap();

    // Test purge flag variations
    for purge in [true, false] {
        let result = executor
            .down_biome("test-biome", false, 30, purge)
            .await;

        // Should handle gracefully
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_show_logs_for_nonexistent_biome_fails() {
    let executor = create_test_executor().await.unwrap();

    let result = executor
        .show_logs("nonexistent-biome", false, 100, false, None, None)
        .await;

    assert!(
        result.is_err(),
        "show_logs should fail for nonexistent biome"
    );
}

#[tokio::test]
async fn test_show_logs_with_different_options() {
    let executor = create_test_executor().await.unwrap();

    // Test various log options
    let test_cases = vec![
        (false, None, false),      // Basic
        (false, Some(10), false),  // With tail
        (false, None, true),       // With timestamps
        (false, Some(50), true),   // Tail + timestamps
    ];

    for (follow, tail, timestamps) in test_cases {
        let result = executor
            .show_logs("test-biome", follow, tail.unwrap_or(100), timestamps, None, None)
            .await;

        // Should fail gracefully for nonexistent biome
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_concurrent_down_biome_calls() {
    let executor = Arc::new(create_test_executor().await.unwrap());
    let barrier = Arc::new(tokio::sync::Barrier::new(10));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let exec = executor.clone();
            let b = barrier.clone();
            tokio::spawn(async move {
                b.wait().await;
                exec.down_biome(format!("biome-{}", i), false, 30, false)
                    .await
            })
        })
        .collect();

    // All should complete
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

