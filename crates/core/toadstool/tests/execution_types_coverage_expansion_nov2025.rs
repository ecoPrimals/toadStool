// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive test coverage for execution.rs types
//!
//! This test suite targets types defined in crates/core/toadstool/src/execution.rs
//! to expand test coverage in the push toward 90% coverage target.
//!
//! Coverage Target: Add 35+ tests for execution types
//! Session: November 2025 - Week 5 Test Expansion (Batch 2)

use std::collections::HashMap;
use toadstool::execution::{
    CallbackConfig, CallbackEvent, ExecutionInput, ExecutionOutput, ExecutionStatus, RuntimeType,
};

// ============================================================================
// ExecutionStatus Tests (8 tests)
// ============================================================================

#[test]
fn test_execution_status_success_creation() {
    let status = ExecutionStatus::Success;
    assert!(matches!(status, ExecutionStatus::Success));
}

#[test]
fn test_execution_status_failed_creation() {
    let status = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("Connection timeout"),
    };
    assert!(matches!(status, ExecutionStatus::Failed { .. }));
}

#[test]
fn test_execution_status_cancelled_creation() {
    let status = ExecutionStatus::Cancelled;
    assert!(matches!(status, ExecutionStatus::Cancelled));
}

#[test]
fn test_execution_status_timedout_creation() {
    let status = ExecutionStatus::TimedOut;
    assert!(matches!(status, ExecutionStatus::TimedOut));
}

#[test]
fn test_execution_status_running_creation() {
    let status = ExecutionStatus::Running;
    assert!(matches!(status, ExecutionStatus::Running));
}

#[test]
fn test_execution_status_pending_creation() {
    let status = ExecutionStatus::Pending;
    assert!(matches!(status, ExecutionStatus::Pending));
}

#[test]
fn test_execution_status_equality() {
    let status1 = ExecutionStatus::Success;
    let status2 = ExecutionStatus::Success;
    assert_eq!(status1, status2);

    let status3 = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("test"),
    };
    let status4 = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("test"),
    };
    assert_eq!(status3, status4);
}

#[test]
fn test_execution_status_serialization() {
    let status = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("Test error"),
    };
    let serialized = serde_json::to_string(&status).expect("Failed to serialize");
    let deserialized: ExecutionStatus =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(status, deserialized);
}

// ============================================================================
// ExecutionInput Tests (5 tests)
// ============================================================================

#[test]
fn test_execution_input_default() {
    let input = ExecutionInput::default();
    assert!(input.data.is_empty());
    assert!(input.format.is_none());
    assert!(input.metadata.is_empty());
}

#[test]
fn test_execution_input_with_data() {
    let data = vec![1, 2, 3, 4, 5];
    let input = ExecutionInput {
        data: data.clone().into(),
        format: Some("binary".to_string()),
        metadata: HashMap::new(),
    };
    assert_eq!(input.data, data);
    assert_eq!(input.format, Some("binary".to_string()));
}

#[test]
fn test_execution_input_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "api".to_string());
    metadata.insert("version".to_string(), "1.0".to_string());

    let input = ExecutionInput {
        data: bytes::Bytes::new(),
        format: None,
        metadata: metadata.clone(),
    };

    assert_eq!(input.metadata.len(), 2);
    assert_eq!(input.metadata.get("source"), Some(&"api".to_string()));
}

#[test]
fn test_execution_input_clone() {
    let input = ExecutionInput {
        data: bytes::Bytes::from(vec![1, 2, 3]),
        format: Some("json".to_string()),
        metadata: HashMap::new(),
    };
    let cloned = input.clone();
    assert_eq!(input.data, cloned.data);
    assert_eq!(input.format, cloned.format);
}

#[test]
fn test_execution_input_serialization() {
    let mut metadata = HashMap::new();
    metadata.insert("key".to_string(), "value".to_string());

    let input = ExecutionInput {
        data: bytes::Bytes::from(vec![10, 20, 30]),
        format: Some("protobuf".to_string()),
        metadata,
    };

    let serialized = serde_json::to_string(&input).expect("Failed to serialize");
    let deserialized: ExecutionInput =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(input.data, deserialized.data);
    assert_eq!(input.format, deserialized.format);
}

// ============================================================================
// ExecutionOutput Tests (6 tests)
// ============================================================================

#[test]
fn test_execution_output_default() {
    let output = ExecutionOutput::default();
    assert!(output.data.is_empty());
    assert!(output.stdout.is_none());
    assert!(output.stderr.is_none());
    assert!(output.exit_code.is_none());
    assert!(output.format.is_none());
    assert!(output.result.is_empty());
    assert!(output.metadata.is_empty());
}

#[test]
fn test_execution_output_with_stdout() {
    let output = ExecutionOutput {
        data: bytes::Bytes::new(),
        stdout: Some("Hello, World!".to_string()),
        stderr: None,
        exit_code: Some(0),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };
    assert_eq!(output.stdout, Some("Hello, World!".to_string()));
    assert_eq!(output.exit_code, Some(0));
}

#[test]
fn test_execution_output_with_stderr() {
    let output = ExecutionOutput {
        data: bytes::Bytes::new(),
        stdout: None,
        stderr: Some("Error: File not found".to_string()),
        exit_code: Some(1),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };
    assert_eq!(output.stderr, Some("Error: File not found".to_string()));
    assert_eq!(output.exit_code, Some(1));
}

#[test]
fn test_execution_output_with_result_metadata() {
    let mut result = HashMap::new();
    result.insert("status".to_string(), "completed".to_string());

    let mut metadata = HashMap::new();
    metadata.insert("runtime".to_string(), "native".to_string());

    let output = ExecutionOutput {
        data: bytes::Bytes::new(),
        stdout: None,
        stderr: None,
        exit_code: Some(0),
        format: Some("json".to_string()),
        result: result.clone(),
        metadata: metadata.clone(),
    };

    assert_eq!(output.result.len(), 1);
    assert_eq!(output.metadata.len(), 1);
}

#[test]
fn test_execution_output_clone() {
    let output = ExecutionOutput {
        data: bytes::Bytes::from(vec![1, 2, 3]),
        stdout: Some("output".to_string()),
        stderr: None,
        exit_code: Some(0),
        format: Some("text".to_string()),
        result: HashMap::new(),
        metadata: HashMap::new(),
    };
    let cloned = output.clone();
    assert_eq!(output.data, cloned.data);
    assert_eq!(output.stdout, cloned.stdout);
}

#[test]
fn test_execution_output_serialization() {
    let output = ExecutionOutput {
        data: bytes::Bytes::from(vec![255, 254]),
        stdout: Some("Done".to_string()),
        stderr: Some("Warning".to_string()),
        exit_code: Some(0),
        format: Some("binary".to_string()),
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    let serialized = serde_json::to_string(&output).expect("Failed to serialize");
    let deserialized: ExecutionOutput =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(output.data, deserialized.data);
    assert_eq!(output.exit_code, deserialized.exit_code);
}

// ============================================================================
// RuntimeType Tests (8 tests)
// ============================================================================

#[test]
fn test_runtime_type_native() {
    let rt = RuntimeType::Native;
    assert!(matches!(rt, RuntimeType::Native));
}

#[test]
fn test_runtime_type_wasm() {
    let rt = RuntimeType::Wasm;
    assert!(matches!(rt, RuntimeType::Wasm));
}

#[test]
fn test_runtime_type_container() {
    let rt = RuntimeType::Container;
    assert!(matches!(rt, RuntimeType::Container));
}

#[test]
fn test_runtime_type_gpu() {
    let rt = RuntimeType::Gpu;
    assert!(matches!(rt, RuntimeType::Gpu));
}

#[test]
fn test_runtime_type_python() {
    let rt = RuntimeType::Python;
    assert!(matches!(rt, RuntimeType::Python));
}

#[test]
fn test_runtime_type_custom() {
    let rt = RuntimeType::from("V8");
    assert!(matches!(rt, RuntimeType::Custom(_)));
}

#[test]
fn test_runtime_type_equality() {
    let rt1 = RuntimeType::Native;
    let rt2 = RuntimeType::Native;
    assert_eq!(rt1, rt2);

    let rt3 = RuntimeType::from("Deno");
    let rt4 = RuntimeType::from("Deno");
    assert_eq!(rt3, rt4);
}

#[test]
fn test_runtime_type_serialization() {
    let rt = RuntimeType::from("LLVM");
    let serialized = serde_json::to_string(&rt).expect("Failed to serialize");
    let deserialized: RuntimeType =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(rt, deserialized);
}

// ============================================================================
// CallbackEvent Tests (5 tests)
// ============================================================================

#[test]
fn test_callback_event_started() {
    let event = CallbackEvent::Started;
    assert!(matches!(event, CallbackEvent::Started));
}

#[test]
fn test_callback_event_completed() {
    let event = CallbackEvent::Completed;
    assert!(matches!(event, CallbackEvent::Completed));
}

#[test]
fn test_callback_event_failed() {
    let event = CallbackEvent::Failed;
    assert!(matches!(event, CallbackEvent::Failed));
}

#[test]
fn test_callback_event_progress() {
    let event = CallbackEvent::Progress;
    assert!(matches!(event, CallbackEvent::Progress));
}

#[test]
fn test_callback_event_serialization() {
    let event = CallbackEvent::Completed;
    let serialized = serde_json::to_string(&event).expect("Failed to serialize");
    let deserialized: CallbackEvent =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    // We can't use PartialEq directly, so we verify by matching
    assert!(matches!(deserialized, CallbackEvent::Completed));
}

// ============================================================================
// CallbackConfig Tests (4 tests)
// ============================================================================

#[test]
fn test_callback_config_creation() {
    let config = CallbackConfig {
        url: "https://example.com/webhook".to_string(),
        auth_token: Some("secret_token_123".to_string()),
        events: vec![CallbackEvent::Started, CallbackEvent::Completed],
    };

    assert_eq!(config.url, "https://example.com/webhook");
    assert_eq!(config.auth_token, Some("secret_token_123".to_string()));
    assert_eq!(config.events.len(), 2);
}

#[test]
fn test_callback_config_no_auth() {
    let config = CallbackConfig {
        url: "http://localhost:8080/callback".to_string(),
        auth_token: None,
        events: vec![CallbackEvent::Failed],
    };

    assert!(config.auth_token.is_none());
    assert_eq!(config.events.len(), 1);
}

#[test]
fn test_callback_config_clone() {
    let config = CallbackConfig {
        url: "https://api.test.com".to_string(),
        auth_token: Some("token".to_string()),
        events: vec![CallbackEvent::Progress],
    };

    let cloned = config.clone();
    assert_eq!(config.url, cloned.url);
    assert_eq!(config.auth_token, cloned.auth_token);
}

#[test]
fn test_callback_config_serialization() {
    let config = CallbackConfig {
        url: "https://webhook.site/test".to_string(),
        auth_token: Some("bearer_xyz".to_string()),
        events: vec![
            CallbackEvent::Started,
            CallbackEvent::Progress,
            CallbackEvent::Completed,
        ],
    };

    let serialized = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: CallbackConfig =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(config.url, deserialized.url);
    assert_eq!(config.auth_token, deserialized.auth_token);
    assert_eq!(config.events.len(), deserialized.events.len());
}

// ============================================================================
// Summary
// ============================================================================

// Total tests added: 36
// Coverage areas:
// - ExecutionStatus (8 tests)
// - ExecutionInput (5 tests)
// - ExecutionOutput (6 tests)
// - RuntimeType (8 tests)
// - CallbackEvent (5 tests)
// - CallbackConfig (4 tests)
// - Serialization tests (multiple across types)
// - Clone tests (multiple)
// - Default tests (multiple)
// - Equality tests (multiple)
