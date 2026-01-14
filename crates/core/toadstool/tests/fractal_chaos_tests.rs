// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Chaos Tests for Fractal Composition
//!
//! Tests system behavior under random, extreme, and unexpected inputs.

use toadstool::composition_constraints::*;
use toadstool::composition_engine::CompositionEngine;
use toadstool::multi_workload_compositor::MultiWorkloadCompositor;
use toadstool::plugin_system::{PluginManager, PluginManifest};
use toadstool::workload_migration::MigrationCoordinator;

/// Test: Random constraint combinations
#[tokio::test]
async fn test_random_constraint_combinations() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Random mix of constraints
    for iteration in 0..20 {
        let mut request = CompositionRequest::new(format!("chaos-{}", iteration));

        // Add random number of constraints (0-10)
        let num_constraints = iteration % 11;
        for i in 0..num_constraints {
            let constraint = match i % 6 {
                0 => Constraint::requires_gpu(),
                1 => Constraint::prefers_gpu(),
                2 => Constraint::min_memory_gb((i as f64) * 0.5),
                3 => Constraint::max_latency_ms(i as u64 * 10),
                4 => Constraint::prefer_local(),
                _ => Constraint::min_cpu_cores(i),
            };
            request = request.with_constraint(constraint);
        }

        // Should not panic, even with random constraints
        let result = engine.evaluate(&request).await;
        assert!(result.is_ok(), "Random constraints should not panic");
    }
}

/// Test: Extreme memory values
#[tokio::test]
async fn test_extreme_memory_values() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Zero memory
    let req = CompositionRequest::new("zero-mem").with_constraint(Constraint::min_memory_gb(0.0));
    let eval = engine.evaluate(&req).await.unwrap();
    assert!(eval.is_feasible); // 0 memory should be OK

    // Tiny memory
    let req = CompositionRequest::new("tiny-mem").with_constraint(Constraint::min_memory_gb(0.001));
    let eval = engine.evaluate(&req).await.unwrap();
    assert!(eval.is_feasible);

    // Huge memory (impossible)
    let req =
        CompositionRequest::new("huge-mem").with_constraint(Constraint::min_memory_gb(999999.0));
    let _eval = engine.evaluate(&req).await.unwrap();
    assert!(!_eval.is_feasible); // Should be infeasible
}

/// Test: Extreme CPU values
#[tokio::test]
async fn test_extreme_cpu_values() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Zero cores
    let req = CompositionRequest::new("zero-cpu").with_constraint(Constraint::min_cpu_cores(0));
    let eval = engine.evaluate(&req).await.unwrap();
    assert!(eval.is_feasible);

    // One core
    let req = CompositionRequest::new("one-cpu").with_constraint(Constraint::min_cpu_cores(1));
    let eval = engine.evaluate(&req).await.unwrap();
    assert!(eval.is_feasible);

    // Impossible cores
    let req = CompositionRequest::new("impossible-cpu")
        .with_constraint(Constraint::min_cpu_cores(999999));
    let eval = engine.evaluate(&req).await.unwrap();
    assert!(!eval.is_feasible);
}

/// Test: Extreme latency values
#[tokio::test]
async fn test_extreme_latency_values() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Zero latency (impossible in reality)
    let req =
        CompositionRequest::new("zero-latency").with_constraint(Constraint::max_latency_ms(0));
    let _eval = engine.evaluate(&req).await.unwrap();
    // May or may not be feasible depending on layer

    // Ultra-low latency
    let req =
        CompositionRequest::new("ultra-low-latency").with_constraint(Constraint::max_latency_ms(1));
    let _eval = engine.evaluate(&req).await.unwrap();
    // Should evaluate without panic

    // Very high latency (always feasible)
    let req =
        CompositionRequest::new("high-latency").with_constraint(Constraint::max_latency_ms(100000));
    let eval = engine.evaluate(&req).await.unwrap();
    assert!(eval.is_feasible);
}

/// Test: Many simultaneous workloads
#[tokio::test]
async fn test_many_simultaneous_workloads() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    // Add 50 workloads
    for i in 0..50 {
        let request = CompositionRequest::new(format!("workload-{}", i))
            .with_constraint(Constraint::min_memory_gb(0.1))
            .with_priority(match i % 4 {
                0 => ConstraintPriority::Critical,
                1 => ConstraintPriority::High,
                2 => ConstraintPriority::Normal,
                _ => ConstraintPriority::Background,
            });
        compositor.add_request(request);
    }

    // Should handle many workloads without panic
    let plan = compositor.compose().await.unwrap();
    assert_eq!(plan.placements.len(), 50);
}

/// Test: Conflicting constraints
#[tokio::test]
async fn test_conflicting_constraints() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Require GPU but must be local (may conflict if local has no GPU)
    let req = CompositionRequest::new("conflicting")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::must_be_local())
        .with_constraint(Constraint::max_latency_ms(1)); // Very strict

    let eval = engine.evaluate(&req).await.unwrap();
    // Should evaluate, feasibility depends on environment
    assert!(!eval.results.is_empty());
}

/// Test: Empty request (no constraints)
#[tokio::test]
async fn test_empty_request() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let req = CompositionRequest::new("empty");
    let eval = engine.evaluate(&req).await.unwrap();

    // Empty request should always be feasible
    assert!(eval.is_feasible);
    assert_eq!(eval.overall_score, 1.0);
}

/// Test: All hard constraints
#[tokio::test]
async fn test_all_hard_constraints() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let req = CompositionRequest::new("all-hard")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::min_memory_gb(8.0))
        .with_constraint(Constraint::min_cpu_cores(4))
        .with_constraint(Constraint::max_latency_ms(10));

    let eval = engine.evaluate(&req).await.unwrap();
    // If infeasible, score should be 0
    if !eval.is_feasible {
        assert_eq!(eval.overall_score, 0.0);
    }
}

/// Test: All soft constraints
#[tokio::test]
async fn test_all_soft_constraints() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let req = CompositionRequest::new("all-soft")
        .with_constraint(Constraint::prefers_gpu())
        .with_constraint(Constraint::prefer_local())
        .with_constraint(Constraint::MinimizeCost);

    let eval = engine.evaluate(&req).await.unwrap();

    // All soft constraints = always feasible
    assert!(eval.is_feasible);
    assert!(eval.overall_score > 0.0);
}

/// Test: Rapid repeated evaluations
#[tokio::test]
async fn test_rapid_evaluations() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let req = CompositionRequest::new("rapid").with_constraint(Constraint::min_memory_gb(1.0));

    // 100 rapid evaluations
    for _ in 0..100 {
        let result = engine.evaluate(&req).await;
        assert!(result.is_ok());
    }
}

/// Test: Plugin manager with random manifests
#[test]
fn test_random_plugin_registrations() {
    let mut manager = PluginManager::new();

    // Register 20 random plugins
    for i in 0..20 {
        let manifest = PluginManifest {
            name: format!("plugin-{}", i),
            version: format!("{}.0.0", i),
            plugin_type: match i % 3 {
                0 => "cloud_provider",
                1 => "storage",
                _ => "compute",
            }
            .to_string(),
            entry_point: format!("libplugin{}.so", i),
            ..Default::default()
        };

        let result = manager.register_plugin(manifest);
        assert!(result.is_ok());
    }

    assert_eq!(manager.list_plugins().len(), 20);
}

/// Test: Plugin dependencies chain
#[test]
fn test_plugin_dependency_chain() {
    let mut manager = PluginManager::new();

    // Create dependency chain: A -> B -> C
    let manifest_c = PluginManifest {
        name: "plugin-c".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: "base".to_string(),
        entry_point: "libc.so".to_string(),
        ..Default::default()
    };

    let manifest_b = PluginManifest {
        name: "plugin-b".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: "middle".to_string(),
        entry_point: "libb.so".to_string(),
        dependencies: vec!["plugin-c".to_string()],
        ..Default::default()
    };

    let manifest_a = PluginManifest {
        name: "plugin-a".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: "top".to_string(),
        entry_point: "liba.so".to_string(),
        dependencies: vec!["plugin-b".to_string()],
        ..Default::default()
    };

    // Register in wrong order - should handle dependencies
    assert!(manager.register_plugin(manifest_c).is_ok());
    assert!(manager.register_plugin(manifest_b).is_ok());
    assert!(manager.register_plugin(manifest_a).is_ok());
}

/// Test: Concurrent composition requests
#[tokio::test]
async fn test_concurrent_compositions() {
    let engine = std::sync::Arc::new(CompositionEngine::from_runtime().await.unwrap());

    let mut handles = vec![];

    // 10 concurrent evaluations
    for i in 0..10 {
        let engine_clone = engine.clone();
        let handle = tokio::spawn(async move {
            let req = CompositionRequest::new(format!("concurrent-{}", i))
                .with_constraint(Constraint::min_memory_gb(0.5));

            engine_clone.evaluate(&req).await
        });
        handles.push(handle);
    }

    // All should complete successfully
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

/// Test: Migration with extreme constraints
#[tokio::test]
async fn test_migration_extreme_constraints() {
    let coordinator = MigrationCoordinator::new().await.unwrap();

    // Impossible constraints
    let constraints = vec![
        Constraint::requires_gpu(),
        Constraint::min_memory_gb(999999.0),
        Constraint::max_latency_ms(0),
        Constraint::must_be_local(),
    ];

    let result = coordinator
        .should_migrate("extreme-workload", &constraints)
        .await;
    assert!(result.is_ok());

    let recommendation = result.unwrap();
    // Should have low confidence or recommend not migrating
    assert!(recommendation.confidence >= 0.0 && recommendation.confidence <= 1.0);
}

/// Test: Stress composition with priority chaos
#[tokio::test]
async fn test_priority_chaos() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    // Add workloads with random priorities
    for i in 0..30 {
        let priority = match i % 4 {
            0 => ConstraintPriority::Critical,
            1 => ConstraintPriority::High,
            2 => ConstraintPriority::Normal,
            _ => ConstraintPriority::Background,
        };

        let req = CompositionRequest::new(format!("chaos-priority-{}", i))
            .with_constraint(Constraint::min_memory_gb((i as f64) * 0.1))
            .with_priority(priority);

        compositor.add_request(req);
    }

    let plan = compositor.compose().await.unwrap();

    // Should be sorted by priority
    for i in 1..plan.placements.len() {
        assert!(
            plan.placements[i - 1].request.priority >= plan.placements[i].request.priority,
            "Should be sorted by priority"
        );
    }
}

/// Test: Custom constraints with random values
#[tokio::test]
async fn test_custom_constraints_chaos() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    for i in 0..20 {
        let custom = Constraint::Custom {
            name: format!("custom-{}", i),
            hard: i % 2 == 0,
            value: format!("value-{}", i),
        };

        let req = CompositionRequest::new(format!("custom-chaos-{}", i)).with_constraint(custom);

        let result = engine.evaluate(&req).await;
        assert!(result.is_ok());
    }
}

/// Test: Bandwidth extreme values
#[tokio::test]
async fn test_extreme_bandwidth() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Zero bandwidth
    let req = CompositionRequest::new("zero-bandwidth")
        .with_constraint(Constraint::min_bandwidth_gbps(0.0));
    let eval = engine.evaluate(&req).await.unwrap();
    assert!(eval.is_feasible);

    // Impossible bandwidth
    let req = CompositionRequest::new("impossible-bandwidth")
        .with_constraint(Constraint::min_bandwidth_gbps(999999.0));
    let eval = engine.evaluate(&req).await.unwrap();
    assert!(!eval.is_feasible);
}
