// SPDX-License-Identifier: AGPL-3.0-only
// ============================================================================
// RuntimeSelectionStrategy Tests
// ============================================================================

#[test]
fn test_runtime_selection_strategy_variants() {
    let first = RuntimeSelectionStrategy::FirstAvailable;
    let balanced = RuntimeSelectionStrategy::LoadBalanced;
    let optimal = RuntimeSelectionStrategy::OptimalMatch;

    assert!(matches!(first, RuntimeSelectionStrategy::FirstAvailable));
    assert!(matches!(balanced, RuntimeSelectionStrategy::LoadBalanced));
    assert!(matches!(optimal, RuntimeSelectionStrategy::OptimalMatch));
}

#[test]
fn test_runtime_selection_strategy_clone() {
    let strategy1 = RuntimeSelectionStrategy::FirstAvailable;
    let strategy2 = strategy1.clone();

    assert!(matches!(
        strategy2,
        RuntimeSelectionStrategy::FirstAvailable
    ));
}

#[test]
fn test_runtime_selection_strategy_debug() {
    let strategy = RuntimeSelectionStrategy::OptimalMatch;
    let debug_str = format!("{:?}", strategy);
    assert!(debug_str.contains("OptimalMatch"));
}

#[test]
fn test_runtime_selection_strategy_all_variants() {
    let strategies = vec![
        RuntimeSelectionStrategy::FirstAvailable,
        RuntimeSelectionStrategy::LoadBalanced,
        RuntimeSelectionStrategy::OptimalMatch,
    ];

    assert_eq!(strategies.len(), 3);
}

// ============================================================================
// RuntimeOrchestrator Creation Tests
// ============================================================================

#[test]
fn test_runtime_orchestrator_new_first_available() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    // Orchestrator should be created successfully
    assert!(std::mem::size_of_val(&orchestrator) > 0);
}

#[test]
fn test_runtime_orchestrator_new_load_balanced() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::LoadBalanced);
    assert!(std::mem::size_of_val(&orchestrator) > 0);
}

#[test]
fn test_runtime_orchestrator_new_optimal_match() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);
    assert!(std::mem::size_of_val(&orchestrator) > 0);
}

#[test]
fn test_runtime_orchestrator_creation_multiple() {
    let orch1 = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let orch2 = RuntimeOrchestrator::new(RuntimeSelectionStrategy::LoadBalanced);
    let orch3 = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);

    // All should be created independently
    assert!(std::mem::size_of_val(&orch1) > 0);
    assert!(std::mem::size_of_val(&orch2) > 0);
    assert!(std::mem::size_of_val(&orch3) > 0);
}

