// SPDX-License-Identifier: AGPL-3.0-or-later
// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Extended tests for property-based test configuration

use std::time::Duration;
use toadstool_testing::properties::{
    DefaultRng, Generator, IntegerGenerator, InvariantProperty, Property, PropertyTestConfig,
    PropertyTestRunner, RandomNumberGenerator, StringGenerator, TestStatistics,
};

#[test]
fn test_custom_property_config() {
    let config = PropertyTestConfig {
        test_name: "custom_property".to_string(),
        test_cases: 50,
        shrink_attempts: 50,
        timeout: Duration::from_mins(1),
        verbose: true,
        seed: Some(12345),
    };

    assert_eq!(config.test_name, "custom_property");
    assert_eq!(config.test_cases, 50);
    assert_eq!(config.shrink_attempts, 50);
    assert_eq!(config.timeout, Duration::from_mins(1));
    assert!(config.verbose);
    assert_eq!(config.seed, Some(12345));
}

#[test]
fn test_property_config_with_no_seed() {
    let config = PropertyTestConfig {
        seed: None,
        ..Default::default()
    };

    assert!(config.seed.is_none());
}

#[test]
fn test_property_config_minimal() {
    let config = PropertyTestConfig {
        test_name: "minimal".to_string(),
        test_cases: 1,
        shrink_attempts: 0,
        timeout: Duration::from_secs(1),
        verbose: false,
        seed: None,
    };

    assert_eq!(config.test_cases, 1);
    assert_eq!(config.shrink_attempts, 0);
}

#[test]
fn test_property_config_extreme_values() {
    let config = PropertyTestConfig {
        test_name: "extreme".to_string(),
        test_cases: 10000,
        shrink_attempts: 1000,
        timeout: Duration::from_hours(1),
        verbose: false,
        seed: Some(u64::MAX),
    };

    assert_eq!(config.test_cases, 10000);
    assert_eq!(config.shrink_attempts, 1000);
    assert_eq!(config.seed, Some(u64::MAX));
}

#[test]
fn test_runner_creation_with_default_config() {
    let config = PropertyTestConfig::default();
    let runner = PropertyTestRunner::new(config);
    drop(runner);
}

#[test]
fn test_runner_creation_with_custom_config() {
    let config = PropertyTestConfig {
        test_name: "custom_runner".to_string(),
        test_cases: 25,
        shrink_attempts: 25,
        timeout: Duration::from_mins(2),
        verbose: false,
        seed: Some(42),
    };

    let runner = PropertyTestRunner::new(config);
    drop(runner);
}

#[test]
fn test_runner_with_seed() {
    let config = PropertyTestConfig {
        seed: Some(99999),
        ..Default::default()
    };

    let runner = PropertyTestRunner::new(config);
    drop(runner);
}

#[test]
fn test_test_statistics_creation() {
    let stats = TestStatistics::new();
    assert!(stats.input_distribution.is_empty());
    assert!(stats.execution_times.is_empty());
    assert!(stats.coverage_metrics.is_empty());
}

#[test]
fn test_test_statistics_with_data() {
    let mut stats = TestStatistics::new();
    stats.execution_times.push(Duration::from_millis(5));
    stats.execution_times.push(Duration::from_millis(15));
    stats.execution_times.push(Duration::from_millis(10));
    stats.input_distribution.insert("i64".to_string(), 10);
    stats
        .coverage_metrics
        .insert("branch_coverage".to_string(), 0.85);

    assert_eq!(stats.execution_times.len(), 3);
    assert_eq!(stats.input_distribution.len(), 1);
    assert_eq!(stats.coverage_metrics.len(), 1);
}

#[test]
fn test_statistics_average_time_empty() {
    let stats = TestStatistics::new();
    assert_eq!(stats.average_execution_time(), Duration::ZERO);
}

#[test]
fn test_statistics_average_time() {
    let mut stats = TestStatistics::new();
    stats.execution_times.push(Duration::from_millis(10));
    stats.execution_times.push(Duration::from_millis(20));
    stats.execution_times.push(Duration::from_millis(30));

    let avg = stats.average_execution_time();
    assert_eq!(avg, Duration::from_millis(20));
}

#[test]
fn test_statistics_max_time_empty() {
    let stats = TestStatistics::new();
    assert_eq!(stats.max_execution_time(), Duration::ZERO);
}

#[test]
fn test_statistics_max_time() {
    let mut stats = TestStatistics::new();
    stats.execution_times.push(Duration::from_millis(5));
    stats.execution_times.push(Duration::from_millis(50));
    stats.execution_times.push(Duration::from_millis(25));

    assert_eq!(stats.max_execution_time(), Duration::from_millis(50));
}

#[test]
fn test_statistics_min_time_empty() {
    let stats = TestStatistics::new();
    assert_eq!(stats.min_execution_time(), Duration::ZERO);
}

#[test]
fn test_statistics_min_time() {
    let mut stats = TestStatistics::new();
    stats.execution_times.push(Duration::from_millis(25));
    stats.execution_times.push(Duration::from_millis(5));
    stats.execution_times.push(Duration::from_millis(50));

    assert_eq!(stats.min_execution_time(), Duration::from_millis(5));
}

#[test]
fn test_default_rng_creation() {
    let mut rng = DefaultRng::new();
    let _ = rng.next_u64();
}

#[test]
fn test_default_rng_with_seed_deterministic() {
    let mut rng1 = DefaultRng::with_seed(123);
    let mut rng2 = DefaultRng::with_seed(123);

    let val1 = rng1.next_u64();
    let val2 = rng2.next_u64();

    assert_eq!(val1, val2, "Same seed should produce same values");
}

#[test]
fn test_default_rng_different_seeds() {
    let mut rng1 = DefaultRng::with_seed(123);
    let mut rng2 = DefaultRng::with_seed(456);

    let val1 = rng1.next_u64();
    let val2 = rng2.next_u64();

    // Very likely to be different (not guaranteed but probability is ~1.0)
    assert_ne!(
        val1, val2,
        "Different seeds should likely produce different values"
    );
}

#[test]
fn test_default_rng_f64_range() {
    let mut rng = DefaultRng::new();
    for _ in 0..100 {
        let val = rng.next_f64();
        assert!((0.0..=1.0).contains(&val), "f64 value should be in [0, 1]");
    }
}

#[test]
fn test_integer_generator_creation() {
    let _gen = IntegerGenerator::new(0, 100);
    // Generator created and dropped automatically
}

#[test]
fn test_integer_generator_negative_range() {
    let _gen = IntegerGenerator::new(-100, -10);
    // Generator created and dropped automatically
}

#[test]
fn test_integer_generator_mixed_range() {
    let _gen = IntegerGenerator::new(-50, 50);
    // Generator created and dropped automatically
}

#[test]
fn test_integer_generator_single_value() {
    let mut generator = IntegerGenerator::new(42, 42);
    let val = generator.generate(10);
    assert_eq!(
        val, 42,
        "Single value range should always produce that value"
    );
}

#[test]
fn test_integer_generator_generate_in_range() {
    let mut generator = IntegerGenerator::new(10, 20);
    for _ in 0..50 {
        let val = generator.generate(10);
        assert!(
            (10..=20).contains(&val),
            "Generated value should be in range"
        );
    }
}

#[test]
fn test_string_generator_creation() {
    let generator = StringGenerator::new(0, 10);
    drop(generator);
}

#[test]
fn test_string_generator_with_custom_charset() {
    let generator = StringGenerator::with_charset(5, 15, "abc123");
    drop(generator);
}

#[test]
fn test_string_generator_min_max_same() {
    let mut generator = StringGenerator::new(5, 5);
    let val = generator.generate(5);
    assert_eq!(
        val.len(),
        5,
        "Fixed length should produce exact length string"
    );
}

#[test]
fn test_string_generator_length_range() {
    let mut generator = StringGenerator::new(5, 10);
    for _ in 0..50 {
        let val = generator.generate(5);
        assert!(
            val.len() >= 5 && val.len() <= 10,
            "Generated string length should be in range"
        );
    }
}

#[test]
fn test_string_generator_empty_allowed() {
    let mut generator = StringGenerator::new(0, 5);
    let _val = generator.generate(0);
    // Just verify it doesn't panic
}

#[test]
fn test_invariant_property_creation() {
    let property = InvariantProperty::new("positive".to_string(), |x: &i64| {
        if *x >= 0 {
            Ok(())
        } else {
            Err(toadstool::ToadStoolError::runtime("negative"))
        }
    });

    assert_eq!(property.name(), "positive");
}

#[test]
fn test_invariant_property_test_success() {
    let property = InvariantProperty::new("always_true".to_string(), |_x: &i64| Ok(()));

    assert!(property.test(&42).is_ok());
    assert!(property.test(&-42).is_ok());
    assert!(property.test(&0).is_ok());
}

#[test]
fn test_invariant_property_test_failure() {
    let property = InvariantProperty::new("always_false".to_string(), |_x: &i64| {
        Err(toadstool::ToadStoolError::runtime("always fails"))
    });

    assert!(property.test(&42).is_err());
    assert!(property.test(&0).is_err());
}

#[test]
fn test_run_simple_property() {
    let config = PropertyTestConfig {
        test_name: "simple_positive".to_string(),
        test_cases: 10,
        shrink_attempts: 5,
        timeout: Duration::from_secs(5),
        verbose: false,
        seed: Some(42),
    };

    let mut runner = PropertyTestRunner::new(config);
    let generator = IntegerGenerator::new(0, 100);
    let property = InvariantProperty::new("non_negative".to_string(), |x: &i64| {
        if *x >= 0 {
            Ok(())
        } else {
            Err(toadstool::ToadStoolError::runtime("negative value"))
        }
    });

    let result = runner.run_test(generator, property);
    assert!(result.success, "Property should pass for positive range");
    assert_eq!(result.test_cases_run, 10);
}

#[test]
fn test_run_failing_property() {
    let config = PropertyTestConfig {
        test_name: "impossible".to_string(),
        test_cases: 10,
        shrink_attempts: 5,
        timeout: Duration::from_secs(5),
        verbose: false,
        seed: Some(42),
    };

    let mut runner = PropertyTestRunner::new(config);
    let generator = IntegerGenerator::new(0, 100);
    let property = InvariantProperty::new("always_fails".to_string(), |_x: &i64| {
        Err(toadstool::ToadStoolError::runtime("always fails"))
    });

    let result = runner.run_test(generator, property);
    assert!(!result.success, "Property should fail");
    assert!(!result.failures.is_empty());
}

#[test]
fn test_config_long_property_name() {
    let long_name = "prop_".repeat(100);
    let config = PropertyTestConfig {
        test_name: long_name.clone(),
        ..Default::default()
    };

    assert_eq!(config.test_name, long_name);
}

#[test]
fn test_statistics_clone() {
    let mut stats = TestStatistics::new();
    stats.execution_times.push(Duration::from_millis(10));
    stats.input_distribution.insert("test".to_string(), 5);

    let cloned = stats.clone();
    assert_eq!(stats.execution_times.len(), cloned.execution_times.len());
    assert_eq!(
        stats.input_distribution.len(),
        cloned.input_distribution.len()
    );
}

#[test]
fn test_config_zero_timeout() {
    let config = PropertyTestConfig {
        timeout: Duration::ZERO,
        ..Default::default()
    };

    assert_eq!(config.timeout, Duration::ZERO);
}
