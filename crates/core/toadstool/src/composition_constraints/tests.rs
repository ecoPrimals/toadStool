// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::float_cmp)]

use super::*;
use std::collections::HashMap;

#[test]
fn test_constraint_creation() {
    let c1 = Constraint::requires_gpu();
    assert!(c1.is_hard());
    assert_eq!(c1.name(), "requires_gpu");
    let c2 = Constraint::prefers_gpu();
    assert!(c2.is_soft());
    assert_eq!(c2.name(), "prefers_gpu");
}

#[test]
fn test_constraint_factory_methods() {
    assert!(Constraint::max_latency_ms(16).is_hard());
    assert!(Constraint::preferred_latency_ms(16).is_soft());
    assert!(Constraint::min_bandwidth_gbps(10.0).is_hard());
    assert!(Constraint::min_memory_gb(8.0).is_hard());
    assert!(Constraint::min_cpu_cores(4).is_hard());
    assert!(Constraint::must_be_local().is_hard());
    assert!(Constraint::prefer_local().is_soft());
    assert!(Constraint::requires_capability("cuda").is_hard());
    assert!(Constraint::prefers_capability("akida").is_soft());
}

#[test]
fn test_constraint_is_hard_soft_all_variants() {
    assert!(Constraint::RequiresGPU.is_hard());
    assert!(Constraint::PrefersGPU.is_soft());
    assert!(Constraint::Custom {
        name: "x".into(),
        hard: true,
        value: "v".into(),
    }
    .is_hard());
    assert!(Constraint::Custom {
        name: "x".into(),
        hard: false,
        value: "v".into(),
    }
    .is_soft());
}

#[test]
fn test_constraint_display() {
    assert_eq!(
        format!("{}", Constraint::max_latency_ms(100)),
        "MaxLatency: 100ms [HARD]"
    );
}

#[test]
fn test_priority_display_and_default() {
    assert_eq!(format!("{}", ConstraintPriority::Background), "Background");
    assert_eq!(ConstraintPriority::default(), ConstraintPriority::Normal);
}

#[test]
fn test_composition_request_builder() {
    let request = CompositionRequest::new("test")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::prefer_local())
        .with_priority(ConstraintPriority::High)
        .with_metadata("workload_type", "gaming");
    assert_eq!(request.name, "test");
    assert_eq!(request.constraints.len(), 2);
    assert_eq!(request.priority, ConstraintPriority::High);
}

#[test]
fn test_composition_request_empty() {
    let request = CompositionRequest::new("empty");
    assert!(request.constraints.is_empty());
    assert!(request.hard_constraints().is_empty());
}

#[test]
fn test_constraint_satisfaction_score() {
    let satisfied = ConstraintSatisfaction::Satisfied;
    assert_eq!(satisfied.score(), 1.0);
    assert!(satisfied.is_satisfied());
    let partial = ConstraintSatisfaction::Partial(0.7);
    assert_eq!(partial.score(), 0.7);
    let unsatisfied = ConstraintSatisfaction::Unsatisfied {
        reason: "no GPU".to_string(),
    };
    assert_eq!(unsatisfied.score(), 0.0);
    assert!(!unsatisfied.is_satisfied());
}

#[test]
fn test_priority_ordering() {
    assert!(ConstraintPriority::Critical > ConstraintPriority::High);
    assert!(ConstraintPriority::High > ConstraintPriority::Normal);
}

#[test]
fn test_constraint_evaluation_get_satisfaction() {
    let request = CompositionRequest::new("eval")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::prefer_local());
    let mut results = HashMap::new();
    results.insert(
        "requires_gpu".to_string(),
        ConstraintSatisfaction::Satisfied,
    );
    results.insert(
        "prefer_local".to_string(),
        ConstraintSatisfaction::Partial(0.5),
    );
    let eval = ConstraintEvaluation {
        request: request.clone(),
        results,
        overall_score: 0.75,
        is_feasible: true,
    };
    assert_eq!(
        eval.get_satisfaction("requires_gpu"),
        Some(&ConstraintSatisfaction::Satisfied)
    );
    assert_eq!(eval.get_satisfaction("nonexistent"), None);
}

#[test]
fn test_constraint_evaluation_soft_constraint_score() {
    let request = CompositionRequest::new("eval")
        .with_constraint(Constraint::prefers_gpu())
        .with_constraint(Constraint::prefer_local());
    let mut results = HashMap::new();
    results.insert("prefers_gpu".to_string(), ConstraintSatisfaction::Satisfied);
    results.insert(
        "prefer_local".to_string(),
        ConstraintSatisfaction::Partial(0.6),
    );
    let eval = ConstraintEvaluation {
        request,
        results,
        overall_score: 0.8,
        is_feasible: true,
    };
    assert!((eval.soft_constraint_score() - 0.8).abs() < 0.001);
}

#[test]
fn test_serialization_constraint_roundtrip() {
    let c = Constraint::RequiresGPU;
    let json = serde_json::to_string(&c).unwrap();
    let deserialized: Constraint = serde_json::from_str(&json).unwrap();
    assert_eq!(&c, &deserialized);
}

#[test]
fn test_serialization_composition_request_roundtrip() {
    let request = CompositionRequest::new("gaming")
        .with_constraint(Constraint::requires_gpu())
        .with_constraint(Constraint::max_latency_ms(16))
        .with_priority(ConstraintPriority::Critical)
        .with_metadata("workload_type", "gaming");
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: CompositionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(request.name, deserialized.name);
    assert_eq!(request.constraints.len(), deserialized.constraints.len());
}
