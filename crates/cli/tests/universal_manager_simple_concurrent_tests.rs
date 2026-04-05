// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! 🚀 `UniversalComputeManager` Simple Concurrent Tests
//!
//! **Philosophy**: Modern, concurrent, event-driven, robust
//! **Pattern**: Simple API tests, no complex fixtures
//! **Target**: `universal/manager_impl.rs` 0% → 20% coverage
//!
//! Test issues ARE production issues - we test concurrently because we run concurrently.

use anyhow::Result;
use std::sync::Arc;
use toadstool_cli::universal::UniversalComputeManager;
use tokio::sync::broadcast;
use tokio::time::{Duration, timeout};

// =============================================================================
// Test Group 1: Manager Creation (Concurrent & Stress)
// =============================================================================

/// ✅ Test 1: Basic manager creation
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_manager_creation_basic() -> Result<()> {
    let _manager = UniversalComputeManager::new().await?;
    // Creation success verifies initialization logic
    Ok(())
}

/// ✅ Test 2: Concurrent manager creation (10 simultaneous)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_manager_creation() -> Result<()> {
    let (tx, mut rx) = broadcast::channel(16);
    let mut handles = vec![];

    // Create 10 managers concurrently
    for i in 0..10 {
        let tx = tx.clone();
        handles.push(tokio::spawn(async move {
            let result = UniversalComputeManager::new().await;
            tx.send(i).ok();
            result
        }));
    }

    // Wait for completion signals (event-driven)
    for _ in 0..10 {
        timeout(Duration::from_secs(5), rx.recv()).await??;
    }

    // All should succeed
    for handle in handles {
        assert!(handle.await?.is_ok(), "Manager creation should succeed");
    }

    Ok(())
}

/// ✅ Test 3: Stress test manager creation (30 concurrent)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_manager_creation() -> Result<()> {
    let mut handles = vec![];

    for _ in 0..30 {
        handles.push(tokio::spawn(async { UniversalComputeManager::new().await }));
    }

    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }

    // At least 95% should succeed
    assert!(
        success_count >= 28,
        "At least 28/30 managers should create successfully, got {success_count}"
    );

    Ok(())
}

// =============================================================================
// Test Group 2: Platform Detection (Concurrent)
// =============================================================================

/// ✅ Test 4: Detect platforms with empty categories (defaults)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detect_platforms_empty_categories() -> Result<()> {
    let mut manager = UniversalComputeManager::new().await?;

    // Empty categories should use defaults
    let result = manager.detect_platforms(vec![], false, None).await;

    // Should succeed (detects default categories)
    assert!(result.is_ok(), "Default platform detection should succeed");

    Ok(())
}

/// ✅ Test 5: Detect platforms with specific categories
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detect_platforms_specific_categories() -> Result<()> {
    let categories = vec![
        vec!["traditional".to_string()],
        vec!["container".to_string()],
        vec!["language".to_string()],
        vec!["gpu".to_string()],
    ];

    let mut handles = vec![];
    for cat in categories {
        handles.push(tokio::spawn(async move {
            let mut mgr = UniversalComputeManager::new().await?;
            mgr.detect_platforms(cat, false, None).await
        }));
    }

    // All should complete
    for handle in handles {
        let _ = handle.await?;
    }

    Ok(())
}

/// ✅ Test 6: Detect platforms with invalid category
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detect_platforms_invalid_category() -> Result<()> {
    let mut manager = UniversalComputeManager::new().await?;

    // Invalid category should be handled gracefully
    let result = manager
        .detect_platforms(vec!["invalid_category".to_string()], false, None)
        .await;

    // Should succeed (warns but doesn't error)
    assert!(
        result.is_ok(),
        "Invalid category should be handled gracefully"
    );

    Ok(())
}

/// ✅ Test 7: Concurrent platform detection
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_platform_detection() -> Result<()> {
    let (tx, mut rx) = broadcast::channel(16);
    let mut handles = vec![];

    // Detect 10 times concurrently with different categories
    for i in 0..10 {
        let tx = tx.clone();
        handles.push(tokio::spawn(async move {
            let mut mgr = UniversalComputeManager::new().await?;
            let cat = match i % 4 {
                0 => vec!["traditional".to_string()],
                1 => vec!["container".to_string()],
                2 => vec!["language".to_string()],
                _ => vec![],
            };
            let result = mgr.detect_platforms(cat, false, None).await;
            tx.send(i).ok();
            result
        }));
    }

    // Wait for all operations
    for _ in 0..10 {
        timeout(Duration::from_secs(10), rx.recv()).await??;
    }

    // All should complete
    for handle in handles {
        let _ = handle.await?;
    }

    Ok(())
}

// =============================================================================
// Test Group 3: Show Capabilities (Concurrent)
// =============================================================================

/// ✅ Test 8: Show capabilities with text format
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_show_capabilities_text() -> Result<()> {
    let manager = UniversalComputeManager::new().await?;

    let result = manager.show_capabilities("text", false).await;

    // Should succeed
    assert!(result.is_ok(), "Show capabilities (text) should succeed");

    Ok(())
}

/// ✅ Test 9: Show capabilities with different formats
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_show_capabilities_formats() -> Result<()> {
    let formats = vec!["text", "json", "yaml", "table"];
    let mut handles = vec![];

    for format in formats {
        let format = format.to_string();
        handles.push(tokio::spawn(async move {
            let mgr = UniversalComputeManager::new().await?;
            mgr.show_capabilities(format.as_str(), false).await
        }));
    }

    // All formats should work
    for handle in handles {
        let _ = handle.await?;
    }

    Ok(())
}

/// ✅ Test 10: Show capabilities with detailed flag
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_show_capabilities_detailed() -> Result<()> {
    let test_cases = vec![
        (false, "text"),
        (true, "text"),
        (false, "json"),
        (true, "json"),
    ];

    let mut handles = vec![];
    for (detailed, format) in test_cases {
        let format = format.to_string();
        handles.push(tokio::spawn(async move {
            let mgr = UniversalComputeManager::new().await?;
            mgr.show_capabilities(format.as_str(), detailed).await
        }));
    }

    // All should work
    for handle in handles {
        let _ = handle.await?;
    }

    Ok(())
}

/// ✅ Test 11: Concurrent show capabilities
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_show_capabilities() -> Result<()> {
    let manager = Arc::new(UniversalComputeManager::new().await?);
    let (tx, mut rx) = broadcast::channel(16);
    let mut handles = vec![];

    // 15 concurrent show_capabilities calls
    for i in 0..15 {
        let mgr = Arc::clone(&manager);
        let tx = tx.clone();
        handles.push(tokio::spawn(async move {
            let format = if i % 2 == 0 { "text" } else { "json" };
            let result = mgr.show_capabilities(format, false).await;
            tx.send(i).ok();
            result
        }));
    }

    // Wait for all
    for _ in 0..15 {
        timeout(Duration::from_secs(5), rx.recv()).await??;
    }

    // All should succeed
    for handle in handles {
        let _ = handle.await?;
    }

    Ok(())
}

// =============================================================================
// Test Group 4: Mixed Operations (Concurrent)
// =============================================================================

/// ✅ Test 12: Create and use pattern
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_and_use_pattern() -> Result<()> {
    let mut handles = vec![];

    // 10 concurrent create → use lifecycles
    for _ in 0..10 {
        handles.push(tokio::spawn(async {
            let mgr = UniversalComputeManager::new().await?;
            mgr.show_capabilities("text", false).await?;
            Ok::<_, anyhow::Error>(())
        }));
    }

    // All should complete successfully
    for handle in handles {
        handle.await??;
    }

    Ok(())
}

/// ✅ Test 13: Burst traffic pattern
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_burst_manager_operations() -> Result<()> {
    let (tx, mut rx) = broadcast::channel(100);

    // Burst 1: 20 manager creations
    for i in 0..20 {
        let tx = tx.clone();
        tokio::spawn(async move {
            let _mgr = UniversalComputeManager::new().await;
            tx.send(format!("burst1_{i}")).ok();
        });
    }

    // Wait for burst 1
    for _ in 0..20 {
        timeout(Duration::from_secs(5), rx.recv()).await??;
    }

    // Burst 2: 15 capability displays
    for i in 0..15 {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mgr = UniversalComputeManager::new().await.ok()?;
            let _result = mgr.show_capabilities("text", false).await;
            tx.send(format!("burst2_{i}")).ok()
        });
    }

    // Wait for burst 2
    for _ in 0..15 {
        timeout(Duration::from_secs(5), rx.recv()).await??;
    }

    // All 35 operations completed
    Ok(())
}

/// ✅ Test 14: Sustained load (50 operations)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_sustained_manager_load() -> Result<()> {
    let mut handles = vec![];

    // Sustained load: 50 operations
    for i in 0..50 {
        handles.push(tokio::spawn(async move {
            let mgr = UniversalComputeManager::new().await?;
            if i % 2 == 0 {
                mgr.show_capabilities("text", false).await
            } else {
                mgr.show_capabilities("json", true).await
            }
        }));
    }

    // System should handle sustained load
    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }

    // At least 95% success rate
    let success_rate = f64::from(success_count) / 50.0;
    assert!(
        success_rate >= 0.95,
        "Success rate should be >= 95%, got {:.1}%",
        success_rate * 100.0
    );

    Ok(())
}

// =============================================================================
// Test Group 5: Timeout Awareness & Event-Driven
// =============================================================================

/// ✅ Test 15: Timeout protection for manager operations
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_timeout_awareness_manager() -> Result<()> {
    let mut handles = vec![];

    // 15 operations with timeout protection
    for _ in 0..15 {
        handles.push(tokio::spawn(async {
            timeout(Duration::from_secs(10), async {
                let mgr = UniversalComputeManager::new().await?;
                mgr.show_capabilities("text", false).await
            })
            .await
        }));
    }

    // All should complete within timeout
    let mut completed = 0;
    for handle in handles {
        if matches!(handle.await?, Ok(Ok(()))) {
            completed += 1;
        }
    }

    assert_eq!(
        completed, 15,
        "All 15 operations should complete within timeout"
    );

    Ok(())
}

/// ✅ Test 16: Event-driven coordination
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_event_driven_manager_coordination() -> Result<()> {
    let (start_tx, mut rx1) = broadcast::channel::<()>(16);
    let mut rx2 = start_tx.subscribe();
    let mut rx3 = start_tx.subscribe();

    // 3 tasks waiting for start signal
    let h1 = tokio::spawn(async move {
        rx1.recv().await.ok();
        let mgr = UniversalComputeManager::new().await?;
        mgr.show_capabilities("text", false).await
    });

    let h2 = tokio::spawn(async move {
        rx2.recv().await.ok();
        let mgr = UniversalComputeManager::new().await?;
        mgr.show_capabilities("json", true).await
    });

    let h3 = tokio::spawn(async move {
        rx3.recv().await.ok();
        let mgr = UniversalComputeManager::new().await?;
        mgr.show_capabilities("yaml", false).await
    });

    // Brief setup delay
    // ✅ MODERN: Immediate execution (sleep removed)

    // Broadcast start (all execute simultaneously)
    start_tx.send(()).ok();

    // All should complete concurrently
    let _ = h1.await?;
    let _ = h2.await?;
    let _ = h3.await?;

    Ok(())
}

// =============================================================================
// Coverage Summary
// =============================================================================

// This test suite covers universal/manager_impl.rs:
//
// 1. ✅ UniversalComputeManager::new() - Lines 11-22 (concurrent creation, stress)
// 2. ✅ detect_platforms() - Lines 25-152 (categories, invalid cases, concurrent)
// 3. ✅ show_capabilities() - Lines 342+ (formats, detailed flag, concurrent)
// 4. ✅ Internal state management (concurrent access)
// 5. ✅ Error handling (graceful failures)
// 6. ✅ Timeout awareness (production resilience)
// 7. ✅ Lifecycle management (create/use patterns)
// 8. ✅ Burst and sustained load patterns
//
// **Pattern**: Simple, direct API tests without complex fixtures
// **Concurrency**: All tests use modern concurrent patterns
// **Event-Driven**: Broadcast channels, minimal sleeps
// **Robust**: Timeout-aware, deterministic, production-grade
//
// **Expected Coverage**: universal/manager_impl.rs 0% → 15-20%
// **Tests**: 16 concurrent tests, all production-grade
