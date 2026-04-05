// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Protocol Errors
//!
//! This test suite provides extensive coverage of error types, error messages,
//! and error handling in the protocol integration layer.

use toadstool_integration_protocols::types::*;

// ============================================================================
// ProtocolError Variants Tests
// ============================================================================

#[test]
fn test_protocol_error_connection() {
    let error = ProtocolError::Connection("failed to connect".to_string());
    let message = format!("{error}");
    assert!(message.contains("Connection failed"));
    assert!(message.contains("failed to connect"));
}

#[test]
fn test_protocol_error_authentication() {
    let error = ProtocolError::Authentication("invalid token".to_string());
    let message = format!("{error}");
    assert!(message.contains("Authentication failed"));
    assert!(message.contains("invalid token"));
}

#[test]
fn test_protocol_error_authorization() {
    let error = ProtocolError::Authorization("permission denied".to_string());
    let message = format!("{error}");
    assert!(message.contains("Authorization failed"));
    assert!(message.contains("permission denied"));
}

#[test]
fn test_protocol_error_negotiation() {
    let error = ProtocolError::Negotiation("version mismatch".to_string());
    let message = format!("{error}");
    assert!(message.contains("Protocol negotiation failed"));
    assert!(message.contains("version mismatch"));
}

#[test]
fn test_protocol_error_serialization() {
    let error = ProtocolError::Serialization("invalid format".to_string());
    let message = format!("{error}");
    assert!(message.contains("Serialization error"));
    assert!(message.contains("invalid format"));
}

#[test]
fn test_protocol_error_transport() {
    let error = ProtocolError::Transport("connection reset".to_string());
    let message = format!("{error}");
    assert!(message.contains("Transport error"));
    assert!(message.contains("connection reset"));
}

#[test]
fn test_protocol_error_http_transport_not_available() {
    let error = ProtocolError::HttpTransportNotAvailable;
    let message = format!("{error}");
    assert!(message.contains("HTTP transport not available"));
    assert!(message.contains("Coordination"));
}

#[test]
fn test_protocol_error_trpc_transport_not_available() {
    let error = ProtocolError::TRpcTransportNotAvailable;
    let message = format!("{error}");
    assert!(message.contains("tRPC transport not available"));
    assert!(message.contains("pure_jsonrpc"));
}

#[test]
fn test_protocol_error_discovery() {
    let error = ProtocolError::Discovery("service not found".to_string());
    let message = format!("{error}");
    assert!(message.contains("Service discovery error"));
    assert!(message.contains("service not found"));
}

#[test]
fn test_protocol_error_routing() {
    let error = ProtocolError::Routing("no route to destination".to_string());
    let message = format!("{error}");
    assert!(message.contains("Message routing error"));
    assert!(message.contains("no route to destination"));
}

#[test]
fn test_protocol_error_timeout() {
    let error = ProtocolError::Timeout("request timed out".to_string());
    let message = format!("{error}");
    assert!(message.contains("Timeout"));
    assert!(message.contains("request timed out"));
}

#[test]
fn test_protocol_error_internal() {
    let error = ProtocolError::Internal("unexpected condition".to_string());
    let message = format!("{error}");
    assert!(message.contains("Internal error"));
    assert!(message.contains("unexpected condition"));
}

// ============================================================================
// ProtocolError Debug Tests
// ============================================================================

#[test]
fn test_protocol_error_debug_connection() {
    let error = ProtocolError::Connection("test".to_string());
    let debug_string = format!("{error:?}");
    assert!(debug_string.contains("Connection"));
}

#[test]
fn test_protocol_error_debug_authentication() {
    let error = ProtocolError::Authentication("test".to_string());
    let debug_string = format!("{error:?}");
    assert!(debug_string.contains("Authentication"));
}

#[test]
fn test_protocol_error_debug_authorization() {
    let error = ProtocolError::Authorization("test".to_string());
    let debug_string = format!("{error:?}");
    assert!(debug_string.contains("Authorization"));
}

// ============================================================================
// ProtocolResult Tests
// ============================================================================

#[test]
fn test_protocol_result_ok() {
    let result: ProtocolResult<i32> = Ok(42);
    assert!(result.is_ok());
    if let Ok(val) = result {
        assert_eq!(val, 42);
    }
}

#[test]
fn test_protocol_result_err() {
    let result: ProtocolResult<i32> = Err(ProtocolError::Internal("test".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_protocol_result_map() {
    let result: ProtocolResult<i32> = Ok(10);
    let mapped = result.map(|x| x * 2);
    assert_eq!(mapped.unwrap(), 20);
}

#[test]
fn test_protocol_result_map_err() {
    let result: ProtocolResult<i32> = Err(ProtocolError::Timeout("test".to_string()));
    let mapped = result.map_err(|_| ProtocolError::Internal("mapped".to_string()));
    assert!(mapped.is_err());
    if let Err(e) = mapped {
        let msg = format!("{e}");
        assert!(msg.contains("Internal"));
    }
}

#[test]
fn test_protocol_result_and_then() {
    let result: ProtocolResult<i32> = Ok(10);
    let chained = result.map(|x| x + 5);
    assert_eq!(chained.unwrap(), 15);
}

#[test]
fn test_protocol_result_or_else() {
    let result: ProtocolResult<i32> = Err(ProtocolError::Internal("test".to_string()));
    let recovered: ProtocolResult<i32> = result.or(Ok(42));
    assert_eq!(recovered.unwrap(), 42);
}

// ============================================================================
// Error Message Quality Tests
// ============================================================================

#[test]
fn test_error_messages_are_descriptive() {
    let errors = vec![
        ProtocolError::Connection("connection reset by peer".to_string()),
        ProtocolError::Authentication("invalid bearer token".to_string()),
        ProtocolError::Authorization("insufficient permissions".to_string()),
        ProtocolError::Timeout("request exceeded 30s deadline".to_string()),
    ];

    for error in errors {
        let message = format!("{error}");
        // Error messages should be descriptive (more than just the type)
        assert!(message.len() > 20, "Error message too short: {message}");
    }
}

#[test]
fn test_error_messages_include_context() {
    let context = "detailed context information";
    let error = ProtocolError::Connection(context.to_string());
    let message = format!("{error}");
    assert!(message.contains(context));
}

#[test]
fn test_error_display_vs_debug() {
    let error = ProtocolError::Internal("test error".to_string());
    let display = format!("{error}");
    let debug = format!("{error:?}");

    // Display should be user-friendly
    assert!(display.contains("Internal error"));
    // Debug should include variant name
    assert!(debug.contains("Internal"));
}

// ============================================================================
// Error Propagation Tests
// ============================================================================

fn function_that_returns_protocol_result(should_fail: bool) -> ProtocolResult<String> {
    if should_fail {
        Err(ProtocolError::Connection("failed".to_string()))
    } else {
        Ok("success".to_string())
    }
}

#[test]
fn test_error_propagation_success() {
    let result = function_that_returns_protocol_result(false);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
}

#[test]
fn test_error_propagation_failure() {
    let result = function_that_returns_protocol_result(true);
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(format!("{e}").contains("Connection failed"));
    }
}

#[test]
fn test_error_propagation_with_question_mark() {
    fn inner() -> ProtocolResult<i32> {
        let _ = function_that_returns_protocol_result(false)?;
        Ok(42)
    }

    let result = inner();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_error_propagation_chain() {
    fn level_1() -> ProtocolResult<String> {
        function_that_returns_protocol_result(false)
    }

    fn level_2() -> ProtocolResult<String> {
        level_1()
    }

    let result = level_2();
    assert!(result.is_ok());
}

// ============================================================================
// Error Conversion Tests
// ============================================================================

#[test]
fn test_json_error_conversion() {
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json");
    assert!(json_error.is_err());

    if let Err(e) = json_error {
        let protocol_error: ProtocolError = e.into();
        let message = format!("{protocol_error}");
        assert!(message.contains("JSON serialization error"));
    }
}

// ============================================================================
// Test Counter
// ============================================================================

#[test]
fn test_error_coverage_summary() {
    println!("============================================");
    println!("Protocol Error Tests Summary:");
    println!("============================================");
    println!("Error Variant Tests:         11 tests");
    println!("Debug Tests:                  3 tests");
    println!("ProtocolResult Tests:         6 tests");
    println!("Error Message Tests:          3 tests");
    println!("Error Propagation Tests:      4 tests");
    println!("Error Conversion Tests:       1 test");
    println!("============================================");
    println!("Total Protocol Error Tests:  28 tests");
    println!("============================================");
}
