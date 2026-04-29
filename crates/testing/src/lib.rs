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

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    refining_impl_trait,
    reason = "mockall RuntimeEngine mock uses concrete Pin<Box<...>> futures"
)]
#![allow(
    clippy::pedantic, // Dev/test helper crate: keep `toadstool` pedantic-clean without duplicating policy here
    clippy::unused_async, // Test helpers/placeholders; async for trait/API consistency
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::cast_lossless,
    clippy::unreadable_literal,
    clippy::needless_continue,
    clippy::format_push_string,
    clippy::used_underscore_binding,
    clippy::unused_self,
    clippy::struct_excessive_bools,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_pass_by_value,
    clippy::match_same_arms,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
)]

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
pub mod gpu_guards;
pub mod integration;
pub mod mocks;
pub mod performance;
pub mod properties;

// Modern concurrent testing helpers
pub mod helpers;

// Re-export commonly used testing utilities.
// Wildcards retained: 50+ items across assertions/builders/fixtures/mocks; test helper crates
// are designed for `use toadstool_testing::*` in tests; all items are used; explicit re-exports
// would be verbose and tests typically glob-import anyway (per rule: 15+ items, all used).
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
pub use serde_yaml_ng;
pub use tempfile;
pub use tokio_test;

/// Common test result type
pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Test configuration constants
pub mod constants {
    use std::time::Duration;

    /// Default timeout for async tests (individual tests can set their own if needed)
    pub const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(5);

    /// Short timeout for unit tests
    pub const UNIT_TEST_TIMEOUT: Duration = Duration::from_secs(2);

    /// Long timeout for integration tests (individual tests can set their own if needed)
    pub const INTEGRATION_TEST_TIMEOUT: Duration = Duration::from_secs(30);

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
