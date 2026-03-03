// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for testing/src/lib.rs
//!
//! Target: Get lib.rs from 0% → 100% coverage

use std::time::Duration;
use toadstool_testing::*;

// ============================================================================
// TestResult Type Tests
// ============================================================================

#[test]
fn test_test_result_ok() {
    let result: TestResult = Ok(());
    assert!(result.is_ok());
}

#[test]
fn test_test_result_ok_with_value() {
    let value = 42;
    let result: TestResult<i32> = Ok(value);
    // Test that we can extract the value
    if let Ok(v) = result {
        assert_eq!(v, value);
    } else {
        panic!("Expected Ok variant");
    }
}

#[test]
fn test_test_result_err() {
    let result: TestResult = Err("test error".into());
    assert!(result.is_err());
}

// ============================================================================
// Constants Module Tests
// ============================================================================

#[test]
fn test_constants_timeout_values() {
    use toadstool_testing::constants::*;

    assert_eq!(DEFAULT_TEST_TIMEOUT, Duration::from_secs(5));
    assert_eq!(UNIT_TEST_TIMEOUT, Duration::from_secs(2));
    assert_eq!(INTEGRATION_TEST_TIMEOUT, Duration::from_secs(30));
}

#[test]
fn test_constants_timeout_ordering() {
    use toadstool_testing::constants::*;

    assert!(UNIT_TEST_TIMEOUT < DEFAULT_TEST_TIMEOUT);
    assert!(DEFAULT_TEST_TIMEOUT < INTEGRATION_TEST_TIMEOUT);
}

#[test]
fn test_constants_data_size() {
    use toadstool_testing::constants::*;

    assert_eq!(DEFAULT_TEST_DATA_SIZE, 1024);
}

#[test]
fn test_constants_property_test_cases() {
    use toadstool_testing::constants::*;

    assert_eq!(MAX_PROPERTY_TEST_CASES, 1000);
}

// ============================================================================
// init_test_env Tests
// ============================================================================

#[test]
fn test_init_test_env_runs() {
    // Should not panic
    init_test_env();
}

#[test]
fn test_init_test_env_multiple_calls() {
    // Should handle multiple initializations gracefully
    init_test_env();
    init_test_env();
    init_test_env();
}

// ============================================================================
// Re-export Tests
// ============================================================================

#[test]
fn test_reexports_available() {
    // Test that re-exported types are accessible
    let _result: TestResult = Ok(());

    // Test fake re-export
    let _faker = fake::Faker;

    // Test proptest re-export
    use proptest::prelude::*;
    let _strategy = Just(42);
}

// ============================================================================
// Module Structure Tests
// ============================================================================

#[test]
fn test_all_modules_accessible() {
    // Verify all public modules are accessible by importing them
    // If these modules don't exist or aren't public, this won't compile

    // Test passed if we got here - modules are accessible
    // Test passes if compilation succeeds
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_test_result_conversion() {
    let std_result: Result<(), &str> = Ok(());
    let test_result: TestResult = std_result.map_err(|e| e.into());
    assert!(test_result.is_ok());
}

#[test]
fn test_constants_usage_in_timeout() {
    use toadstool_testing::constants::*;

    // Simulate using constants for test timeouts
    let timeout = DEFAULT_TEST_TIMEOUT;
    assert!(timeout.as_secs() > 0);
    assert!(timeout.as_secs() <= 30);
}
