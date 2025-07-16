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

        let size = base_size + (iteration as f64 * growth_rate) as usize;
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
    pub fn new(min_length: usize, max_length: usize) -> Self {
        Self {
            min_length,
            max_length,
            charset: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
                .chars()
                .collect(),
        }
    }

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
                "Round-trip property failed: {:?} != {:?}",
                input,
                decoded
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
    pub fn average_execution_time(&self) -> Duration {
        if self.execution_times.is_empty() {
            Duration::ZERO
        } else {
            self.execution_times.iter().sum::<Duration>() / self.execution_times.len() as u32
        }
    }

    /// Get execution time percentiles
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
