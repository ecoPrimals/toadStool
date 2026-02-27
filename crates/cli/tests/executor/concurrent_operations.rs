//! Concurrent operations tests
//!
//! Tests for concurrency, race conditions, and stress testing.

use super::*;

// ============================================================================
// CONCURRENT EXECUTION TESTS (Modern Async Patterns)
// ============================================================================

#[tokio::test]
async fn test_list_biomes_succeeds_on_new_executor() {
    let executor = create_test_executor().await.unwrap();

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

#[tokio::test]
async fn test_concurrent_list_biomes_calls() {
    // Multiple concurrent list_biomes() calls should be safe
    let executor = Arc::new(create_test_executor().await.unwrap());

    let handles: Vec<_> = (0..20)
        .map(|_| {
            let exec = executor.clone();
            tokio::spawn(async move {
                exec.list_biomes(false, "table".to_string(), false, None)
                    .await
            })
        })
        .collect();

    // All should complete successfully
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent list_biomes should succeed");
    }
}

#[tokio::test]
async fn test_concurrent_executor_operations() {
    // Test multiple operations on shared executor
    let executor = Arc::new(create_test_executor().await.unwrap());
    let barrier = Arc::new(tokio::sync::Barrier::new(15));

    let mut handles = vec![];

    // 5 list_biomes calls
    for _ in 0..5 {
        let exec = executor.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            exec.list_biomes(false, "table".to_string(), false, None)
                .await
        }));
    }

    // 5 list_biomes with different formats
    for format in ["table", "json", "yaml", "table", "json"] {
        let exec = executor.clone();
        let b = barrier.clone();
        let fmt = format.to_string();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            exec.list_biomes(false, fmt, false, None).await
        }));
    }

    // 5 list_biomes with different options
    for all in [true, false, true, false, true] {
        let exec = executor.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            exec.list_biomes(all, "table".to_string(), false, None)
                .await
        }));
    }

    // All should complete
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_stress_concurrent_operations() {
    let executor = Arc::new(create_test_executor().await.unwrap());
    let barrier = Arc::new(tokio::sync::Barrier::new(30));

    let mut handles = vec![];

    // 10 concurrent list_biomes() calls
    for _ in 0..10 {
        let exec = executor.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            exec.list_biomes(false, "table".to_string(), false, None)
                .await
        }));
    }

    // 10 concurrent executor creations (separate instances)
    for _ in 0..10 {
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            create_test_executor().await.map(|_| ())
        }));
    }

    // 10 concurrent context creations (lightweight)
    for _ in 0..10 {
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            let _ = create_test_context();
            Ok::<(), anyhow::Error>(())
        }));
    }

    // All should complete without panicking
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_concurrent_manifest_operations() {
    // Test concurrent operations with manifests
    let barrier = Arc::new(tokio::sync::Barrier::new(10));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let b = barrier.clone();
            tokio::spawn(async move {
                b.wait().await;
                let executor = create_test_executor().await.unwrap();
                let ctx = create_test_context();
                let manifest = create_test_manifest_file(&format!("concurrent_{}", i))
                    .await
                    .unwrap();

                let opts = run_biome_opts(
                    manifest.clone(),
                    Some(format!("test-{}", i)),
                    vec![],
                    false,
                    None,
                    None,
                    "basic".to_string(),
                );
                let result = executor.run_biome(&ctx, opts).await;

                cleanup_test_manifest(&manifest).await.ok();
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
async fn test_executor_under_load() {
    // Simulate load with many rapid operations
    let executor = Arc::new(create_test_executor().await.unwrap());
    let handles: Vec<_> = (0..50)
        .map(|_| {
            let exec = executor.clone();
            tokio::spawn(async move {
                exec.list_biomes(false, "table".to_string(), false, None)
                    .await
            })
        })
        .collect();

    // Should handle load gracefully
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    assert!(
        success_count > 45,
        "Should handle most requests under load, got {}",
        success_count
    );
}

#[tokio::test]
async fn test_race_condition_safety() {
    // Test that concurrent access doesn't cause race conditions
    let executor = Arc::new(create_test_executor().await.unwrap());
    let barrier = Arc::new(tokio::sync::Barrier::new(20));

    let handles: Vec<_> = (0..20)
        .map(|_| {
            let exec = executor.clone();
            let b = barrier.clone();
            tokio::spawn(async move {
                b.wait().await; // All start at exactly the same time
                exec.list_biomes(false, "table".to_string(), false, None)
                    .await
            })
        })
        .collect();

    // All should complete safely
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_concurrent_different_formats() {
    let executor = Arc::new(create_test_executor().await.unwrap());
    let formats = vec!["table", "json", "yaml"];

    let handles: Vec<_> = formats
        .iter()
        .flat_map(|fmt| {
            (0..5).map(move |_| {
                let exec = executor.clone();
                let format = fmt.to_string();
                tokio::spawn(async move {
                    exec.list_biomes(false, format, false, None).await
                })
            })
        })
        .collect();

    // All formats should work concurrently
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

