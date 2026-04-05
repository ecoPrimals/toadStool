// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::composition_constraints::Constraint;

#[tokio::test]
async fn test_engine_initialization() {
    let result = CompositionEngine::from_runtime().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_gpu_constraint_evaluation() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request = CompositionRequest::new("test_gpu").with_constraint(Constraint::requires_gpu());

    let eval = engine.evaluate(&request).await.unwrap();

    assert!(eval.results.contains_key("requires_gpu"));

    let has_gpu = engine.runtime.has_gpu_access();
    assert_eq!(eval.is_feasible, has_gpu);
}

#[tokio::test]
async fn test_soft_constraint_evaluation() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request = CompositionRequest::new("test_soft")
        .with_constraint(Constraint::prefers_gpu())
        .with_constraint(Constraint::prefer_local());

    let eval = engine.evaluate(&request).await.unwrap();

    assert!(eval.is_feasible);

    assert!(eval.results.contains_key("prefers_gpu"));
    assert!(eval.results.contains_key("prefer_local"));
}

#[tokio::test]
async fn test_memory_constraint() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request =
        CompositionRequest::new("test_memory").with_constraint(Constraint::min_memory_gb(0.1));

    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.is_feasible);
}

#[tokio::test]
async fn test_multiple_constraints() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let request = CompositionRequest::new("test_multi")
        .with_constraint(Constraint::min_memory_gb(0.1))
        .with_constraint(Constraint::min_cpu_cores(1))
        .with_constraint(Constraint::prefer_local());

    let eval = engine.evaluate(&request).await.unwrap();

    assert_eq!(eval.results.len(), 3);

    assert!(eval.results.get("min_memory_gb").unwrap().is_satisfied());
    assert!(eval.results.get("min_cpu_cores").unwrap().is_satisfied());
}

#[tokio::test]
async fn test_engine_stats() {
    let engine = CompositionEngine::from_runtime().await.unwrap();

    let initial_stats = engine.stats().await;
    assert_eq!(initial_stats.total_evaluations, 0);

    let request = CompositionRequest::new("test");
    engine.evaluate(&request).await.unwrap();

    let updated_stats = engine.stats().await;
    assert_eq!(updated_stats.total_evaluations, 1);
}

#[tokio::test]
#[expect(clippy::float_cmp, reason = "expected literal from empty constraints")]
async fn test_empty_constraints_returns_score_one() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("empty");
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.is_empty());
    assert_eq!(eval.overall_score, 1.0);
    assert!(eval.is_feasible);
}

#[tokio::test]
async fn test_cpu_constraint_insufficient_cores() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request =
        CompositionRequest::new("huge_cpu").with_constraint(Constraint::min_cpu_cores(999_999));
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("min_cpu_cores"));
    let sat = eval.results.get("min_cpu_cores").unwrap();
    assert!(!sat.is_satisfied());
    assert!(!eval.is_feasible);
}

#[tokio::test]
async fn test_memory_constraint_insufficient() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request =
        CompositionRequest::new("huge_mem").with_constraint(Constraint::min_memory_gb(999_999.0));
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("min_memory_gb"));
    let sat = eval.results.get("min_memory_gb").unwrap();
    assert!(!sat.is_satisfied());
    assert!(!eval.is_feasible);
}

#[tokio::test]
async fn test_max_latency_constraint() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request =
        CompositionRequest::new("low_latency").with_constraint(Constraint::max_latency_ms(0));
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("max_latency_ms"));
}

#[tokio::test]
async fn test_preferred_latency_partial_satisfaction() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("pref_latency")
        .with_constraint(Constraint::preferred_latency_ms(1));
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("preferred_latency_ms"));
    assert!(eval.is_feasible);
}

#[tokio::test]
async fn test_bandwidth_constraints() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("bandwidth")
        .with_constraint(Constraint::min_bandwidth_gbps(1000.0))
        .with_constraint(Constraint::PreferredBandwidthGbps(500.0));
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("min_bandwidth_gbps"));
    assert!(eval.results.contains_key("preferred_bandwidth_gbps"));
}

#[tokio::test]
async fn test_requires_capability() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("cap").with_constraint(Constraint::requires_capability(
        "nonexistent-capability-xyz",
    ));
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("requires_capability"));
    let sat = eval.results.get("requires_capability").unwrap();
    assert!(!sat.is_satisfied());
    assert!(!eval.is_feasible);
}

#[tokio::test]
async fn test_prefers_capability_soft() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("pref_cap")
        .with_constraint(Constraint::prefers_capability("optional-cap"));
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("prefers_capability"));
    assert!(eval.is_feasible);
}

#[tokio::test]
async fn test_layer_constraints() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("layer")
        .with_constraint(Constraint::RequiresLayer("NonExistentLayer".to_string()))
        .with_constraint(Constraint::PrefersLayer("SomeLayer".to_string()));
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("requires_layer"));
    assert!(eval.results.contains_key("prefers_layer"));
}

#[tokio::test]
async fn test_storage_and_cost_constraints() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("storage_cost")
        .with_constraint(Constraint::RequiresPersistentStorage)
        .with_constraint(Constraint::MaxCostPerHour(0.001))
        .with_constraint(Constraint::MinimizeCost);
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("requires_persistent_storage"));
    assert!(eval.results.contains_key("max_cost_per_hour"));
    assert!(eval.results.contains_key("minimize_cost"));
}

#[tokio::test]
async fn test_custom_constraint() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("custom").with_constraint(Constraint::Custom {
        name: "custom_test".to_string(),
        hard: false,
        value: "test_value".to_string(),
    });
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("custom_test"));
    assert!(eval.results.get("custom_test").unwrap().is_satisfied());
}

#[tokio::test]
#[expect(
    clippy::float_cmp,
    reason = "expected literal when hard constraint fails"
)]
async fn test_hard_constraint_failure_zeroes_score() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("hard_fail")
        .with_constraint(Constraint::min_cpu_cores(999_999))
        .with_constraint(Constraint::prefer_local());
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(!eval.is_feasible);
    assert_eq!(eval.overall_score, 0.0);
}

#[tokio::test]
async fn test_capabilities_accessor() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let caps = engine.capabilities();
    assert!(caps.compute.cpu_cores.is_some() || caps.compute.memory_bytes.is_some());
}

#[tokio::test]
async fn test_stats_track_feasible_infeasible() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    engine
        .evaluate(&CompositionRequest::new("ok"))
        .await
        .unwrap();
    engine
        .evaluate(
            &CompositionRequest::new("fail").with_constraint(Constraint::min_cpu_cores(999_999)),
        )
        .await
        .unwrap();
    let stats = engine.stats().await;
    assert_eq!(stats.total_evaluations, 2);
    assert!(stats.feasible_count + stats.infeasible_count == 2);
}

#[tokio::test]
async fn test_must_be_local_and_prefer_local() {
    let engine = CompositionEngine::from_runtime().await.unwrap();
    let request = CompositionRequest::new("local")
        .with_constraint(Constraint::must_be_local())
        .with_constraint(Constraint::prefer_local());
    let eval = engine.evaluate(&request).await.unwrap();
    assert!(eval.results.contains_key("must_be_local"));
    assert!(eval.results.contains_key("prefer_local"));
}
