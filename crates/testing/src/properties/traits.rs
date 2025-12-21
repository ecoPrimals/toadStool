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

//! Core traits for property-based testing

use anyhow::Result;

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

/// Random number generator trait for testability
pub trait RandomNumberGenerator {
    fn next_u64(&mut self) -> u64;
    fn next_f64(&mut self) -> f64;
    fn seed(&mut self, seed: u64);
}

/// Default RNG implementation for property testing
#[derive(Debug)]
pub struct DefaultRng {
    state: u64,
}

impl DefaultRng {
    /// Create a new RNG with default seed
    #[must_use]
    pub fn new() -> Self {
        Self { state: 42 }
    }

    /// Create a new RNG with specified seed
    #[must_use]
    pub fn with_seed(seed: u64) -> Self {
        Self { state: seed }
    }
}

impl Default for DefaultRng {
    fn default() -> Self {
        Self::new()
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
