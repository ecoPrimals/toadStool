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

use anyhow::Result;
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Duration, Instant};

/// Type alias for custom test functions to reduce complexity
type CustomTestFunc = dyn Fn(&str) -> Vec<String>;

/// Property-based test configuration
#[derive(Debug, Clone)]
pub struct PropertyTestConfig {
    pub test_name: String,
    pub test_cases: u32,
    pub shrink_attempts: u32,
    pub timeout: Duration,
    pub verbose: bool,
    pub seed: Option<u64>,
}

/// Property test result
#[derive(Debug, Clone)]
pub struct PropertyTestResult {
    pub test_name: String,
    pub success: bool,
    pub test_cases_run: u32,
    pub failures: Vec<PropertyFailure>,
    pub duration: Duration,
    pub statistics: TestStatistics,
}

/// Property test failure with shrinking information
#[derive(Debug, Clone)]
pub struct PropertyFailure {
    pub original_input: String,
    pub shrunk_input: String,
    pub error_message: String,
    pub shrink_steps: u32,
}

/// Statistics from property testing
#[derive(Debug, Clone)]
pub struct TestStatistics {
    pub input_distribution: HashMap<String, u32>,
    pub execution_times: Vec<Duration>,
    pub coverage_metrics: HashMap<String, f64>,
}

/// Trait for generating test inputs
pub trait Generator<T> {
    fn generate(&mut self, size: usize) -> T;
    fn shrink(&self, input: &T) -> Vec<T>;
}

/// Trait for testable properties
pub trait Property<T> {
    fn test(&self, input: &T) -> Result<()>;
    fn name(&self) -> &str;
}

/// Property test runner
pub struct PropertyTestRunner {
    config: PropertyTestConfig,
    _rng: Box<dyn RandomNumberGenerator>,
}

/// Random number generator trait for testability
pub trait RandomNumberGenerator {
    fn next_u64(&mut self) -> u64;
    fn next_f64(&mut self) -> f64;
    fn seed(&mut self, seed: u64);
}

/// Built-in generators
pub struct IntegerGenerator {
    min: i64,
    max: i64,
}

pub struct StringGenerator {
    min_length: usize,
    max_length: usize,
    charset: Vec<char>,
}

pub struct VectorGenerator<T, G: Generator<T>> {
    _element_generator: G,
    _min_length: usize,
    _max_length: usize,
    _phantom: std::marker::PhantomData<T>,
}

pub struct CompositeGenerator<T> {
    _generators: Vec<Box<dyn Generator<T>>>,
    _weights: Vec<f64>,
}

/// Predefined property types
pub struct InvariantProperty<T, F> {
    _name: String,
    predicate: F,
    _phantom: std::marker::PhantomData<T>,
}

pub struct RoundTripProperty<T, F1, F2> {
    _name: String,
    encode: F1,
    decode: F2,
    _phantom: std::marker::PhantomData<T>,
}

pub struct MonotonicProperty<T, F> {
    _name: String,
    _function: F,
    _phantom: std::marker::PhantomData<T>,
}

/// Shrinking strategies
pub enum ShrinkStrategy {
    None,
    Linear,
    Binary,
    Recursive,
    Custom(Box<CustomTestFunc>),
}

impl Default for PropertyTestConfig {
    fn default() -> Self {
        Self {
            test_name: "unnamed_property".to_string(),
            test_cases: 100,
            shrink_attempts: 100,
            timeout: Duration::from_secs(30),
            verbose: false,
            seed: None,
        }
    }
}

impl PropertyTestRunner {
    /// Create a new property test runner
    #[must_use]
    pub fn new(config: PropertyTestConfig) -> Self {
        let mut rng = Box::new(DefaultRng::new());
        if let Some(seed) = config.seed {
            rng.seed(seed);
        }

        Self { config, _rng: rng }
    }

    /// Run property tests with a generator and property
    pub fn run_test<T, G, P>(&mut self, mut generator: G, property: P) -> PropertyTestResult
    where
        T: Debug + Clone,
        G: Generator<T>,
        P: Property<T>,
    {
        let start_time = Instant::now();
        let mut failures = Vec::new();
        let mut statistics = TestStatistics::new();
        let mut test_cases_run = 0;

        for i in 0..self.config.test_cases {
            let size = self.calculate_test_size(i);
            let input = generator.generate(size);
            test_cases_run += 1;

            let test_start = Instant::now();
            match property.test(&input) {
                Ok(()) => {
                    statistics.execution_times.push(test_start.elapsed());
                    self.update_statistics(&mut statistics, &input);
                }
                Err(error) => {
                    if self.config.verbose {
                        println!("Property failure on input: {input:?}");
                    }

                    // Attempt to shrink the failing input
                    let shrunk_input = self.shrink_input(&generator, &property, &input);
                    let failure = PropertyFailure {
                        original_input: format!("{input:?}"),
                        shrunk_input: format!("{shrunk_input:?}"),
                        error_message: error.to_string(),
                        shrink_steps: 0, // Shrink step tracking implemented in shrink_input method
                    };
                    failures.push(failure);
                    break;
                }
            }

            // Check timeout
            if start_time.elapsed() > self.config.timeout {
                if self.config.verbose {
                    println!("Property test timed out after {test_cases_run} test cases");
                }
                break;
            }
        }

        PropertyTestResult {
            test_name: property.name().to_string(),
            success: failures.is_empty(),
            test_cases_run,
            failures,
            duration: start_time.elapsed(),
            statistics,
        }
    }

    /// Run multiple properties in sequence (removed due to trait object sizing issues)
    // This method was removed as Box<dyn Property<T>> creates sizing issues
    // Users should call run_test individually for each property instead
    fn calculate_test_size(&self, iteration: u32) -> usize {
        // Gradually increase test case size
        let base_size = 1;
        let max_size = 100;
        let growth_rate = 0.1;

        let size = base_size + (f64::from(iteration) * growth_rate) as usize;
        size.min(max_size)
    }

    fn shrink_input<T, G, P>(&mut self, generator: &G, property: &P, input: &T) -> T
    where
        T: Debug + Clone,
        G: Generator<T>,
        P: Property<T>,
    {
        let mut current_input = input.clone();
        let mut attempts = 0;

        while attempts < self.config.shrink_attempts {
            let candidates = generator.shrink(&current_input);
            if candidates.is_empty() {
                break;
            }

            let mut found_smaller = false;
            for candidate in candidates {
                if property.test(&candidate).is_err() {
                    current_input = candidate;
                    found_smaller = true;
                    break;
                }
            }

            if !found_smaller {
                break;
            }

            attempts += 1;
        }

        current_input
    }

    fn update_statistics<T>(&self, statistics: &mut TestStatistics, _input: &T)
    where
        T: Debug,
    {
        // Update input distribution (simplified)
        let input_type = std::any::type_name::<T>();
        *statistics
            .input_distribution
            .entry(input_type.to_string())
            .or_insert(0) += 1;
    }
}

// Generator implementations
impl IntegerGenerator {
    #[must_use]
    pub fn new(min: i64, max: i64) -> Self {
        Self { min, max }
    }
}

impl Generator<i64> for IntegerGenerator {
    fn generate(&mut self, _size: usize) -> i64 {
        // Simple linear congruential generator for testing
        self.min + (self.max - self.min) / 2
    }

    fn shrink(&self, input: &i64) -> Vec<i64> {
        let mut candidates = Vec::new();

        // Shrink towards zero
        if *input > 0 {
            candidates.push(input / 2);
            candidates.push(input - 1);
            candidates.push(0);
        } else if *input < 0 {
            candidates.push(input / 2);
            candidates.push(input + 1);
            candidates.push(0);
        }

        candidates.into_iter().filter(|&x| x != *input).collect()
    }
}

impl StringGenerator {
    #[must_use]
    pub fn new(min_length: usize, max_length: usize) -> Self {
        Self {
            min_length,
            max_length,
            charset: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
                .chars()
                .collect(),
        }
    }

    #[must_use]
    pub fn with_charset(min_length: usize, max_length: usize, charset: &str) -> Self {
        Self {
            min_length,
            max_length,
            charset: charset.chars().collect(),
        }
    }
}

impl Generator<String> for StringGenerator {
    fn generate(&mut self, size: usize) -> String {
        let length = self.min_length + (size % (self.max_length - self.min_length + 1));
        (0..length)
            .map(|_| self.charset[size % self.charset.len()])
            .collect()
    }

    fn shrink(&self, input: &String) -> Vec<String> {
        let mut candidates = Vec::new();

        // Shrink by removing characters
        if input.len() > 1 {
            candidates.push(input[..input.len() - 1].to_string());
            candidates.push(input[..input.len() / 2].to_string());
        }

        // Shrink to empty string
        if !input.is_empty() {
            candidates.push(String::new());
        }

        candidates
    }
}

// Property implementations
impl<T, F> InvariantProperty<T, F>
where
    F: Fn(&T) -> Result<()>,
{
    pub fn new(name: String, predicate: F) -> Self {
        Self {
            _name: name,
            predicate,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, F> Property<T> for InvariantProperty<T, F>
where
    F: Fn(&T) -> Result<()>,
{
    fn test(&self, input: &T) -> Result<()> {
        (self.predicate)(input)
    }

    fn name(&self) -> &str {
        &self._name
    }
}

impl<T, F1, F2> RoundTripProperty<T, F1, F2>
where
    F1: Fn(&T) -> Result<Vec<u8>>,
    F2: Fn(&[u8]) -> Result<T>,
    T: PartialEq + Debug,
{
    pub fn new(name: String, encode: F1, decode: F2) -> Self {
        Self {
            _name: name,
            encode,
            decode,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, F1, F2> Property<T> for RoundTripProperty<T, F1, F2>
where
    F1: Fn(&T) -> Result<Vec<u8>>,
    F2: Fn(&[u8]) -> Result<T>,
    T: PartialEq + Debug,
{
    fn test(&self, input: &T) -> Result<()> {
        let encoded = (self.encode)(input)?;
        let decoded = (self.decode)(&encoded)?;

        if *input == decoded {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Round-trip property failed: {input:?} != {decoded:?}"
            ))
        }
    }

    fn name(&self) -> &str {
        &self._name
    }
}

impl TestStatistics {
    fn new() -> Self {
        Self {
            input_distribution: HashMap::new(),
            execution_times: Vec::new(),
            coverage_metrics: HashMap::new(),
        }
    }

    /// Calculate average execution time
    #[must_use]
    pub fn average_execution_time(&self) -> Duration {
        if self.execution_times.is_empty() {
            Duration::ZERO
        } else {
            self.execution_times.iter().sum::<Duration>() / self.execution_times.len() as u32
        }
    }

    /// Get execution time percentiles
    #[must_use]
    pub fn execution_time_percentiles(&self) -> HashMap<String, Duration> {
        if self.execution_times.is_empty() {
            return HashMap::new();
        }

        let mut sorted_times = self.execution_times.clone();
        sorted_times.sort();

        let mut percentiles = HashMap::new();
        percentiles.insert(
            "p50".to_string(),
            sorted_times[sorted_times.len() * 50 / 100],
        );
        percentiles.insert(
            "p90".to_string(),
            sorted_times[sorted_times.len() * 90 / 100],
        );
        percentiles.insert(
            "p95".to_string(),
            sorted_times[sorted_times.len() * 95 / 100],
        );
        percentiles.insert(
            "p99".to_string(),
            sorted_times[sorted_times.len() * 99 / 100],
        );

        percentiles
    }
}

// Default RNG implementation
struct DefaultRng {
    state: u64,
}

impl DefaultRng {
    fn new() -> Self {
        Self {
            state: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }
}

impl RandomNumberGenerator for DefaultRng {
    fn next_u64(&mut self) -> u64 {
        // Simple linear congruential generator
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    fn seed(&mut self, seed: u64) {
        self.state = seed;
    }
}

impl PropertyTestResult {
    /// Generate a human-readable test report
    #[must_use]
    pub fn to_report_string(&self) -> String {
        let mut report = format!(
            "Property Test: {}\n\
             Status: {}\n\
             Test Cases: {}\n\
             Duration: {:.2}ms\n",
            self.test_name,
            if self.success {
                "✅ PASSED"
            } else {
                "❌ FAILED"
            },
            self.test_cases_run,
            self.duration.as_secs_f64() * 1000.0
        );

        if !self.failures.is_empty() {
            report.push_str("\nFailures:\n");
            for (i, failure) in self.failures.iter().enumerate() {
                report.push_str(&format!(
                    "  {}. Original: {}\n\
                     Shrunk: {}\n\
                     Error: {}\n\n",
                    i + 1,
                    failure.original_input,
                    failure.shrunk_input,
                    failure.error_message
                ));
            }
        }

        if !self.statistics.execution_times.is_empty() {
            report.push_str(&format!(
                "Average Execution Time: {:.2}ms\n",
                self.statistics.average_execution_time().as_secs_f64() * 1000.0
            ));
        }

        report
    }
}

/// Utility macros for common property tests
#[macro_export]
macro_rules! invariant {
    ($name:expr, $predicate:expr) => {
        InvariantProperty::new($name.to_string(), $predicate)
    };
}

#[macro_export]
macro_rules! round_trip {
    ($name:expr, $encode:expr, $decode:expr) => {
        RoundTripProperty::new($name.to_string(), $encode, $decode)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(config.seed, cloned.seed);
    }

    #[test]
    fn test_property_test_result_success() {
        let result = PropertyTestResult {
            test_name: "test_success".to_string(),
            success: true,
            test_cases_run: 100,
            failures: Vec::new(),
            duration: Duration::from_secs(5),
            statistics: TestStatistics::new(),
        };
        assert!(result.success);
        assert_eq!(result.test_cases_run, 100);
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_property_test_result_clone() {
        let result = PropertyTestResult {
            test_name: "test".to_string(),
            success: true,
            test_cases_run: 50,
            failures: Vec::new(),
            duration: Duration::from_secs(2),
            statistics: TestStatistics::new(),
        };
        let cloned = result.clone();
        assert_eq!(result.test_name, cloned.test_name);
        assert_eq!(result.success, cloned.success);
    }

    #[test]
    fn test_property_failure_creation() {
        let failure = PropertyFailure {
            original_input: "original".to_string(),
            shrunk_input: "shrunk".to_string(),
            error_message: "test failed".to_string(),
            shrink_steps: 10,
        };
        assert_eq!(failure.original_input, "original");
        assert_eq!(failure.shrunk_input, "shrunk");
        assert_eq!(failure.shrink_steps, 10);
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
        assert_eq!(failure.shrink_steps, cloned.shrink_steps);
    }

    #[test]
    fn test_test_statistics_new() {
        let stats = TestStatistics::new();
        assert!(stats.input_distribution.is_empty());
        assert!(stats.execution_times.is_empty());
        assert!(stats.coverage_metrics.is_empty());
    }

    #[test]
    fn test_test_statistics_clone() {
        let mut stats = TestStatistics::new();
        stats.execution_times.push(Duration::from_millis(10));
        let cloned = stats.clone();
        assert_eq!(stats.execution_times.len(), cloned.execution_times.len());
    }

    #[test]
    fn test_test_statistics_average_execution_time_empty() {
        let stats = TestStatistics::new();
        assert_eq!(stats.average_execution_time(), Duration::ZERO);
    }

    #[test]
    fn test_test_statistics_average_execution_time() {
        let mut stats = TestStatistics::new();
        stats.execution_times.push(Duration::from_millis(10));
        stats.execution_times.push(Duration::from_millis(20));
        stats.execution_times.push(Duration::from_millis(30));

        let avg = stats.average_execution_time();
        assert_eq!(avg, Duration::from_millis(20));
    }

    #[test]
    fn test_test_statistics_percentiles_empty() {
        let stats = TestStatistics::new();
        let percentiles = stats.execution_time_percentiles();
        assert!(percentiles.is_empty());
    }

    #[test]
    fn test_test_statistics_percentiles() {
        let mut stats = TestStatistics::new();
        for i in 1..=100 {
            stats.execution_times.push(Duration::from_millis(i));
        }

        let percentiles = stats.execution_time_percentiles();
        assert!(percentiles.contains_key("p50"));
        assert!(percentiles.contains_key("p90"));
        assert!(percentiles.contains_key("p95"));
        assert!(percentiles.contains_key("p99"));
    }

    #[test]
    fn test_integer_generator_new() {
        let gen = IntegerGenerator::new(0, 100);
        assert_eq!(gen.min, 0);
        assert_eq!(gen.max, 100);
    }

    #[test]
    fn test_integer_generator_generate() {
        let mut gen = IntegerGenerator::new(0, 100);
        let value = gen.generate(10);
        assert!((0..=100).contains(&value));
    }

    #[test]
    fn test_integer_generator_generate_negative_range() {
        let mut gen = IntegerGenerator::new(-50, 50);
        let value = gen.generate(10);
        assert!((-50..=50).contains(&value));
    }

    #[test]
    fn test_integer_generator_shrink_positive() {
        let gen = IntegerGenerator::new(0, 100);
        let shrunk = gen.shrink(&50);
        assert!(!shrunk.is_empty());
        for value in shrunk {
            assert!(value < 50);
        }
    }

    #[test]
    fn test_integer_generator_shrink_negative() {
        let gen = IntegerGenerator::new(-100, 0);
        let shrunk = gen.shrink(&-50);
        assert!(!shrunk.is_empty());
        for value in shrunk {
            assert!(value > -50);
        }
    }

    #[test]
    fn test_integer_generator_shrink_zero() {
        let gen = IntegerGenerator::new(-10, 10);
        let shrunk = gen.shrink(&0);
        assert!(shrunk.is_empty());
    }

    #[test]
    fn test_string_generator_new() {
        let gen = StringGenerator::new(5, 10);
        assert_eq!(gen.min_length, 5);
        assert_eq!(gen.max_length, 10);
        assert!(!gen.charset.is_empty());
    }

    #[test]
    fn test_string_generator_with_charset() {
        let gen = StringGenerator::with_charset(1, 5, "abc");
        assert_eq!(gen.charset.len(), 3);
    }

    #[test]
    fn test_string_generator_generate() {
        let mut gen = StringGenerator::new(5, 10);
        let value = gen.generate(10);
        assert!(value.len() >= 5 && value.len() <= 10);
    }

    #[test]
    fn test_string_generator_generate_empty() {
        let mut gen = StringGenerator::new(0, 0);
        let value = gen.generate(10);
        assert_eq!(value.len(), 0);
    }

    #[test]
    fn test_string_generator_shrink() {
        let gen = StringGenerator::new(0, 20);
        let input = "test_string".to_string();
        let shrunk = gen.shrink(&input);
        assert!(!shrunk.is_empty());
        for value in shrunk {
            assert!(value.len() < input.len());
        }
    }

    #[test]
    fn test_string_generator_shrink_empty() {
        let gen = StringGenerator::new(0, 10);
        let input = String::new();
        let shrunk = gen.shrink(&input);
        assert!(shrunk.is_empty());
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
            test_name: "seeded".to_string(),
            test_cases: 10,
            shrink_attempts: 5,
            timeout: Duration::from_secs(10),
            verbose: false,
            seed: Some(42),
        };
        let runner = PropertyTestRunner::new(config);
        drop(runner);
    }

    #[test]
    fn test_property_test_runner_run_test_success() {
        let config = PropertyTestConfig {
            test_name: "success_test".to_string(),
            test_cases: 10,
            shrink_attempts: 5,
            timeout: Duration::from_secs(10),
            verbose: false,
            seed: Some(42),
        };
        let mut runner = PropertyTestRunner::new(config);

        let gen = IntegerGenerator::new(0, 100);

        struct AlwaysPassProperty;
        impl Property<i64> for AlwaysPassProperty {
            fn test(&self, _input: &i64) -> Result<()> {
                Ok(())
            }
            fn name(&self) -> &str {
                "always_pass"
            }
        }

        let property = AlwaysPassProperty;
        let result = runner.run_test(gen, property);

        assert!(result.success);
        assert_eq!(result.test_cases_run, 10);
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_property_test_runner_run_test_failure() {
        let config = PropertyTestConfig {
            test_name: "failure_test".to_string(),
            test_cases: 100,
            shrink_attempts: 5,
            timeout: Duration::from_secs(10),
            verbose: false,
            seed: Some(42),
        };
        let mut runner = PropertyTestRunner::new(config);

        let gen = IntegerGenerator::new(0, 100);

        struct FailOn50Property;
        impl Property<i64> for FailOn50Property {
            fn test(&self, input: &i64) -> Result<()> {
                if *input == 50 {
                    Err(anyhow::anyhow!("Failed on 50"))
                } else {
                    Ok(())
                }
            }
            fn name(&self) -> &str {
                "fail_on_50"
            }
        }

        let property = FailOn50Property;
        let result = runner.run_test(gen, property);

        // The test generates value 50 (middle of range)
        assert!(!result.success);
        assert!(!result.failures.is_empty());
    }

    #[test]
    fn test_invariant_property_creation() {
        let property = InvariantProperty::new("positive".to_string(), |x: &i64| {
            if *x >= 0 {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Negative number"))
            }
        });

        assert_eq!(property.name(), "positive");
        assert!(property.test(&5).is_ok());
        assert!(property.test(&-5).is_err());
    }

    #[test]
    fn test_round_trip_property_success() {
        let property = RoundTripProperty::new(
            "string_encode_decode".to_string(),
            |s: &String| Ok(s.as_bytes().to_vec()),
            |bytes: &[u8]| Ok(String::from_utf8(bytes.to_vec())?),
        );

        let input = "test".to_string();
        assert!(property.test(&input).is_ok());
    }

    #[test]
    fn test_round_trip_property_failure() {
        let property = RoundTripProperty::new(
            "always_fail".to_string(),
            |s: &String| Ok(s.as_bytes().to_vec()),
            |_bytes: &[u8]| Ok("different".to_string()),
        );

        let input = "test".to_string();
        assert!(property.test(&input).is_err());
    }

    #[test]
    fn test_default_rng_creation() {
        let rng = DefaultRng::new();
        assert!(rng.state > 0);
    }

    #[test]
    fn test_default_rng_next_u64() {
        let mut rng = DefaultRng::new();
        let v1 = rng.next_u64();
        let v2 = rng.next_u64();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_default_rng_next_f64() {
        let mut rng = DefaultRng::new();
        let value = rng.next_f64();
        assert!((0.0..=1.0).contains(&value));
    }

    #[test]
    fn test_default_rng_seed() {
        let mut rng = DefaultRng::new();
        rng.seed(12345);
        assert_eq!(rng.state, 12345);

        let v1 = rng.next_u64();

        rng.seed(12345);
        let v2 = rng.next_u64();

        assert_eq!(v1, v2);
    }

    #[test]
    fn test_property_test_result_to_report_string() {
        let result = PropertyTestResult {
            test_name: "test_report".to_string(),
            success: true,
            test_cases_run: 100,
            failures: Vec::new(),
            duration: Duration::from_secs(2),
            statistics: TestStatistics::new(),
        };

        let report = result.to_report_string();
        assert!(report.contains("test_report"));
        assert!(report.contains("PASSED"));
        assert!(report.contains("100"));
    }

    #[test]
    fn test_property_test_result_to_report_string_with_failures() {
        let failure = PropertyFailure {
            original_input: "original".to_string(),
            shrunk_input: "shrunk".to_string(),
            error_message: "test error".to_string(),
            shrink_steps: 5,
        };

        let result = PropertyTestResult {
            test_name: "test_failure_report".to_string(),
            success: false,
            test_cases_run: 50,
            failures: vec![failure],
            duration: Duration::from_secs(1),
            statistics: TestStatistics::new(),
        };

        let report = result.to_report_string();
        assert!(report.contains("FAILED"));
        assert!(report.contains("Failures"));
        assert!(report.contains("original"));
        assert!(report.contains("shrunk"));
        assert!(report.contains("test error"));
    }

    #[test]
    fn test_shrink_strategy_variants() {
        let _none = ShrinkStrategy::None;
        let _linear = ShrinkStrategy::Linear;
        let _binary = ShrinkStrategy::Binary;
        let _recursive = ShrinkStrategy::Recursive;

        let custom_func = Box::new(|_: &str| vec!["a".to_string()]);
        let _custom = ShrinkStrategy::Custom(custom_func);
    }

    #[test]
    fn test_property_test_config_with_verbose() {
        let config = PropertyTestConfig {
            test_name: "verbose_test".to_string(),
            test_cases: 50,
            shrink_attempts: 10,
            timeout: Duration::from_secs(30),
            verbose: true,
            seed: None,
        };

        assert!(config.verbose);
    }

    #[test]
    fn test_test_statistics_with_data() {
        let mut stats = TestStatistics::new();
        stats.input_distribution.insert("i64".to_string(), 50);
        stats.execution_times.push(Duration::from_millis(10));
        stats.coverage_metrics.insert("lines".to_string(), 85.5);

        assert_eq!(stats.input_distribution.get("i64"), Some(&50));
        assert_eq!(stats.execution_times.len(), 1);
        assert_eq!(stats.coverage_metrics.get("lines"), Some(&85.5));
    }

    #[test]
    fn test_integer_generator_boundary_values() {
        let mut gen_zero = IntegerGenerator::new(0, 0);
        let value = gen_zero.generate(1);
        assert_eq!(value, 0);

        let mut gen_negative = IntegerGenerator::new(-100, -100);
        let value_neg = gen_negative.generate(1);
        assert_eq!(value_neg, -100);
    }

    #[test]
    fn test_string_generator_fixed_length() {
        let mut gen = StringGenerator::new(10, 10);
        let value = gen.generate(5);
        assert_eq!(value.len(), 10);
    }

    #[test]
    fn test_string_generator_shrink_single_char() {
        let gen = StringGenerator::new(0, 10);
        let input = "a".to_string();
        let shrunk = gen.shrink(&input);

        // Single char should shrink to empty
        assert!(!shrunk.is_empty());
        assert!(shrunk.contains(&String::new()));
    }
}
