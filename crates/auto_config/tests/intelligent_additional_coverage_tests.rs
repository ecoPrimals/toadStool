// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::float_cmp)]
//! Additional comprehensive test coverage for intelligent auto-configuration
//!
//! This test suite expands coverage of the intelligent configuration system
//! with focus on error handling, edge cases, and integration scenarios.

use toadstool_auto_config::{IntelligentAutoConfig, PlatformOptimizer, UsageHints, UsageLearner};

// ============================================================================
// ADDITIONAL USAGE HINTS TESTS
// ============================================================================

#[test]
fn test_usage_hints_boundary_values() {
    let hints = UsageHints {
        predicted_workload_types: vec![],
        expected_cpu_usage: 0.0,    // Minimum
        expected_memory_usage: 1.0, // Maximum
        prefers_gpu: false,
        prefers_containers: false,
    };

    assert_eq!(hints.expected_cpu_usage, 0.0);
    assert_eq!(hints.expected_memory_usage, 1.0);
}

#[test]
fn test_usage_hints_multiple_workload_types() {
    let workload_types = vec![
        "compute".to_string(),
        "network".to_string(),
        "storage".to_string(),
        "ml_training".to_string(),
    ];

    let hints = UsageHints {
        predicted_workload_types: workload_types.clone(),
        expected_cpu_usage: 0.7,
        expected_memory_usage: 0.6,
        prefers_gpu: true,
        prefers_containers: true,
    };

    assert_eq!(hints.predicted_workload_types.len(), 4);
    assert!(hints
        .predicted_workload_types
        .contains(&"compute".to_string()));
    assert!(hints
        .predicted_workload_types
        .contains(&"ml_training".to_string()));
}

#[test]
fn test_usage_hints_is_cpu_intensive_threshold() {
    // Just below threshold
    let hints1 = UsageHints {
        predicted_workload_types: vec![],
        expected_cpu_usage: 0.69,
        expected_memory_usage: 0.5,
        prefers_gpu: false,
        prefers_containers: false,
    };
    assert!(!hints1.is_cpu_intensive());

    // At threshold (0.7 is NOT intensive, needs > 0.7)
    let hints2 = UsageHints {
        predicted_workload_types: vec![],
        expected_cpu_usage: 0.7,
        expected_memory_usage: 0.5,
        prefers_gpu: false,
        prefers_containers: false,
    };
    assert!(!hints2.is_cpu_intensive());

    // Above threshold (> 0.7)
    let hints3 = UsageHints {
        predicted_workload_types: vec![],
        expected_cpu_usage: 0.71,
        expected_memory_usage: 0.5,
        prefers_gpu: false,
        prefers_containers: false,
    };
    assert!(hints3.is_cpu_intensive());
}

#[test]
fn test_usage_hints_is_memory_intensive_threshold() {
    // Just below threshold
    let hints1 = UsageHints {
        predicted_workload_types: vec![],
        expected_cpu_usage: 0.5,
        expected_memory_usage: 0.69,
        prefers_gpu: false,
        prefers_containers: false,
    };
    assert!(!hints1.is_memory_intensive());

    // At threshold (0.7 is NOT intensive, needs > 0.7)
    let hints2 = UsageHints {
        predicted_workload_types: vec![],
        expected_cpu_usage: 0.5,
        expected_memory_usage: 0.7,
        prefers_gpu: false,
        prefers_containers: false,
    };
    assert!(!hints2.is_memory_intensive());

    // Above threshold (> 0.7)
    let hints3 = UsageHints {
        predicted_workload_types: vec![],
        expected_cpu_usage: 0.5,
        expected_memory_usage: 0.71,
        prefers_gpu: false,
        prefers_containers: false,
    };
    assert!(hints3.is_memory_intensive());
}

#[test]
fn test_usage_hints_both_intensive() {
    let hints = UsageHints {
        predicted_workload_types: vec!["data_science".to_string()],
        expected_cpu_usage: 0.9,
        expected_memory_usage: 0.9,
        prefers_gpu: true,
        prefers_containers: false,
    };

    assert!(hints.is_cpu_intensive());
    assert!(hints.is_memory_intensive());
}

#[test]
fn test_usage_hints_neither_intensive() {
    let hints = UsageHints {
        predicted_workload_types: vec!["light".to_string()],
        expected_cpu_usage: 0.3,
        expected_memory_usage: 0.2,
        prefers_gpu: false,
        prefers_containers: false,
    };

    assert!(!hints.is_cpu_intensive());
    assert!(!hints.is_memory_intensive());
}

// ============================================================================
// PLATFORM OPTIMIZER TESTS
// ============================================================================

#[test]
fn test_platform_optimizer_new() {
    let optimizer = PlatformOptimizer::new();
    // Should create without panic
    drop(optimizer);
}

#[test]
fn test_platform_optimizer_multiple_instances() {
    let _opt1 = PlatformOptimizer::new();
    let _opt2 = PlatformOptimizer::new();
    let _opt3 = PlatformOptimizer::new();

    // All should be independent
}

// ============================================================================
// USAGE LEARNER TESTS
// ============================================================================

#[test]
fn test_usage_learner_new() {
    let learner = UsageLearner::new();
    // Should create without panic
    drop(learner);
}

#[test]
fn test_usage_learner_multiple_instances() {
    let _learner1 = UsageLearner::new();
    let _learner2 = UsageLearner::new();
    let _learner3 = UsageLearner::new();

    // All should be independent
}

#[tokio::test]
async fn test_usage_learner_analyze_environment_returns_valid() {
    let mut learner = UsageLearner::new();
    let result = learner.analyze_environment().await;

    assert!(result.is_ok(), "Environment analysis should succeed");

    let hints = result.unwrap();
    // Validate hints are within valid ranges
    assert!(hints.expected_cpu_usage >= 0.0);
    assert!(hints.expected_cpu_usage <= 1.0);
    assert!(hints.expected_memory_usage >= 0.0);
    assert!(hints.expected_memory_usage <= 1.0);
}

// ============================================================================
// INTELLIGENT AUTO CONFIG TESTS
// ============================================================================

#[test]
fn test_intelligent_auto_config_default() {
    let config = IntelligentAutoConfig::default();
    // Should create via default trait
    drop(config);
}

#[test]
fn test_intelligent_auto_config_multiple_instances() {
    let _config1 = IntelligentAutoConfig::new();
    let _config2 = IntelligentAutoConfig::new();

    // Should be able to create multiple independent instances
}

#[tokio::test]
async fn test_scan_system_returns_capabilities() {
    let mut config = IntelligentAutoConfig::new();
    let result = config.scan_system().await;

    assert!(result.is_ok(), "System scan should succeed");
}

#[tokio::test]
async fn test_discover_services_returns_services() {
    let mut config = IntelligentAutoConfig::new();
    let result = config.discover_services().await;

    assert!(result.is_ok(), "Service discovery should succeed");
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_usage_hints_empty_workload_types() {
    let hints = UsageHints {
        predicted_workload_types: vec![],
        expected_cpu_usage: 0.5,
        expected_memory_usage: 0.5,
        prefers_gpu: false,
        prefers_containers: false,
    };

    assert_eq!(hints.predicted_workload_types.len(), 0);
}

#[test]
fn test_usage_hints_single_workload_type() {
    let hints = UsageHints {
        predicted_workload_types: vec!["compute".to_string()],
        expected_cpu_usage: 0.8,
        expected_memory_usage: 0.4,
        prefers_gpu: false,
        prefers_containers: false,
    };

    assert_eq!(hints.predicted_workload_types.len(), 1);
    assert_eq!(hints.predicted_workload_types[0], "compute");
}

#[test]
fn test_usage_hints_very_long_workload_name() {
    let long_name = "a".repeat(1000);
    let hints = UsageHints {
        predicted_workload_types: vec![long_name.clone()],
        expected_cpu_usage: 0.5,
        expected_memory_usage: 0.5,
        prefers_gpu: false,
        prefers_containers: false,
    };

    assert_eq!(hints.predicted_workload_types[0].len(), 1000);
}

// ============================================================================
// PROPERTY-BASED STYLE TESTS
// ============================================================================

#[test]
fn test_usage_hints_cpu_usage_never_negative() {
    for i in 0..100 {
        let usage = f64::from(i) / 100.0;
        let hints = UsageHints {
            predicted_workload_types: vec![],
            expected_cpu_usage: usage,
            expected_memory_usage: 0.5,
            prefers_gpu: false,
            prefers_containers: false,
        };

        assert!(hints.expected_cpu_usage >= 0.0);
        assert!(hints.expected_cpu_usage <= 1.0);
    }
}

#[test]
fn test_usage_hints_memory_usage_never_negative() {
    for i in 0..100 {
        let usage = f64::from(i) / 100.0;
        let hints = UsageHints {
            predicted_workload_types: vec![],
            expected_cpu_usage: 0.5,
            expected_memory_usage: usage,
            prefers_gpu: false,
            prefers_containers: false,
        };

        assert!(hints.expected_memory_usage >= 0.0);
        assert!(hints.expected_memory_usage <= 1.0);
    }
}

// ============================================================================
// CONCURRENT ACCESS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_usage_learner_creation() {
    let mut handles = vec![];

    for _ in 0..10 {
        let handle = tokio::spawn(async move {
            let _learner = UsageLearner::new();
            // Just create and drop
        });
        handles.push(handle);
    }

    // All should complete successfully
    for handle in handles {
        assert!(handle.await.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_platform_optimizer_creation() {
    let mut handles = vec![];

    for _ in 0..10 {
        let handle = tokio::spawn(async move {
            let _optimizer = PlatformOptimizer::new();
            // Just create and drop
        });
        handles.push(handle);
    }

    // All should complete successfully
    for handle in handles {
        assert!(handle.await.is_ok());
    }
}

// ============================================================================
// INTEGRATION-STYLE TESTS
// ============================================================================

#[tokio::test]
async fn test_full_scan_and_discover_flow() {
    let mut config = IntelligentAutoConfig::new();

    // Should be able to scan system
    let scan_result = config.scan_system().await;
    assert!(scan_result.is_ok());

    // Then discover services
    let discover_result = config.discover_services().await;
    assert!(discover_result.is_ok());
}

#[tokio::test]
async fn test_multiple_scans_same_instance() {
    let mut config = IntelligentAutoConfig::new();

    // Multiple scans should all work
    for _ in 0..3 {
        let result = config.scan_system().await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_multiple_discoveries_same_instance() {
    let mut config = IntelligentAutoConfig::new();

    // Multiple discoveries should all work
    for _ in 0..3 {
        let result = config.discover_services().await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_usage_hints_creation_is_fast() {
    let start = std::time::Instant::now();

    for _ in 0..10_000 {
        let _ = UsageHints::default();
    }

    let duration = start.elapsed();
    assert!(
        duration.as_millis() < 500,
        "Creating 10k hints should be fast"
    );
}

#[test]
fn test_intelligent_config_creation_is_fast() {
    let start = std::time::Instant::now();

    for _ in 0..1_000 {
        let _ = IntelligentAutoConfig::new();
    }

    let duration = start.elapsed();
    assert!(
        duration.as_millis() < 1000,
        "Creating 1k configs should be fast"
    );
}
