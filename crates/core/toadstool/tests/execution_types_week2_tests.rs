// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::all)]
// Copyright (C) 2025 ecoPrimals

//! Comprehensive tests for execution.rs types.

use std::collections::HashMap;
use std::time::Duration;
use toadstool::RuntimeMetrics;
use toadstool::execution::*;
use uuid::Uuid;

// ============================================================================
// ExecutionStatus Tests
// ============================================================================

#[test]
fn test_execution_status_success() {
    let status = ExecutionStatus::Success;
    assert_eq!(status, ExecutionStatus::Success);
}

#[test]
fn test_execution_status_failed() {
    let status = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("Test error"),
    };

    match status {
        ExecutionStatus::Failed { error } => assert_eq!(error, "Test error"),
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_execution_status_cancelled() {
    let status = ExecutionStatus::Cancelled;
    assert_eq!(status, ExecutionStatus::Cancelled);
}

#[test]
fn test_execution_status_timed_out() {
    let status = ExecutionStatus::TimedOut;
    assert_eq!(status, ExecutionStatus::TimedOut);
}

#[test]
fn test_execution_status_running() {
    let status = ExecutionStatus::Running;
    assert_eq!(status, ExecutionStatus::Running);
}

#[test]
fn test_execution_status_pending() {
    let status = ExecutionStatus::Pending;
    assert_eq!(status, ExecutionStatus::Pending);
}

#[test]
fn test_execution_status_equality() {
    assert_eq!(ExecutionStatus::Success, ExecutionStatus::Success);
    assert_eq!(ExecutionStatus::Cancelled, ExecutionStatus::Cancelled);
    assert_ne!(ExecutionStatus::Success, ExecutionStatus::Cancelled);

    let failed1 = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("error1"),
    };
    let failed2 = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("error1"),
    };
    let failed3 = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("error2"),
    };

    assert_eq!(failed1, failed2);
    assert_ne!(failed1, failed3);
}

#[test]
fn test_execution_status_clone() {
    let status = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("test"),
    };
    let cloned = status.clone();
    assert_eq!(status, cloned);
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
    let custom = RuntimeType::from("tensorflow");

    assert!(matches!(native, RuntimeType::Native));
    assert!(matches!(wasm, RuntimeType::Wasm));
    assert!(matches!(container, RuntimeType::Container));
    assert!(matches!(gpu, RuntimeType::Gpu));
    assert!(matches!(python, RuntimeType::Python));
    assert!(matches!(custom, RuntimeType::Custom(_)));
}

#[test]
fn test_runtime_type_equality() {
    assert_eq!(RuntimeType::Native, RuntimeType::Native);
    assert_eq!(RuntimeType::Wasm, RuntimeType::Wasm);
    assert_ne!(RuntimeType::Native, RuntimeType::Wasm);

    let custom1 = RuntimeType::from("pytorch");
    let custom2 = RuntimeType::from("pytorch");
    let custom3 = RuntimeType::from("jax");

    assert_eq!(custom1, custom2);
    assert_ne!(custom1, custom3);
}

#[test]
fn test_runtime_type_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(RuntimeType::Native);
    set.insert(RuntimeType::Wasm);
    set.insert(RuntimeType::Container);
    set.insert(RuntimeType::Native); // Duplicate

    assert_eq!(set.len(), 3); // Native counted only once
    assert!(set.contains(&RuntimeType::Native));
    assert!(set.contains(&RuntimeType::Wasm));
}

#[test]
fn test_runtime_type_clone() {
    let rt = RuntimeType::from("custom_runtime");
    let cloned = rt.clone();
    assert_eq!(rt, cloned);
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
    let data = b"Hello, World!".to_vec();
    let input = ExecutionInput {
        data: data.clone().into(),
        format: Some("text/plain".to_string()),
        metadata: HashMap::new(),
    };

    assert_eq!(input.data, data);
    assert_eq!(input.format, Some("text/plain".to_string()));
}

#[test]
fn test_execution_input_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "api".to_string());
    metadata.insert("version".to_string(), "1.0".to_string());

    let input = ExecutionInput {
        data: bytes::Bytes::new(),
        format: Some("application/json".to_string()),
        metadata,
    };

    assert_eq!(input.metadata.len(), 2);
    assert_eq!(input.metadata.get("source"), Some(&"api".to_string()));
}

#[test]
fn test_execution_input_clone() {
    let input = ExecutionInput {
        data: bytes::Bytes::from_static(b"test"),
        format: Some("text".to_string()),
        metadata: HashMap::new(),
    };

    let cloned = input.clone();
    assert_eq!(input.data, cloned.data);
    assert_eq!(input.format, cloned.format);
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
fn test_execution_output_success() {
    let output = ExecutionOutput {
        data: bytes::Bytes::from_static(b"output data"),
        stdout: Some("Success message".to_string()),
        stderr: None,
        exit_code: Some(0),
        format: Some("text/plain".to_string()),
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert_eq!(output.data, &b"output data"[..]);
    assert_eq!(output.stdout, Some("Success message".to_string()));
    assert_eq!(output.exit_code, Some(0));
}

#[test]
fn test_execution_output_failure() {
    let output = ExecutionOutput {
        data: bytes::Bytes::new(),
        stdout: Some("Partial output".to_string()),
        stderr: Some("Error: something went wrong".to_string()),
        exit_code: Some(1),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert!(output.stderr.is_some());
    assert_eq!(output.exit_code, Some(1));
}

#[test]
fn test_execution_output_with_result_metadata() {
    let mut result = HashMap::new();
    result.insert("items_processed".to_string(), "1000".to_string());
    result.insert("duration_ms".to_string(), "523".to_string());

    let mut metadata = HashMap::new();
    metadata.insert("worker_id".to_string(), "worker-1".to_string());

    let output = ExecutionOutput {
        data: bytes::Bytes::new(),
        stdout: None,
        stderr: None,
        exit_code: Some(0),
        format: Some("application/json".to_string()),
        result,
        metadata,
    };

    assert_eq!(output.result.len(), 2);
    assert_eq!(output.metadata.len(), 1);
    assert_eq!(
        output.result.get("items_processed"),
        Some(&"1000".to_string())
    );
}

#[test]
fn test_execution_output_clone() {
    let output = ExecutionOutput {
        data: bytes::Bytes::from_static(b"test"),
        stdout: Some("output".to_string()),
        stderr: None,
        exit_code: Some(0),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    let cloned = output.clone();
    assert_eq!(output.data, cloned.data);
    assert_eq!(output.stdout, cloned.stdout);
    assert_eq!(output.exit_code, cloned.exit_code);
}

// ============================================================================
// ExecutionRequest Tests
// ============================================================================

#[test]
fn test_execution_request_default() {
    let request = ExecutionRequest::default();

    assert!(request.runtime_hint.is_none());
    assert_eq!(request.timeout, Some(Duration::from_mins(5)));
    assert!(request.environment.is_empty());
    assert!(request.callback_config.is_none());
}

#[test]
fn test_execution_request_with_timeout() {
    let request = ExecutionRequest {
        timeout: Some(Duration::from_mins(1)),
        ..Default::default()
    };

    assert_eq!(request.timeout, Some(Duration::from_mins(1)));
}

#[test]
fn test_execution_request_with_environment() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin".to_string());
    env.insert("HOME".to_string(), "/home/user".to_string());

    let request = ExecutionRequest {
        environment: env.clone(),
        ..Default::default()
    };

    assert_eq!(request.environment.len(), 2);
    assert_eq!(
        request.environment.get("PATH"),
        Some(&"/usr/bin".to_string())
    );
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
fn test_execution_request_clone() {
    let request = ExecutionRequest::default();
    let cloned = request.clone();

    assert_eq!(request.execution_id, cloned.execution_id);
    assert_eq!(request.timeout, cloned.timeout);
}

// ============================================================================
// ExecutionResponse Tests
// ============================================================================

#[test]
fn test_execution_response_default() {
    let response = ExecutionResponse::default();

    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.runtime_used, RuntimeType::Native);
    assert_eq!(response.duration, Duration::from_secs(0));
    assert!(response.warnings.is_empty());
}

#[test]
fn test_execution_response_success() {
    let response = ExecutionResponse {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Success,
        output: ExecutionOutput::default(),
        metrics: RuntimeMetrics::default(),
        duration: Duration::from_millis(123),
        runtime_used: RuntimeType::Wasm,
        warnings: vec![],
    };

    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.runtime_used, RuntimeType::Wasm);
    assert_eq!(response.duration, Duration::from_millis(123));
}

#[test]
fn test_execution_response_with_warnings() {
    let warnings = vec![
        "Warning: deprecated API used".to_string(),
        "Warning: high memory usage".to_string(),
    ];

    let response = ExecutionResponse {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Success,
        output: ExecutionOutput::default(),
        metrics: RuntimeMetrics::default(),
        duration: Duration::from_secs(1),
        runtime_used: RuntimeType::Native,
        warnings: warnings.clone(),
    };

    assert_eq!(response.warnings.len(), 2);
    assert_eq!(response.warnings, warnings);
}

#[test]
fn test_execution_response_failed() {
    let response = ExecutionResponse {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Failed {
            error: std::borrow::Cow::Borrowed("Out of memory"),
        },
        output: ExecutionOutput::default(),
        metrics: RuntimeMetrics::default(),
        duration: Duration::from_secs(5),
        runtime_used: RuntimeType::Container,
        warnings: vec![],
    };

    match response.status {
        ExecutionStatus::Failed { error } => assert_eq!(error, "Out of memory"),
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_execution_response_clone() {
    let response = ExecutionResponse::default();
    let cloned = response.clone();

    assert_eq!(response.execution_id, cloned.execution_id);
    assert_eq!(response.status, cloned.status);
    assert_eq!(response.runtime_used, cloned.runtime_used);
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
    let cloned = event;

    assert!(matches!(cloned, CallbackEvent::Started));
}

// ============================================================================
// CallbackConfig Tests
// ============================================================================

#[test]
fn test_callback_config_basic() {
    let config = CallbackConfig {
        url: "https://api.example.com/callback".to_string(),
        auth_token: None,
        events: vec![CallbackEvent::Completed],
    };

    assert_eq!(config.url, "https://api.example.com/callback");
    assert!(config.auth_token.is_none());
    assert_eq!(config.events.len(), 1);
}

#[test]
fn test_callback_config_with_auth() {
    let config = CallbackConfig {
        url: "https://secure.example.com/webhook".to_string(),
        auth_token: Some("secret-token-123".to_string()),
        events: vec![
            CallbackEvent::Started,
            CallbackEvent::Completed,
            CallbackEvent::Failed,
        ],
    };

    assert_eq!(config.auth_token, Some("secret-token-123".to_string()));
    assert_eq!(config.events.len(), 3);
}

#[test]
fn test_callback_config_all_events() {
    let config = CallbackConfig {
        url: "https://api.example.com".to_string(),
        auth_token: Some("token".to_string()),
        events: vec![
            CallbackEvent::Started,
            CallbackEvent::Completed,
            CallbackEvent::Failed,
            CallbackEvent::Progress,
        ],
    };

    assert_eq!(config.events.len(), 4);
}

#[test]
fn test_callback_config_clone() {
    let config = CallbackConfig {
        url: "https://test.com".to_string(),
        auth_token: Some("token".to_string()),
        events: vec![CallbackEvent::Started],
    };

    let cloned = config.clone();
    assert_eq!(config.url, cloned.url);
    assert_eq!(config.auth_token, cloned.auth_token);
}

// ============================================================================
// RuntimeConfig Tests
// ============================================================================

#[test]
fn test_runtime_config_default() {
    let config = RuntimeConfig::default();

    assert!(config.settings.is_empty());
    assert!(config.resource_limits.is_none());
    assert!(config.security_settings.is_none());
    assert!(config.logging.is_none());
}

#[test]
fn test_runtime_config_with_settings() {
    let mut settings = HashMap::new();
    settings.insert("max_memory".to_string(), serde_json::json!("4GB"));
    settings.insert("enable_jit".to_string(), serde_json::json!(true));

    let config = RuntimeConfig {
        settings,
        resource_limits: None,
        security_settings: None,
        logging: None,
    };

    assert_eq!(config.settings.len(), 2);
    assert_eq!(
        config.settings.get("enable_jit"),
        Some(&serde_json::json!(true))
    );
}

#[test]
fn test_runtime_config_with_logging() {
    let logging = LoggingConfig {
        level: "debug".to_string(),
        format: "json".to_string(),
        destination: "stdout".to_string(),
    };

    let config = RuntimeConfig {
        settings: HashMap::new(),
        resource_limits: None,
        security_settings: None,
        logging: Some(logging),
    };

    assert!(config.logging.is_some());
    assert_eq!(config.logging.as_ref().unwrap().level, "debug");
}

#[test]
fn test_runtime_config_clone() {
    let config = RuntimeConfig::default();
    let cloned = config.clone();

    assert_eq!(config.settings.len(), cloned.settings.len());
}

// ============================================================================
// LoggingConfig Tests
// ============================================================================

#[test]
fn test_logging_config_basic() {
    let config = LoggingConfig {
        level: "info".to_string(),
        format: "text".to_string(),
        destination: "file".to_string(),
    };

    assert_eq!(config.level, "info");
    assert_eq!(config.format, "text");
    assert_eq!(config.destination, "file");
}

#[test]
fn test_logging_config_levels() {
    let levels = vec!["trace", "debug", "info", "warn", "error"];

    for level in levels {
        let config = LoggingConfig {
            level: level.to_string(),
            format: "json".to_string(),
            destination: "stdout".to_string(),
        };
        assert_eq!(config.level, level);
    }
}

#[test]
fn test_logging_config_formats() {
    let formats = vec!["json", "text", "structured"];

    for format in formats {
        let config = LoggingConfig {
            level: "info".to_string(),
            format: format.to_string(),
            destination: "stdout".to_string(),
        };
        assert_eq!(config.format, format);
    }
}

#[test]
fn test_logging_config_clone() {
    let config = LoggingConfig {
        level: "debug".to_string(),
        format: "json".to_string(),
        destination: "stderr".to_string(),
    };

    let cloned = config.clone();
    assert_eq!(config.level, cloned.level);
    assert_eq!(config.format, cloned.format);
    assert_eq!(config.destination, cloned.destination);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_execution_request_to_response_workflow() {
    // Create a request
    let request = ExecutionRequest::default();
    let request_id = request.execution_id;

    // Simulate successful execution
    let response = ExecutionResponse {
        execution_id: request_id,
        status: ExecutionStatus::Success,
        output: ExecutionOutput {
            data: bytes::Bytes::from_static(b"result"),
            stdout: Some("Execution completed".to_string()),
            stderr: None,
            exit_code: Some(0),
            format: Some("text/plain".to_string()),
            result: HashMap::new(),
            metadata: HashMap::new(),
        },
        metrics: RuntimeMetrics::default(),
        duration: Duration::from_millis(250),
        runtime_used: RuntimeType::Native,
        warnings: vec![],
    };

    assert_eq!(response.execution_id, request_id);
    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.output.exit_code, Some(0));
}

#[test]
fn test_execution_with_callback_workflow() {
    let callback = CallbackConfig {
        url: "https://api.example.com/hooks".to_string(),
        auth_token: Some("bearer-token".to_string()),
        events: vec![CallbackEvent::Started, CallbackEvent::Completed],
    };

    let mut request = ExecutionRequest::default();
    request.callback_config = Some(callback);

    assert!(request.callback_config.is_some());
    let config = request.callback_config.unwrap();
    assert_eq!(config.events.len(), 2);
    assert!(config.auth_token.is_some());
}

#[test]
fn test_runtime_types_collection() {
    let runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Gpu,
        RuntimeType::Python,
        RuntimeType::from("julia"),
    ];

    assert_eq!(runtimes.len(), 6);

    for runtime in &runtimes {
        match runtime {
            RuntimeType::Native
            | RuntimeType::Wasm
            | RuntimeType::Container
            | RuntimeType::Gpu
            | RuntimeType::Python
            | RuntimeType::Custom(_) => { /* Valid */ }
        }
    }
}

#[test]
fn test_execution_status_progression() {
    let statuses = vec![
        ExecutionStatus::Pending,
        ExecutionStatus::Running,
        ExecutionStatus::Success,
    ];

    // Verify we can track execution progression
    assert_eq!(statuses.len(), 3);
    assert_eq!(statuses[0], ExecutionStatus::Pending);
    assert_eq!(statuses[1], ExecutionStatus::Running);
    assert_eq!(statuses[2], ExecutionStatus::Success);
}
