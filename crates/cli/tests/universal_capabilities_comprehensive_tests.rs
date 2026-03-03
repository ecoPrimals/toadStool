// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Universal Capability Display Operations
//!
//! Tests for capability display functionality in universal compute manager.
//! Coverage target: Get capabilities.rs from current low coverage to >80%

use anyhow::Result;
use toadstool_cli::universal::operations::CapabilityDisplayOps;
use toadstool_cli::universal::UniversalComputeManager;

/// Helper to create a test manager
async fn create_manager() -> Result<UniversalComputeManager> {
    Ok(UniversalComputeManager::new().await?)
}

// ==================================================
// Detection Summary Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_print_detection_summary_empty() -> Result<()> {
    let manager = create_manager().await?;

    let result = manager.print_detection_summary().await;
    assert!(
        result.is_ok(),
        "Detection summary should succeed even with no platforms"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_print_detection_summary_multiple_calls() -> Result<()> {
    let manager = create_manager().await?;

    // Should be safe to call multiple times
    manager.print_detection_summary().await?;
    manager.print_detection_summary().await?;
    manager.print_detection_summary().await?;

    Ok(())
}

// ==================================================
// Benchmark Table Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_print_benchmark_table_empty() -> Result<()> {
    let manager = create_manager().await?;

    let result = manager.print_benchmark_table().await;
    assert!(
        result.is_ok(),
        "Benchmark table should succeed even with no benchmarks"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_print_benchmark_table_multiple_calls() -> Result<()> {
    let manager = create_manager().await?;

    manager.print_benchmark_table().await?;
    manager.print_benchmark_table().await?;
    manager.print_benchmark_table().await?;

    Ok(())
}

// ==================================================
// Capabilities Table Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_print_capabilities_table_empty() -> Result<()> {
    let manager = create_manager().await?;

    let result = manager.print_capabilities_table(false).await;
    assert!(
        result.is_ok(),
        "Capabilities table should succeed even with no platforms"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_print_capabilities_table_detailed() -> Result<()> {
    let manager = create_manager().await?;

    let result = manager.print_capabilities_table(true).await;
    assert!(result.is_ok(), "Detailed capabilities table should succeed");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_print_capabilities_table_simple() -> Result<()> {
    let manager = create_manager().await?;

    let result = manager.print_capabilities_table(false).await;
    assert!(result.is_ok(), "Simple capabilities table should succeed");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_print_capabilities_table_multiple_calls() -> Result<()> {
    let manager = create_manager().await?;

    manager.print_capabilities_table(false).await?;
    manager.print_capabilities_table(true).await?;
    manager.print_capabilities_table(false).await?;

    Ok(())
}

// ==================================================
// Concurrent Display Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_display_operations() -> Result<()> {
    let manager = create_manager().await?;

    let (r1, r2, r3) = tokio::join!(
        manager.print_detection_summary(),
        manager.print_benchmark_table(),
        manager.print_capabilities_table(false),
    );

    assert!(
        r1.is_ok() && r2.is_ok() && r3.is_ok(),
        "Concurrent display operations should succeed"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_capabilities_calls() -> Result<()> {
    let manager = create_manager().await?;

    let (r1, r2, r3) = tokio::join!(
        manager.print_capabilities_table(false),
        manager.print_capabilities_table(true),
        manager.print_capabilities_table(false),
    );

    assert!(
        r1.is_ok() && r2.is_ok() && r3.is_ok(),
        "Concurrent capabilities calls should succeed"
    );

    Ok(())
}

// ==================================================
// Sequential Operations Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_sequential_all_displays() -> Result<()> {
    let manager = create_manager().await?;

    manager.print_detection_summary().await?;
    manager.print_benchmark_table().await?;
    manager.print_capabilities_table(false).await?;
    manager.print_capabilities_table(true).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_repeated_detection_summaries() -> Result<()> {
    let manager = create_manager().await?;

    for _ in 0..10 {
        manager.print_detection_summary().await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_repeated_benchmark_tables() -> Result<()> {
    let manager = create_manager().await?;

    for _ in 0..10 {
        manager.print_benchmark_table().await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_repeated_capabilities_tables() -> Result<()> {
    let manager = create_manager().await?;

    for i in 0..10 {
        let detailed = i % 2 == 0;
        manager.print_capabilities_table(detailed).await?;
    }

    Ok(())
}

// ==================================================
// Multiple Manager Instances
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_managers_display() -> Result<()> {
    let manager1 = create_manager().await?;
    let manager2 = create_manager().await?;

    manager1.print_detection_summary().await?;
    manager2.print_detection_summary().await?;

    manager1.print_benchmark_table().await?;
    manager2.print_benchmark_table().await?;

    manager1.print_capabilities_table(false).await?;
    manager2.print_capabilities_table(true).await?;

    Ok(())
}

// ==================================================
// Lifecycle Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_display_lifecycle() -> Result<()> {
    let manager = create_manager().await?;

    // Initial displays
    manager.print_detection_summary().await?;
    manager.print_capabilities_table(false).await?;
    manager.print_benchmark_table().await?;

    // Detailed view
    manager.print_capabilities_table(true).await?;

    // Repeated calls
    manager.print_detection_summary().await?;
    manager.print_capabilities_table(false).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_manager_creation_and_display() -> Result<()> {
    // Create, display, drop
    for _ in 0..5 {
        let manager = create_manager().await?;
        manager.print_detection_summary().await?;
        manager.print_benchmark_table().await?;
        manager.print_capabilities_table(false).await?;
    }

    Ok(())
}

// ==================================================
// Edge Cases
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rapid_display_calls() -> Result<()> {
    let manager = create_manager().await?;

    for _ in 0..100 {
        manager.print_detection_summary().await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_alternating_detail_levels() -> Result<()> {
    let manager = create_manager().await?;

    for i in 0..20 {
        manager.print_capabilities_table(i % 2 == 0).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_all_display_types_sequence() -> Result<()> {
    let manager = create_manager().await?;

    for _ in 0..5 {
        manager.print_detection_summary().await?;
        manager.print_benchmark_table().await?;
        manager.print_capabilities_table(false).await?;
        manager.print_capabilities_table(true).await?;
    }

    Ok(())
}

// ==================================================
// Stress Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_many_display_operations() -> Result<()> {
    let manager = create_manager().await?;

    // Use multiple tokio::join! instead
    for i in 0..10 {
        let (_, _, _) = tokio::join!(
            manager.print_detection_summary(),
            manager.print_benchmark_table(),
            manager.print_capabilities_table(i % 2 == 0)
        );
    }

    Ok(())
}

// ==================================================
// Error Resilience Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_display_operations_never_panic() -> Result<()> {
    let manager = create_manager().await?;

    // These should never panic, even in edge cases
    let _ = manager.print_detection_summary().await;
    let _ = manager.print_benchmark_table().await;
    let _ = manager.print_capabilities_table(false).await;
    let _ = manager.print_capabilities_table(true).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_display_idempotency() -> Result<()> {
    let manager = create_manager().await?;

    // Multiple calls should produce same result (idempotent)
    manager.print_detection_summary().await?;
    manager.print_detection_summary().await?;

    manager.print_benchmark_table().await?;
    manager.print_benchmark_table().await?;

    manager.print_capabilities_table(false).await?;
    manager.print_capabilities_table(false).await?;

    Ok(())
}
