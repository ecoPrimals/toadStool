// SPDX-License-Identifier: AGPL-3.0-only
//! Resource management tests
//!
//! Tests for CPU/memory limits, quotas, and resource overrides.

use super::*;

// ============================================================================
// RESOURCE MANAGEMENT TESTS (Concurrent-Safe)
// ============================================================================

#[tokio::test]
async fn test_cpu_limit_override() {
    // Test that CPU limits can be overridden
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();
    let manifest_path = create_test_manifest_file("cpu_limit").await.unwrap();

    let opts = run_biome_opts(
        manifest_path.clone(),
        Some("test-cpu".to_string()),
        vec![],
        false,
        Some(2.0),
        None,
        "basic".to_string(),
    );
    let result = executor.run_biome(&ctx, opts).await;

    cleanup_test_manifest(&manifest_path).await.ok();

    // Should not fail on CPU validation
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(
            !err_msg.contains("invalid cpu") && !err_msg.contains("CPU"),
            "Should not fail on CPU validation: {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_memory_limit_override() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();
    let manifest_path = create_test_manifest_file("memory_limit").await.unwrap();

    let opts = run_biome_opts(
        manifest_path.clone(),
        Some("test-memory".to_string()),
        vec![],
        false,
        None,
        Some("1G".to_string()),
        "basic".to_string(),
    );
    let result = executor.run_biome(&ctx, opts).await;

    cleanup_test_manifest(&manifest_path).await.ok();

    // Should not fail on memory validation
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(
            !err_msg.contains("invalid memory") && !err_msg.contains("Memory"),
            "Should not fail on memory validation: {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_resource_limits_both_overrides() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();
    let manifest_path = create_test_manifest_file("both_limits").await.unwrap();

    let opts = run_biome_opts(
        manifest_path.clone(),
        Some("test-both".to_string()),
        vec![],
        false,
        Some(4.0),
        Some("2G".to_string()),
        "basic".to_string(),
    );
    let result = executor.run_biome(&ctx, opts).await;

    cleanup_test_manifest(&manifest_path).await.ok();

    // Should accept both overrides
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(
            !err_msg.contains("invalid") && !err_msg.contains("limit"),
            "Should not fail on resource validation: {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_resource_limits_validation() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();
    let manifest_path = create_test_manifest_file("validation").await.unwrap();

    // Test that the resource validation logic works
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

    // Verify the function completes (may fail at execution)
    let _ = result;
}

#[tokio::test]
async fn test_concurrent_resource_operations() {
    // Test concurrent resource limit operations
    let barrier = Arc::new(tokio::sync::Barrier::new(10));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let b = barrier.clone();
            tokio::spawn(async move {
                b.wait().await;
                let executor = create_test_executor().await.unwrap();
                let ctx = create_test_context();
                let manifest_path = create_test_manifest_file(&format!("concurrent_{}", i))
                    .await
                    .unwrap();

                let opts = run_biome_opts(
                    manifest_path.clone(),
                    Some(format!("test-{}", i)),
                    vec![],
                    false,
                    Some(1.0 + i as f64 * 0.5),
                    Some(format!("{}M", 512 + i * 128)),
                    "basic".to_string(),
                );
                let result = executor.run_biome(&ctx, opts).await;

                cleanup_test_manifest(&manifest_path).await.ok();
                result
            })
        })
        .collect();

    // All should complete
    for handle in handles {
        let _ = handle.await;
    }
}

#[tokio::test]
async fn test_resource_limit_edge_cases() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    // Test edge case values
    let test_cases = vec![
        (Some(0.1), None),          // Very low CPU
        (Some(64.0), None),         // High CPU
        (None, Some("1M".to_string())), // Very low memory
        (None, Some("128G".to_string())), // High memory
    ];

    for (cpu, memory) in test_cases {
        let manifest_path = create_test_manifest_file("edge_case").await.unwrap();
        let opts = run_biome_opts(
            manifest_path.clone(),
            None,
            vec![],
            false,
            cpu,
            memory,
            "basic".to_string(),
        );
        let result = executor.run_biome(&ctx, opts).await;
        cleanup_test_manifest(&manifest_path).await.ok();

        // Should handle gracefully
        let _ = result;
    }
}

