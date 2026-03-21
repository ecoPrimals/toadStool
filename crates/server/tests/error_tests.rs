// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::match_same_arms,
    clippy::no_effect_underscore_binding,
    clippy::unreadable_literal
)]
//! Comprehensive tests for server error types
//!
//! Tests for `ServerError` variants and error handling.

use toadstool::ToadStoolError;
use toadstool_server::*;

// ============================================================================
// ServerError Variants Tests
// ============================================================================

#[test]
fn test_server_error_initialization() {
    let error = ServerError::Initialization("Failed to bind socket".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Server initialization failed"));
    assert!(error_string.contains("Failed to bind socket"));
}

#[test]
fn test_server_error_runtime_engine() {
    let error = ServerError::RuntimeEngine("WASM engine crashed".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Runtime engine error"));
    assert!(error_string.contains("WASM engine crashed"));
}

#[test]
fn test_server_error_resource_exhaustion() {
    let error = ServerError::ResourceExhaustion("Out of memory".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Resource exhausted"));
    assert!(error_string.contains("Out of memory"));
}

#[test]
fn test_server_error_authentication() {
    let error = ServerError::Authentication("Invalid API key".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Authentication failed"));
    assert!(error_string.contains("Invalid API key"));
}

#[test]
fn test_server_error_authorization() {
    let error = ServerError::Authorization("Insufficient permissions".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Authorization failed"));
    assert!(error_string.contains("Insufficient permissions"));
}

#[test]
fn test_server_error_configuration() {
    let error = ServerError::Configuration("Invalid port number".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Invalid configuration"));
    assert!(error_string.contains("Invalid port number"));
}

#[test]
fn test_server_error_network() {
    let error = ServerError::Network("Connection refused".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Network error"));
    assert!(error_string.contains("Connection refused"));
}

#[test]
fn test_server_error_execution() {
    let error = ServerError::Execution("Execution timed out".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Execution failed"));
    assert!(error_string.contains("Execution timed out"));
}

#[test]
fn test_server_error_internal() {
    let error = ServerError::Internal("Unexpected panic".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Internal server error"));
    assert!(error_string.contains("Unexpected panic"));
}

// ============================================================================
// Error Conversion Tests
// ============================================================================

#[test]
fn test_server_error_from_toadstool_error() {
    let toadstool_error = ToadStoolError::runtime("Test runtime error");
    let server_error: ServerError = toadstool_error.into();

    let error_string = server_error.to_string();
    // Runtime errors should be mapped to Execution errors
    assert!(error_string.contains("Execution failed"));
    assert!(error_string.contains("Test runtime error"));
}

#[test]
fn test_server_error_from_toadstool_config_error() {
    let toadstool_error = ToadStoolError::configuration("Invalid config value");
    let server_error: ServerError = toadstool_error.into();

    let error_string = server_error.to_string();
    // Config errors should be mapped to Configuration errors
    assert!(error_string.contains("Invalid configuration"));
    assert!(error_string.contains("Invalid config value"));
}

// ============================================================================
// ServerResult Tests
// ============================================================================

#[test]
fn test_server_result_ok() {
    let result: ServerResult<i32> = Ok(42);

    assert!(result.is_ok());
    if let Ok(val) = result {
        assert_eq!(val, 42);
    }
}

#[test]
fn test_server_result_err() {
    let result: ServerResult<i32> = Err(ServerError::Internal("Error".to_string()));

    assert!(result.is_err());
}

#[test]
fn test_server_result_map() {
    let result: ServerResult<i32> = Ok(10);
    let mapped = result.map(|x| x * 2);

    assert_eq!(mapped.unwrap(), 20);
}

#[test]
fn test_server_result_and_then() {
    let result: ServerResult<i32> = Ok(10);
    let chained = result.map(|x| x + 5);

    assert_eq!(chained.unwrap(), 15);
}

#[test]
fn test_server_result_or_else() {
    let result: ServerResult<i32> = Err(ServerError::Internal("Error".to_string()));
    let recovered: ServerResult<i32> = result.or(Ok(100));

    assert_eq!(recovered.unwrap(), 100);
}

// ============================================================================
// Error Debugging Tests
// ============================================================================

#[test]
fn test_server_error_debug_format() {
    let error = ServerError::Configuration("Test config error".to_string());
    let debug_string = format!("{error:?}");

    assert!(debug_string.contains("Configuration"));
    assert!(debug_string.contains("Test config error"));
}

#[test]
fn test_server_error_display_format() {
    let error = ServerError::Authentication("Bad credentials".to_string());
    let display_string = format!("{error}");

    assert!(display_string.contains("Authentication failed"));
    assert!(display_string.contains("Bad credentials"));
}

// ============================================================================
// Error Propagation Tests
// ============================================================================

fn function_that_returns_server_result() -> ServerResult<String> {
    Err(ServerError::Network("Test error".to_string()))
}

#[test]
fn test_error_propagation_with_question_mark() {
    fn calling_function() -> ServerResult<String> {
        let _value = function_that_returns_server_result()?;
        Ok("Success".to_string())
    }

    let result = calling_function();
    assert!(result.is_err());

    if let Err(ServerError::Network(msg)) = result {
        assert_eq!(msg, "Test error");
    } else {
        panic!("Expected Network error");
    }
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_server_error_empty_message() {
    let error = ServerError::Internal(String::new());
    let error_string = error.to_string();

    assert!(error_string.contains("Internal server error"));
}

#[test]
fn test_server_error_long_message() {
    let long_message = "x".repeat(1000);
    let error = ServerError::Internal(long_message.clone());
    let error_string = error.to_string();

    assert!(error_string.contains("Internal server error"));
    assert!(error_string.contains(&long_message));
}

#[test]
fn test_server_error_special_characters() {
    let error = ServerError::Internal("Error with special chars: @#$%^&*()".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("@#$%^&*()"));
}

#[test]
fn test_server_error_unicode() {
    let error = ServerError::Internal("错误: 服务器失败 🚨".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("错误"));
    assert!(error_string.contains("🚨"));
}

// ============================================================================
// Error Matching Tests
// ============================================================================

#[test]
fn test_match_server_error_variants() {
    let errors = vec![
        ServerError::Initialization("init".to_string()),
        ServerError::RuntimeEngine("runtime".to_string()),
        ServerError::ResourceExhaustion("resource".to_string()),
        ServerError::Authentication("auth".to_string()),
        ServerError::Authorization("authz".to_string()),
        ServerError::Configuration("config".to_string()),
        ServerError::Network("network".to_string()),
        ServerError::Execution("exec".to_string()),
        ServerError::NotFound("not found".to_string()),
        ServerError::Internal("internal".to_string()),
    ];

    for error in errors {
        match error {
            ServerError::Initialization(_) => {}
            ServerError::RuntimeEngine(_) => {}
            ServerError::ResourceExhaustion(_) => {}
            ServerError::Authentication(_) => {}
            ServerError::Authorization(_) => {}
            ServerError::Configuration(_) => {}
            ServerError::Network(_) => {}
            ServerError::Execution(_) => {}
            ServerError::NotFound(_) => {}
            ServerError::Internal(_) => {}
        }
    }
}

// ============================================================================
// Error Context Tests
// ============================================================================

#[test]
fn test_server_error_with_context() {
    let original_error = "File not found";
    let error = ServerError::Internal(format!("Context: {original_error}"));
    let error_string = error.to_string();

    assert!(error_string.contains("Context"));
    assert!(error_string.contains("File not found"));
}

#[test]
fn test_server_error_chained_context() {
    let root_cause = "Database connection failed";
    let context1 = format!("Failed to load user: {root_cause}");
    let error = ServerError::Internal(format!("Request failed: {context1}"));
    let error_string = error.to_string();

    assert!(error_string.contains("Request failed"));
    assert!(error_string.contains("Failed to load user"));
    assert!(error_string.contains("Database connection failed"));
}
