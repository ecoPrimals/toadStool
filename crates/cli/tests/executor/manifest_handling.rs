// SPDX-License-Identifier: AGPL-3.0-only
//! Manifest handling tests
//!
//! Tests for manifest parsing, validation, and error handling.

use super::*;

// ============================================================================
// MANIFEST HANDLING TESTS (TDD: Error Paths)
// ============================================================================

#[tokio::test]
async fn test_run_biome_with_nonexistent_manifest_fails() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    let nonexistent = std::path::PathBuf::from("/nonexistent/manifest.toml");

    let opts = run_biome_opts(nonexistent, None, vec![], false, None, None, "basic".to_string());
    let result = executor.run_biome(&ctx, opts).await;

    assert!(result.is_err(), "Should fail with nonexistent manifest");
}

#[tokio::test]
async fn test_run_biome_with_invalid_manifest_fails() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    // Create invalid manifest
    let temp_dir = std::env::temp_dir();
    let manifest_path = temp_dir.join(format!("invalid-{}.toml", uuid::Uuid::new_v4()));
    tokio::fs::write(&manifest_path, "invalid toml content {{{")
        .await
        .unwrap();

    let opts = run_biome_opts(
        manifest_path.clone(),
        None,
        vec![],
        false,
        None,
        None,
        "basic".to_string(),
    );
    let result = executor.run_biome(&ctx, opts).await;

    // Cleanup
    let _ = tokio::fs::remove_file(&manifest_path).await;

    assert!(result.is_err(), "Should fail with invalid manifest");
}

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
async fn test_manifest_path_validation() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    // Test with various invalid paths
    let invalid_paths = vec![
        std::path::PathBuf::from(""),
        std::path::PathBuf::from("/"),
        std::path::PathBuf::from("/tmp"),
    ];

    for path in invalid_paths {
        let opts = run_biome_opts(path, None, vec![], false, None, None, "basic".to_string());
        let result = executor.run_biome(&ctx, opts).await;

        // Should handle gracefully (either error or proceed based on implementation)
        // We're testing that it doesn't panic
        let _ = result;
    }
}

#[tokio::test]
async fn test_manifest_content_types() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    // Test with minimal manifest
    let manifest_path = create_test_manifest_file("minimal").await.unwrap();

    let opts = run_biome_opts(
        manifest_path.clone(),
        None,
        vec![],
        false,
        None,
        None,
        "basic".to_string(),
    );
    let result = executor.run_biome(&ctx, opts).await;

    cleanup_test_manifest(&manifest_path).await.ok();

    // Result may fail due to no services, but not due to manifest parsing
    let _ = result;
}

#[tokio::test]
async fn test_concurrent_manifest_access() {
    // Test that multiple executors can access manifests concurrently
    let manifest_path = create_test_manifest_file("concurrent").await.unwrap();
    let barrier = Arc::new(tokio::sync::Barrier::new(5));

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let path = manifest_path.clone();
            let b = barrier.clone();
            tokio::spawn(async move {
                b.wait().await;
                let executor = create_test_executor().await.unwrap();
                let ctx = create_test_context();
                let opts = run_biome_opts(path, None, vec![], false, None, None, "basic".to_string());
                executor.run_biome(&ctx, opts).await
            })
        })
        .collect();

    // All should complete (may fail at execution, not manifest access)
    for handle in handles {
        let _ = handle.await;
    }

    cleanup_test_manifest(&manifest_path).await.ok();
}

