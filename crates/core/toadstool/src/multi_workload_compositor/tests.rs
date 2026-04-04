// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2024-2025 ToadStool Project

use super::MultiWorkloadCompositor;
use crate::composition_constraints::{CompositionRequest, Constraint, ConstraintPriority};

#[tokio::test]
async fn test_compositor_initialization() {
    let result = MultiWorkloadCompositor::from_runtime().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_add_requests() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    assert_eq!(compositor.request_count(), 0);

    let req1 = CompositionRequest::new("test1");
    compositor.add_request(req1);

    assert_eq!(compositor.request_count(), 1);

    let req2 = CompositionRequest::new("test2");
    let req3 = CompositionRequest::new("test3");
    compositor.add_requests(vec![req2, req3]);

    assert_eq!(compositor.request_count(), 3);
}

#[tokio::test]
async fn test_empty_composition() {
    let compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();
    let plan = compositor.compose().await.unwrap();

    assert!(plan.overall_feasibility);
    assert_eq!(plan.placements.len(), 0);
    assert_eq!(plan.conflicts.len(), 0);
}

#[tokio::test]
async fn test_single_workload_composition() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let request = CompositionRequest::new("test").with_constraint(Constraint::min_memory_gb(0.1));

    compositor.add_request(request);

    let plan = compositor.compose().await.unwrap();

    assert_eq!(plan.placements.len(), 1);
    assert!(plan.overall_feasibility);
}

#[tokio::test]
async fn test_multi_workload_composition() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let req1 = CompositionRequest::new("workload1")
        .with_constraint(Constraint::min_memory_gb(0.1))
        .with_priority(ConstraintPriority::High);

    let req2 = CompositionRequest::new("workload2")
        .with_constraint(Constraint::min_cpu_cores(1))
        .with_priority(ConstraintPriority::Normal);

    compositor.add_request(req1);
    compositor.add_request(req2);

    let plan = compositor.compose().await.unwrap();

    assert_eq!(plan.placements.len(), 2);

    assert_eq!(plan.feasible_placements().len(), 2);
    assert!(plan.overall_feasibility);
}

#[tokio::test]
async fn test_priority_ordering() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let critical = CompositionRequest::new("critical").with_priority(ConstraintPriority::Critical);

    let background =
        CompositionRequest::new("background").with_priority(ConstraintPriority::Background);

    let high = CompositionRequest::new("high").with_priority(ConstraintPriority::High);

    compositor.add_request(background);
    compositor.add_request(high);
    compositor.add_request(critical);

    let plan = compositor.compose().await.unwrap();

    assert_eq!(plan.placements[0].request.name, "critical");
    assert_eq!(plan.placements[1].request.name, "high");
    assert_eq!(plan.placements[2].request.name, "background");
}

#[tokio::test]
async fn test_resource_utilization() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let req = CompositionRequest::new("test")
        .with_constraint(Constraint::min_memory_gb(1.0))
        .with_constraint(Constraint::min_cpu_cores(2));

    compositor.add_request(req);

    let plan = compositor.compose().await.unwrap();

    assert!(plan.resource_utilization.memory_gb_total > 0.0);
    assert!(plan.resource_utilization.cpu_cores_total > 0);
}

#[tokio::test]
async fn test_plan_statistics() {
    let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

    let req1 = CompositionRequest::new("test1").with_constraint(Constraint::min_memory_gb(0.1));
    let req2 = CompositionRequest::new("test2").with_constraint(Constraint::prefer_local());

    compositor.add_request(req1);
    compositor.add_request(req2);

    let plan = compositor.compose().await.unwrap();

    let avg_score = plan.average_score();
    assert!((0.0..=1.0).contains(&avg_score));

    assert_eq!(plan.feasible_placements().len(), 2);
    assert_eq!(plan.infeasible_placements().len(), 0);
}
