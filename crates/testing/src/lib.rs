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

#![deny(unsafe_code)]

//! # `ToadStool` Testing Infrastructure
//!
//! This crate provides comprehensive testing utilities, mocks, and infrastructure
//! for testing `ToadStool` components. It includes:
//!
//! - Mock implementations of runtime engines
//! - Test data generators and builders
//! - Integration test utilities
//! - Performance testing helpers
//! - Property-based testing support

pub mod assertions;
pub mod builders;
pub mod chaos;
pub mod fixtures;
pub mod integration;
pub mod mocks;
pub mod performance;
pub mod properties;

// Modern concurrent testing helpers
pub mod helpers;

// Re-export commonly used testing utilities
pub use assertions::*;
pub use builders::*;
pub use fixtures::*;
pub use mocks::*;

// Re-export concurrent helpers for modern testing
pub use helpers::concurrent::*;
pub use helpers::isolation::*;
pub use helpers::timeout::*;

// Re-export external testing dependencies for convenience
pub use fake;
pub use proptest;
pub use serde_json;
pub use serde_yaml;
pub use tempfile;
pub use tokio_test;

/// Common test result type
pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Test configuration constants
pub mod constants {
    use std::time::Duration;

    /// Default timeout for async tests
    pub const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(30);

    /// Short timeout for unit tests
    pub const UNIT_TEST_TIMEOUT: Duration = Duration::from_secs(5);

    /// Long timeout for integration tests
    pub const INTEGRATION_TEST_TIMEOUT: Duration = Duration::from_secs(120);

    /// Default test data size
    pub const DEFAULT_TEST_DATA_SIZE: usize = 1024;

    /// Maximum test iterations for property tests
    pub const MAX_PROPERTY_TEST_CASES: u32 = 1000;
}

/// Initialize test environment with proper logging and tracing
pub fn init_test_env() {
    // Initialize tracing for tests if not already done
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .try_init();
}

/// Macro for creating async tests with timeout
#[macro_export]
macro_rules! async_test {
    ($name:ident, $timeout:expr, $body:expr) => {
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        #[timeout($timeout)]
        async fn $name() -> $crate::TestResult {
            $crate::init_test_env();
            $body.await
        }
    };
    ($name:ident, $body:expr) => {
        async_test!($name, $crate::constants::DEFAULT_TEST_TIMEOUT, $body);
    };
}

/// Macro for creating property tests
#[macro_export]
macro_rules! property_test {
    ($name:ident, $strategy:expr, $test:expr) => {
        #[test]
        fn $name() {
            $crate::init_test_env();
            use $crate::proptest::prelude::*;
            proptest!($strategy, $test);
        }
    };
}
