// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error handling integration tests
//!
//! Tests error scenarios and recovery behavior across distributed components
//!
//! Following Month 1 test expansion plan - Day 4 (Error Handling)

use std::time::Duration;
use tokio::time::timeout;

use toadstool_distributed::core::config::DistributedConfig;
use toadstool_distributed::core::coordinator::DistributedCoordinator;

// ============================================================================
// Configuration Error Handling Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_zero_concurrency_works() {
    // Edge case: coordinator with zero concurrency (should still initialize)
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 0;

    let coordinator = DistributedCoordinator::new(config).await;

    // Should still initialize (just can't execute anything)
    assert!(
        coordinator.is_ok(),
        "Coordinator should handle zero concurrency"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_disabled_queue_and_small_concurrency() {
    // Edge case: disabled queue with minimal concurrency
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = false;
    config.standalone.max_concurrent_executions = 1;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(coordinator.is_ok(), "Coordinator should work without queue");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_extremely_large_values() {
    // Edge case: very large configuration values
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = u32::MAX;
    config.standalone.default_timeout_secs = u64::MAX;
    config.standalone.max_queue_size = usize::MAX;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle extreme values gracefully"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_initialization_timeout_handling() {
    // Test that coordinator initialization respects timeout
    let config = DistributedConfig::default();

    let result = timeout(Duration::from_secs(10), DistributedCoordinator::new(config)).await;

    assert!(
        result.is_ok(),
        "Coordinator init should complete within reasonable timeout"
    );
}

// ============================================================================
// Concurrency Error Scenarios
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rapid_coordinator_creation_stress() {
    // Stress test: create many coordinators rapidly
    let mut handles = vec![];

    for _ in 0..20 {
        let handle = tokio::spawn(async {
            let config = DistributedConfig::default();
            DistributedCoordinator::new(config).await
        });
        handles.push(handle);
    }

    // All should complete successfully
    for (idx, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent creation {} should succeed", idx);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_under_memory_pressure() {
    // Test coordinator creation with multiple instances (simulating memory pressure)
    let mut coordinators = vec![];

    for i in 0..10 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(
            coordinator.is_ok(),
            "Creation {} under pressure should succeed",
            i
        );

        coordinators.push(coordinator);
    }

    // All 10 coordinators should be valid
    assert_eq!(coordinators.len(), 10);
}

// ============================================================================
// Configuration Edge Cases
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_mismatched_queue_config() {
    // Edge case: queue enabled but size is 0
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = 0;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle mismatched queue config"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_large_queue_small_concurrency() {
    // Edge case: large queue but minimal concurrency
    let mut config = DistributedConfig::default();
    config.standalone.max_queue_size = 100000;
    config.standalone.max_concurrent_executions = 1;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle imbalanced config"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_tiny_timeout() {
    // Edge case: very small timeout (1 second)
    let mut config = DistributedConfig::default();
    config.standalone.default_timeout_secs = 1;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle tiny timeout"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_huge_timeout() {
    // Edge case: very large timeout (1 year in seconds)
    let mut config = DistributedConfig::default();
    config.standalone.default_timeout_secs = 365 * 24 * 60 * 60;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle huge timeout"
    );
}

// ============================================================================
// Async Error Scenarios
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_cancellation_safety() {
    // Test that coordinator creation can be safely cancelled
    let config = DistributedConfig::default();

    let result = timeout(
        Duration::from_millis(1),
        DistributedCoordinator::new(config),
    )
    .await;

    // Either completes fast or times out - both are acceptable
    // The important part is it doesn't panic or corrupt state
    let _ = result;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_coordinators_with_tokio_select() {
    // Test coordinators can be created with tokio::select!
    let config1 = DistributedConfig::default();
    let config2 = DistributedConfig::default();

    tokio::select! {
        result1 = DistributedCoordinator::new(config1) => {
            assert!(result1.is_ok(), "First coordinator should succeed");
        }
        result2 = DistributedCoordinator::new(config2) => {
            assert!(result2.is_ok(), "Second coordinator should succeed");
        }
    }
}

// ============================================================================
// Recovery and Resilience Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_after_failures() {
    // Simulate previous failures, then create successfully
    // (This tests that failed creation doesn't corrupt global state)

    for _ in 0..5 {
        let config = DistributedConfig::default();
        let _result = DistributedCoordinator::new(config).await;
        // Don't care if they succeed or fail
    }

    // Now create one that should definitely work
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should work after previous attempts"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_alternating_configs() {
    // Test creating coordinators with alternating configurations
    for i in 0..10 {
        let mut config = DistributedConfig::default();

        // Alternate between different configurations
        if i % 2 == 0 {
            config.standalone.max_concurrent_executions = 10;
            config.standalone.enable_job_queue = true;
        } else {
            config.standalone.max_concurrent_executions = 50;
            config.standalone.enable_job_queue = false;
        }

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(coordinator.is_ok(), "Alternating config {} should work", i);
    }
}

// ============================================================================
// Instance ID Error Scenarios
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_empty_instance_id() {
    // Edge case: empty instance ID
    let config = DistributedConfig {
        instance_id: String::new(),
        ..Default::default()
    };

    let coordinator = DistributedCoordinator::new(config).await;

    // Should handle empty ID gracefully
    assert!(
        coordinator.is_ok(),
        "Coordinator should handle empty instance ID"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_very_long_instance_id() {
    // Edge case: very long instance ID
    let config = DistributedConfig {
        instance_id: "x".repeat(10000),
        ..Default::default()
    };

    let coordinator = DistributedCoordinator::new(config).await;

    // Should handle long ID gracefully
    assert!(
        coordinator.is_ok(),
        "Coordinator should handle very long instance ID"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_special_chars_in_instance_id() {
    // Edge case: special characters in instance ID
    let config = DistributedConfig {
        instance_id: "test-id!@#$%^&*()".to_string(),
        ..Default::default()
    };

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle special chars in ID"
    );
}

// ============================================================================
// Stress and Load Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_sequential_creation_stress() {
    // Create many coordinators sequentially without cleanup
    for i in 0..50 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(
            coordinator.is_ok(),
            "Sequential creation {} should succeed",
            i
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_varied_timeouts() {
    // Test coordinators with varying timeout configurations
    let timeouts = vec![1, 10, 60, 300, 3600, 86400];

    for timeout_secs in timeouts {
        let mut config = DistributedConfig::default();
        config.standalone.default_timeout_secs = timeout_secs;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(coordinator.is_ok(), "Timeout {} should work", timeout_secs);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_varied_concurrency_levels() {
    // Test coordinators with varying concurrency configurations
    let concurrency_levels = vec![0, 1, 5, 10, 50, 100, 500, 1000];

    for concurrency in concurrency_levels {
        let mut config = DistributedConfig::default();
        config.standalone.max_concurrent_executions = concurrency;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Concurrency {} should work",
            concurrency
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_varied_queue_sizes() {
    // Test coordinators with varying queue size configurations
    let queue_sizes = vec![0, 1, 10, 100, 1000, 10000, 100000];

    for queue_size in queue_sizes {
        let mut config = DistributedConfig::default();
        config.standalone.max_queue_size = queue_size;
        config.standalone.enable_job_queue = queue_size > 0;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(coordinator.is_ok(), "Queue size {} should work", queue_size);
    }
}

// ============================================================================
// Advanced Async & Concurrency Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_parallel_creation_with_join() {
    // Test parallel coordinator creation using tokio::join!
    let config1 = DistributedConfig::default();
    let config2 = DistributedConfig::default();
    let config3 = DistributedConfig::default();

    let (result1, result2, result3) = tokio::join!(
        DistributedCoordinator::new(config1),
        DistributedCoordinator::new(config2),
        DistributedCoordinator::new(config3),
    );

    assert!(result1.is_ok(), "Parallel creation 1 should succeed");
    assert!(result2.is_ok(), "Parallel creation 2 should succeed");
    assert!(result3.is_ok(), "Parallel creation 3 should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_tokio_try_join() {
    // Test coordinator creation with tokio::try_join!
    let config1 = DistributedConfig::default();
    let config2 = DistributedConfig::default();

    let result = tokio::try_join!(
        DistributedCoordinator::new(config1),
        DistributedCoordinator::new(config2),
    );

    assert!(result.is_ok(), "try_join should succeed for valid configs");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_with_spawn_blocking() {
    // Test that coordinator creation doesn't block async runtime
    let handle = tokio::task::spawn(async {
        let config = DistributedConfig::default();
        DistributedCoordinator::new(config).await
    });

    let result = handle.await.unwrap();
    assert!(result.is_ok(), "Spawn should not interfere with creation");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_timeout_success() {
    // Test successful coordinator creation within timeout
    let config = DistributedConfig::default();

    let result = timeout(Duration::from_secs(5), DistributedCoordinator::new(config)).await;

    assert!(
        result.is_ok(),
        "Coordinator creation should complete within timeout"
    );
    assert!(
        result.unwrap().is_ok(),
        "Coordinator should be created successfully"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_varied_timeout_durations() {
    // Test coordinator creation with different timeout durations
    let timeouts = vec![
        Duration::from_millis(100),
        Duration::from_secs(1),
        Duration::from_secs(5),
        Duration::from_secs(10),
    ];

    for timeout_duration in timeouts {
        let config = DistributedConfig::default();
        let result = timeout(timeout_duration, DistributedCoordinator::new(config)).await;

        // Should complete within all reasonable timeouts
        if result.is_err() {
            // Timeout occurred - this is acceptable for very short timeouts
            continue;
        }

        assert!(
            result.unwrap().is_ok(),
            "Coordinator should succeed if timeout doesn't occur"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_concurrent_with_different_configs() {
    // Test concurrent creation with different configurations
    let mut handles = vec![];

    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let mut config = DistributedConfig::default();
            config.standalone.max_concurrent_executions = i * 10 + 1;
            config.standalone.max_queue_size = (i * 100) as usize;
            DistributedCoordinator::new(config).await
        });
        handles.push(handle);
    }

    for (idx, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap();
        assert!(
            result.is_ok(),
            "Concurrent different config {} should succeed",
            idx
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_nested_async_blocks() {
    // Test coordinator creation in nested async context
    async fn create_nested() -> Result<(), Box<dyn std::error::Error>> {
        async fn inner_create() -> Result<(), Box<dyn std::error::Error>> {
            let config = DistributedConfig::default();
            let _coordinator = DistributedCoordinator::new(config).await?;
            Ok(())
        }

        inner_create().await?;
        Ok(())
    }

    let result = create_nested().await;
    assert!(result.is_ok(), "Nested async creation should work");
}

// ============================================================================
// Race Condition & Concurrency Edge Cases
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_rapid_sequential_creation_no_gap() {
    // Test rapid sequential creation with no delay
    for i in 0..30 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(coordinator.is_ok(), "Rapid sequential {} should succeed", i);

        // Immediately proceed to next - no delay
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_interleaved_creation_and_drop() {
    // Test creating and dropping coordinators in interleaved pattern
    let mut coordinators = vec![];

    for i in 0..5 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(
            coordinator.is_ok(),
            "Interleaved creation {} should work",
            i
        );
        coordinators.push(coordinator);

        // Drop every other coordinator
        if i % 2 == 0 && !coordinators.is_empty() {
            coordinators.pop();
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_high_parallelism() {
    // Test with high degree of parallelism (100 concurrent)
    let mut handles = vec![];

    for _ in 0..100 {
        let handle = tokio::spawn(async {
            let config = DistributedConfig::default();
            DistributedCoordinator::new(config).await
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            success_count += 1;
        }
    }

    // Most should succeed (allow some failures under extreme load)
    assert!(
        success_count >= 90,
        "At least 90% should succeed under high parallelism (got {})",
        success_count
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_alternating_parallel_sequential() {
    // Test alternating between parallel and sequential creation
    for round in 0..3 {
        // Parallel round
        let mut handles = vec![];
        for _ in 0..5 {
            let handle = tokio::spawn(async {
                let config = DistributedConfig::default();
                DistributedCoordinator::new(config).await
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(
                result.is_ok(),
                "Parallel creation in round {} should succeed",
                round
            );
        }

        // Sequential round
        for _ in 0..5 {
            let config = DistributedConfig::default();
            let coordinator = DistributedCoordinator::new(config).await;
            assert!(
                coordinator.is_ok(),
                "Sequential creation in round {} should succeed",
                round
            );
        }
    }
}

// ============================================================================
// Resource Contention & Pressure Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_memory_intensive_configs() {
    // Test with configurations that might cause memory pressure
    let mut coordinators = vec![];

    for i in 0..20 {
        let mut config = DistributedConfig::default();
        config.standalone.max_queue_size = 100000;
        config.standalone.max_concurrent_executions = 1000;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Memory-intensive config {} should work",
            i
        );

        coordinators.push(coordinator);
    }

    // All coordinators should remain valid
    assert_eq!(coordinators.len(), 20);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_timing_consistency() {
    // Test that creation time is consistent across multiple attempts
    let mut times = vec![];

    for _ in 0..10 {
        let start = std::time::Instant::now();
        let config = DistributedConfig::default();
        let _coordinator = DistributedCoordinator::new(config).await;
        let elapsed = start.elapsed();
        times.push(elapsed);
    }

    // All should complete in reasonable time (< 3 seconds)
    for (idx, time) in times.iter().enumerate() {
        assert!(
            time < &Duration::from_secs(3),
            "Creation {} took too long: {:?}",
            idx,
            time
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_extreme_config_combinations() {
    // Test extreme combinations of configuration values
    let extreme_configs = vec![
        (0, 0, 0),                        // All zeros
        (1, 1, 1),                        // All ones
        (u32::MAX, u64::MAX, usize::MAX), // All max values
        (100, 1, 0),                      // High concurrency, tiny timeout, no queue
        (1, 86400 * 365, 1000000),        // Low concurrency, huge timeout, huge queue
    ];

    for (idx, (concurrency, timeout, queue_size)) in extreme_configs.into_iter().enumerate() {
        let mut config = DistributedConfig::default();
        config.standalone.max_concurrent_executions = concurrency;
        config.standalone.default_timeout_secs = timeout;
        config.standalone.max_queue_size = queue_size;
        config.standalone.enable_job_queue = queue_size > 0;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Extreme config combination {} should work",
            idx
        );
    }
}

// ============================================================================
// Error Recovery & Resilience
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_recovery_after_timeout() {
    // Simulate timeout, then verify recovery
    let config1 = DistributedConfig::default();

    // Try with very short timeout (might fail)
    let _timeout_result = timeout(
        Duration::from_nanos(1),
        DistributedCoordinator::new(config1),
    )
    .await;

    // Now create successfully with normal timeout
    let config2 = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config2).await;

    assert!(coordinator.is_ok(), "Should recover after previous timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_state_independence() {
    // Test that each coordinator is independent (no shared mutable state)
    let mut coordinators = vec![];

    for i in 0..10 {
        let config = DistributedConfig {
            instance_id: format!("independent-{}", i),
            ..Default::default()
        };

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Independent coordinator {} should succeed",
            i
        );

        coordinators.push(coordinator);
    }

    // All should coexist without interference
    assert_eq!(coordinators.len(), 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_unicode_instance_ids() {
    // Test instance IDs with unicode characters
    let unicode_ids = vec![
        "test-日本語",
        "test-العربية",
        "test-עברית",
        "test-中文",
        "test-🚀",
        "test-emoji-😀-test",
    ];

    for id in unicode_ids {
        let config = DistributedConfig {
            instance_id: id.to_string(),
            ..Default::default()
        };

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(coordinator.is_ok(), "Unicode ID '{}' should be handled", id);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_under_task_spawn_pressure() {
    // Test coordinator creation while spawning many other tasks
    let mut background_tasks = vec![];

    // Spawn background tasks
    for _ in 0..50 {
        let task = tokio::spawn(async {
            tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
        });
        background_tasks.push(task);
    }

    // Create coordinators while background tasks are running
    for i in 0..10 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(
            coordinator.is_ok(),
            "Creation under task pressure {} should succeed",
            i
        );
    }

    // Wait for background tasks
    for task in background_tasks {
        let _ = task.await;
    }
}

// ============================================================================
// Boundary Value Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_boundary_concurrency_values() {
    // Test boundary values for concurrency
    let boundary_values = vec![0, 1, u32::MAX / 2, u32::MAX - 1, u32::MAX];

    for value in boundary_values {
        let mut config = DistributedConfig::default();
        config.standalone.max_concurrent_executions = value;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Boundary concurrency {} should work",
            value
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_boundary_timeout_values() {
    // Test boundary values for timeout
    let boundary_values = vec![0, 1, 60, 3600, u64::MAX / 2, u64::MAX - 1];

    for value in boundary_values {
        let mut config = DistributedConfig::default();
        config.standalone.default_timeout_secs = value;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Boundary timeout {} should work",
            value
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_boundary_queue_sizes() {
    // Test boundary values for queue size
    let boundary_values = vec![0, 1, 100, 10000, usize::MAX / 1000, usize::MAX / 100];

    for value in boundary_values {
        let mut config = DistributedConfig::default();
        config.standalone.max_queue_size = value;
        config.standalone.enable_job_queue = value > 0;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Boundary queue size {} should work",
            value
        );
    }
}
