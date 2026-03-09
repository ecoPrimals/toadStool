// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for NestGate Error Types
//!
//! This test suite provides extensive coverage of NestGate error handling,
//! including all error variants, conversions, and display formatting.

use toadstool_integration_nestgate::*;

// ============================================================================
// NestGateError Variant Tests
// ============================================================================

#[test]
fn test_nestgate_error_connection() {
    let error = NestGateError::Connection("timeout".to_string());
    let error_str = format!("{error}");
    assert!(error_str.contains("Connection failed"));
    assert!(error_str.contains("timeout"));
}

#[test]
fn test_nestgate_error_authentication() {
    let error = NestGateError::Authentication("invalid token".to_string());
    let error_str = format!("{error}");
    assert!(error_str.contains("Authentication failed"));
    assert!(error_str.contains("invalid token"));
}

#[test]
fn test_nestgate_error_storage() {
    let error = NestGateError::Storage("disk full".to_string());
    let error_str = format!("{error}");
    assert!(error_str.contains("Storage operation failed"));
    assert!(error_str.contains("disk full"));
}

#[test]
fn test_nestgate_error_pipeline() {
    let error = NestGateError::Pipeline("invalid stage".to_string());
    let error_str = format!("{error}");
    assert!(error_str.contains("Data pipeline error"));
    assert!(error_str.contains("invalid stage"));
}

#[test]
fn test_nestgate_error_versioning() {
    let error = NestGateError::Versioning("conflict detected".to_string());
    let error_str = format!("{error}");
    assert!(error_str.contains("Versioning error"));
    assert!(error_str.contains("conflict detected"));
}

#[test]
fn test_nestgate_error_network() {
    let error = NestGateError::Network("connection refused".to_string());
    let error_str = format!("{error}");
    assert!(error_str.contains("Network error"));
    assert!(error_str.contains("connection refused"));
}

#[test]
fn test_nestgate_error_serialization() {
    let json_error = serde_json::from_str::<serde_json::Value>("{invalid}");
    assert!(json_error.is_err());

    if let Err(e) = json_error {
        let error: NestGateError = e.into();
        let error_str = format!("{error}");
        assert!(error_str.contains("Serialization error"));
    }
}

#[test]
fn test_nestgate_error_io() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let error: NestGateError = io_error.into();
    let error_str = format!("{error}");
    assert!(error_str.contains("IO error"));
    assert!(error_str.contains("file not found"));
}

#[test]
fn test_nestgate_error_internal() {
    let error = NestGateError::Internal("unexpected state".to_string());
    let error_str = format!("{error}");
    assert!(error_str.contains("Internal error"));
    assert!(error_str.contains("unexpected state"));
}

// ============================================================================
// Error Display and Debug Tests
// ============================================================================

#[test]
fn test_nestgate_error_debug_format() {
    let error = NestGateError::Connection("test".to_string());
    let debug_str = format!("{error:?}");
    assert!(debug_str.contains("Connection"));
    assert!(debug_str.contains("test"));
}

#[test]
fn test_nestgate_error_display_vs_debug() {
    let error = NestGateError::Storage("test error".to_string());
    let display = format!("{error}");
    let debug = format!("{error:?}");

    // Display is user-friendly
    assert!(display.contains("Storage operation failed"));
    // Debug shows variant name
    assert!(debug.contains("Storage"));
}

// ============================================================================
// Error Conversion Tests
// ============================================================================

#[test]
fn test_nestgate_error_from_serde() {
    let json_str = "{invalid json}";
    let parse_result = serde_json::from_str::<serde_json::Value>(json_str);
    assert!(parse_result.is_err());

    let serde_error = parse_result.unwrap_err();
    let nestgate_error: NestGateError = serde_error.into();

    match nestgate_error {
        NestGateError::Serialization(_) => {} // Expected
        _ => panic!("Expected Serialization error variant"),
    }
}

#[test]
fn test_nestgate_error_from_io() {
    let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let nestgate_error: NestGateError = io_error.into();

    match nestgate_error {
        NestGateError::Io(_) => {} // Expected
        _ => panic!("Expected Io error variant"),
    }
}

// ============================================================================
// NestGateResult Type Tests
// ============================================================================

#[test]
fn test_nestgate_result_ok() {
    let result: NestGateResult<i32> = Ok(42);
    assert!(result.is_ok());
    if let Ok(val) = result {
        assert_eq!(val, 42);
    }
}

#[test]
fn test_nestgate_result_err() {
    let result: NestGateResult<i32> = Err(NestGateError::Internal("test".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_nestgate_result_map() {
    let result: NestGateResult<i32> = Ok(10);
    let mapped = result.map(|x| x * 2);
    assert_eq!(mapped.unwrap(), 20);
}

#[test]
fn test_nestgate_result_and_then() {
    let result: NestGateResult<i32> = Ok(10);
    let chained = result.map(|x| x + 5);
    assert_eq!(chained.unwrap(), 15);
}

#[test]
fn test_nestgate_result_or_else() {
    let result: NestGateResult<i32> = Err(NestGateError::Internal("test".to_string()));
    let recovered: NestGateResult<i32> = result.or(Ok(42));
    assert_eq!(recovered.unwrap(), 42);
}

// ============================================================================
// Error Propagation Tests
// ============================================================================

fn function_that_returns_nestgate_result(should_fail: bool) -> NestGateResult<String> {
    if should_fail {
        Err(NestGateError::Connection("failed".to_string()))
    } else {
        Ok("success".to_string())
    }
}

#[test]
fn test_error_propagation_success() {
    let result = function_that_returns_nestgate_result(false);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
}

#[test]
fn test_error_propagation_failure() {
    let result = function_that_returns_nestgate_result(true);
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(format!("{e}").contains("Connection failed"));
    }
}

#[test]
fn test_error_propagation_with_question_mark() {
    fn inner() -> NestGateResult<i32> {
        let _ = function_that_returns_nestgate_result(false)?;
        Ok(42)
    }

    let result = inner();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

// ============================================================================
// Error Message Quality Tests
// ============================================================================

#[test]
fn test_error_messages_are_descriptive() {
    let errors = vec![
        NestGateError::Connection("timeout after 30s".to_string()),
        NestGateError::Authentication("token expired".to_string()),
        NestGateError::Storage("out of disk space".to_string()),
        NestGateError::Pipeline("missing required stage".to_string()),
        NestGateError::Versioning("version conflict".to_string()),
        NestGateError::Network("DNS resolution failed".to_string()),
        NestGateError::Internal("null pointer dereference".to_string()),
    ];

    for error in errors {
        let message = format!("{error}");
        // All error messages should be at least 10 characters
        assert!(message.len() >= 10, "Error message too short: {message}");
        // All error messages should contain context
        assert!(!message.is_empty());
    }
}

#[test]
fn test_error_context_preservation() {
    let original_message = "very specific error context that should be preserved";
    let error = NestGateError::Pipeline(original_message.to_string());
    let formatted = format!("{error}");

    assert!(formatted.contains(original_message));
}

// ============================================================================
// Test Counter
// ============================================================================

#[test]
fn test_nestgate_error_coverage_summary() {
    println!("============================================");
    println!("NestGate Error Tests Summary:");
    println!("============================================");
    println!("Error Variants:          9 tests");
    println!("Display/Debug:           2 tests");
    println!("Conversions:             2 tests");
    println!("NestGateResult:          5 tests");
    println!("Error Propagation:       3 tests");
    println!("Message Quality:         2 tests");
    println!("============================================");
    println!("Total Error Tests:      23 tests");
    println!("============================================");
}
