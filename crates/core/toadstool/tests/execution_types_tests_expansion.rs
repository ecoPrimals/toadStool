// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive expansion tests for execution types

use std::collections::HashMap;
use std::time::Duration;
use toadstool::*;
use uuid::Uuid;

// ============================================================================
// ExecutionStatus Advanced Tests
// ============================================================================

#[test]
fn test_execution_status_failed_empty_error() {
    let status = ExecutionStatus::Failed {
        error: String::new().into(),
    };

    match status {
        ExecutionStatus::Failed { error } => {
            assert!(error.is_empty());
        }
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_execution_status_failed_long_error() {
    let long_error = "a".repeat(10000);
    let status = ExecutionStatus::Failed {
        error: long_error.clone().into(),
    };

    match status {
        ExecutionStatus::Failed { error } => {
            assert_eq!(error.len(), 10000);
        }
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_execution_status_serialization_roundtrip() {
    let statuses = vec![
        ExecutionStatus::Success,
        ExecutionStatus::Failed {
            error: std::borrow::Cow::Borrowed("test error"),
        },
        ExecutionStatus::Cancelled,
        ExecutionStatus::TimedOut,
        ExecutionStatus::Running,
        ExecutionStatus::Pending,
    ];

    for status in statuses {
        let serialized = serde_json::to_string(&status).unwrap();
        let deserialized: ExecutionStatus = serde_json::from_str(&serialized).unwrap();
        assert_eq!(status, deserialized);
    }
}

#[test]
fn test_execution_status_debug_format() {
    let status = ExecutionStatus::Success;
    let debug_str = format!("{status:?}");
    assert!(debug_str.contains("Success"));
}

#[test]
fn test_execution_status_ordering() {
    let success = ExecutionStatus::Success;
    let failed = ExecutionStatus::Failed {
        error: std::borrow::Cow::Borrowed("error"),
    };

    // Just verify they can be compared
    assert_ne!(success, failed);
}

// ============================================================================
// ExecutionInput Advanced Tests
// ============================================================================

#[test]
fn test_execution_input_empty_format() {
    let input = ExecutionInput {
        data: bytes::Bytes::from(vec![1, 2, 3]),
        format: Some(String::new()),
        metadata: HashMap::new(),
    };

    assert_eq!(input.format, Some(String::new()));
}

#[test]
fn test_execution_input_various_formats() {
    let formats = vec!["json", "binary", "text", "protobuf", "msgpack", "yaml"];

    for format in formats {
        let input = ExecutionInput {
            data: bytes::Bytes::new(),
            format: Some(format.to_string()),
            metadata: HashMap::new(),
        };

        assert_eq!(input.format, Some(format.to_string()));
    }
}

#[test]
fn test_execution_input_many_metadata_entries() {
    let mut metadata = HashMap::new();
    for i in 0..1000 {
        metadata.insert(format!("key{i}"), format!("value{i}"));
    }

    let input = ExecutionInput {
        data: bytes::Bytes::new(),
        format: None,
        metadata: metadata.clone(),
    };

    assert_eq!(input.metadata.len(), 1000);
}

#[test]
fn test_execution_input_serialization_roundtrip() {
    let mut metadata = HashMap::new();
    metadata.insert("key".to_string(), "value".to_string());

    let input = ExecutionInput {
        data: bytes::Bytes::from(vec![1, 2, 3, 4, 5]),
        format: Some("json".to_string()),
        metadata,
    };

    let serialized = serde_json::to_string(&input).unwrap();
    let deserialized: ExecutionInput = serde_json::from_str(&serialized).unwrap();

    assert_eq!(input.data, deserialized.data);
    assert_eq!(input.format, deserialized.format);
}

#[test]
fn test_execution_input_debug_format() {
    let input = ExecutionInput::default();
    let debug_str = format!("{input:?}");
    assert!(debug_str.contains("ExecutionInput"));
}

#[test]
fn test_execution_input_zero_size_data() {
    let input = ExecutionInput {
        data: bytes::Bytes::new(),
        format: Some("empty".to_string()),
        metadata: HashMap::new(),
    };

    assert_eq!(input.data.len(), 0);
}

#[test]
fn test_execution_input_very_large_data() {
    let data = vec![0u8; 100 * 1024 * 1024]; // 100MB
    let input = ExecutionInput {
        data: data.into(),
        format: Some("binary".to_string()),
        metadata: HashMap::new(),
    };

    assert_eq!(input.data.len(), 100 * 1024 * 1024);
}

// ============================================================================
// ExecutionOutput Advanced Tests
// ============================================================================

#[test]
fn test_execution_output_empty_streams() {
    let output = ExecutionOutput {
        data: bytes::Bytes::new(),
        stdout: Some(String::new()),
        stderr: Some(String::new()),
        exit_code: Some(0),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert_eq!(output.stdout, Some(String::new()));
    assert_eq!(output.stderr, Some(String::new()));
}

#[test]
fn test_execution_output_large_stdout() {
    let large_stdout = "line\n".repeat(100_000);
    let output = ExecutionOutput {
        data: bytes::Bytes::new(),
        stdout: Some(large_stdout.clone()),
        stderr: None,
        exit_code: Some(0),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert!(output.stdout.unwrap().len() > 400_000);
}

#[test]
fn test_execution_output_negative_exit_codes() {
    let codes = vec![-1, -127, -255];

    for code in codes {
        let output = ExecutionOutput {
            data: bytes::Bytes::new(),
            stdout: None,
            stderr: None,
            exit_code: Some(code),
            format: None,
            result: HashMap::new(),
            metadata: HashMap::new(),
        };

        assert_eq!(output.exit_code, Some(code));
    }
}

#[test]
fn test_execution_output_serialization_roundtrip() {
    let mut result = HashMap::new();
    result.insert("status".to_string(), "ok".to_string());

    let output = ExecutionOutput {
        data: bytes::Bytes::from(vec![1, 2, 3]),
        stdout: Some("output".to_string()),
        stderr: Some("error".to_string()),
        exit_code: Some(42),
        format: Some("json".to_string()),
        result,
        metadata: HashMap::new(),
    };

    let serialized = serde_json::to_string(&output).unwrap();
    let deserialized: ExecutionOutput = serde_json::from_str(&serialized).unwrap();

    assert_eq!(output.exit_code, deserialized.exit_code);
}

#[test]
fn test_execution_output_debug_format() {
    let output = ExecutionOutput::default();
    let debug_str = format!("{output:?}");
    assert!(debug_str.contains("ExecutionOutput"));
}

#[test]
fn test_execution_output_both_streams_and_data() {
    let output = ExecutionOutput {
        data: bytes::Bytes::from(vec![1, 2, 3, 4, 5]),
        stdout: Some("Standard output".to_string()),
        stderr: Some("Standard error".to_string()),
        exit_code: Some(0),
        format: Some("mixed".to_string()),
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert!(!output.data.is_empty());
    assert!(output.stdout.is_some());
    assert!(output.stderr.is_some());
}

#[test]
fn test_execution_output_unicode_streams() {
    let output = ExecutionOutput {
        data: bytes::Bytes::new(),
        stdout: Some("Hello 世界 🌍".to_string()),
        stderr: Some("Error: été ñoño".to_string()),
        exit_code: Some(0),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert!(output.stdout.unwrap().contains("世界"));
    assert!(output.stderr.unwrap().contains("été"));
}

// ============================================================================
// CallbackConfig Advanced Tests
// ============================================================================

#[test]
fn test_callback_config_empty_url() {
    let config = CallbackConfig {
        url: String::new(),
        auth_token: None,
        events: vec![],
    };

    assert!(config.url.is_empty());
}

#[test]
fn test_callback_config_long_auth_token() {
    let long_token = "a".repeat(10000);
    let config = CallbackConfig {
        url: "https://example.com".to_string(),
        auth_token: Some(long_token.clone()),
        events: vec![],
    };

    assert_eq!(config.auth_token.unwrap().len(), 10000);
}

#[test]
fn test_callback_config_duplicate_events() {
    let config = CallbackConfig {
        url: "https://example.com".to_string(),
        auth_token: None,
        events: vec![
            CallbackEvent::Started,
            CallbackEvent::Started,
            CallbackEvent::Completed,
        ],
    };

    assert_eq!(config.events.len(), 3);
}

#[test]
fn test_callback_config_serialization_roundtrip() {
    let config = CallbackConfig {
        url: "https://example.com/callback".to_string(),
        auth_token: Some("token".to_string()),
        events: vec![CallbackEvent::Started, CallbackEvent::Completed],
    };

    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: CallbackConfig = serde_json::from_str(&serialized).unwrap();

    assert_eq!(config.url, deserialized.url);
    assert_eq!(config.events.len(), deserialized.events.len());
}

#[test]
fn test_callback_config_debug_format() {
    let config = CallbackConfig {
        url: "https://example.com".to_string(),
        auth_token: None,
        events: vec![],
    };

    let debug_str = format!("{config:?}");
    assert!(debug_str.contains("CallbackConfig"));
}

// ============================================================================
// CallbackEvent Advanced Tests
// ============================================================================

#[test]
fn test_callback_event_all_variants() {
    let events = [
        CallbackEvent::Started,
        CallbackEvent::Completed,
        CallbackEvent::Failed,
        CallbackEvent::Progress,
    ];

    assert_eq!(events.len(), 4);
}

#[test]
fn test_callback_event_serialization_roundtrip() {
    let events = vec![
        CallbackEvent::Started,
        CallbackEvent::Completed,
        CallbackEvent::Failed,
        CallbackEvent::Progress,
    ];

    for event in events {
        let serialized = serde_json::to_string(&event).unwrap();
        let _deserialized: CallbackEvent = serde_json::from_str(&serialized).unwrap();
        // Can't directly compare, but we can verify serialization works
        assert!(!serialized.is_empty());
    }
}

#[test]
fn test_callback_event_debug_format() {
    let event = CallbackEvent::Progress;
    let debug_str = format!("{event:?}");
    assert!(debug_str.contains("Progress"));
}

// ============================================================================
// ExecutionRequest Advanced Tests
// ============================================================================

#[test]
fn test_execution_request_nil_uuid() {
    let request = ExecutionRequest {
        execution_id: Uuid::nil(),
        ..Default::default()
    };

    assert_eq!(request.execution_id, Uuid::nil());
}

#[test]
fn test_execution_request_zero_timeout() {
    let request = ExecutionRequest {
        timeout: Some(Duration::from_secs(0)),
        ..Default::default()
    };

    assert_eq!(request.timeout, Some(Duration::from_secs(0)));
}

#[test]
fn test_execution_request_very_long_timeout() {
    let request = ExecutionRequest {
        timeout: Some(Duration::from_secs(86400 * 365)), // 1 year
        ..Default::default()
    };

    assert_eq!(request.timeout, Some(Duration::from_secs(86400 * 365)));
}

#[test]
fn test_execution_request_many_environment_variables() {
    let mut env = HashMap::new();
    for i in 0..1000 {
        env.insert(format!("VAR{i}"), format!("value{i}"));
    }

    let request = ExecutionRequest {
        environment: env,
        ..Default::default()
    };

    assert_eq!(request.environment.len(), 1000);
}

#[test]
fn test_execution_request_all_runtime_types() {
    let runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Gpu,
        RuntimeType::Python,
        RuntimeType::from("custom"),
    ];

    for runtime in runtimes {
        let request = ExecutionRequest {
            runtime_hint: Some(runtime.clone()),
            ..Default::default()
        };

        assert_eq!(request.runtime_hint, Some(runtime));
    }
}

#[test]
fn test_execution_request_serialization_roundtrip() {
    let request = ExecutionRequest::default();

    let serialized = serde_json::to_string(&request).unwrap();
    let deserialized: ExecutionRequest = serde_json::from_str(&serialized).unwrap();

    assert_eq!(request.execution_id, deserialized.execution_id);
}

#[test]
fn test_execution_request_debug_format() {
    let request = ExecutionRequest::default();
    let debug_str = format!("{request:?}");
    assert!(debug_str.contains("ExecutionRequest"));
}

#[test]
fn test_execution_request_complex_environment() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
    env.insert("HOME".to_string(), "/home/user".to_string());
    env.insert("UNICODE".to_string(), "Hello 世界".to_string());

    let request = ExecutionRequest {
        environment: env,
        ..Default::default()
    };

    assert!(request.environment.contains_key("UNICODE"));
}

// ============================================================================
// ExecutionResponse Advanced Tests
// ============================================================================

#[test]
fn test_execution_response_zero_duration() {
    let response = ExecutionResponse {
        duration: Duration::from_secs(0),
        ..Default::default()
    };

    assert_eq!(response.duration, Duration::from_secs(0));
}

#[test]
fn test_execution_response_microsecond_duration() {
    let response = ExecutionResponse {
        duration: Duration::from_micros(42),
        ..Default::default()
    };

    assert_eq!(response.duration.as_micros(), 42);
}

#[test]
fn test_execution_response_many_warnings() {
    let warnings: Vec<String> = (0..1000).map(|i| format!("Warning {i}")).collect();

    let response = ExecutionResponse {
        warnings: warnings.clone(),
        ..Default::default()
    };

    assert_eq!(response.warnings.len(), 1000);
}

#[test]
fn test_execution_response_custom_runtime() {
    let response = ExecutionResponse {
        runtime_used: RuntimeType::from("MyCustomRuntime"),
        ..Default::default()
    };

    match response.runtime_used {
        RuntimeType::Custom(name) => {
            assert_eq!(name.as_ref(), "MyCustomRuntime");
        }
        _ => panic!("Expected Custom runtime"),
    }
}

#[test]
fn test_execution_response_serialization_roundtrip() {
    let response = ExecutionResponse {
        warnings: vec!["warning1".to_string()],
        ..Default::default()
    };

    let serialized = serde_json::to_string(&response).unwrap();
    let deserialized: ExecutionResponse = serde_json::from_str(&serialized).unwrap();

    assert_eq!(response.execution_id, deserialized.execution_id);
}

#[test]
fn test_execution_response_debug_format() {
    let response = ExecutionResponse::default();
    let debug_str = format!("{response:?}");
    assert!(debug_str.contains("ExecutionResponse"));
}

#[test]
fn test_execution_response_failed_with_warnings() {
    let response = ExecutionResponse {
        status: ExecutionStatus::Failed {
            error: std::borrow::Cow::Borrowed("Critical error"),
        },
        warnings: vec![
            "Warning 1".to_string(),
            "Warning 2".to_string(),
            "Warning 3".to_string(),
        ],
        ..Default::default()
    };

    assert_eq!(response.warnings.len(), 3);
    match response.status {
        ExecutionStatus::Failed { error } => {
            assert_eq!(error, "Critical error");
        }
        _ => panic!("Expected Failed status"),
    }
}

// ============================================================================
// RuntimeType Advanced Tests
// ============================================================================

#[test]
fn test_runtime_type_equality() {
    assert_eq!(RuntimeType::Native, RuntimeType::Native);
    assert_eq!(RuntimeType::Wasm, RuntimeType::Wasm);
    assert_ne!(RuntimeType::Native, RuntimeType::Wasm);
}

#[test]
fn test_runtime_type_custom_variants() {
    let custom1 = RuntimeType::from("runtime1");
    let custom2 = RuntimeType::from("runtime2");
    let custom3 = RuntimeType::from("runtime1");

    assert_ne!(custom1, custom2);
    assert_eq!(custom1, custom3);
}

#[test]
fn test_runtime_type_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(RuntimeType::Native);
    set.insert(RuntimeType::Wasm);
    set.insert(RuntimeType::Native); // Duplicate

    assert_eq!(set.len(), 2);
}

#[test]
fn test_runtime_type_serialization() {
    let runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Gpu,
        RuntimeType::Python,
        RuntimeType::from("test"),
    ];

    for runtime in runtimes {
        let serialized = serde_json::to_string(&runtime).unwrap();
        let deserialized: RuntimeType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(runtime, deserialized);
    }
}

#[test]
fn test_runtime_type_debug_format() {
    let runtime = RuntimeType::Gpu;
    let debug_str = format!("{runtime:?}");
    assert!(debug_str.contains("Gpu"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_execution_request_response_flow() {
    let request_id = Uuid::new_v4();

    let request = ExecutionRequest {
        execution_id: request_id,
        runtime_hint: Some(RuntimeType::Wasm),
        timeout: Some(Duration::from_secs(60)),
        ..Default::default()
    };

    let response = ExecutionResponse {
        execution_id: request_id,
        status: ExecutionStatus::Success,
        duration: Duration::from_secs(30),
        runtime_used: RuntimeType::Wasm,
        ..Default::default()
    };

    assert_eq!(request.execution_id, response.execution_id);
    assert_eq!(request.runtime_hint.unwrap(), response.runtime_used);
}

#[test]
fn test_callback_with_all_events() {
    let config = CallbackConfig {
        url: "https://example.com/webhook".to_string(),
        auth_token: Some("secret".to_string()),
        events: vec![
            CallbackEvent::Started,
            CallbackEvent::Progress,
            CallbackEvent::Completed,
            CallbackEvent::Failed,
        ],
    };

    let request = ExecutionRequest {
        callback_config: Some(config),
        encryption_config: None,
        ..Default::default()
    };

    assert!(request.callback_config.is_some());
    assert_eq!(request.callback_config.unwrap().events.len(), 4);
}

#[test]
fn test_execution_with_input_and_output() {
    let input = ExecutionInput {
        data: bytes::Bytes::from_static(b"input data"),
        format: Some("text".to_string()),
        metadata: HashMap::new(),
    };

    let request = ExecutionRequest {
        input_data: input.clone(),
        ..Default::default()
    };

    let output = ExecutionOutput {
        data: bytes::Bytes::from_static(b"output data"),
        stdout: Some("Processing complete".to_string()),
        exit_code: Some(0),
        ..Default::default()
    };

    let response = ExecutionResponse {
        output: output.clone(),
        status: ExecutionStatus::Success,
        ..Default::default()
    };

    assert_eq!(request.input_data.data, &b"input data"[..]);
    assert_eq!(response.output.data, &b"output data"[..]);
}
