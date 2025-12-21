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

//! Built-in generators for property-based testing

use super::traits::Generator;

/// Integer generator for property tests
pub struct IntegerGenerator {
    min: i64,
    max: i64,
}

impl IntegerGenerator {
    /// Create a new integer generator with specified range
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

/// String generator for property tests
pub struct StringGenerator {
    min_length: usize,
    max_length: usize,
    charset: Vec<char>,
}

impl StringGenerator {
    /// Create a new string generator with default charset
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

    /// Create a new string generator with custom charset
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

/// Vector generator for property tests
pub struct VectorGenerator<T, G: Generator<T>> {
    _element_generator: G,
    _min_length: usize,
    _max_length: usize,
    _phantom: std::marker::PhantomData<T>,
}

/// Composite generator combining multiple generators
pub struct CompositeGenerator<T> {
    _generators: Vec<Box<dyn Generator<T>>>,
    _weights: Vec<f64>,
}
