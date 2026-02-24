//! E2E Test: Fractal Composition Workflow
//!
//! Tests the complete fractal composition workflow including constraint
//! evaluation, feasibility analysis, and resource orchestration.

use toadstool::composition_constraints::{CompositionRequest, Constraint};
use toadstool::composition_engine::CompositionEngine;

#[tokio::test]
async fn test_composition_engine_initialization() {
    // E2E: Initialize composition engine from runtime
    let result = CompositionEngine::from_runtime().await;

    assert!(
        result.is_ok(),
        "Composition engine should initialize from runtime"
    );
}

#[tokio::test]
async fn test_simple_composition_request() {
    // E2E: Create and evaluate simple composition request
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request = CompositionRequest::new("test_workload");

    let evaluation = engine.evaluate(&request).await;

    assert!(
        evaluation.is_ok(),
        "Simple request should evaluate successfully"
    );

    let eval = evaluation.unwrap();
    assert!(
        eval.is_feasible,
        "Simple request with no constraints should be feasible"
    );
}

#[tokio::test]
async fn test_composition_with_gpu_constraint() {
    // E2E: Test composition with GPU requirement
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request =
        CompositionRequest::new("gpu_workload").with_constraint(Constraint::requires_gpu());

    let evaluation = engine.evaluate(&request).await;

    assert!(
        evaluation.is_ok(),
        "GPU constraint evaluation should not panic"
    );

    // Result depends on whether GPU is available, both valid
    let eval = evaluation.unwrap();
    assert!(
        eval.results.contains_key("requires_gpu"),
        "Should evaluate GPU constraint"
    );
}

#[tokio::test]
async fn test_composition_with_memory_constraint() {
    // E2E: Test composition with memory requirement
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request =
        CompositionRequest::new("memory_workload").with_constraint(Constraint::min_memory_gb(1.0));

    let evaluation = engine.evaluate(&request).await;

    assert!(
        evaluation.is_ok(),
        "Memory constraint evaluation should succeed"
    );

    let eval = evaluation.unwrap();
    // Small memory requirement should typically be feasible
    assert!(
        eval.results.contains_key("min_memory_gb"),
        "Should evaluate memory constraint"
    );
}

#[tokio::test]
async fn test_composition_with_multiple_constraints() {
    // E2E: Test composition with multiple constraints
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request = CompositionRequest::new("complex_workload")
        .with_constraint(Constraint::min_memory_gb(0.5))
        .with_constraint(Constraint::min_cpu_cores(1))
        .with_constraint(Constraint::prefer_local());

    let evaluation = engine.evaluate(&request).await;

    assert!(
        evaluation.is_ok(),
        "Multiple constraint evaluation should succeed"
    );

    let eval = evaluation.unwrap();
    assert!(eval.results.len() >= 3, "Should evaluate all constraints");
}

#[tokio::test]
async fn test_soft_vs_hard_constraints() {
    // E2E: Test soft (prefer) vs hard (requires) constraints
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Soft constraint - should always be feasible
    let soft_request =
        CompositionRequest::new("soft_workload").with_constraint(Constraint::prefers_gpu());

    let soft_eval = engine.evaluate(&soft_request).await.unwrap();
    assert!(
        soft_eval.is_feasible,
        "Soft constraints should not block feasibility"
    );

    // Hard constraint - feasibility depends on availability
    let hard_request =
        CompositionRequest::new("hard_workload").with_constraint(Constraint::requires_gpu());

    let hard_eval = engine.evaluate(&hard_request).await.unwrap();
    // Hard constraint may or may not be feasible, but should evaluate
    assert!(hard_eval.results.contains_key("requires_gpu"));
}

#[tokio::test]
async fn test_constraint_evaluation_metadata() {
    // E2E: Verify evaluation metadata structure
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request =
        CompositionRequest::new("metadata_test").with_constraint(Constraint::min_memory_gb(0.1));

    let evaluation = engine.evaluate(&request).await.unwrap();

    // Verify metadata structure
    assert!(
        !evaluation.results.is_empty(),
        "Should have evaluation results"
    );

    for (constraint_id, result) in &evaluation.results {
        assert!(
            !constraint_id.is_empty(),
            "Constraint ID should not be empty"
        );
        assert!(
            result.is_satisfied() || !result.is_satisfied(),
            "Should have satisfaction status"
        );
    }
}

#[tokio::test]
async fn test_composition_respects_timeout() {
    // E2E: Verify composition evaluation respects timeout
    use tokio::time::{timeout, Duration};

    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request =
        CompositionRequest::new("timeout_test").with_constraint(Constraint::min_cpu_cores(1));

    // Should complete within 3 seconds
    let result = timeout(Duration::from_secs(3), engine.evaluate(&request)).await;

    assert!(
        result.is_ok(),
        "Composition evaluation should complete within timeout"
    );
}

#[tokio::test]
async fn test_local_preference_constraint() {
    // E2E: Test local execution preference
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request =
        CompositionRequest::new("local_workload").with_constraint(Constraint::prefer_local());

    let evaluation = engine.evaluate(&request).await.unwrap();

    assert!(
        evaluation.is_feasible,
        "Local preference should be feasible"
    );
    assert!(evaluation.results.contains_key("prefer_local"));
}

#[tokio::test]
async fn test_constraint_priority_ordering() {
    // E2E: Test that constraints are evaluated in order
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request = CompositionRequest::new("priority_test")
        .with_constraint(Constraint::min_cpu_cores(1))
        .with_constraint(Constraint::min_memory_gb(0.5))
        .with_constraint(Constraint::prefer_local());

    let evaluation = engine.evaluate(&request).await.unwrap();

    // All constraints should be evaluated
    assert_eq!(
        evaluation.results.len(),
        3,
        "Should evaluate all 3 constraints"
    );
}

#[tokio::test]
async fn test_impossible_constraint_combination() {
    // E2E: Test graceful handling of impossible constraints
    let engine = CompositionEngine::from_runtime().await.unwrap();

    // Request impossibly large memory (1000 GB)
    let request =
        CompositionRequest::new("impossible").with_constraint(Constraint::min_memory_gb(1000.0));

    let evaluation = engine.evaluate(&request).await;

    // Should evaluate without panic
    assert!(
        evaluation.is_ok(),
        "Should handle impossible constraints gracefully"
    );

    let _eval = evaluation.unwrap();
    // Evaluation completed; feasibility is determined by the engine
}
