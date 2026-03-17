// SPDX-License-Identifier: AGPL-3.0-only
// ToadStool - Universal Compute Platform
// Tests for property-based testing utilities

use std::collections::HashMap;
use std::time::Duration;
use toadstool_testing::properties::*;

// ============================================================================
// PropertyTestConfig Tests
// ============================================================================

#[test]
fn test_property_test_config_default() {
    let config = PropertyTestConfig::default();

    assert_eq!(config.test_name, "unnamed_property");
    assert_eq!(config.test_cases, 100);
    assert_eq!(config.shrink_attempts, 100);
    assert_eq!(config.timeout, Duration::from_secs(30)); // Default is 30 seconds
    assert!(!config.verbose);
    assert!(config.seed.is_none());
}

#[test]
fn test_property_test_config_custom() {
    let config = PropertyTestConfig {
        test_name: "custom_property".to_string(),
        test_cases: 50,
        shrink_attempts: 20,
        timeout: Duration::from_secs(30),
        verbose: true,
        seed: Some(12345),
    };

    assert_eq!(config.test_name, "custom_property");
    assert_eq!(config.test_cases, 50);
    assert_eq!(config.shrink_attempts, 20);
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert!(config.verbose);
    assert_eq!(config.seed, Some(12345));
}

#[test]
fn test_property_test_config_builder_pattern() {
    let config = PropertyTestConfig {
        test_name: "builder_test".to_string(),
        test_cases: 200,
        shrink_attempts: 50,
        timeout: Duration::from_secs(120),
        verbose: false,
        seed: None,
    };

    assert_eq!(config.test_name, "builder_test");
    assert_eq!(config.test_cases, 200);
}

// ============================================================================
// PropertyTestResult Tests
// ============================================================================

#[test]
fn test_property_test_result_success() {
    let result = PropertyTestResult {
        test_name: "test_success".to_string(),
        success: true,
        test_cases_run: 100,
        failures: Vec::new(),
        duration: Duration::from_secs(5),
        statistics: TestStatistics {
            input_distribution: HashMap::new(),
            execution_times: Vec::new(),
            coverage_metrics: HashMap::new(),
        },
    };

    assert!(result.success);
    assert_eq!(result.test_name, "test_success");
    assert_eq!(result.test_cases_run, 100);
    assert!(result.failures.is_empty());
    assert_eq!(result.duration, Duration::from_secs(5));
}

#[test]
fn test_property_test_result_failure() {
    let failure = PropertyFailure {
        original_input: "original".to_string(),
        shrunk_input: "shrunk".to_string(),
        error_message: "test failed".to_string(),
        shrink_steps: 10,
    };

    let result = PropertyTestResult {
        test_name: "test_failure".to_string(),
        success: false,
        test_cases_run: 50,
        failures: vec![failure.clone()],
        duration: Duration::from_secs(2),
        statistics: TestStatistics {
            input_distribution: HashMap::new(),
            execution_times: Vec::new(),
            coverage_metrics: HashMap::new(),
        },
    };

    assert!(!result.success);
    assert_eq!(result.test_cases_run, 50);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].error_message, "test failed");
}

// ============================================================================
// PropertyFailure Tests
// ============================================================================

#[test]
fn test_property_failure_creation() {
    let failure = PropertyFailure {
        original_input: "original_value".to_string(),
        shrunk_input: "shrunk_value".to_string(),
        error_message: "assertion failed".to_string(),
        shrink_steps: 5,
    };

    assert_eq!(failure.original_input, "original_value");
    assert_eq!(failure.shrunk_input, "shrunk_value");
    assert_eq!(failure.error_message, "assertion failed");
    assert_eq!(failure.shrink_steps, 5);
}

#[test]
fn test_property_failure_clone() {
    let failure = PropertyFailure {
        original_input: "test".to_string(),
        shrunk_input: "t".to_string(),
        error_message: "error".to_string(),
        shrink_steps: 3,
    };

    let cloned = failure.clone();
    assert_eq!(failure.original_input, cloned.original_input);
    assert_eq!(failure.shrunk_input, cloned.shrunk_input);
    assert_eq!(failure.error_message, cloned.error_message);
    assert_eq!(failure.shrink_steps, cloned.shrink_steps);
}

// ============================================================================
// TestStatistics Tests
// ============================================================================

#[test]
fn test_statistics_empty() {
    let stats = TestStatistics {
        input_distribution: HashMap::new(),
        execution_times: Vec::new(),
        coverage_metrics: HashMap::new(),
    };

    assert!(stats.input_distribution.is_empty());
    assert!(stats.execution_times.is_empty());
    assert!(stats.coverage_metrics.is_empty());
}

#[test]
fn test_statistics_with_data() {
    let mut input_dist = HashMap::new();
    input_dist.insert("type_a".to_string(), 50);
    input_dist.insert("type_b".to_string(), 30);

    let mut coverage = HashMap::new();
    coverage.insert("branch_coverage".to_string(), 85.5);
    coverage.insert("line_coverage".to_string(), 92.3);

    let stats = TestStatistics {
        input_distribution: input_dist.clone(),
        execution_times: vec![
            Duration::from_millis(10),
            Duration::from_millis(15),
            Duration::from_millis(12),
        ],
        coverage_metrics: coverage.clone(),
    };

    assert_eq!(stats.input_distribution.len(), 2);
    assert_eq!(stats.input_distribution.get("type_a"), Some(&50));
    assert_eq!(stats.execution_times.len(), 3);
    assert_eq!(stats.coverage_metrics.get("branch_coverage"), Some(&85.5));
}

// ============================================================================
// IntegerGenerator Tests
// ============================================================================

#[test]
fn test_integer_generator_creation() {
    let _gen = IntegerGenerator::new(0, 100);
    // Creation should succeed (if we get here, test passes)
}

#[test]
fn test_integer_generator_with_negative_range() {
    let _gen = IntegerGenerator::new(-50, 50);
}

#[test]
fn test_integer_generator_single_value() {
    let _gen = IntegerGenerator::new(42, 42);
}

// ============================================================================
// StringGenerator Tests
// ============================================================================

#[test]
fn test_string_generator_creation() {
    let generator = StringGenerator::new(0, 10);
    drop(generator);
}

#[test]
fn test_string_generator_with_charset() {
    let charset = "abc";
    let generator = StringGenerator::with_charset(5, 15, charset);
    drop(generator);
}

#[test]
fn test_string_generator_fixed_length() {
    let generator = StringGenerator::new(10, 10);
    drop(generator);
}

// ============================================================================
// ShrinkStrategy Tests
// ============================================================================

#[test]
fn test_shrink_strategy_none() {
    let strategy = ShrinkStrategy::None;
    match strategy {
        ShrinkStrategy::None => {}
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_shrink_strategy_linear() {
    let strategy = ShrinkStrategy::Linear;
    match strategy {
        ShrinkStrategy::Linear => {}
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_shrink_strategy_binary() {
    let strategy = ShrinkStrategy::Binary;
    match strategy {
        ShrinkStrategy::Binary => {}
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_shrink_strategy_recursive() {
    let strategy = ShrinkStrategy::Recursive;
    match strategy {
        ShrinkStrategy::Recursive => {}
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_shrink_strategy_custom() {
    let custom_func = Box::new(|_input: &str| vec!["shrunk1".to_string(), "shrunk2".to_string()]);
    let strategy = ShrinkStrategy::Custom(custom_func);

    match strategy {
        ShrinkStrategy::Custom(_) => {}
        _ => panic!("Wrong variant"),
    }
}

// ============================================================================
// PropertyTestRunner Tests
// ============================================================================

#[test]
fn test_property_test_runner_creation() {
    let config = PropertyTestConfig::default();
    let runner = PropertyTestRunner::new(config);
    drop(runner);
}

#[test]
fn test_property_test_runner_with_seed() {
    let config = PropertyTestConfig {
        test_name: "seeded_test".to_string(),
        test_cases: 50,
        shrink_attempts: 20,
        timeout: Duration::from_secs(30),
        verbose: false,
        seed: Some(42),
    };

    let runner = PropertyTestRunner::new(config);
    drop(runner);
}

#[test]
fn test_property_test_runner_custom_config() {
    let config = PropertyTestConfig {
        test_name: "custom_runner".to_string(),
        test_cases: 200,
        shrink_attempts: 50,
        timeout: Duration::from_secs(120),
        verbose: true,
        seed: None,
    };

    let runner = PropertyTestRunner::new(config);
    drop(runner);
}

// ============================================================================
// Generator Trait Tests (via concrete implementations)
// ============================================================================

#[test]
fn test_integer_generator_implements_generator() {
    let mut generator = IntegerGenerator::new(1, 100);
    let value = generator.generate(10);

    // Value should be within range
    assert!((1..=100).contains(&value));
}

#[test]
fn test_integer_generator_shrink() {
    let generator = IntegerGenerator::new(1, 100);
    let shrunk = generator.shrink(&50);

    // Shrinking should produce smaller values
    assert!(!shrunk.is_empty());
    for value in shrunk {
        assert!(value < 50);
    }
}

#[test]
fn test_string_generator_implements_generator() {
    let mut generator = StringGenerator::new(5, 10);
    let value = generator.generate(10);

    // String length should be within range
    assert!(value.len() >= 5 && value.len() <= 10);
}

#[test]
fn test_string_generator_shrink() {
    let generator = StringGenerator::new(1, 20);
    let input = "test_string".to_string();
    let shrunk = generator.shrink(&input);

    // Shrinking should produce shorter strings
    assert!(!shrunk.is_empty());
    for value in shrunk {
        assert!(value.len() < input.len());
    }
}

// ============================================================================
// Complex Scenario Tests
// ============================================================================

#[test]
fn test_property_test_result_with_multiple_failures() {
    let failure1 = PropertyFailure {
        original_input: "input1".to_string(),
        shrunk_input: "i1".to_string(),
        error_message: "first error".to_string(),
        shrink_steps: 3,
    };

    let failure2 = PropertyFailure {
        original_input: "input2".to_string(),
        shrunk_input: "i2".to_string(),
        error_message: "second error".to_string(),
        shrink_steps: 5,
    };

    let result = PropertyTestResult {
        test_name: "multi_failure_test".to_string(),
        success: false,
        test_cases_run: 150,
        failures: vec![failure1, failure2],
        duration: Duration::from_secs(10),
        statistics: TestStatistics {
            input_distribution: HashMap::new(),
            execution_times: Vec::new(),
            coverage_metrics: HashMap::new(),
        },
    };

    assert!(!result.success);
    assert_eq!(result.failures.len(), 2);
    assert_eq!(result.failures[0].error_message, "first error");
    assert_eq!(result.failures[1].error_message, "second error");
}

#[test]
fn test_statistics_comprehensive() {
    let mut input_dist = HashMap::new();
    input_dist.insert("small".to_string(), 30);
    input_dist.insert("medium".to_string(), 50);
    input_dist.insert("large".to_string(), 20);

    let execution_times = vec![
        Duration::from_millis(5),
        Duration::from_millis(10),
        Duration::from_millis(15),
        Duration::from_millis(8),
        Duration::from_millis(12),
    ];

    let mut coverage = HashMap::new();
    coverage.insert("statement_coverage".to_string(), 95.5);
    coverage.insert("branch_coverage".to_string(), 87.2);
    coverage.insert("condition_coverage".to_string(), 78.9);

    let stats = TestStatistics {
        input_distribution: input_dist,
        execution_times,
        coverage_metrics: coverage,
    };

    assert_eq!(stats.input_distribution.len(), 3);
    assert_eq!(stats.execution_times.len(), 5);
    assert_eq!(stats.coverage_metrics.len(), 3);

    // Verify specific values
    assert_eq!(stats.input_distribution.get("medium"), Some(&50));
    assert_eq!(
        stats.coverage_metrics.get("statement_coverage"),
        Some(&95.5)
    );
}

#[test]
fn test_integer_generator_boundary_values() {
    let mut gen_zero = IntegerGenerator::new(0, 0);
    let value_zero = gen_zero.generate(1);
    assert_eq!(value_zero, 0);

    let mut gen_negative = IntegerGenerator::new(-100, -100);
    let value_negative = gen_negative.generate(1);
    assert_eq!(value_negative, -100);
}

#[test]
fn test_string_generator_empty_string() {
    let mut generator = StringGenerator::new(0, 0);
    let value = generator.generate(1);
    assert_eq!(value, "");
}

#[test]
fn test_property_test_config_zero_cases() {
    let config = PropertyTestConfig {
        test_name: "zero_cases".to_string(),
        test_cases: 0,
        shrink_attempts: 0,
        timeout: Duration::from_secs(1),
        verbose: false,
        seed: None,
    };

    assert_eq!(config.test_cases, 0);
    assert_eq!(config.shrink_attempts, 0);
}

// ============================================================================
// Clone and Debug trait tests
// ============================================================================

#[test]
fn test_config_clone() {
    let config = PropertyTestConfig::default();
    let cloned = config.clone();

    assert_eq!(config.test_name, cloned.test_name);
    assert_eq!(config.test_cases, cloned.test_cases);
    assert_eq!(config.shrink_attempts, cloned.shrink_attempts);
}

#[test]
fn test_result_clone() {
    let result = PropertyTestResult {
        test_name: "clone_test".to_string(),
        success: true,
        test_cases_run: 100,
        failures: Vec::new(),
        duration: Duration::from_secs(5),
        statistics: TestStatistics {
            input_distribution: HashMap::new(),
            execution_times: Vec::new(),
            coverage_metrics: HashMap::new(),
        },
    };

    let cloned = result.clone();
    assert_eq!(result.test_name, cloned.test_name);
    assert_eq!(result.success, cloned.success);
    assert_eq!(result.test_cases_run, cloned.test_cases_run);
}

#[test]
fn test_statistics_clone() {
    let stats = TestStatistics {
        input_distribution: HashMap::new(),
        execution_times: vec![Duration::from_millis(10)],
        coverage_metrics: HashMap::new(),
    };

    let cloned = stats.clone();
    assert_eq!(stats.execution_times.len(), cloned.execution_times.len());
}

// ============================================================================
// Edge Cases and Error Conditions
// ============================================================================

#[test]
fn test_property_failure_with_empty_strings() {
    let failure = PropertyFailure {
        original_input: String::new(),
        shrunk_input: String::new(),
        error_message: "empty input error".to_string(),
        shrink_steps: 0,
    };

    assert!(failure.original_input.is_empty());
    assert!(failure.shrunk_input.is_empty());
    assert_eq!(failure.shrink_steps, 0);
}

#[test]
fn test_property_test_result_long_duration() {
    let result = PropertyTestResult {
        test_name: "long_test".to_string(),
        success: true,
        test_cases_run: 10000,
        failures: Vec::new(),
        duration: Duration::from_secs(3600), // 1 hour
        statistics: TestStatistics {
            input_distribution: HashMap::new(),
            execution_times: Vec::new(),
            coverage_metrics: HashMap::new(),
        },
    };

    assert_eq!(result.duration, Duration::from_secs(3600));
    assert_eq!(result.test_cases_run, 10000);
}

#[test]
fn test_integer_generator_large_range() {
    let _gen = IntegerGenerator::new(i64::MIN, i64::MAX);
    // Test passes if generator can be created with large range
}

#[test]
fn test_string_generator_large_length() {
    let generator = StringGenerator::new(0, 10000);
    drop(generator);
}
