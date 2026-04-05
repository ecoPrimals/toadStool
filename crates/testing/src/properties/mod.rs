// SPDX-License-Identifier: AGPL-3.0-or-later
// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Property-based testing utilities
//!
//! This module provides comprehensive property-based testing infrastructure including:
//! - Generator trait for test input generation
//! - Property trait for defining testable properties
//! - Built-in generators (integer, string, vector, composite)
//! - Property types (invariant, round-trip, monotonic)
//! - Test runner with shrinking support
//!
//! # Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Core data structures and configuration
//! - `traits`: Core traits (Generator, Property, RandomNumberGenerator)
//! - `generators`: Built-in generator implementations
//! - `property_impls`: Common property type implementations
//! - `runner`: Test execution logic with shrinking
//!
//! # Example
//!
//! ```rust,ignore
//! use toadstool_testing::properties::{
//!     PropertyTestConfig, PropertyTestRunner, IntegerGenerator, InvariantProperty
//! };
//!
//! let config = PropertyTestConfig {
//!     test_name: "integer_property".to_string(),
//!     test_cases: 100,
//!     ..Default::default()
//! };
//!
//! let mut runner = PropertyTestRunner::new(config);
//! let generator = IntegerGenerator::new(0, 100);
//! let property = InvariantProperty::new(
//!     "positive".to_string(),
//!     |x: &i64| if *x >= 0 { Ok(()) } else { Err(toadstool::ToadStoolError::runtime("negative")) }
//! );
//!
//! let result = runner.run_test(generator, property);
//! ```

mod generators;
mod property_impls;
mod runner;
mod traits;
mod types;

// Re-export public API
pub use generators::{CompositeGenerator, IntegerGenerator, StringGenerator, VectorGenerator};
pub use property_impls::{InvariantProperty, MonotonicProperty, RoundTripProperty};
pub use runner::PropertyTestRunner;
pub use traits::{DefaultRng, Generator, Property, RandomNumberGenerator};
pub use types::{
    CustomTestFunc, PropertyFailure, PropertyTestConfig, PropertyTestResult, ShrinkStrategy,
    TestStatistics,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_property_test_config_default() {
        let config = PropertyTestConfig::default();
        assert_eq!(config.test_name, "unnamed_property");
        assert_eq!(config.test_cases, 100);
        assert_eq!(config.shrink_attempts, 100);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(!config.verbose);
        assert!(config.seed.is_none());
    }

    #[test]
    fn test_property_test_config_clone() {
        let config = PropertyTestConfig {
            test_name: "test".to_string(),
            test_cases: 50,
            shrink_attempts: 20,
            timeout: Duration::from_secs(60),
            verbose: true,
            seed: Some(42),
        };
        let cloned = config.clone();
        assert_eq!(config.test_name, cloned.test_name);
        assert_eq!(config.test_cases, cloned.test_cases);
        assert_eq!(config.verbose, cloned.verbose);
    }

    #[test]
    fn test_test_statistics_new() {
        let stats = TestStatistics::new();
        assert!(stats.input_distribution.is_empty());
        assert!(stats.execution_times.is_empty());
        assert!(stats.coverage_metrics.is_empty());
    }

    #[test]
    fn test_test_statistics_average_time() {
        let mut stats = TestStatistics::new();
        assert_eq!(stats.average_execution_time(), Duration::ZERO);

        stats.execution_times.push(Duration::from_millis(10));
        stats.execution_times.push(Duration::from_millis(20));
        stats.execution_times.push(Duration::from_millis(30));

        let avg = stats.average_execution_time();
        assert_eq!(avg, Duration::from_millis(20));
    }

    #[test]
    fn test_test_statistics_max_time() {
        let mut stats = TestStatistics::new();
        assert_eq!(stats.max_execution_time(), Duration::ZERO);

        stats.execution_times.push(Duration::from_millis(10));
        stats.execution_times.push(Duration::from_millis(30));
        stats.execution_times.push(Duration::from_millis(20));

        assert_eq!(stats.max_execution_time(), Duration::from_millis(30));
    }

    #[test]
    fn test_test_statistics_min_time() {
        let mut stats = TestStatistics::new();
        assert_eq!(stats.min_execution_time(), Duration::ZERO);

        stats.execution_times.push(Duration::from_millis(20));
        stats.execution_times.push(Duration::from_millis(10));
        stats.execution_times.push(Duration::from_millis(30));

        assert_eq!(stats.min_execution_time(), Duration::from_millis(10));
    }

    #[test]
    fn test_integer_generator_new() {
        let generator = IntegerGenerator::new(0, 100);
        // Just verify creation succeeds
        let _ = generator; // Automatic cleanup
    }

    #[test]
    fn test_integer_generator_generate() {
        let mut generator = IntegerGenerator::new(0, 100);
        let value = generator.generate(10);
        assert!((0..=100).contains(&value));
    }

    #[test]
    fn test_integer_generator_shrink_positive() {
        let generator = IntegerGenerator::new(0, 100);
        let shrunk = generator.shrink(&10);
        assert!(!shrunk.is_empty());
        assert!(shrunk.contains(&5) || shrunk.contains(&9) || shrunk.contains(&0));
    }

    #[test]
    fn test_integer_generator_shrink_negative() {
        let generator = IntegerGenerator::new(-100, 0);
        let shrunk = generator.shrink(&-10);
        assert!(!shrunk.is_empty());
    }

    #[test]
    fn test_integer_generator_shrink_zero() {
        let generator = IntegerGenerator::new(-10, 10);
        let shrunk = generator.shrink(&0);
        assert!(shrunk.is_empty());
    }

    #[test]
    fn test_string_generator_new() {
        let generator = StringGenerator::new(0, 10);
        drop(generator);
    }

    #[test]
    fn test_string_generator_with_charset() {
        let generator = StringGenerator::with_charset(0, 10, "abc");
        drop(generator);
    }

    #[test]
    fn test_string_generator_generate() {
        let mut generator = StringGenerator::new(5, 10);
        let value = generator.generate(5);
        assert!(value.len() >= 5 && value.len() <= 10);
    }

    #[test]
    fn test_string_generator_shrink() {
        let generator = StringGenerator::new(0, 100);
        let input = "hello".to_string();
        let shrunk = generator.shrink(&input);
        assert!(!shrunk.is_empty());
    }

    #[test]
    fn test_string_generator_shrink_empty() {
        let generator = StringGenerator::new(0, 100);
        let input = String::new();
        let shrunk = generator.shrink(&input);
        assert!(shrunk.is_empty());
    }

    #[test]
    fn test_default_rng_new() {
        let mut rng = DefaultRng::new();
        let _ = rng.next_u64();
    }

    #[test]
    fn test_default_rng_with_seed() {
        let mut rng = DefaultRng::with_seed(42);
        let value1 = rng.next_u64();

        let mut rng2 = DefaultRng::with_seed(42);
        let value2 = rng2.next_u64();

        assert_eq!(value1, value2);
    }

    #[test]
    fn test_default_rng_next_f64() {
        let mut rng = DefaultRng::new();
        let value = rng.next_f64();
        assert!((0.0..=1.0).contains(&value));
    }

    #[test]
    fn test_property_test_runner_new() {
        let config = PropertyTestConfig::default();
        let runner = PropertyTestRunner::new(config);
        drop(runner);
    }

    #[test]
    fn test_property_test_runner_with_seed() {
        let config = PropertyTestConfig {
            seed: Some(42),
            ..Default::default()
        };
        let runner = PropertyTestRunner::new(config);
        drop(runner);
    }

    #[test]
    fn test_property_test_result_to_report_string() {
        let result = PropertyTestResult {
            test_name: "test".to_string(),
            success: true,
            test_cases_run: 100,
            failures: vec![],
            duration: Duration::from_secs(1),
            statistics: TestStatistics::new(),
        };

        let report = result.to_report_string();
        assert!(report.contains("Property Test: test"));
        assert!(report.contains("✅ PASSED"));
        assert!(report.contains("Test Cases: 100"));
    }

    #[test]
    fn test_property_test_result_to_report_string_with_failures() {
        let failure = PropertyFailure {
            original_input: "42".to_string(),
            shrunk_input: "0".to_string(),
            error_message: "test error".to_string(),
            shrink_steps: 5,
        };

        let result = PropertyTestResult {
            test_name: "test".to_string(),
            success: false,
            test_cases_run: 10,
            failures: vec![failure],
            duration: Duration::from_millis(500),
            statistics: TestStatistics::new(),
        };

        let report = result.to_report_string();
        assert!(report.contains("❌ FAILED"));
        assert!(report.contains("Failures:"));
        assert!(report.contains("Original: 42"));
        assert!(report.contains("Shrunk: 0"));
    }

    #[test]
    fn test_invariant_property() {
        let property = InvariantProperty::new("positive".to_string(), |x: &i64| {
            if *x >= 0 {
                Ok(())
            } else {
                Err(toadstool::ToadStoolError::runtime("negative value"))
            }
        });

        assert_eq!(property.name(), "positive");
        assert!(property.test(&42).is_ok());
        assert!(property.test(&-1).is_err());
    }

    #[test]
    fn test_round_trip_property_success() {
        let property = RoundTripProperty::new(
            "u32_le_bytes",
            |x: &u32| Ok(x.to_le_bytes().to_vec()),
            |bytes: &[u8]| {
                let arr: [u8; 4] = bytes
                    .try_into()
                    .map_err(|_| toadstool::ToadStoolError::runtime("bad length"))?;
                Ok(u32::from_le_bytes(arr))
            },
        );

        assert_eq!(property.name(), "u32_le_bytes");
        assert!(property.test(&42).is_ok());
        assert!(property.test(&0).is_ok());
        assert!(property.test(&u32::MAX).is_ok());
    }

    #[test]
    fn test_round_trip_property_failure() {
        let property = RoundTripProperty::new(
            "always_wrong",
            |x: &u32| Ok(x.to_le_bytes().to_vec()),
            |_bytes: &[u8]| Ok(999_u32),
        );

        let result = property.test(&42);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Round-trip property failed"));
    }

    #[test]
    fn test_shrink_strategy_debug() {
        assert_eq!(
            format!("{:?}", ShrinkStrategy::None),
            "ShrinkStrategy::None"
        );
        assert_eq!(
            format!("{:?}", ShrinkStrategy::Linear),
            "ShrinkStrategy::Linear"
        );
        assert_eq!(
            format!("{:?}", ShrinkStrategy::Binary),
            "ShrinkStrategy::Binary"
        );
        assert_eq!(
            format!("{:?}", ShrinkStrategy::Recursive),
            "ShrinkStrategy::Recursive"
        );
        let custom = ShrinkStrategy::Custom(Box::new(|_| vec![]));
        assert_eq!(format!("{custom:?}"), "ShrinkStrategy::Custom(<function>)");
    }

    #[test]
    fn test_test_statistics_empty_times() {
        let stats = TestStatistics::new();
        assert_eq!(stats.average_execution_time(), Duration::ZERO);
        assert_eq!(stats.max_execution_time(), Duration::ZERO);
        assert_eq!(stats.min_execution_time(), Duration::ZERO);
    }

    #[test]
    fn test_test_statistics_default() {
        let stats = TestStatistics::default();
        assert!(stats.input_distribution.is_empty());
        assert!(stats.execution_times.is_empty());
        assert!(stats.coverage_metrics.is_empty());
    }

    #[test]
    fn test_property_test_result_report_with_stats() {
        let mut stats = TestStatistics::new();
        stats.execution_times.push(Duration::from_millis(10));
        stats.execution_times.push(Duration::from_millis(20));

        let result = PropertyTestResult {
            test_name: "with_stats".to_string(),
            success: true,
            test_cases_run: 2,
            failures: vec![],
            duration: Duration::from_millis(30),
            statistics: stats,
        };

        let report = result.to_report_string();
        assert!(report.contains("with_stats"));
        assert!(report.contains("PASSED"));
        assert!(report.contains("Average Execution Time"));
    }
}
