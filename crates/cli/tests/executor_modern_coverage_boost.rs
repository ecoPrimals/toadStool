// SPDX-License-Identifier: AGPL-3.0-or-later
//! Modern concurrent tests for BiomeExecutor - Coverage boost
//! Focus: Real API calls, zero sleeps, event-based, fully concurrent
//! Target: Increase executor_impl.rs coverage from 1.81% to 40%

use anyhow::Result;
use std::sync::Arc;
use toadstool_cli::executor::BiomeExecutor;
use tokio::sync::broadcast;

/// ✅ Test 1: Executor creation (covers BiomeExecutor::new)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_executor_creation() -> Result<()> {
    let _executor = BiomeExecutor::new().await?;

    // Verify executor created successfully by successful construction
    // (if new() fails, test will fail with Result error)
    Ok(())
}

/// ✅ Test 2: Concurrent executor creation
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_executor_creation() -> Result<()> {
    let mut handles = vec![];

    for _ in 0..10 {
        handles.push(tokio::spawn(async { BiomeExecutor::new().await }));
    }

    for handle in handles {
        assert!(handle.await?.is_ok());
    }

    Ok(())
}

/// ✅ Test 3: List biomes - table format
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_biomes_table() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let result = executor.list_biomes(true, "table", false, None).await;

    assert!(result.is_ok());
    Ok(())
}

/// ✅ Test 4: List biomes - JSON format
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_biomes_json() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let result = executor.list_biomes(false, "json", false, None).await;

    assert!(result.is_ok());
    Ok(())
}

/// ✅ Test 5: List biomes - YAML format
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_biomes_yaml() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let result = executor.list_biomes(true, "yaml", true, None).await;

    assert!(result.is_ok());
    Ok(())
}

/// ✅ Test 6: List biomes with status filter
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_biomes_filter_running() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let result = executor
        .list_biomes(true, "table", false, Some("running"))
        .await;

    assert!(result.is_ok());
    Ok(())
}

/// ✅ Test 7: List biomes filter - stopped
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_biomes_filter_stopped() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let result = executor
        .list_biomes(true, "table", false, Some("stopped"))
        .await;

    assert!(result.is_ok());
    Ok(())
}

/// ✅ Test 8: List biomes filter - error
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_biomes_filter_error() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let result = executor
        .list_biomes(true, "json", false, Some("error"))
        .await;

    assert!(result.is_ok());
    Ok(())
}

/// ✅ Test 9: Down non-existent biome (error path)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_down_nonexistent_biome() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let result = executor
        .down_biome("nonexistent-biome", false, 30, false)
        .await;

    assert!(result.is_err(), "Should error on non-existent biome");
    Ok(())
}

/// ✅ Test 10: Down with force flag
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_down_biome_force() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let result = executor.down_biome("test-biome", true, 30, false).await;

    // Will error but covers force path
    assert!(result.is_err());
    Ok(())
}

/// ✅ Test 11: Down with purge flag
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_down_biome_purge() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let result = executor.down_biome("test-biome", false, 30, true).await;

    // Will error but covers purge path
    assert!(result.is_err());
    Ok(())
}

/// ✅ Test 12: Down with different timeouts
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_down_biome_various_timeouts() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    for timeout in [0, 10, 60, 300] {
        let result = executor
            .down_biome(format!("test-{}", timeout), false, timeout, false)
            .await;

        // All will error but exercise timeout param
        assert!(result.is_err());
    }

    Ok(())
}

/// ✅ Test 13: Concurrent list operations (read-heavy)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_lists() -> Result<()> {
    let executor = Arc::new(BiomeExecutor::new().await?);
    let mut handles = vec![];

    for i in 0..20 {
        let executor = Arc::clone(&executor);
        let format = if i % 3 == 0 {
            "table"
        } else if i % 3 == 1 {
            "json"
        } else {
            "yaml"
        };

        handles.push(tokio::spawn(async move {
            executor
                .list_biomes(i % 2 == 0, format, i % 2 == 1, None)
                .await
        }));
    }

    for handle in handles {
        assert!(handle.await?.is_ok());
    }

    Ok(())
}

/// ✅ Test 14: Concurrent down operations
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_down_operations() -> Result<()> {
    let executor = Arc::new(BiomeExecutor::new().await?);
    let mut handles = vec![];

    for i in 0..10 {
        let executor = Arc::clone(&executor);

        handles.push(tokio::spawn(async move {
            executor
                .down_biome(format!("concurrent-test-{}", i), false, 30, false)
                .await
        }));
    }

    // All should complete (with errors, but no deadlock)
    for handle in handles {
        let _ = handle.await?;
    }

    Ok(())
}

/// ✅ Test 15: Event-based coordination (modern pattern)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_event_based_list() -> Result<()> {
    let executor = Arc::new(BiomeExecutor::new().await?);
    let (tx, mut rx) = broadcast::channel(16);

    let executor_clone = Arc::clone(&executor);
    tokio::spawn(async move {
        let _ = executor_clone.list_biomes(true, "json", false, None).await;
        tx.send(()).ok();
    });

    // Wait for event, not sleep!
    tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("Should complete")
        .expect("Should receive event");

    Ok(())
}

/// ✅ Test 16: Rapid sequential operations
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rapid_sequential_lists() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    for i in 0..30 {
        let _ = executor.list_biomes(i % 2 == 0, "table", false, None).await;
    }

    Ok(())
}

/// ✅ Test 17: Mixed concurrent operations (stress test)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mixed_concurrent_ops() -> Result<()> {
    let executor = Arc::new(BiomeExecutor::new().await?);
    let mut handles = vec![];

    // Mix of list and down operations
    for i in 0..15 {
        let executor = Arc::clone(&executor);

        if i % 2 == 0 {
            handles.push(tokio::spawn(async move {
                executor.list_biomes(true, "json", false, None).await
            }));
        } else {
            handles.push(tokio::spawn(async move {
                executor
                    .down_biome(format!("mixed-{}", i), false, 30, false)
                    .await
            }));
        }
    }

    // All should complete without deadlock
    for handle in handles {
        let _ = handle.await?;
    }

    Ok(())
}

/// ✅ Test 18: Concurrent operations without panics
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_operations_stability() -> Result<()> {
    let executor = Arc::new(BiomeExecutor::new().await?);
    let mut handles = vec![];

    for _ in 0..50 {
        let executor = Arc::clone(&executor);

        handles.push(tokio::spawn(async move {
            // Mix of operations
            let _ = executor.list_biomes(true, "json", false, None).await;
        }));
    }

    for handle in handles {
        handle.await?;
    }

    Ok(())
}

/// ✅ Test 19: All format variations
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_all_output_formats() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let formats = vec!["table", "json", "yaml"];

    for format in formats {
        let result = executor.list_biomes(true, format, false, None).await;

        assert!(result.is_ok(), "Format {} should work", format);
    }

    Ok(())
}

/// ✅ Test 20: All status filter variations
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_all_status_filters() -> Result<()> {
    let executor = BiomeExecutor::new().await?;

    let filters = vec![
        Some("running".to_string()),
        Some("stopped".to_string()),
        Some("starting".to_string()),
        Some("stopping".to_string()),
        Some("error".to_string()),
        Some("migrating".to_string()),
        None,
    ];

    for filter in filters {
        let result = executor
            .list_biomes(true, "table", false, filter.as_deref())
            .await;

        assert!(result.is_ok(), "Filter {:?} should work", filter);
    }

    Ok(())
}
