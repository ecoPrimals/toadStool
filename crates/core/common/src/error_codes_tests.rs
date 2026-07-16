// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from error_codes.rs (S335).

use super::error_codes::*;

#[test]
fn test_error_code_structure() {
    let code = codes::EXEC_RUNTIME_001;
    assert_eq!(code.code, "EXEC-RUNTIME-001");
    assert!(!code.message.is_empty());
    assert_eq!(code.category, ErrorCategory::Execution);
    assert!(code.remediation.is_some());
}

#[test]
fn test_error_with_context() {
    let code = codes::CONFIG_PARSE_001;
    let error = code.into_error_with_context("Invalid YAML at line 42");
    assert!(error.contains("CONFIG-PARSE-001"));
    assert!(error.contains("Invalid YAML at line 42"));
}

#[test]
fn test_category_strings() {
    assert_eq!(codes::EXEC_RUNTIME_001.category_str(), "execution");
    assert_eq!(codes::CONFIG_PARSE_001.category_str(), "configuration");
    assert_eq!(codes::RESOURCE_ALLOC_001.category_str(), "resource");
    assert_eq!(codes::SECURITY_AUTH_001.category_str(), "security");
}

#[test]
fn test_all_error_codes_unique() {
    use std::collections::HashSet;
    let mut seen = HashSet::new();

    // Add all codes (this is a sample, extend for all codes)
    let all_codes = vec![
        codes::EXEC_RUNTIME_001.code,
        codes::EXEC_TIMEOUT_001.code,
        codes::CONFIG_PARSE_001.code,
        codes::RESOURCE_ALLOC_001.code,
        codes::SECURITY_AUTH_001.code,
        codes::NETWORK_CONNECT_001.code,
        codes::SYSTEM_IO_001.code,
    ];

    for code in all_codes {
        assert!(seen.insert(code), "Duplicate error code: {code}");
    }
}

#[test]
fn test_serialization() {
    let code = codes::EXEC_RUNTIME_001;
    let json = serde_json::to_string(&code).unwrap();
    assert!(json.contains("EXEC-RUNTIME-001"));
    assert!(json.contains("execution"));
}

#[test]
fn test_zero_copy_error_message() {
    let code = codes::CONFIG_PARSE_001;
    let msg = code.to_error_message();

    // Should be borrowed (zero-copy)
    assert!(matches!(msg, std::borrow::Cow::Borrowed(_)));
    assert_eq!(msg.as_ref(), "Failed to parse configuration file");
}

#[test]
fn test_zero_copy_error_message_with_context_empty() {
    let code = codes::CONFIG_PARSE_001;
    let msg = code.to_error_message_with_context("");

    // Should be borrowed when context is empty (zero-copy)
    assert!(matches!(msg, std::borrow::Cow::Borrowed(_)));
}

#[test]
fn test_zero_copy_error_message_with_context_present() {
    let code = codes::CONFIG_PARSE_001;
    let msg = code.to_error_message_with_context("line 42");

    // Should be owned when context is present (allocation)
    assert!(matches!(msg, std::borrow::Cow::Owned(_)));
    assert!(msg.contains("line 42"));
    assert!(msg.contains("CONFIG-PARSE-001"));
}

#[test]
fn test_full_message_format() {
    let code = codes::EXEC_RUNTIME_001;
    let msg = code.full_message();

    assert_eq!(
        msg,
        "EXEC-RUNTIME-001: Runtime engine initialization failed"
    );
}
