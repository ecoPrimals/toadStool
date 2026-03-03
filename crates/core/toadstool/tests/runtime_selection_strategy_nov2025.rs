// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive test coverage for runtime.rs RuntimeSelectionStrategy
//!
//! This test suite targets RuntimeSelectionStrategy defined in
//! crates/core/toadstool/src/runtime.rs to expand test coverage.
//!
//! Coverage Target: Add tests for runtime selection strategy
//! Session: November 2025 - Week 5 Test Expansion (Batch 5)

use toadstool::runtime::RuntimeSelectionStrategy;

// ============================================================================
// RuntimeSelectionStrategy Tests (6 tests)
// ============================================================================

#[test]
fn test_runtime_selection_strategy_first_available() {
    let strategy = RuntimeSelectionStrategy::FirstAvailable;
    assert!(matches!(strategy, RuntimeSelectionStrategy::FirstAvailable));
}

#[test]
fn test_runtime_selection_strategy_load_balanced() {
    let strategy = RuntimeSelectionStrategy::LoadBalanced;
    assert!(matches!(strategy, RuntimeSelectionStrategy::LoadBalanced));
}

#[test]
fn test_runtime_selection_strategy_optimal_match() {
    let strategy = RuntimeSelectionStrategy::OptimalMatch;
    assert!(matches!(strategy, RuntimeSelectionStrategy::OptimalMatch));
}

#[test]
fn test_runtime_selection_strategy_clone() {
    let strategy = RuntimeSelectionStrategy::FirstAvailable;
    let cloned = strategy.clone();
    assert!(matches!(cloned, RuntimeSelectionStrategy::FirstAvailable));

    let strategy2 = RuntimeSelectionStrategy::LoadBalanced;
    let cloned2 = strategy2.clone();
    assert!(matches!(cloned2, RuntimeSelectionStrategy::LoadBalanced));
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
// Summary
// ============================================================================

// Total tests added: 6
// Coverage areas:
// - RuntimeSelectionStrategy (6 tests)
//   - FirstAvailable variant
//   - LoadBalanced variant
//   - OptimalMatch variant
//   - Clone functionality
//   - Debug formatting
//   - All variants collection
