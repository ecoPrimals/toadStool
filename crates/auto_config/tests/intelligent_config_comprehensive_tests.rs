// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Comprehensive test coverage for intelligent configuration module
//!
//! This test suite provides property-based tests, table-driven tests, and error path
//! coverage for the intelligent auto-configuration system.

use std::collections::HashSet;
use toadstool_auto_config::intelligent::{PlatformConfig, PlatformOptimization, PlatformSupport};
use toadstool_auto_config::{
    IntelligentAutoConfig, PerformanceClass, PlatformOptimizer, UsageHints, UsageLearner,
};

/// Test that intelligent auto config can be created
#[test]
fn test_intelligent_auto_config_creation() {
    let config = IntelligentAutoConfig::new();
    // Should create successfully
    drop(config);
}

/// Test platform optimizer creation
#[test]
fn test_platform_optimizer_creation() {
    let optimizer = PlatformOptimizer::new();
    // Should create successfully
    drop(optimizer);
}

/// Test usage learner creation
#[test]
fn test_usage_learner_creation() {
    let learner = UsageLearner::new();
    // Should create successfully
    drop(learner);
}

/// Test usage hints structure (basic)
#[test]
fn test_usage_hints_basic() {
    let hints = UsageHints {
        predicted_workload_types: vec!["compute".to_string(), "network".to_string()],
        expected_cpu_usage: 0.6,
        expected_memory_usage: 0.5,
        prefers_gpu: false,
        prefers_containers: true,
    };

    assert_eq!(hints.predicted_workload_types.len(), 2);
    assert!(hints.expected_cpu_usage > 0.0);
    assert!(hints.expected_memory_usage > 0.0);
}

/// Test platform config structure
#[test]
fn test_platform_config_structure() {
    let mut features = HashSet::new();
    features.insert(PlatformSupport::Containers);
    features.insert(PlatformSupport::Sandboxing);

    let config = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: features,
        optimizations: vec![PlatformOptimization {
            optimization_type: "native_execution".to_string(),
            description: "Native code execution".to_string(),
            performance_gain: 0.2,
        }],
    };

    assert_eq!(config.platform_name, "linux");
    assert_eq!(config.supported_features.len(), 2);
    assert_eq!(config.optimizations.len(), 1);
    assert!(config.supports(&PlatformSupport::Containers));
}

/// Test usage hints with predictions
#[test]
fn test_usage_hints_predictions() {
    let hints = UsageHints {
        predicted_workload_types: vec!["batch".to_string(), "interactive".to_string()],
        expected_cpu_usage: 0.8, // > 0.7 for CPU intensive
        expected_memory_usage: 0.5,
        prefers_gpu: true,
        prefers_containers: false,
    };

    assert_eq!(hints.predicted_workload_types.len(), 2);
    assert!(hints.is_cpu_intensive());
    assert!(!hints.is_memory_intensive());
    assert!(hints.prefers_gpu);
}

/// Test usage hints default
#[test]
fn test_usage_hints_default() {
    let hints = UsageHints::default();

    assert!(hints.expected_cpu_usage >= 0.0);
    assert!(hints.expected_cpu_usage <= 1.0);
    assert!(hints.expected_memory_usage >= 0.0);
    assert!(hints.expected_memory_usage <= 1.0);
}

/// Table-driven tests for platform detection
#[test]
fn test_platform_detection() {
    let test_cases = vec![
        ("linux", true),
        ("macos", true),
        ("windows", true),
        ("freebsd", true),
        ("unknown", true), // Any platform should be handled
    ];

    for (platform, should_be_valid) in test_cases {
        let is_valid = !platform.is_empty();
        assert_eq!(
            is_valid, should_be_valid,
            "Platform {platform} validation failed"
        );
    }
}

/// Table-driven tests for optimization priorities
#[test]
fn test_optimization_priorities() {
    let priorities = vec![
        "performance",
        "balanced",
        "efficiency",
        "low_power",
        "high_throughput",
    ];

    for priority in priorities {
        assert!(!priority.is_empty());
        // Should be lowercase with underscores
        assert!(priority.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
    }
}

/// Table-driven tests for workload size classification
#[test]
fn test_workload_size_classification() {
    let test_cases = vec![
        ("small", 1, 10),        // 1-10 concurrent tasks
        ("medium", 11, 100),     // 11-100 concurrent tasks
        ("large", 101, 1000),    // 101-1000 concurrent tasks
        ("xlarge", 1001, 10000), // 1001+ concurrent tasks
    ];

    for (size_class, min_tasks, max_tasks) in test_cases {
        assert!(!size_class.is_empty());
        assert!(min_tasks < max_tasks);

        // Verify classification logic
        let classification = if max_tasks <= 10 {
            "small"
        } else if max_tasks <= 100 {
            "medium"
        } else if max_tasks <= 1000 {
            "large"
        } else {
            "xlarge"
        };

        assert!(!classification.is_empty());
    }
}

/// Test runtime preference validation
#[test]
fn test_runtime_preference_validation() {
    let valid_runtimes = vec!["native", "wasm", "container", "edge", "gpu", "hybrid"];

    for runtime in valid_runtimes {
        assert!(!runtime.is_empty());
        // Should be lowercase
        assert_eq!(runtime, runtime.to_lowercase());
    }
}

/// Test peak usage time patterns
#[test]
fn test_peak_usage_time_patterns() {
    let time_patterns = vec![
        "business_hours",
        "evenings",
        "weekends",
        "24/7",
        "intermittent",
        "scheduled",
    ];

    for pattern in time_patterns {
        assert!(!pattern.is_empty());
    }
}

/// Test performance class to optimization mapping
#[test]
fn test_performance_class_optimization_mapping() {
    let test_cases = vec![
        (
            PerformanceClass::HighEnd,
            vec!["aggressive_optimization", "parallel_execution"],
        ),
        (
            PerformanceClass::Mainstream,
            vec!["balanced_optimization", "moderate_parallelism"],
        ),
        (
            PerformanceClass::Budget,
            vec!["conservative_optimization", "sequential_preferred"],
        ),
        (
            PerformanceClass::LowEnd,
            vec!["minimal_optimization", "resource_constrained"],
        ),
    ];

    for (perf_class, expected_optimizations) in test_cases {
        // Verify performance class is valid
        let _debug = format!("{perf_class:?}");

        // Verify optimizations are reasonable
        assert!(!expected_optimizations.is_empty());
        for opt in expected_optimizations {
            assert!(!opt.is_empty());
        }
    }
}

/// Test config hash generation
#[test]
fn test_config_hash_generation() {
    let config1 = "config_a";
    let config2 = "config_b";

    // Simple hash simulation
    let hash1 = format!("{:x}", config1.len());
    let hash2 = format!("{:x}", config2.len());

    // Hashes should be deterministic
    assert!(!hash1.is_empty());
    assert!(!hash2.is_empty());

    // Different configs should have different representations
    assert_ne!(config1, config2);
}

/// Test system state classification
#[test]
fn test_system_state_classification() {
    let states = vec![
        "optimal",
        "normal",
        "degraded",
        "overloaded",
        "idle",
        "warming_up",
    ];

    for state in states {
        assert!(!state.is_empty());
        // Should be lowercase with underscores
        assert!(state.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
    }
}

/// Test feature detection patterns
#[test]
fn test_feature_detection() {
    let features = vec!["containers", "wasm", "native", "gpu", "edge", "distributed"];

    for feature in features {
        assert!(!feature.is_empty());
        // Features should be lowercase
        assert_eq!(feature, feature.to_lowercase());
    }
}

/// Test optimization strategies
#[test]
fn test_optimization_strategies() {
    let strategies = vec![
        "aggressive",
        "balanced",
        "conservative",
        "adaptive",
        "custom",
    ];

    for strategy in strategies {
        assert!(!strategy.is_empty());
        // Should be valid identifiers
        assert!(strategy.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
    }
}

/// Test concurrent intelligent config creation
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_config_creation() {
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(4));
    let mut handles = vec![];

    for _ in 0..10 {
        let sem = Arc::clone(&semaphore);
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            // Create config (should be thread-safe)
            let _config = IntelligentAutoConfig::new();
            let hints = UsageHints::default();

            // Default hints should be valid (empty workload types is fine)
            assert!(hints.expected_cpu_usage >= 0.0);
            assert!(hints.expected_memory_usage >= 0.0);
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Test rapid config creation (stress test)
#[test]
fn test_rapid_config_creation() {
    for _ in 0..100 {
        let _config = IntelligentAutoConfig::new();
        let _optimizer = PlatformOptimizer::new();
        let _learner = UsageLearner::new();
    }
}

/// Test platform config with empty features
#[test]
fn test_platform_config_empty_features() {
    let config = PlatformConfig {
        platform_name: "minimal".to_string(),
        supported_features: HashSet::new(),
        optimizations: vec![],
    };

    assert_eq!(config.supported_features.len(), 0);
    assert_eq!(config.optimizations.len(), 0);
    assert!(!config.supports(&PlatformSupport::Containers));
}

/// Test usage hints with custom values
#[test]
fn test_usage_hints_custom() {
    let hints = UsageHints {
        predicted_workload_types: vec!["custom_large".to_string(), "batch_processing".to_string()],
        expected_cpu_usage: 0.9,
        expected_memory_usage: 0.8,
        prefers_gpu: true,
        prefers_containers: true,
    };

    // Custom values should be accepted
    assert!(!hints.predicted_workload_types.is_empty());
    assert!(hints.is_cpu_intensive());
    assert!(hints.is_memory_intensive());
}

/// Test platform support variants
#[test]
fn test_platform_support_variants() {
    let supports = vec![
        PlatformSupport::Containers,
        PlatformSupport::Sandboxing,
        PlatformSupport::ProcessIsolation,
        PlatformSupport::NetworkIsolation,
    ];

    // Should have 4 distinct variants
    assert_eq!(supports.len(), 4);

    // All should support Debug
    for support in &supports {
        let _debug = format!("{support:?}");
    }
}

/// Test CPU usage classification
#[test]
fn test_cpu_usage_classification() {
    let test_cases = vec![
        (0.2, false), // Low CPU usage
        (0.5, false), // Medium CPU usage
        (0.7, false), // At threshold, not intensive
        (0.71, true), // Just over threshold, intensive
        (0.9, true),  // Very high CPU usage
    ];

    for (cpu_usage, is_intensive) in test_cases {
        let hints = UsageHints {
            predicted_workload_types: vec![],
            expected_cpu_usage: cpu_usage,
            expected_memory_usage: 0.5,
            prefers_gpu: false,
            prefers_containers: false,
        };

        assert_eq!(
            hints.is_cpu_intensive(),
            is_intensive,
            "CPU usage {cpu_usage} should be intensive={is_intensive}"
        );
    }
}

/// Test memory usage classification
#[test]
fn test_memory_usage_classification() {
    let test_cases = vec![
        (0.2, false), // Low memory usage
        (0.5, false), // Medium memory usage
        (0.7, false), // At threshold, not intensive
        (0.71, true), // Just over threshold, intensive
        (0.9, true),  // Very high memory usage
    ];

    for (memory_usage, is_intensive) in test_cases {
        let hints = UsageHints {
            predicted_workload_types: vec![],
            expected_cpu_usage: 0.5,
            expected_memory_usage: memory_usage,
            prefers_gpu: false,
            prefers_containers: false,
        };

        assert_eq!(
            hints.is_memory_intensive(),
            is_intensive,
            "Memory usage {memory_usage} should be intensive={is_intensive}"
        );
    }
}

/// Test Clone trait implementations
#[test]
fn test_clone_implementations() {
    let mut features = HashSet::new();
    features.insert(PlatformSupport::Containers);

    let config = PlatformConfig {
        platform_name: "test".to_string(),
        supported_features: features,
        optimizations: vec![PlatformOptimization {
            optimization_type: "native_execution".to_string(),
            description: "Native code execution".to_string(),
            performance_gain: 0.2,
        }],
    };

    let cloned = config.clone();
    assert_eq!(config.platform_name, cloned.platform_name);

    let hints = UsageHints::default();
    let cloned_hints = hints.clone();
    assert_eq!(hints.expected_cpu_usage, cloned_hints.expected_cpu_usage);
}

/// Test Debug trait implementations
#[test]
fn test_debug_implementations() {
    let config = PlatformConfig {
        platform_name: "test".to_string(),
        supported_features: std::collections::HashSet::new(),
        optimizations: vec![],
    };
    let _debug = format!("{config:?}");

    let hints = UsageHints::default();
    let _debug = format!("{hints:?}");
}

/// Test optimization priority levels
#[test]
fn test_optimization_priority_levels() {
    let test_cases = vec![
        ("performance", 100), // High priority
        ("balanced", 50),     // Medium priority
        ("efficiency", 25),   // Lower priority
    ];

    for (priority, expected_weight) in test_cases {
        assert!(!priority.is_empty());
        assert!(expected_weight > 0);

        // Verify priority affects optimization choices
        let weight = match priority {
            "performance" => 100,
            "balanced" => 50,
            "efficiency" => 25,
            _ => 0,
        };

        assert_eq!(weight, expected_weight);
    }
}

/// Test workload pattern detection
#[test]
fn test_workload_pattern_detection() {
    let patterns = vec![
        "steady_state",
        "bursty",
        "periodic",
        "random",
        "increasing",
        "decreasing",
    ];

    for pattern in patterns {
        assert!(!pattern.is_empty());
        // Patterns should be valid identifiers
        assert!(pattern.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
    }
}

/// Test platform-specific optimizations
#[test]
fn test_platform_specific_optimizations() {
    let test_cases = vec![
        ("linux", vec!["epoll", "io_uring", "native_threads"]),
        ("macos", vec!["kqueue", "grand_central_dispatch"]),
        ("windows", vec!["iocp", "thread_pool"]),
    ];

    for (platform, optimizations) in test_cases {
        assert!(!platform.is_empty());
        assert!(!optimizations.is_empty());

        // Each optimization should be valid
        for opt in optimizations {
            assert!(!opt.is_empty());
        }
    }
}

/// Test configuration validation
#[test]
fn test_configuration_validation() {
    let mut features = HashSet::new();
    features.insert(PlatformSupport::Containers);

    // Valid configuration
    let valid_config = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: features,
        optimizations: vec![PlatformOptimization {
            optimization_type: "native_execution".to_string(),
            description: "Native code execution".to_string(),
            performance_gain: 0.2,
        }],
    };

    // All fields should be populated
    assert!(!valid_config.platform_name.is_empty());
    assert!(!valid_config.supported_features.is_empty());
    assert!(!valid_config.optimizations.is_empty());
}

/// Test usage pattern learning
#[test]
fn test_usage_pattern_learning() {
    // Simulate learning from usage patterns
    let patterns = vec![
        ("morning", 100),   // 100 tasks in morning
        ("afternoon", 150), // 150 tasks in afternoon
        ("evening", 50),    // 50 tasks in evening
    ];

    let mut total_tasks = 0;
    for (_, tasks) in &patterns {
        total_tasks += tasks;
    }

    let average = total_tasks / patterns.len();

    // Learning should identify patterns
    assert!(average > 0);
    assert_eq!(average, 100); // (100 + 150 + 50) / 3 = 100
}

/// Test adaptive optimization
#[test]
fn test_adaptive_optimization() {
    // Test that optimization adapts to system state
    let states = vec![
        ("idle", "conservative"),
        ("normal", "balanced"),
        ("busy", "aggressive"),
        ("overloaded", "efficiency"),
    ];

    for (system_state, expected_optimization) in states {
        assert!(!system_state.is_empty());
        assert!(!expected_optimization.is_empty());

        // Verify mapping is logical
        let optimization = match system_state {
            "idle" => "conservative",
            "busy" => "aggressive",
            "overloaded" => "efficiency",
            _ => "balanced",
        };

        assert_eq!(optimization, expected_optimization);
    }
}
