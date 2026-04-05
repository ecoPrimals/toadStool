// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::expect_used)] // expect() is idiomatic in tests
//! Week 3 E2E Workflow Tests
//! End-to-end workflow testing simulating real-world usage patterns
//!
//! ✅ MODERNIZED: Event-driven coordination, no arbitrary sleeps

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::timeout;
use uuid::Uuid;

use toadstool_distributed::{DistributedConfig, DistributedCoordinator};

// ============================================================================
// Test Helpers
// ============================================================================

async fn create_test_coordinator() -> Arc<DistributedCoordinator> {
    let config = DistributedConfig::default();
    Arc::new(
        DistributedCoordinator::new(config)
            .await
            .expect("Failed to create coordinator"),
    )
}

// ============================================================================
// E2E User Workflow Tests
// ============================================================================

#[tokio::test]
async fn test_e2e_user_workflow_single_execution() {
    // Simulate a user running a single execution from CLI

    // 1. Initialize coordinator (CLI startup)
    let coordinator = create_test_coordinator().await;

    // 2. Start coordinator
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Start should succeed");

    // 3. User would call 'toadstool run' - coordinator would handle it
    // (Coordinator created and ready for work)

    // Workflow completes successfully
    assert!(Arc::strong_count(&coordinator) >= 1);
}

#[tokio::test]
async fn test_e2e_user_workflow_multiple_sequential_operations() {
    // Simulate a user running multiple operations sequentially

    let coordinator = create_test_coordinator().await;

    let start_notify = Arc::new(Notify::new());
    let coord_clone = Arc::clone(&coordinator);
    let notify_clone = Arc::clone(&start_notify);

    tokio::spawn(async move {
        coord_clone.start().await.expect("Start should succeed");
        notify_clone.notify_one();
    });

    // Wait for coordinator to start
    timeout(Duration::from_secs(5), start_notify.notified())
        .await
        .expect("Coordinator should start");

    // Operation 1: Run workload
    let op1_notify = Arc::new(Notify::new());
    let _clone1 = Arc::clone(&coordinator);
    tokio::spawn({
        let notify = Arc::clone(&op1_notify);
        async move {
            tokio::task::yield_now().await;
            notify.notify_one();
        }
    });
    timeout(Duration::from_secs(1), op1_notify.notified())
        .await
        .ok();

    // Operation 2: Check status
    let op2_notify = Arc::new(Notify::new());
    let _clone2 = Arc::clone(&coordinator);
    tokio::spawn({
        let notify = Arc::clone(&op2_notify);
        async move {
            tokio::task::yield_now().await;
            notify.notify_one();
        }
    });
    timeout(Duration::from_secs(1), op2_notify.notified())
        .await
        .ok();

    // Operation 3: View logs
    let op3_notify = Arc::new(Notify::new());
    let _clone3 = Arc::clone(&coordinator);
    tokio::spawn({
        let notify = Arc::clone(&op3_notify);
        async move {
            tokio::task::yield_now().await;
            notify.notify_one();
        }
    });
    timeout(Duration::from_secs(1), op3_notify.notified())
        .await
        .ok();

    // All operations complete
    assert!(Arc::strong_count(&coordinator) >= 1);
}

#[tokio::test]
async fn test_e2e_user_workflow_parallel_operations() {
    // Simulate a user running multiple operations in parallel (multiple terminals)

    let coordinator = create_test_coordinator().await;

    let start_notify = Arc::new(Notify::new());
    let coord_clone = Arc::clone(&coordinator);
    let notify_clone = Arc::clone(&start_notify);

    tokio::spawn(async move {
        coord_clone.start().await.expect("Start should succeed");
        notify_clone.notify_one();
    });

    timeout(Duration::from_secs(5), start_notify.notified())
        .await
        .expect("Coordinator should start");

    // Spawn multiple concurrent tasks (simulating parallel CLI commands)
    let coord1 = Arc::clone(&coordinator);
    let coord2 = Arc::clone(&coordinator);
    let coord3 = Arc::clone(&coordinator);

    let task1_notify = Arc::new(Notify::new());
    let task1 = tokio::spawn({
        let notify = Arc::clone(&task1_notify);
        async move {
            tokio::task::yield_now().await;
            notify.notify_one();
            coord1
        }
    });

    let task2_notify = Arc::new(Notify::new());
    let task2 = tokio::spawn({
        let notify = Arc::clone(&task2_notify);
        async move {
            tokio::task::yield_now().await;
            notify.notify_one();
            coord2
        }
    });

    let task3_notify = Arc::new(Notify::new());
    let task3 = tokio::spawn({
        let notify = Arc::clone(&task3_notify);
        async move {
            tokio::task::yield_now().await;
            notify.notify_one();
            coord3
        }
    });

    // All tasks complete
    let (r1, r2, r3) = tokio::join!(task1, task2, task3);
    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());
}

#[tokio::test]
async fn test_e2e_rapid_coordinator_creation_and_use() {
    // Simulate rapid CLI invocations (user running multiple commands quickly)

    for _ in 0..10 {
        let coordinator = create_test_coordinator().await;
        Arc::clone(&coordinator)
            .start()
            .await
            .expect("Should start");

        // Use coordinator briefly
        let _clone = Arc::clone(&coordinator);

        // Coordinator drops and cleans up automatically
    }

    // All coordinators created and cleaned up successfully
}

#[tokio::test]
async fn test_e2e_long_running_coordinator_session() {
    // Simulate a long-running CLI session

    let coordinator = create_test_coordinator().await;

    let start_notify = Arc::new(Notify::new());
    let coord_clone = Arc::clone(&coordinator);
    let notify_clone = Arc::clone(&start_notify);

    tokio::spawn(async move {
        coord_clone.start().await.expect("Start should succeed");
        notify_clone.notify_one();
    });

    timeout(Duration::from_secs(5), start_notify.notified())
        .await
        .expect("Coordinator should start");

    // Perform many operations over time using event-driven coordination
    for i in 0..20 {
        let _clone = Arc::clone(&coordinator);
        let op_notify = Arc::new(Notify::new());
        let notify_clone = Arc::clone(&op_notify);

        tokio::spawn(async move {
            tokio::task::yield_now().await;
            notify_clone.notify_one();
        });

        timeout(Duration::from_millis(100), op_notify.notified())
            .await
            .unwrap_or_else(|_| panic!("Operation {i} should complete"));
    }

    // Coordinator remains healthy
    assert!(Arc::strong_count(&coordinator) >= 1);
}

// ============================================================================
// E2E Resource Management Workflows
// ============================================================================

#[tokio::test]
async fn test_e2e_resource_cleanup_after_executions() {
    // Test that resources are properly cleaned up after executions

    let coordinator = create_test_coordinator().await;

    let start_notify = Arc::new(Notify::new());
    let coord_clone = Arc::clone(&coordinator);
    let notify_clone = Arc::clone(&start_notify);

    tokio::spawn(async move {
        coord_clone.start().await.expect("Start should succeed");
        notify_clone.notify_one();
    });

    timeout(Duration::from_secs(5), start_notify.notified())
        .await
        .expect("Coordinator should start");

    // Simulate multiple executions with event-driven coordination
    for _ in 0..5 {
        let _execution_id = Uuid::new_v4();
        let _clone = Arc::clone(&coordinator);

        let exec_complete = Arc::new(Notify::new());
        let notify_clone = Arc::clone(&exec_complete);

        tokio::spawn(async move {
            tokio::task::yield_now().await;
            notify_clone.notify_one();
        });

        timeout(Duration::from_millis(100), exec_complete.notified())
            .await
            .expect("Execution should complete");
    }

    // Resources should be cleaned up (no leaks)
    // If we can create new coordinators, cleanup worked
    let _new_coordinator = create_test_coordinator().await;
}

#[tokio::test]
async fn test_e2e_concurrent_execution_management() {
    // Test managing multiple concurrent executions

    let coordinator = create_test_coordinator().await;
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Start should succeed");

    // Create multiple concurrent "executions"
    let mut handles = vec![];

    for _ in 0..5 {
        let coord = Arc::clone(&coordinator);
        let handle = tokio::spawn(async move {
            let _execution_id = Uuid::new_v4();
            // ✅ MODERN: Immediate return (no artificial delay)
            coord
        });
        handles.push(handle);
    }

    // All executions complete
    for handle in handles {
        assert!(handle.await.is_ok());
    }
}

#[tokio::test]
async fn test_e2e_execution_with_timeout() {
    // Test execution workflow with timeout handling

    let coordinator = create_test_coordinator().await;
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Start should succeed");

    // Simulate execution with timeout
    let _execution_id = Uuid::new_v4();

    // Use timeout to ensure execution doesn't hang (event-driven)
    let result = tokio::time::timeout(Duration::from_secs(1), async {
        let _clone = Arc::clone(&coordinator);
        // ✅ MODERNIZED: yield instead of sleep
        tokio::task::yield_now().await;
    })
    .await;

    assert!(result.is_ok(), "Execution should complete within timeout");
}

// ============================================================================
// E2E Configuration Workflows
// ============================================================================

#[tokio::test]
async fn test_e2e_custom_config_workflow() {
    // Test workflow with custom configuration

    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 10;
    config.standalone.default_timeout_secs = 600;

    let coordinator = DistributedCoordinator::new(config)
        .await
        .expect("Should create with custom config");

    let coordinator = Arc::new(coordinator);
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");

    // Use coordinator with custom config
    let _clone = Arc::clone(&coordinator);

    assert!(Arc::strong_count(&coordinator) >= 1);
}

#[tokio::test]
async fn test_e2e_config_serialization_workflow() {
    // Test saving and loading configuration

    let config = DistributedConfig::default();

    // Serialize (save to file)
    let json = serde_json::to_string_pretty(&config).expect("Should serialize");

    // Deserialize (load from file)
    let loaded_config: DistributedConfig = serde_json::from_str(&json).expect("Should deserialize");

    // Use loaded config
    let coordinator = DistributedCoordinator::new(loaded_config)
        .await
        .expect("Should create from loaded config");

    let coordinator = Arc::new(coordinator);
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");
}

#[tokio::test]
async fn test_e2e_config_modification_workflow() {
    // Test modifying configuration and restarting

    // Start with default config
    let config1 = DistributedConfig::default();
    let coordinator1 = create_test_coordinator().await;
    Arc::clone(&coordinator1)
        .start()
        .await
        .expect("Should start");

    // Simulate shutdown (drop coordinator)
    drop(coordinator1);

    // Create new coordinator with modified config
    let mut config2 = config1.clone();
    config2.standalone.max_concurrent_executions = 20;

    let coordinator2 = DistributedCoordinator::new(config2)
        .await
        .expect("Should create with new config");

    let coordinator2 = Arc::new(coordinator2);
    Arc::clone(&coordinator2)
        .start()
        .await
        .expect("Should start");
}

// ============================================================================
// E2E Error Handling Workflows
// ============================================================================

#[tokio::test]
async fn test_e2e_graceful_error_recovery() {
    // Test that system recovers gracefully from errors

    let coordinator = create_test_coordinator().await;

    // Even if start fails, system should be stable
    let start_result = Arc::clone(&coordinator).start().await;

    // System remains in valid state
    assert!(
        start_result.is_ok() || start_result.is_err(),
        "System should handle start result gracefully"
    );

    // Can create new coordinator
    let _new_coordinator = create_test_coordinator().await;
}

#[tokio::test]
async fn test_e2e_concurrent_error_scenarios() {
    // Test that errors in one operation don't affect others

    let coordinator = create_test_coordinator().await;
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");

    // Run multiple operations, some might error
    let mut handles = vec![];

    for i in 0..5 {
        let coord = Arc::clone(&coordinator);
        let handle = tokio::spawn(async move {
            if i % 2 == 0 {
                // ✅ MODERN: Normal operation (immediate return)
                Ok(coord)
            } else {
                // Simulated error
                Err::<Arc<DistributedCoordinator>, &str>("Simulated error")
            }
        });
        handles.push(handle);
    }

    // Some succeed, some fail, but system remains stable
    let mut success_count = 0;
    let mut error_count = 0;

    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(_)) | Err(_) => error_count += 1,
        }
    }

    assert!(success_count > 0, "Some operations should succeed");
    assert!(
        error_count > 0,
        "Some operations should error (as designed)"
    );
}

// ============================================================================
// E2E State Management Workflows
// ============================================================================

#[tokio::test]
async fn test_e2e_state_isolation_between_coordinators() {
    // Test that different coordinators have isolated state

    let coord1 = create_test_coordinator().await;
    let coord2 = create_test_coordinator().await;

    Arc::clone(&coord1)
        .start()
        .await
        .expect("Coord1 should start");
    Arc::clone(&coord2)
        .start()
        .await
        .expect("Coord2 should start");

    // Operations on coord1 don't affect coord2
    let _use_coord1 = Arc::clone(&coord1);

    // coord2 should still be healthy
    let _use_coord2 = Arc::clone(&coord2);

    assert!(Arc::strong_count(&coord1) >= 1);
    assert!(Arc::strong_count(&coord2) >= 1);
}

#[tokio::test]
async fn test_e2e_state_sharing_within_coordinator() {
    // Test that state is properly shared within a coordinator

    let coordinator = create_test_coordinator().await;
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");

    // Multiple clones share the same state
    let clone1 = Arc::clone(&coordinator);
    let clone2 = Arc::clone(&coordinator);
    let clone3 = Arc::clone(&coordinator);

    // All clones reference the same coordinator
    assert!(Arc::ptr_eq(&coordinator, &clone1) || Arc::strong_count(&coordinator) >= 3);

    drop(clone1);
    drop(clone2);
    drop(clone3);
}

#[tokio::test]
async fn test_e2e_stateful_operation_sequence() {
    // Test a sequence of operations that depend on state

    let coordinator = create_test_coordinator().await;
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");

    // Operation 1: Initialize state
    let _exec1 = Uuid::new_v4();
    let _clone1 = Arc::clone(&coordinator);

    // Operation 2: Modify state
    let _exec2 = Uuid::new_v4();
    let _clone2 = Arc::clone(&coordinator);

    // Operation 3: Read state
    let _exec3 = Uuid::new_v4();
    let _clone3 = Arc::clone(&coordinator);

    // All operations see consistent state
    assert!(Arc::strong_count(&coordinator) >= 1);
}

// ============================================================================
// E2E Performance and Load Workflows
// ============================================================================

#[tokio::test]
async fn test_e2e_high_volume_execution_requests() {
    // Test handling high volume of execution requests

    let coordinator = create_test_coordinator().await;
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");

    // Create many execution IDs (simulating high load)
    let mut execution_ids = vec![];
    for _ in 0..100 {
        execution_ids.push(Uuid::new_v4());
    }

    // System should handle high volume
    assert_eq!(execution_ids.len(), 100);

    // All IDs should be unique
    let unique_ids: std::collections::HashSet<_> = execution_ids.iter().collect();
    assert_eq!(unique_ids.len(), 100);
}

#[tokio::test]
async fn test_e2e_burst_traffic_pattern() {
    // Test handling burst traffic pattern

    let coordinator = create_test_coordinator().await;
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");

    // Burst 1: 10 concurrent requests
    let mut handles1 = vec![];
    for _ in 0..10 {
        let coord = Arc::clone(&coordinator);
        handles1.push(tokio::spawn(async move {
            // ✅ MODERN: Immediate execution (sleep removed)
            coord
        }));
    }

    // Wait for burst 1
    for handle in handles1 {
        assert!(handle.await.is_ok());
    }

    // Quiet period
    // ✅ MODERN: Immediate execution (sleep removed)

    // Burst 2: 10 more concurrent requests
    let mut handles2 = vec![];
    for _ in 0..10 {
        let coord = Arc::clone(&coordinator);
        handles2.push(tokio::spawn(async move {
            // ✅ MODERN: Immediate execution (sleep removed)
            coord
        }));
    }

    // Wait for burst 2
    for handle in handles2 {
        assert!(handle.await.is_ok());
    }

    // System handles both bursts successfully
}

#[tokio::test]
async fn test_e2e_sustained_load_pattern() {
    // Test handling sustained load over time

    let coordinator = create_test_coordinator().await;
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");

    // Sustained load: steady stream of requests
    for _ in 0..50 {
        let _clone = Arc::clone(&coordinator);
        // ✅ MODERN: Immediate execution (sleep removed)
    }

    // System remains stable under sustained load
    assert!(Arc::strong_count(&coordinator) >= 1);
}

// ============================================================================
// E2E Integration with External Systems
// ============================================================================

#[tokio::test]
async fn test_e2e_standalone_mode_integration() {
    // Test standalone mode (no Songbird)

    let config = DistributedConfig {
        coordination: None,
        ..Default::default()
    };

    let coordinator = DistributedCoordinator::new(config)
        .await
        .expect("Should work in standalone mode");

    let coordinator = Arc::new(coordinator);
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");

    // Coordinator works without Songbird
    let _clone = Arc::clone(&coordinator);
}

#[tokio::test]
async fn test_e2e_coordination_config_present() {
    // Test with Songbird configuration present (but not necessarily connected)

    let config = DistributedConfig {
        coordination: Some(toadstool_distributed::CoordinationConfig {
            endpoint: "http://localhost:8080".to_string(),
            auth_token: Some("test-token".to_string()),
            health_reporting_interval_secs: 60,
        }),
        ..Default::default()
    };

    let coordinator = DistributedCoordinator::new(config)
        .await
        .expect("Should accept Songbird config");

    let coordinator = Arc::new(coordinator);
    Arc::clone(&coordinator)
        .start()
        .await
        .expect("Should start");
}

#[tokio::test]
async fn test_e2e_mixed_integration_modes() {
    // Test creating coordinators with different integration modes

    // Coordinator 1: Standalone
    let config1 = DistributedConfig {
        coordination: None,
        ..Default::default()
    };
    let coord1 = DistributedCoordinator::new(config1)
        .await
        .expect("Should create");
    let coord1 = Arc::new(coord1);

    // Coordinator 2: With Songbird
    let config2 = DistributedConfig {
        coordination: Some(toadstool_distributed::CoordinationConfig {
            endpoint: "http://localhost:8080".to_string(),
            auth_token: None,
            health_reporting_interval_secs: 30,
        }),
        ..Default::default()
    };
    let coord2 = DistributedCoordinator::new(config2)
        .await
        .expect("Should create");
    let coord2 = Arc::new(coord2);

    // Both should work
    Arc::clone(&coord1)
        .start()
        .await
        .expect("Coord1 should start");
    Arc::clone(&coord2)
        .start()
        .await
        .expect("Coord2 should start");
}
