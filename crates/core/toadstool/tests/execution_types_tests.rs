//! Comprehensive tests for execution types

use std::collections::HashMap;
use std::time::Duration;
use toadstool::*;
use uuid::Uuid;

// ============================================================================
// ExecutionStatus Tests
// ============================================================================

#[test]
fn test_execution_status_success() {
    let status = ExecutionStatus::Success;
    assert!(matches!(status, ExecutionStatus::Success));
}

#[test]
fn test_execution_status_failed() {
    let status = ExecutionStatus::Failed {
        error: "Test error".to_string(),
    };

    match status {
        ExecutionStatus::Failed { error } => {
            assert_eq!(error, "Test error");
        }
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_execution_status_cancelled() {
    let status = ExecutionStatus::Cancelled;
    assert!(matches!(status, ExecutionStatus::Cancelled));
}

#[test]
fn test_execution_status_timed_out() {
    let status = ExecutionStatus::TimedOut;
    assert!(matches!(status, ExecutionStatus::TimedOut));
}

#[test]
fn test_execution_status_running() {
    let status = ExecutionStatus::Running;
    assert!(matches!(status, ExecutionStatus::Running));
}

#[test]
fn test_execution_status_pending() {
    let status = ExecutionStatus::Pending;
    assert!(matches!(status, ExecutionStatus::Pending));
}

#[test]
fn test_execution_status_clone() {
    let status1 = ExecutionStatus::Success;
    let status2 = status1.clone();

    assert_eq!(status1, status2);
}

#[test]
fn test_execution_status_equality() {
    let status1 = ExecutionStatus::Success;
    let status2 = ExecutionStatus::Success;
    assert_eq!(status1, status2);
}

#[test]
fn test_execution_status_inequality() {
    let status1 = ExecutionStatus::Success;
    let status2 = ExecutionStatus::Cancelled;
    assert_ne!(status1, status2);
}

#[test]
fn test_execution_status_serialization() {
    let status = ExecutionStatus::Success;
    let serialized = serde_json::to_string(&status).unwrap();
    assert!(!serialized.is_empty());
}

#[test]
fn test_execution_status_deserialization() {
    let json = r#""Success""#;
    let status: ExecutionStatus = serde_json::from_str(json).unwrap();
    assert!(matches!(status, ExecutionStatus::Success));
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
    let data = vec![1, 2, 3, 4, 5];
    let input = ExecutionInput {
        data: data.clone(),
        format: Some("binary".to_string()),
        metadata: HashMap::new(),
    };

    assert_eq!(input.data, data);
    assert_eq!(input.format, Some("binary".to_string()));
}

#[test]
fn test_execution_input_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("key1".to_string(), "value1".to_string());
    metadata.insert("key2".to_string(), "value2".to_string());

    let input = ExecutionInput {
        data: vec![],
        format: None,
        metadata: metadata.clone(),
    };

    assert_eq!(input.metadata.len(), 2);
    assert_eq!(input.metadata.get("key1"), Some(&"value1".to_string()));
}

#[test]
fn test_execution_input_clone() {
    let input1 = ExecutionInput {
        data: vec![1, 2, 3],
        format: Some("json".to_string()),
        metadata: HashMap::new(),
    };

    let input2 = input1.clone();

    assert_eq!(input1.data, input2.data);
    assert_eq!(input1.format, input2.format);
}

#[test]
fn test_execution_input_serialization() {
    let input = ExecutionInput::default();
    let serialized = serde_json::to_string(&input).unwrap();
    assert!(!serialized.is_empty());
}

#[test]
fn test_execution_input_with_large_data() {
    let data = vec![0u8; 1024 * 1024]; // 1MB
    let input = ExecutionInput {
        data,
        format: Some("binary".to_string()),
        metadata: HashMap::new(),
    };

    assert_eq!(input.data.len(), 1024 * 1024);
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
        data: vec![],
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
        data: vec![],
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
fn test_execution_output_with_all_fields() {
    let mut result = HashMap::new();
    result.insert("status".to_string(), "completed".to_string());

    let mut metadata = HashMap::new();
    metadata.insert("timestamp".to_string(), "2025-10-14".to_string());

    let output = ExecutionOutput {
        data: vec![1, 2, 3],
        stdout: Some("Output".to_string()),
        stderr: Some("Warnings".to_string()),
        exit_code: Some(0),
        format: Some("json".to_string()),
        result,
        metadata,
    };

    assert_eq!(output.data, vec![1, 2, 3]);
    assert!(output.stdout.is_some());
    assert!(output.stderr.is_some());
    assert_eq!(output.exit_code, Some(0));
    assert_eq!(output.result.len(), 1);
    assert_eq!(output.metadata.len(), 1);
}

#[test]
fn test_execution_output_clone() {
    let output1 = ExecutionOutput {
        data: vec![1, 2, 3],
        stdout: Some("test".to_string()),
        stderr: None,
        exit_code: Some(0),
        format: None,
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    let output2 = output1.clone();

    assert_eq!(output1.data, output2.data);
    assert_eq!(output1.stdout, output2.stdout);
}

#[test]
fn test_execution_output_with_exit_codes() {
    let exit_codes = vec![0, 1, 127, 255, -1];

    for code in exit_codes {
        let output = ExecutionOutput {
            data: vec![],
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
        events: vec![CallbackEvent::Completed],
    };

    assert!(config.auth_token.is_none());
}

#[test]
fn test_callback_config_clone() {
    let config1 = CallbackConfig {
        url: "https://test.com".to_string(),
        auth_token: Some("token".to_string()),
        events: vec![CallbackEvent::Failed],
    };

    let config2 = config1.clone();

    assert_eq!(config1.url, config2.url);
    assert_eq!(config1.auth_token, config2.auth_token);
}

#[test]
fn test_callback_config_with_all_events() {
    let config = CallbackConfig {
        url: "https://example.com".to_string(),
        auth_token: None,
        events: vec![
            CallbackEvent::Started,
            CallbackEvent::Completed,
            CallbackEvent::Failed,
        ],
    };

    assert_eq!(config.events.len(), 3);
}

// ============================================================================
// CallbackEvent Tests
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
fn test_callback_event_clone() {
    let event1 = CallbackEvent::Started;
    let event2 = event1.clone();

    match (event1, event2) {
        (CallbackEvent::Started, CallbackEvent::Started) => {}
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_callback_event_serialization() {
    let event = CallbackEvent::Completed;
    let serialized = serde_json::to_string(&event).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// ExecutionRequest Tests
// ============================================================================

#[test]
fn test_execution_request_default() {
    let request = ExecutionRequest::default();

    assert!(request.execution_id != Uuid::nil());
    assert!(request.runtime_hint.is_none());
    assert_eq!(request.timeout, Some(Duration::from_secs(300)));
    assert!(request.environment.is_empty());
    assert!(request.callback_config.is_none());
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
fn test_execution_request_with_timeout() {
    let request = ExecutionRequest {
        timeout: Some(Duration::from_secs(600)),
        ..Default::default()
    };

    assert_eq!(request.timeout, Some(Duration::from_secs(600)));
}

#[test]
fn test_execution_request_with_environment() {
    let mut env = HashMap::new();
    env.insert("VAR1".to_string(), "value1".to_string());
    env.insert("VAR2".to_string(), "value2".to_string());

    let request = ExecutionRequest {
        environment: env.clone(),
        ..Default::default()
    };

    assert_eq!(request.environment.len(), 2);
    assert_eq!(request.environment.get("VAR1"), Some(&"value1".to_string()));
}

#[test]
fn test_execution_request_with_callback() {
    let callback = CallbackConfig {
        url: "https://example.com/callback".to_string(),
        auth_token: Some("token".to_string()),
        events: vec![CallbackEvent::Completed],
    };

    let request = ExecutionRequest {
        callback_config: Some(callback),
        ..Default::default()
    };

    assert!(request.callback_config.is_some());
}

#[test]
fn test_execution_request_clone() {
    let request1 = ExecutionRequest::default();
    let request2 = request1.clone();

    assert_eq!(request1.execution_id, request2.execution_id);
    assert_eq!(request1.timeout, request2.timeout);
}

#[test]
fn test_execution_request_serialization() {
    let request = ExecutionRequest::default();
    let serialized = serde_json::to_string(&request).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// ExecutionResponse Tests
// ============================================================================

#[test]
fn test_execution_response_default() {
    let response = ExecutionResponse::default();

    assert!(response.execution_id != Uuid::nil());
    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.duration, Duration::from_secs(0));
    assert_eq!(response.runtime_used, RuntimeType::Native);
    assert!(response.warnings.is_empty());
}

#[test]
fn test_execution_response_with_status() {
    let response = ExecutionResponse {
        status: ExecutionStatus::Failed {
            error: "Test error".to_string(),
        },
        ..Default::default()
    };

    match response.status {
        ExecutionStatus::Failed { error } => {
            assert_eq!(error, "Test error");
        }
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_execution_response_with_duration() {
    let response = ExecutionResponse {
        duration: Duration::from_secs(42),
        ..Default::default()
    };

    assert_eq!(response.duration, Duration::from_secs(42));
}

#[test]
fn test_execution_response_with_warnings() {
    let response = ExecutionResponse {
        warnings: vec!["Warning 1".to_string(), "Warning 2".to_string()],
        ..Default::default()
    };

    assert_eq!(response.warnings.len(), 2);
    assert_eq!(response.warnings[0], "Warning 1");
}

#[test]
fn test_execution_response_with_different_runtimes() {
    let runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Python,
        RuntimeType::Gpu,
    ];

    for runtime in runtimes {
        let response = ExecutionResponse {
            runtime_used: runtime.clone(),
            ..Default::default()
        };

        assert_eq!(response.runtime_used, runtime);
    }
}

#[test]
fn test_execution_response_clone() {
    let response1 = ExecutionResponse::default();
    let response2 = response1.clone();

    assert_eq!(response1.execution_id, response2.execution_id);
    assert_eq!(response1.status, response2.status);
    assert_eq!(response1.duration, response2.duration);
}

#[test]
fn test_execution_response_serialization() {
    let response = ExecutionResponse::default();
    let serialized = serde_json::to_string(&response).unwrap();
    assert!(!serialized.is_empty());
}
