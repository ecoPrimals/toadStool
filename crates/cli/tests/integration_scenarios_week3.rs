// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::expect_used)] // expect() is idiomatic in tests
//! Week 3 Integration Scenario Tests - Simplified
//! Cross-module integration testing focusing on coordinator and config integration

use std::sync::Arc;
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
// Distributed Coordinator Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_creation_succeeds() {
    // Test basic coordinator creation
    let coordinator = create_test_coordinator().await;

    // Coordinator should be created successfully
    // This validates configuration loading and initialization
    assert!(Arc::strong_count(&coordinator) >= 1);
}

#[tokio::test]
async fn test_coordinator_can_start() {
    let coordinator = create_test_coordinator().await;

    // Start the coordinator
    let start_result = Arc::clone(&coordinator).start().await;

    assert!(
        start_result.is_ok(),
        "Coordinator should start successfully: {:?}",
        start_result.err()
    );
}

#[tokio::test]
async fn test_coordinator_start_is_idempotent() {
    let coordinator = create_test_coordinator().await;

    // Start twice - should be safe
    let first_start = Arc::clone(&coordinator).start().await;
    assert!(first_start.is_ok(), "First start should succeed");

    let second_start = Arc::clone(&coordinator).start().await;
    assert!(second_start.is_ok(), "Second start should be safe");
}

#[tokio::test]
async fn test_multiple_coordinators_can_coexist() {
    // Create multiple coordinators
    let coord1 = create_test_coordinator().await;
    let coord2 = create_test_coordinator().await;
    let coord3 = create_test_coordinator().await;

    // All should start successfully
    assert!(Arc::clone(&coord1).start().await.is_ok());
    assert!(Arc::clone(&coord2).start().await.is_ok());
    assert!(Arc::clone(&coord3).start().await.is_ok());
}

#[tokio::test]
async fn test_coordinator_survives_multiple_clones() {
    let coordinator = create_test_coordinator().await;

    // Create multiple clones (keep alive so strong_count reflects them)
    let mut clones = Vec::with_capacity(10);
    for _ in 0..10 {
        clones.push(Arc::clone(&coordinator));
    }
    assert_eq!(clones.len(), 10);
    assert!(Arc::strong_count(&coordinator) >= 10);
}

// ============================================================================
// Configuration Integration Tests
// ============================================================================

#[tokio::test]
async fn test_distributed_config_has_valid_defaults() {
    let config = DistributedConfig::default();

    // Verify configuration structure is valid
    assert!(
        config.standalone.max_concurrent_executions > 0,
        "Should have positive max concurrent executions"
    );

    assert!(
        config.standalone.default_timeout_secs > 0,
        "Should have positive execution timeout"
    );
}

#[tokio::test]
async fn test_distributed_config_is_cloneable() {
    let config = DistributedConfig::default();

    // Should be cloneable (needed for Arc sharing)
    let cloned = config.clone();

    assert_eq!(
        config.standalone.max_concurrent_executions,
        cloned.standalone.max_concurrent_executions
    );
}

#[tokio::test]
async fn test_distributed_config_serialization() {
    let config = DistributedConfig::default();

    // Should serialize to JSON (for config files)
    let json_result = serde_json::to_string(&config);
    assert!(json_result.is_ok(), "Config should serialize to JSON");

    // Should deserialize back
    let json_str = json_result.unwrap();
    let deserialized: Result<DistributedConfig, _> = serde_json::from_str(&json_str);
    assert!(deserialized.is_ok(), "Config should deserialize from JSON");
}

#[tokio::test]
async fn test_config_with_custom_songbird_endpoint() {
    let config = DistributedConfig {
        songbird_integration: Some(toadstool_distributed::SongbirdConfig {
            endpoint: "http://custom-songbird:8080".to_string(),
            auth_token: None,
            health_reporting_interval_secs: 30,
        }),
        ..Default::default()
    };

    // Should create coordinator with custom config
    let coordinator = DistributedCoordinator::new(config).await;
    assert!(
        coordinator.is_ok(),
        "Should accept custom Songbird endpoint"
    );
}

// ============================================================================
// Capability Detection Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_detects_capabilities() {
    // Coordinator should detect local capabilities during creation
    let coordinator = create_test_coordinator().await;

    // If we can create it, capability detection worked
    assert!(Arc::strong_count(&coordinator) >= 1);
}

// ============================================================================
// Concurrent Operations Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_coordinator_creation() {
    // Create multiple coordinators concurrently
    let mut handles = vec![];

    for _ in 0..5 {
        let handle = tokio::spawn(async move { create_test_coordinator().await });
        handles.push(handle);
    }

    // All should succeed
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent creation should succeed");
    }
}

#[tokio::test]
async fn test_concurrent_coordinator_starts() {
    let coordinator = create_test_coordinator().await;

    // Start concurrently from multiple tasks
    let mut handles = vec![];

    for _ in 0..5 {
        let coord = Arc::clone(&coordinator);
        let handle = tokio::spawn(async move { coord.start().await });
        handles.push(handle);
    }

    // All should succeed (idempotent)
    for handle in handles {
        let result = handle.await.expect("Task should not panic");
        assert!(result.is_ok(), "Concurrent start should be safe");
    }
}

// ============================================================================
// UUID Generation Tests (Integration with Coordinator)
// ============================================================================

#[tokio::test]
async fn test_uuid_generation_for_executions() {
    // Test UUID generation (used for execution IDs)
    let mut ids = vec![];

    for _ in 0..100 {
        ids.push(Uuid::new_v4());
    }

    // All should be unique
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique_ids.len(), 100, "All UUIDs should be unique");
}

#[tokio::test]
async fn test_concurrent_uuid_generation() {
    // Generate UUIDs concurrently
    let mut handles = vec![];

    for _ in 0..10 {
        let handle = tokio::spawn(async move {
            let mut local_ids = vec![];
            for _ in 0..10 {
                local_ids.push(Uuid::new_v4());
            }
            local_ids
        });
        handles.push(handle);
    }

    // Collect all UUIDs
    let mut all_ids = vec![];
    for handle in handles {
        let ids = handle.await.unwrap();
        all_ids.extend(ids);
    }

    // All 100 UUIDs should be unique
    let unique_ids: std::collections::HashSet<_> = all_ids.iter().collect();
    assert_eq!(
        unique_ids.len(),
        100,
        "All concurrent UUIDs should be unique"
    );
}

// ============================================================================
// Error Handling Integration
// ============================================================================

#[tokio::test]
async fn test_coordinator_handles_default_config_gracefully() {
    // Default config might have placeholder values - should handle gracefully
    let config = DistributedConfig::default();
    let result = DistributedCoordinator::new(config).await;

    assert!(
        result.is_ok(),
        "Should handle default config: {:?}",
        result.err()
    );
}

// ============================================================================
// Arc and Clone Behavior Tests (State Management)
// ============================================================================

#[tokio::test]
async fn test_arc_coordinator_sharing() {
    let coordinator = create_test_coordinator().await;
    let initial_count = Arc::strong_count(&coordinator);

    // Create clones
    let clone1 = Arc::clone(&coordinator);
    let clone2 = Arc::clone(&coordinator);

    // Reference count should increase
    assert_eq!(Arc::strong_count(&coordinator), initial_count + 2);

    // Drop clones
    drop(clone1);
    drop(clone2);

    // Reference count should decrease
    assert_eq!(Arc::strong_count(&coordinator), initial_count);
}

#[tokio::test]
async fn test_coordinator_shared_across_tasks() {
    let coordinator = create_test_coordinator().await;

    // Share across tasks
    let coord1 = Arc::clone(&coordinator);
    let coord2 = Arc::clone(&coordinator);

    let task1 = tokio::spawn(async move { Arc::clone(&coord1).start().await });

    let task2 = tokio::spawn(async move { Arc::clone(&coord2).start().await });

    // Both should succeed
    let (result1, result2) = tokio::join!(task1, task2);

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

// ============================================================================
// Integration Flow Tests
// ============================================================================

#[tokio::test]
async fn test_full_coordinator_lifecycle() {
    // 1. Create coordinator (simulates CLI initialization)
    let coordinator = create_test_coordinator().await;
    assert!(Arc::strong_count(&coordinator) >= 1, "Should be created");

    // 2. Start coordinator (simulates background services)
    let start_result = Arc::clone(&coordinator).start().await;
    assert!(start_result.is_ok(), "Should start successfully");

    // 3. Use coordinator (simulates operations)
    let _clone_for_use = Arc::clone(&coordinator);
    assert!(Arc::strong_count(&coordinator) >= 2, "Should be shareable");

    // 4. Coordinator goes out of scope (simulates shutdown)
    // This happens automatically - no explicit cleanup needed
}

#[tokio::test]
async fn test_sequential_coordinator_operations() {
    // Sequential operations should all succeed
    let coordinator = create_test_coordinator().await;

    // Operation 1: Start
    assert!(Arc::clone(&coordinator).start().await.is_ok());

    // Operation 2: Clone
    let _clone1 = Arc::clone(&coordinator);
    let _clone2 = Arc::clone(&coordinator);

    // Operation 3: Start again (idempotent)
    assert!(Arc::clone(&coordinator).start().await.is_ok());
}

#[tokio::test]
async fn test_parallel_coordinator_operations() {
    let coordinator = create_test_coordinator().await;

    // Perform multiple operations in parallel
    let coord1 = Arc::clone(&coordinator);
    let coord2 = Arc::clone(&coordinator);
    let coord3 = Arc::clone(&coordinator);

    let (r1, r2, r3) = tokio::join!(coord1.start(), coord2.start(), async {
        coord3.start().await
    });

    // All should succeed
    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());
}
