// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for execution module
//!
//! Sprint 19: execution.rs coverage → 60%+
//! Target: Core execution types and interfaces
//! Estimated: ~40-50 tests

#![allow(clippy::field_reassign_with_default)]

use std::collections::HashMap;
use std::time::Duration;
use toadstool::execution::*;

// ============================================================================
// ExecutionStatus Tests
// ============================================================================

#[test]
fn test_execution_status_variants() {
    let success = ExecutionStatus::Success;
    let failed = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("Test error"),
    };
    let cancelled = ExecutionStatus::Cancelled;
    let timed_out = ExecutionStatus::TimedOut;
    let running = ExecutionStatus::Running;
    let pending = ExecutionStatus::Pending;

    assert_eq!(success, ExecutionStatus::Success);
    assert_eq!(cancelled, ExecutionStatus::Cancelled);
    assert_eq!(timed_out, ExecutionStatus::TimedOut);
    assert_eq!(running, ExecutionStatus::Running);
    assert_eq!(pending, ExecutionStatus::Pending);

    match failed {
        ExecutionStatus::Failed { error } => assert_eq!(error, "Test error"),
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_execution_status_equality() {
    assert_eq!(ExecutionStatus::Success, ExecutionStatus::Success);
    assert_ne!(ExecutionStatus::Success, ExecutionStatus::Pending);
}

#[test]
fn test_execution_status_clone() {
    let status = ExecutionStatus::Running;
    let cloned = status.clone();

    assert_eq!(cloned, status);
}

#[test]
fn test_execution_status_debug() {
    let status = ExecutionStatus::Success;
    let debug = format!("{:?}", status);

    assert!(!debug.is_empty());
}

#[test]
fn test_execution_status_serialization() {
    let status = ExecutionStatus::Success;
    let json = serde_json::to_string(&status);

    assert!(json.is_ok());
}

// ============================================================================
// RuntimeType Tests
// ============================================================================

#[test]
fn test_runtime_type_variants() {
    let native = RuntimeType::Native;
    let wasm = RuntimeType::Wasm;
    let container = RuntimeType::Container;
    let gpu = RuntimeType::Gpu;
    let python = RuntimeType::Python;
    let custom = RuntimeType::from("MyRuntime");

    assert_eq!(native, RuntimeType::Native);
    assert_eq!(wasm, RuntimeType::Wasm);
    assert_eq!(container, RuntimeType::Container);
    assert_eq!(gpu, RuntimeType::Gpu);
    assert_eq!(python, RuntimeType::Python);
    assert_eq!(custom, RuntimeType::from("MyRuntime"));
}

#[test]
fn test_runtime_type_equality() {
    assert_eq!(RuntimeType::Native, RuntimeType::Native);
    assert_ne!(RuntimeType::Native, RuntimeType::Wasm);
}

#[test]
fn test_runtime_type_clone() {
    let runtime = RuntimeType::Native;
    let cloned = runtime.clone();

    assert_eq!(cloned, runtime);
}

#[test]
fn test_runtime_type_debug() {
    let runtime = RuntimeType::Wasm;
    let debug = format!("{:?}", runtime);

    assert!(!debug.is_empty());
}

#[test]
fn test_runtime_type_serialization() {
    let runtime = RuntimeType::Container;
    let json = serde_json::to_string(&runtime);

    assert!(json.is_ok());
}

#[test]
fn test_runtime_type_custom() {
    let custom1 = RuntimeType::from("Runtime1");
    let custom2 = RuntimeType::from("Runtime1");
    let custom3 = RuntimeType::from("Runtime2");

    assert_eq!(custom1, custom2);
    assert_ne!(custom1, custom3);
}

// ============================================================================
// ExecutionInput Tests
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
    let input = ExecutionInput {
        data: bytes::Bytes::from(vec![1, 2, 3, 4]),
        format: Some("binary".to_string()),
        metadata: HashMap::new(),
    };

    assert_eq!(input.data, vec![1, 2, 3, 4]);
    assert_eq!(input.format, Some("binary".to_string()));
}

#[test]
fn test_execution_input_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("key1".to_string(), "value1".to_string());

    let input = ExecutionInput {
        data: bytes::Bytes::new(),
        format: None,
        metadata,
    };

    assert_eq!(input.metadata.get("key1"), Some(&"value1".to_string()));
}

#[test]
fn test_execution_input_clone() {
    let input = ExecutionInput::default();
    let cloned = input.clone();

    assert_eq!(cloned.data, input.data);
}

#[test]
fn test_execution_input_debug() {
    let input = ExecutionInput::default();
    let debug = format!("{:?}", input);

    assert!(!debug.is_empty());
}

#[test]
fn test_execution_input_serialization() {
    let input = ExecutionInput::default();
    let json = serde_json::to_string(&input);

    assert!(json.is_ok());
}

// ============================================================================
// ExecutionOutput Tests
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
        stdout: Some("Hello World".to_string()),
        stderr: None,
        exit_code: Some(0),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert_eq!(output.stdout, Some("Hello World".to_string()));
    assert_eq!(output.exit_code, Some(0));
}

#[test]
fn test_execution_output_with_stderr() {
    let output = ExecutionOutput {
        data: bytes::Bytes::new(),
        stdout: None,
        stderr: Some("Error occurred".to_string()),
        exit_code: Some(1),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert_eq!(output.stderr, Some("Error occurred".to_string()));
    assert_eq!(output.exit_code, Some(1));
}

#[test]
fn test_execution_output_with_data() {
    let output = ExecutionOutput {
        data: bytes::Bytes::from(vec![5, 6, 7, 8]),
        stdout: None,
        stderr: None,
        exit_code: None,
        format: Some("binary".to_string()),
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert_eq!(output.data, vec![5, 6, 7, 8]);
    assert_eq!(output.format, Some("binary".to_string()));
}

#[test]
fn test_execution_output_clone() {
    let output = ExecutionOutput::default();
    let cloned = output.clone();

    assert_eq!(cloned.exit_code, output.exit_code);
}

#[test]
fn test_execution_output_debug() {
    let output = ExecutionOutput::default();
    let debug = format!("{:?}", output);

    assert!(!debug.is_empty());
}

#[test]
fn test_execution_output_serialization() {
    let output = ExecutionOutput::default();
    let json = serde_json::to_string(&output);

    assert!(json.is_ok());
}

// ============================================================================
// CallbackEvent Tests
// ============================================================================

#[test]
fn test_callback_event_variants() {
    let started = CallbackEvent::Started;
    let completed = CallbackEvent::Completed;
    let failed = CallbackEvent::Failed;
    let progress = CallbackEvent::Progress;

    assert!(matches!(started, CallbackEvent::Started));
    assert!(matches!(completed, CallbackEvent::Completed));
    assert!(matches!(failed, CallbackEvent::Failed));
    assert!(matches!(progress, CallbackEvent::Progress));
}

#[test]
fn test_callback_event_clone() {
    let event = CallbackEvent::Started;
    let cloned = event.clone();

    assert!(matches!(cloned, CallbackEvent::Started));
}

#[test]
fn test_callback_event_debug() {
    let event = CallbackEvent::Completed;
    let debug = format!("{:?}", event);

    assert!(!debug.is_empty());
}

#[test]
fn test_callback_event_serialization() {
    let event = CallbackEvent::Failed;
    let json = serde_json::to_string(&event);

    assert!(json.is_ok());
}

// ============================================================================
// CallbackConfig Tests
// ============================================================================

#[test]
fn test_callback_config_creation() {
    let config = CallbackConfig {
        url: "https://example.com/callback".to_string(),
        auth_token: Some("token123".to_string()),
        events: vec![CallbackEvent::Started, CallbackEvent::Completed],
    };

    assert_eq!(config.url, "https://example.com/callback");
    assert_eq!(config.auth_token, Some("token123".to_string()));
    assert_eq!(config.events.len(), 2);
}

#[test]
fn test_callback_config_without_auth() {
    let config = CallbackConfig {
        url: "https://example.com/callback".to_string(),
        auth_token: None,
        events: vec![CallbackEvent::Failed],
    };

    assert!(config.auth_token.is_none());
    assert_eq!(config.events.len(), 1);
}

#[test]
fn test_callback_config_clone() {
    let config = CallbackConfig {
        url: "https://example.com".to_string(),
        auth_token: None,
        events: vec![],
    };
    let cloned = config.clone();

    assert_eq!(cloned.url, config.url);
}

#[test]
fn test_callback_config_debug() {
    let config = CallbackConfig {
        url: "https://example.com".to_string(),
        auth_token: None,
        events: vec![],
    };
    let debug = format!("{:?}", config);

    assert!(!debug.is_empty());
}

#[test]
fn test_callback_config_serialization() {
    let config = CallbackConfig {
        url: "https://example.com".to_string(),
        auth_token: None,
        events: vec![CallbackEvent::Progress],
    };
    let json = serde_json::to_string(&config);

    assert!(json.is_ok());
}

// ============================================================================
// ExecutionRequest Tests
// ============================================================================

#[test]
fn test_execution_request_default() {
    let request = ExecutionRequest::default();

    assert_eq!(request.timeout, Some(Duration::from_secs(300)));
    assert!(request.environment.is_empty());
    assert!(request.runtime_hint.is_none());
}

#[test]
fn test_execution_request_with_runtime_hint() {
    let request = ExecutionRequest {
        runtime_hint: Some(RuntimeType::Wasm),
        ..Default::default()
    };

    assert_eq!(request.runtime_hint, Some(RuntimeType::Wasm));
}

#[test]
fn test_execution_request_with_environment() {
    let mut request = ExecutionRequest::default();
    request
        .environment
        .insert("KEY1".to_string(), "value1".to_string());

    assert_eq!(request.environment.get("KEY1"), Some(&"value1".to_string()));
}

#[test]
fn test_execution_request_with_timeout() {
    let request = ExecutionRequest {
        timeout: Some(Duration::from_secs(600)),
        ..Default::default()
    };

    assert_eq!(request.timeout, Some(Duration::from_secs(600)));
}

#[test]
fn test_execution_request_clone() {
    let request = ExecutionRequest::default();
    let cloned = request.clone();

    assert_eq!(cloned.timeout, request.timeout);
}

#[test]
fn test_execution_request_debug() {
    let request = ExecutionRequest::default();
    let debug = format!("{:?}", request);

    assert!(!debug.is_empty());
}

#[test]
fn test_execution_request_serialization() {
    let request = ExecutionRequest::default();
    let json = serde_json::to_string(&request);

    assert!(json.is_ok());
}

// ============================================================================
// ExecutionResponse Tests
// ============================================================================

#[test]
fn test_execution_response_default() {
    let response = ExecutionResponse::default();

    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.duration, Duration::from_secs(0));
    assert_eq!(response.runtime_used, RuntimeType::Native);
    assert!(response.warnings.is_empty());
}

#[test]
fn test_execution_response_with_failure() {
    let response = ExecutionResponse {
        status: ExecutionStatus::Failed {
            error: std::borrow::Cow::Borrowed("Test failure"),
        },
        ..Default::default()
    };

    match response.status {
        ExecutionStatus::Failed { error } => assert_eq!(error, "Test failure"),
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_execution_response_with_warnings() {
    let mut response = ExecutionResponse::default();
    response.warnings.push("Warning 1".to_string());
    response.warnings.push("Warning 2".to_string());

    assert_eq!(response.warnings.len(), 2);
}

#[test]
fn test_execution_response_with_duration() {
    let response = ExecutionResponse {
        duration: Duration::from_millis(1500),
        ..Default::default()
    };

    assert_eq!(response.duration, Duration::from_millis(1500));
}

#[test]
fn test_execution_response_clone() {
    let response = ExecutionResponse::default();
    let cloned = response.clone();

    assert_eq!(cloned.status, response.status);
}

#[test]
fn test_execution_response_debug() {
    let response = ExecutionResponse::default();
    let debug = format!("{:?}", response);

    assert!(!debug.is_empty());
}

#[test]
fn test_execution_response_serialization() {
    let response = ExecutionResponse::default();
    let json = serde_json::to_string(&response);

    assert!(json.is_ok());
}

// ============================================================================
// Serialization Round-trip Tests
// ============================================================================

#[test]
fn test_execution_status_round_trip() {
    let original = ExecutionStatus::Running;

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ExecutionStatus = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, original);
}

#[test]
fn test_runtime_type_round_trip() {
    let original = RuntimeType::Wasm;

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: RuntimeType = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, original);
}

#[test]
fn test_execution_input_round_trip() {
    let original = ExecutionInput::default();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ExecutionInput = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.data, original.data);
}

#[test]
fn test_execution_output_round_trip() {
    let original = ExecutionOutput::default();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ExecutionOutput = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.exit_code, original.exit_code);
}

#[test]
fn test_callback_event_round_trip() {
    let original = CallbackEvent::Progress;

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: CallbackEvent = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, CallbackEvent::Progress));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_execution_input_large_data() {
    let large_data = vec![0u8; 1024 * 1024]; // 1MB
    let input = ExecutionInput {
        data: large_data.clone().into(),
        format: None,
        metadata: HashMap::new(),
    };

    assert_eq!(input.data.len(), 1024 * 1024);
}

#[test]
fn test_execution_output_many_warnings() {
    let mut response = ExecutionResponse::default();
    for i in 0..100 {
        response.warnings.push(format!("Warning {}", i));
    }

    assert_eq!(response.warnings.len(), 100);
}

#[test]
fn test_execution_request_no_timeout() {
    let mut request = ExecutionRequest::default();
    request.timeout = None;

    assert!(request.timeout.is_none());
}

#[test]
fn test_callback_config_empty_events() {
    let config = CallbackConfig {
        url: "https://example.com".to_string(),
        auth_token: None,
        events: vec![],
    };

    assert!(config.events.is_empty());
}
