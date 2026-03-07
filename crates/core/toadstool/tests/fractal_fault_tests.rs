// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fault Tests for Fractal Composition
//!
//! Tests error handling, edge cases, and graceful degradation.

use toadstool::composition_constraints::*;
use toadstool::composition_engine::CompositionEngine;
use toadstool::deployment_layer::{DeploymentLayer, LayerDetector};
use toadstool::layer_adaptation::LayerCapabilityAdapter;
use toadstool::multi_workload_compositor::MultiWorkloadCompositor;
use toadstool::plugin_system::{PluginConfig, PluginManager, PluginManifest};
use toadstool::workload_migration::MigrationCoordinator;

/// Test: Invalid plugin manifest (empty name)
#[test]
fn test_invalid_plugin_manifest_empty_name() {
    let mut manager = PluginManager::new();

    let manifest = PluginManifest {
        name: String::new(), // Empty name
        version: "1.0.0".to_string(),
        plugin_type: "test".to_string(),
        entry_point: "lib.so".to_string(),
        ..Default::default()
    };

    let result = manager.register_plugin(manifest);
    assert!(result.is_err());
}

/// Test: Invalid plugin manifest (empty version)
#[test]
fn test_invalid_plugin_manifest_empty_version() {
    let mut manager = PluginManager::new();

    let manifest = PluginManifest {
        name: "test".to_string(),
        version: String::new(), // Empty version
        plugin_type: "test".to_string(),
        entry_point: "lib.so".to_string(),
        ..Default::default()
    };

    let result = manager.register_plugin(manifest);
    assert!(result.is_err());
}

/// Test: Plugin dependency not met
#[test]
fn test_plugin_missing_dependency() {
    let mut manager = PluginManager::new();

    let manifest = PluginManifest {
        name: "dependent".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: "test".to_string(),
        entry_point: "lib.so".to_string(),
        dependencies: vec!["missing-plugin".to_string()],
        ..Default::default()
    };

    let result = manager.register_plugin(manifest);
    assert!(result.is_err());
}

/// Test: Plugin limit reached
#[test]
fn test_plugin_limit_reached() {
    let config = PluginConfig {
        max_plugins: 3,
        ..Default::default()
    };

    let mut manager = PluginManager::with_config(config);

    // Register 3 plugins (at limit)
    for i in 0..3 {
        let manifest = PluginManifest {
            name: format!("plugin-{i}"),
            version: "1.0.0".to_string(),
            plugin_type: "test".to_string(),
            entry_point: format!("lib{i}.so"),
            ..Default::default()
        };
        assert!(manager.register_plugin(manifest).is_ok());
    }

    // 4th should fail
    let manifest = PluginManifest {
        name: "plugin-4".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: "test".to_string(),
        entry_point: "lib4.so".to_string(),
        ..Default::default()
    };

    let result = manager.register_plugin(manifest);
    assert!(result.is_err());
}

/// Test: Load non-existent plugin
#[test]
fn test_load_nonexistent_plugin() {
    let mut manager = PluginManager::new();

    let result = manager.load_plugin("non-existent");
    assert!(result.is_err());
}

/// Test: Unload non-existent plugin
#[test]
fn test_unload_nonexistent_plugin() {
    let mut manager = PluginManager::new();

    let result = manager.unload_plugin("non-existent");
    assert!(result.is_err());
}

/// Test: Composition with no workloads
#[tokio::test]
async fn test_empty_composition() {
    let compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let plan = compositor.compose().await.unwrap();

    assert!(plan.overall_feasibility);
    assert_eq!(plan.placements.len(), 0);
    assert_eq!(plan.conflicts.len(), 0);
}

/// Test: All hard constraints failing
#[tokio::test]
async fn test_all_hard_constraints_fail() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let req = CompositionRequest::new("impossible")
        .with_constraint(Constraint::min_memory_gb(999999.0))
        .with_constraint(Constraint::min_cpu_cores(999999))
        .with_constraint(Constraint::max_latency_ms(0));

    let eval = engine.evaluate(&req).await.unwrap();

    assert!(!eval.is_feasible);
    assert_eq!(eval.overall_score, 0.0);
}

/// Test: Migration recommendation for non-tracked workload
#[tokio::test]
async fn test_migration_untracked_workload() {
    let coordinator = MigrationCoordinator::new().await.unwrap();

    let location = coordinator.get_workload_location("non-existent").await;
    assert!(location.is_none());
}

/// Test: Constraint with negative values (should handle gracefully)
#[tokio::test]
async fn test_negative_constraint_values() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Negative memory (should still evaluate)
    let req =
        CompositionRequest::new("negative-mem").with_constraint(Constraint::min_memory_gb(-1.0));

    let result = engine.evaluate(&req).await;
    assert!(result.is_ok()); // Should not panic
}

/// Test: Layer adaptation for all layer types
#[test]
fn test_all_layer_adaptations() {
    // BareMetalOS
    let adapter = LayerCapabilityAdapter::new(DeploymentLayer::BareMetalOS);
    let caps = adapter.get_adapted_capabilities();
    assert!(!caps.to_capability_list().is_empty());

    // MiddlewareLayer
    let adapter = LayerCapabilityAdapter::new(DeploymentLayer::MiddlewareLayer {
        host_os: "Linux".to_string(),
        host_version: Some("5.0".to_string()),
    });
    let caps = adapter.get_adapted_capabilities();
    assert!(!caps.to_capability_list().is_empty());

    // ServiceLayer
    let adapter = LayerCapabilityAdapter::new(DeploymentLayer::ServiceLayer {
        guest_os: vec!["Alpine".to_string()],
    });
    let caps = adapter.get_adapted_capabilities();
    assert!(!caps.to_capability_list().is_empty());

    // ContainerLayer
    let adapter = LayerCapabilityAdapter::new(DeploymentLayer::ContainerLayer {
        runtime: toadstool::deployment_layer::ContainerRuntime::Docker,
        container_id: Some("abc123".to_string()),
    });
    let caps = adapter.get_adapted_capabilities();
    assert!(!caps.to_capability_list().is_empty());

    // VMLayer
    let adapter = LayerCapabilityAdapter::new(DeploymentLayer::VMLayer {
        hypervisor: "KVM".to_string(),
        gpu_passthrough: true,
    });
    let caps = adapter.get_adapted_capabilities();
    assert!(!caps.to_capability_list().is_empty());

    // CloudLayer
    let adapter = LayerCapabilityAdapter::new(DeploymentLayer::CloudLayer {
        provider: toadstool::deployment_layer::CloudProvider::AWS,
        instance_type: Some("t3.micro".to_string()),
        region: Some("us-west-1".to_string()),
    });
    let caps = adapter.get_adapted_capabilities();
    assert!(!caps.to_capability_list().is_empty());
}

/// Test: Repeated layer detection (should be consistent)
#[tokio::test]
async fn test_repeated_layer_detection() {
    let mut detector1 = LayerDetector::new();
    let mut detector2 = LayerDetector::new();
    let mut detector3 = LayerDetector::new();

    let layer1 = detector1.detect().await.unwrap();
    let layer2 = detector2.detect().await.unwrap();
    let layer3 = detector3.detect().await.unwrap();

    // All should detect the same layer
    let s1 = format!("{layer1}");
    let s2 = format!("{layer2}");
    let s3 = format!("{layer3}");

    assert_eq!(s1, s2);
    assert_eq!(s2, s3);
}

/// Test: Composition with duplicate workload names
#[tokio::test]
async fn test_duplicate_workload_names() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    // Add same workload name twice
    let req1 = CompositionRequest::new("duplicate").with_constraint(Constraint::min_memory_gb(1.0));

    let req2 = CompositionRequest::new("duplicate").with_constraint(Constraint::min_memory_gb(2.0));

    compositor.add_request(req1);
    compositor.add_request(req2);

    let plan = compositor.compose().await.unwrap();

    // Should handle duplicates (both should be evaluated)
    assert_eq!(plan.placements.len(), 2);
}

/// Test: Clear and re-add workloads
#[tokio::test]
async fn test_clear_and_readd_workloads() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let req = CompositionRequest::new("test").with_constraint(Constraint::min_memory_gb(1.0));

    compositor.add_request(req.clone());
    assert_eq!(compositor.request_count(), 1);

    compositor.clear_requests();
    assert_eq!(compositor.request_count(), 0);

    compositor.add_request(req);
    assert_eq!(compositor.request_count(), 1);
}

/// Test: Plugin system disabled
#[test]
fn test_plugin_system_disabled() {
    let config = PluginConfig {
        enabled: false,
        ..Default::default()
    };

    let mut manager = PluginManager::with_config(config);

    let manifest = PluginManifest {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: "test".to_string(),
        entry_point: "lib.so".to_string(),
        ..Default::default()
    };

    let result = manager.register_plugin(manifest);
    assert!(result.is_err()); // Should fail when disabled
}

/// Test: Constraint satisfaction edge cases
#[tokio::test]
async fn test_constraint_satisfaction_edge_cases() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Minimum values
    let req = CompositionRequest::new("min-values")
        .with_constraint(Constraint::min_memory_gb(0.0))
        .with_constraint(Constraint::min_cpu_cores(0))
        .with_constraint(Constraint::min_bandwidth_gbps(0.0));

    let eval = engine.evaluate(&req).await.unwrap();
    assert!(eval.is_feasible); // Minimum requirements should always work
}

/// Test: Engine statistics accumulation
#[tokio::test]
async fn test_engine_stats_accumulation() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let initial_stats = engine.stats().await;
    let initial_count = initial_stats.total_evaluations;

    // Perform evaluations
    for i in 0..5 {
        let req = CompositionRequest::new(format!("test-{i}"))
            .with_constraint(Constraint::min_memory_gb(1.0));
        engine.evaluate(&req).await.unwrap();
    }

    let final_stats = engine.stats().await;
    assert_eq!(final_stats.total_evaluations, initial_count + 5);
}

/// Test: Migration stats tracking
#[tokio::test]
async fn test_migration_stats_tracking() {
    let coordinator = MigrationCoordinator::new().await.unwrap();

    let initial_stats = coordinator.stats().await;
    let initial_migrations = initial_stats.total_migrations;

    // Perform migration
    let result = coordinator.migrate_workload("test-workload").await;
    assert!(result.is_ok());

    let final_stats = coordinator.stats().await;
    assert_eq!(final_stats.total_migrations, initial_migrations + 1);
    assert_eq!(
        final_stats.successful_migrations,
        initial_stats.successful_migrations + 1
    );
}

/// Test: Workload location persistence
#[tokio::test]
async fn test_workload_location_persistence() {
    let coordinator = MigrationCoordinator::new().await.unwrap();

    use toadstool::cloud_provider_trait::WorkloadLocation;

    let location = WorkloadLocation::Local {
        hostname: "test-host".to_string(),
    };

    coordinator
        .track_workload("persistent-workload".to_string(), location.clone())
        .await;

    let retrieved = coordinator
        .get_workload_location("persistent-workload")
        .await;
    assert!(retrieved.is_some());
}

/// Test: Concurrent plugin operations
#[tokio::test]
async fn test_concurrent_plugin_operations() {
    let manager = std::sync::Arc::new(tokio::sync::Mutex::new(PluginManager::new()));

    let mut handles = vec![];

    // 10 concurrent plugin registrations
    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let manifest = PluginManifest {
                name: format!("concurrent-plugin-{i}"),
                version: "1.0.0".to_string(),
                plugin_type: "test".to_string(),
                entry_point: format!("lib{i}.so"),
                ..Default::default()
            };

            let mut mgr = manager_clone.lock().await;
            mgr.register_plugin(manifest)
        });
        handles.push(handle);
    }

    // All should complete successfully
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
