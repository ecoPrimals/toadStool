// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from comprehensive_benchmark.rs (S335).

use super::comprehensive_benchmark::*;

fn format_time(time: Option<f64>) -> String {
    match time {
        Some(t) => format!("{t:8.1}"),
        None => "    -   ".to_string(),
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{s:max_len$}")
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[test]
fn test_get_benchmark_suite() {
    let suite = get_benchmark_suite();
    assert!(!suite.is_empty());
    assert!(suite.len() >= 15);
    assert!(suite.iter().any(|s| s.name.contains("ReLU")));
    assert!(suite.iter().any(|s| s.name.contains("MatMul")));
}

#[test]
fn test_workload_spec_structure() {
    let spec = &get_benchmark_suite()[0];
    assert!(!spec.name.is_empty());
    assert!(spec.size > 0);
    assert!(!spec.expected_winner.is_empty());
}

#[test]
fn test_workload_category_variants() {
    let _ = WorkloadCategory::ElementWise;
    let _ = WorkloadCategory::Reduction;
    let _ = WorkloadCategory::MemoryBound;
    let _ = WorkloadCategory::ComputeBound;
    let _ = WorkloadCategory::Normalization;
    let _ = WorkloadCategory::Mixed;
}

#[test]
fn test_benchmark_result_structure() {
    let result = BenchmarkResult {
        workload: "Test".to_string(),
        category: WorkloadCategory::ElementWise,
        cpu_time_us: Some(100.0),
        gpu_amd_time_us: None,
        gpu_nvidia_time_us: None,
        npu_time_us: None,
        winner: "CPU".to_string(),
        speedup: 1.5,
    };
    assert_eq!(result.winner, "CPU");
    assert!((result.speedup - 1.5).abs() < f64::EPSILON);
}

#[test]
fn test_print_results_summary_no_panic() {
    let results = vec![BenchmarkResult {
        workload: "Test Workload".to_string(),
        category: WorkloadCategory::ElementWise,
        cpu_time_us: Some(50.0),
        gpu_amd_time_us: Some(20.0),
        gpu_nvidia_time_us: None,
        npu_time_us: None,
        winner: "GPU AMD".to_string(),
        speedup: 2.5,
    }];
    print_results_summary(&results);
}

#[test]
fn test_benchmark_suite_categories() {
    let suite = get_benchmark_suite();
    let categories: Vec<_> = suite.iter().map(|s| s.category).collect();
    assert!(categories.contains(&WorkloadCategory::ElementWise));
    assert!(categories.contains(&WorkloadCategory::Reduction));
    assert!(categories.contains(&WorkloadCategory::MemoryBound));
    assert!(categories.contains(&WorkloadCategory::ComputeBound));
    assert!(categories.contains(&WorkloadCategory::Normalization));
}

#[test]
fn test_benchmark_suite_operations() {
    let suite = get_benchmark_suite();
    assert!(suite.iter().any(|s| s.name.contains("ReLU")));
    assert!(suite.iter().any(|s| s.name.contains("Tanh")));
    assert!(suite.iter().any(|s| s.name.contains("Sigmoid")));
    assert!(suite.iter().any(|s| s.name.contains("MatMul")));
    assert!(suite.iter().any(|s| s.name.contains("Reduce")));
    assert!(suite.iter().any(|s| s.name.contains("Transpose")));
}

#[test]
fn test_workload_spec_expected_winner() {
    let suite = get_benchmark_suite();
    for spec in &suite {
        assert!(!spec.expected_winner.is_empty());
        assert!(spec.size > 0);
    }
}

#[test]
fn test_benchmark_result_all_fields() {
    let result = BenchmarkResult {
        workload: "Full Test".to_string(),
        category: WorkloadCategory::ComputeBound,
        cpu_time_us: Some(100.0),
        gpu_amd_time_us: Some(50.0),
        gpu_nvidia_time_us: Some(40.0),
        npu_time_us: Some(10.0),
        winner: "NPU".to_string(),
        speedup: 10.0,
    };
    assert_eq!(result.winner, "NPU");
    assert!((result.speedup - 10.0).abs() < f64::EPSILON);
    assert_eq!(result.category, WorkloadCategory::ComputeBound);
}

#[test]
fn test_format_time_and_truncate() {
    assert_eq!(format_time(Some(123.45)), "   123.5");
    assert_eq!(format_time(None), "    -   ");
    assert_eq!(truncate("short", 10), "short     ");
    assert!(truncate("verylongstring", 5).ends_with("..."));
}
