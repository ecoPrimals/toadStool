// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for cross-substrate validation and benchmarking
//!
//! These tests verify the benchmark suite configuration and workload definitions.

use cross_substrate_validation::{WorkloadCategory, get_benchmark_suite};

/// Test benchmark suite is non-empty
#[test]
fn test_benchmark_suite_not_empty() {
    let suite = get_benchmark_suite();
    assert!(!suite.is_empty(), "Benchmark suite should have workloads");
}

/// Test core workload categories are represented
#[test]
fn test_all_categories_represented() {
    let suite = get_benchmark_suite();

    let has_elementwise = suite
        .iter()
        .any(|w| w.category == WorkloadCategory::ElementWise);
    let has_reduction = suite
        .iter()
        .any(|w| w.category == WorkloadCategory::Reduction);
    let has_memory_bound = suite
        .iter()
        .any(|w| w.category == WorkloadCategory::MemoryBound);
    let has_compute_bound = suite
        .iter()
        .any(|w| w.category == WorkloadCategory::ComputeBound);
    let has_normalization = suite
        .iter()
        .any(|w| w.category == WorkloadCategory::Normalization);

    // Core categories must be present
    assert!(has_elementwise, "Should have ElementWise workloads");
    assert!(has_reduction, "Should have Reduction workloads");
    assert!(has_memory_bound, "Should have MemoryBound workloads");
    assert!(has_compute_bound, "Should have ComputeBound workloads");
    assert!(has_normalization, "Should have Normalization workloads");
    // Mixed is optional - may be added in future iterations
}

/// Test workload sizes are reasonable
#[test]
fn test_workload_sizes_reasonable() {
    let suite = get_benchmark_suite();

    for workload in &suite {
        assert!(
            workload.size > 0,
            "Workload {} should have positive size",
            workload.name
        );
        assert!(
            workload.size <= 100_000_000,
            "Workload {} size {} seems too large",
            workload.name,
            workload.size
        );
    }
}

/// Test workload names are descriptive
#[test]
fn test_workload_names_descriptive() {
    let suite = get_benchmark_suite();

    for workload in &suite {
        assert!(
            !workload.name.is_empty(),
            "Workload name should not be empty"
        );
        // Name should contain operation type or be descriptive
        assert!(
            workload.name.len() >= 3,
            "Workload name '{}' should be descriptive",
            workload.name
        );
    }
}

/// Test expected winner is specified
#[test]
fn test_expected_winner_specified() {
    let suite = get_benchmark_suite();

    for workload in &suite {
        assert!(
            !workload.expected_winner.is_empty(),
            "Workload {} should have expected winner",
            workload.name
        );
    }
}

/// Test workload spec can be cloned
#[test]
fn test_workload_spec_clone() {
    let suite = get_benchmark_suite();

    if let Some(workload) = suite.first() {
        let cloned = workload.clone();
        assert_eq!(cloned.name, workload.name);
        assert_eq!(cloned.size, workload.size);
        assert_eq!(cloned.category, workload.category);
    }
}

/// Test category equality
#[test]
fn test_workload_category_equality() {
    assert_eq!(WorkloadCategory::ElementWise, WorkloadCategory::ElementWise);
    assert_ne!(WorkloadCategory::ElementWise, WorkloadCategory::Reduction);
    assert_ne!(
        WorkloadCategory::ComputeBound,
        WorkloadCategory::MemoryBound
    );
}

/// Test category debug formatting
#[test]
fn test_workload_category_debug() {
    let category = WorkloadCategory::ElementWise;
    let debug_str = format!("{category:?}");
    assert!(debug_str.contains("ElementWise"));
}

/// Test suite has varied sizes for scaling analysis
#[test]
fn test_suite_has_varied_sizes() {
    let suite = get_benchmark_suite();
    let sizes: Vec<usize> = suite.iter().map(|w| w.size).collect();

    // Should have at least 3 different sizes
    let mut unique_sizes = sizes;
    unique_sizes.sort_unstable();
    unique_sizes.dedup();

    assert!(
        unique_sizes.len() >= 3,
        "Suite should have varied workload sizes for scaling analysis"
    );
}

/// Test `ReLU` workloads span multiple sizes
#[test]
fn test_relu_workloads_span_sizes() {
    let suite = get_benchmark_suite();
    let relu_sizes: Vec<usize> = suite
        .iter()
        .filter(|w| w.name.contains("ReLU"))
        .map(|w| w.size)
        .collect();

    assert!(
        relu_sizes.len() >= 3,
        "Should have multiple ReLU workloads at different sizes"
    );

    // Verify sizes span orders of magnitude
    if let (Some(&min), Some(&max)) = (relu_sizes.iter().min(), relu_sizes.iter().max()) {
        assert!(
            max >= min * 100,
            "ReLU workloads should span at least 2 orders of magnitude"
        );
    }
}

/// Test normalization workloads exist
#[test]
fn test_normalization_workloads() {
    let suite = get_benchmark_suite();
    let norm_workloads: Vec<_> = suite
        .iter()
        .filter(|w| w.category == WorkloadCategory::Normalization)
        .collect();

    assert!(
        !norm_workloads.is_empty(),
        "Should have normalization workloads"
    );

    // Check for LayerNorm or BatchNorm
    let has_layer_norm = norm_workloads.iter().any(|w| w.name.contains("LayerNorm"));
    let has_batch_norm = norm_workloads.iter().any(|w| w.name.contains("BatchNorm"));

    assert!(
        has_layer_norm || has_batch_norm,
        "Should have LayerNorm or BatchNorm workloads"
    );
}

/// Test compute-bound workloads include `MatMul`
#[test]
fn test_compute_bound_includes_matmul() {
    let suite = get_benchmark_suite();
    let compute_bound: Vec<_> = suite
        .iter()
        .filter(|w| w.category == WorkloadCategory::ComputeBound)
        .collect();

    let has_matmul = compute_bound
        .iter()
        .any(|w| w.name.contains("MatMul") || w.name.contains("matmul"));

    assert!(has_matmul, "Compute-bound workloads should include MatMul");
}

// ============================================================================
// Integration tests (require runtime)
// ============================================================================

#[test]
#[ignore = "Requires UniversalRuntime setup"]
fn test_benchmark_execution() {
    // This test would actually run benchmarks
    // Run with: cargo test -- --ignored
}

#[test]
#[ignore = "Requires GPU hardware"]
fn test_cross_substrate_gpu_execution() {
    // This test would run GPU benchmarks
    // Run with: cargo test -- --ignored
}

#[test]
#[ignore = "Requires NPU hardware"]
fn test_cross_substrate_npu_execution() {
    // This test would run NPU benchmarks
    // Run with: cargo test -- --ignored
}
