// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for client error types
//!
//! Week 14 Day 4: Client Error Tests
//! Target: Achieve complete coverage of client/error.rs

use std::fmt::Write;
use toadstool_client::{ClientError, ClientResult};

// =============================================================================
// Error Creation & Display Tests
// =============================================================================

#[test]
fn test_client_error_authentication() {
    let error = ClientError::Authentication("Invalid credentials".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("Authentication failed"));
    assert!(error_string.contains("Invalid credentials"));
}

#[test]
fn test_client_error_authentication_token_expired() {
    let error = ClientError::Authentication("Token expired".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("Token expired"));
}

#[test]
fn test_client_error_configuration() {
    let error = ClientError::Configuration("Missing API endpoint".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("Invalid configuration"));
    assert!(error_string.contains("Missing API endpoint"));
}

#[test]
fn test_client_error_configuration_invalid_url() {
    let error = ClientError::Configuration("Invalid URL format".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("Invalid URL format"));
}

#[test]
fn test_client_error_server() {
    let error = ClientError::Server("Internal server error".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("Server error"));
    assert!(error_string.contains("Internal server error"));
}

#[test]
fn test_client_error_server_503() {
    let error = ClientError::Server("Service unavailable (503)".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("Service unavailable"));
}

#[test]
fn test_client_error_timeout() {
    let error = ClientError::Timeout("Request took too long".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("Timeout"));
    assert!(error_string.contains("Request took too long"));
}

#[test]
fn test_client_error_timeout_with_duration() {
    let error = ClientError::Timeout("Exceeded 30s timeout".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("30s"));
}

// =============================================================================
// Error Conversion Tests (From trait)
// =============================================================================

#[test]
fn test_client_error_from_serde_json() {
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let client_error: ClientError = json_error.into();
    let error_string = client_error.to_string();
    assert!(error_string.contains("Serialization error"));
}

#[test]
fn test_client_error_from_url_parse() {
    let url_error = url::Url::parse("not a valid url").unwrap_err();
    let client_error: ClientError = url_error.into();
    let error_string = client_error.to_string();
    assert!(error_string.contains("URL parse error"));
}

// =============================================================================
// Debug Formatting Tests
// =============================================================================

#[test]
fn test_client_error_debug_format() {
    let error = ClientError::Authentication("test".to_string());
    let debug_string = format!("{:?}", error);
    assert!(!debug_string.is_empty());
}

#[test]
fn test_all_error_variants_debug() {
    let errors = vec![
        ClientError::Authentication("auth error".to_string()),
        ClientError::Configuration("config error".to_string()),
        ClientError::Server("server error".to_string()),
        ClientError::Timeout("timeout error".to_string()),
    ];

    for error in errors {
        let debug_string = format!("{:?}", error);
        assert!(!debug_string.is_empty());
    }
}

// =============================================================================
// ClientResult Type Alias Tests
// =============================================================================

#[test]
fn test_client_result_ok() {
    let result: ClientResult<String> = Ok("success".to_string());
    assert!(result.is_ok());
    if let Ok(value) = result {
        assert_eq!(value, "success");
    }
}

#[test]
fn test_client_result_err() {
    let result: ClientResult<String> = Err(ClientError::Server("failed".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_client_result_with_authentication_error() {
    let result: ClientResult<i32> = Err(ClientError::Authentication("unauthorized".to_string()));
    assert!(result.is_err());

    match result {
        Err(ClientError::Authentication(msg)) => assert_eq!(msg, "unauthorized"),
        _ => panic!("Expected Authentication error"),
    }
}

#[test]
fn test_client_result_with_timeout_error() {
    let result: ClientResult<Vec<u8>> = Err(ClientError::Timeout("30s exceeded".to_string()));
    assert!(result.is_err());
}

// =============================================================================
// Error Message Content Tests
// =============================================================================

#[test]
fn test_error_messages_are_descriptive() {
    let errors = vec![
        (
            ClientError::Authentication("test".to_string()),
            "Authentication failed",
        ),
        (
            ClientError::Configuration("test".to_string()),
            "Invalid configuration",
        ),
        (ClientError::Server("test".to_string()), "Server error"),
        (ClientError::Timeout("test".to_string()), "Timeout"),
    ];

    for (error, expected_prefix) in errors {
        let msg = error.to_string();
        assert!(
            msg.contains(expected_prefix),
            "Error message '{}' should contain '{}'",
            msg,
            expected_prefix
        );
    }
}

#[test]
fn test_error_with_special_characters() {
    let error = ClientError::Server("Error: \"quoted\" and 'single' quotes".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("quoted"));
    assert!(error_string.contains("single"));
}

#[test]
fn test_error_with_unicode() {
    let error = ClientError::Configuration("错误信息 🚀 test".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("错误信息"));
    assert!(error_string.contains("🚀"));
}

#[test]
fn test_error_with_newlines() {
    let error = ClientError::Server("Line 1\nLine 2\nLine 3".to_string());
    let error_string = error.to_string();
    assert!(error_string.contains("Line 1"));
    assert!(error_string.contains("Line 2"));
}

// =============================================================================
// Error Pattern Matching Tests
// =============================================================================

#[test]
fn test_match_all_error_variants() {
    let errors = vec![
        ClientError::Authentication("auth".to_string()),
        ClientError::Configuration("config".to_string()),
        ClientError::Server("server".to_string()),
        ClientError::Timeout("timeout".to_string()),
    ];

    for error in errors {
        match error {
            ClientError::Authentication(_) => {}
            ClientError::Configuration(_) => {}
            ClientError::Server(_) => {}
            ClientError::Timeout(_) => {}
            ClientError::Http(_) => {}
            ClientError::Serialization(_) => {}
            ClientError::UrlParse(_) => {}
            ClientError::Io(_) => {}
        }
    }
}

// =============================================================================
// Error Chaining / Cause Tests
// =============================================================================

#[test]
fn test_error_source_for_serialization() {
    let json_error = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
    let client_error: ClientError = json_error.into();

    // Error should be a ClientError::Serialization
    match client_error {
        ClientError::Serialization(_) => {}
        _ => panic!("Expected Serialization error"),
    }
}

#[test]
fn test_error_source_for_url_parse() {
    let url_error = url::Url::parse(":::invalid:::").unwrap_err();
    let client_error: ClientError = url_error.into();

    match client_error {
        ClientError::UrlParse(_) => {}
        _ => panic!("Expected UrlParse error"),
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_error_in_result_chain() {
    fn might_fail() -> ClientResult<String> {
        Err(ClientError::Server("failed".to_string()))
    }

    fn call_might_fail() -> ClientResult<String> {
        might_fail()?;
        Ok("success".to_string())
    }

    let result = call_might_fail();
    assert!(result.is_err());
}

#[test]
fn test_error_with_question_mark_operator() {
    fn parse_json() -> ClientResult<serde_json::Value> {
        let value = serde_json::from_str("{\"key\": \"value\"}")?;
        Ok(value)
    }

    let result = parse_json();
    assert!(result.is_ok());
}

#[test]
fn test_error_with_question_mark_url_parse() {
    fn parse_url(url_str: &str) -> ClientResult<url::Url> {
        let url = url::Url::parse(url_str)?;
        Ok(url)
    }

    let result = parse_url("https://example.com");
    assert!(result.is_ok());

    let result = parse_url("not a url");
    assert!(result.is_err());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_error_with_very_long_message() {
    let long_message = "error ".repeat(1000);
    let error = ClientError::Server(long_message.clone());
    let error_string = error.to_string();
    assert!(error_string.contains(&long_message));
}

#[test]
fn test_error_display_formatting() {
    let error = ClientError::Timeout("request timeout".to_string());
    let mut buffer = String::new();
    write!(&mut buffer, "{}", error).unwrap();
    assert!(buffer.contains("Timeout"));
    assert!(buffer.contains("request timeout"));
}
