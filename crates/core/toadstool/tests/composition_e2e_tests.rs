// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! End-to-End Composition Tests
//!
//! These tests validate the complete composition system from request to plan.

use toadstool::composition_constraints::*;
use toadstool::composition_engine::CompositionEngine;
use toadstool::multi_workload_compositor::MultiWorkloadCompositor;

/// Test: Gaming workload with GPU requirement
#[tokio::test]
async fn test_gaming_workload() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let gaming = CompositionRequest::new("gaming")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::max_latency_ms(16)) // 60 FPS
        .with_priority(ConstraintPriority::Critical)
        .with_metadata("fps_target", "60");

    let eval = engine.evaluate(&gaming).await.unwrap();

    // Gaming should be evaluated (feasibility depends on GPU availability)
    assert!(!eval.results.is_empty());
    assert!(eval.results.contains_key("requires_gpu"));
    assert!(eval.results.contains_key("max_latency_ms"));
}

/// Test: `OpenFold` workload with GPU preference
#[tokio::test]
async fn test_openfold_workload() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let openfold = CompositionRequest::new("openfold")
        .with_constraint(Constraint::prefers_gpu())
        .with_constraint(Constraint::min_memory_gb(4.0))
        .with_constraint(Constraint::min_bandwidth_gbps(10.0))
        .with_priority(ConstraintPriority::High);

    let eval = engine.evaluate(&openfold).await.unwrap();

    // OpenFold always feasible (GPU is preferred, not required)
    // Memory might fail if system has < 4GB
    assert!(!eval.results.is_empty());
}

/// Test: Streaming workload (CPU-only)
#[tokio::test]
async fn test_streaming_workload() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let streaming = CompositionRequest::new("streaming")
        .with_constraint(Constraint::min_cpu_cores(2))
        .with_constraint(Constraint::min_bandwidth_gbps(5.0))
        .with_constraint(Constraint::max_latency_ms(100))
        .with_priority(ConstraintPriority::Normal);

    let eval = engine.evaluate(&streaming).await.unwrap();

    // Should evaluate all constraints
    assert_eq!(eval.results.len(), 3);
}

/// Test: AI training workload (background, minimize cost)
#[tokio::test]
async fn test_ai_training_workload() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let ai_training = CompositionRequest::new("ai_training")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::min_memory_gb(8.0))
        .with_constraint(Constraint::MinimizeCost)
        .with_priority(ConstraintPriority::Background);

    let eval = engine.evaluate(&ai_training).await.unwrap();

    // Should have GPU, memory, and cost constraints evaluated
    assert!(eval.results.len() >= 3);
}

/// Test: Gaming + `OpenFold` composition
#[tokio::test]
async fn test_gaming_plus_openfold() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let gaming = CompositionRequest::new("gaming")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::max_latency_ms(16))
        .with_priority(ConstraintPriority::Critical);

    let openfold = CompositionRequest::new("openfold")
        .with_constraint(Constraint::prefers_gpu())
        .with_constraint(Constraint::min_memory_gb(2.0))
        .with_priority(ConstraintPriority::High);

    compositor.add_request(gaming);
    compositor.add_request(openfold);

    let plan = compositor.compose().await.unwrap();

    // Should have placements for both
    assert_eq!(plan.placements.len(), 2);

    // Gaming should be evaluated first (Critical > High)
    assert_eq!(plan.placements[0].request.name, "gaming");
    assert_eq!(plan.placements[1].request.name, "openfold");
}

/// Test: Full "impossible stack" (Gaming + `OpenFold` + Streaming + AI)
#[tokio::test]
async fn test_impossible_stack() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let gaming = CompositionRequest::new("gaming")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::max_latency_ms(16))
        .with_priority(ConstraintPriority::Critical);

    let openfold = CompositionRequest::new("openfold")
        .with_constraint(Constraint::prefers_gpu())
        .with_constraint(Constraint::min_bandwidth_gbps(10.0))
        .with_priority(ConstraintPriority::High);

    let streaming = CompositionRequest::new("streaming")
        .with_constraint(Constraint::min_cpu_cores(2))
        .with_constraint(Constraint::max_latency_ms(100))
        .with_priority(ConstraintPriority::Normal);

    let ai_training = CompositionRequest::new("ai_training")
        .with_constraint(Constraint::prefers_gpu())
        .with_constraint(Constraint::MinimizeCost)
        .with_priority(ConstraintPriority::Background);

    compositor.add_request(gaming);
    compositor.add_request(openfold);
    compositor.add_request(streaming);
    compositor.add_request(ai_training);

    let plan = compositor.compose().await.unwrap();

    // Should have placements for all 4 workloads
    assert_eq!(plan.placements.len(), 4);

    // Should be ordered by priority
    assert_eq!(
        plan.placements[0].request.priority,
        ConstraintPriority::Critical
    );
    assert_eq!(
        plan.placements[1].request.priority,
        ConstraintPriority::High
    );
    assert_eq!(
        plan.placements[2].request.priority,
        ConstraintPriority::Normal
    );
    assert_eq!(
        plan.placements[3].request.priority,
        ConstraintPriority::Background
    );

    // At least some should be feasible (depends on system)
    let feasible_count = plan.feasible_placements().len();
    assert!(
        feasible_count >= 1,
        "At least one workload should be feasible"
    );

    // Average score should be reasonable
    let avg_score = plan.average_score();
    assert!((0.0..=1.0).contains(&avg_score));
}

/// Test: Priority-based resource allocation
#[tokio::test]
async fn test_priority_allocation() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    // Both require GPU, but different priorities
    let critical = CompositionRequest::new("critical_task")
        .with_constraint(Constraint::requires_gpu())
        .with_priority(ConstraintPriority::Critical);

    let normal = CompositionRequest::new("normal_task")
        .with_constraint(Constraint::requires_gpu())
        .with_priority(ConstraintPriority::Normal);

    compositor.add_request(normal); // Add lower priority first
    compositor.add_request(critical); // Add higher priority second

    let plan = compositor.compose().await.unwrap();

    // Critical should be evaluated first despite being added second
    assert_eq!(plan.placements[0].request.name, "critical_task");
    assert_eq!(plan.placements[1].request.name, "normal_task");
}

/// Test: Resource utilization tracking
#[tokio::test]
async fn test_resource_utilization_tracking() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let req1 = CompositionRequest::new("task1")
        .with_constraint(Constraint::min_memory_gb(1.0))
        .with_constraint(Constraint::min_cpu_cores(2));

    let req2 = CompositionRequest::new("task2")
        .with_constraint(Constraint::min_memory_gb(2.0))
        .with_constraint(Constraint::min_cpu_cores(4));

    compositor.add_request(req1);
    compositor.add_request(req2);

    let plan = compositor.compose().await.unwrap();

    // Should have resource utilization info
    let util = &plan.resource_utilization;

    assert!(util.memory_gb_total > 0.0);
    assert!(util.cpu_cores_total > 0);

    // Utilization percentages should be valid
    assert!(util.memory_utilization_percent() >= 0.0);
    assert!(util.cpu_utilization_percent() >= 0.0);
    assert!(util.gpu_utilization_percent() >= 0.0);
}

/// Test: Soft constraint scoring
#[tokio::test]
async fn test_soft_constraint_scoring() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let workload = CompositionRequest::new("test")
        .with_constraint(Constraint::prefers_gpu())
        .with_constraint(Constraint::prefer_local())
        .with_constraint(Constraint::MinimizeCost);

    let eval = engine.evaluate(&workload).await.unwrap();

    // All soft constraints - should always be feasible
    assert!(eval.is_feasible);

    // Score should reflect satisfaction of soft constraints
    assert!(eval.overall_score > 0.0);
}

/// Test: Conflict detection
#[tokio::test]
async fn test_conflict_detection() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    // Create workloads that might conflict (both require very high resources)
    let req1 = CompositionRequest::new("heavy1")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::min_memory_gb(999.0)) // Intentionally impossible
        .with_priority(ConstraintPriority::High);

    let req2 = CompositionRequest::new("heavy2")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::min_memory_gb(999.0)) // Intentionally impossible
        .with_priority(ConstraintPriority::Normal);

    compositor.add_request(req1);
    compositor.add_request(req2);

    let plan = compositor.compose().await.unwrap();

    // At least one should be infeasible (probably both due to impossible memory)
    assert!(!plan.overall_feasibility);

    // Should have detected conflicts
    assert!(!plan.conflicts.is_empty());
}

/// Test: Mixed hard and soft constraints
#[tokio::test]
async fn test_mixed_constraints() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let workload = CompositionRequest::new("mixed")
        .with_constraint(Constraint::min_memory_gb(0.1)) // Hard
        .with_constraint(Constraint::min_cpu_cores(1)) // Hard
        .with_constraint(Constraint::prefers_gpu()) // Soft
        .with_constraint(Constraint::prefer_local()); // Soft

    let eval = engine.evaluate(&workload).await.unwrap();

    // Hard constraints (memory, CPU) should be satisfied
    assert!(eval.results.get("min_memory_gb").unwrap().is_satisfied());
    assert!(eval.results.get("min_cpu_cores").unwrap().is_satisfied());

    // Should be feasible (hard constraints OK)
    assert!(eval.is_feasible);

    // Score reflects both hard and soft satisfaction
    assert!(eval.overall_score > 0.0);
}

/// Test: Composition plan statistics
#[tokio::test]
async fn test_composition_plan_statistics() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    // Add mix of feasible and potentially infeasible workloads
    let feasible =
        CompositionRequest::new("feasible").with_constraint(Constraint::min_memory_gb(0.1));

    let might_fail = CompositionRequest::new("might_fail")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::min_memory_gb(100.0));

    compositor.add_request(feasible);
    compositor.add_request(might_fail);

    let plan = compositor.compose().await.unwrap();

    // Should have exactly 2 placements
    assert_eq!(plan.placements.len(), 2);

    // Feasible count should be at least 1 (the easy one)
    assert!(plan.feasible_placements().len() == 1 || plan.feasible_placements().len() > 1);

    // Average score should be calculable
    let avg = plan.average_score();
    assert!((0.0..=1.0).contains(&avg));
}

/// Test: Empty composition is valid
#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#[tokio::test]
async fn test_empty_composition_valid() {
    let compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();
    let plan = compositor.compose().await.unwrap();

    // Empty composition is always feasible
    assert!(plan.overall_feasibility);
    assert_eq!(plan.placements.len(), 0);
    assert_eq!(plan.conflicts.len(), 0);
    assert_eq!(plan.average_score(), 0.0);
}

/// Test: Custom constraints
#[tokio::test]
async fn test_custom_constraints() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let workload = CompositionRequest::new("custom").with_constraint(Constraint::Custom {
        name: "needs_akida".to_string(),
        hard: true,
        value: "true".to_string(),
    });

    let eval = engine.evaluate(&workload).await.unwrap();

    // Custom constraint should be evaluated
    assert!(eval.results.contains_key("needs_akida"));
}
