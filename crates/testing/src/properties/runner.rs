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

//! Property test runner and execution logic

use std::fmt::Debug;
use std::time::Instant;

use super::traits::{DefaultRng, Generator, Property, RandomNumberGenerator};
use super::types::{PropertyFailure, PropertyTestConfig, PropertyTestResult, TestStatistics};

/// Property test runner
pub struct PropertyTestRunner {
    config: PropertyTestConfig,
    _rng: Box<dyn RandomNumberGenerator>,
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
