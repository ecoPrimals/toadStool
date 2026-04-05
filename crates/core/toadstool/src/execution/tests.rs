// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use proptest::prelude::*;

fn arb_runtime_type() -> impl Strategy<Value = RuntimeType> {
    prop_oneof![
        Just(RuntimeType::Native),
        Just(RuntimeType::Wasm),
        Just(RuntimeType::Container),
        Just(RuntimeType::Gpu),
        Just(RuntimeType::Python),
        "[a-zA-Z0-9_-]{1,50}".prop_map(|s| RuntimeType::Custom(Arc::from(s))),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_runtime_type_json_roundtrip(rt in arb_runtime_type()) {
        let json = serde_json::to_string(&rt).unwrap();
        let restored: RuntimeType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(rt, restored);
    }
}

#[test]
fn test_execution_status_equality() {
    assert_eq!(ExecutionStatus::Success, ExecutionStatus::Success);
    assert_eq!(ExecutionStatus::Cancelled, ExecutionStatus::Cancelled);
    assert_eq!(ExecutionStatus::TimedOut, ExecutionStatus::TimedOut);
    assert_eq!(ExecutionStatus::Running, ExecutionStatus::Running);
    assert_eq!(ExecutionStatus::Pending, ExecutionStatus::Pending);
    assert_eq!(
        ExecutionStatus::Failed {
            error: std::borrow::Cow::Borrowed("test")
        },
        ExecutionStatus::Failed {
            error: std::borrow::Cow::Borrowed("test")
        }
    );
}

#[test]
fn test_execution_status_inequality() {
    assert_ne!(
        ExecutionStatus::Success,
        ExecutionStatus::Failed {
            error: std::borrow::Cow::Borrowed("error")
        }
    );
    assert_ne!(ExecutionStatus::Success, ExecutionStatus::Cancelled);
}

#[test]
fn test_execution_status_serde_roundtrip() {
    let statuses = vec![
        ExecutionStatus::Success,
        ExecutionStatus::Cancelled,
        ExecutionStatus::TimedOut,
        ExecutionStatus::Running,
        ExecutionStatus::Pending,
        ExecutionStatus::Failed {
            error: std::borrow::Cow::Borrowed("test error"),
        },
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ExecutionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }
}

#[test]
fn test_runtime_type_equality() {
    assert_eq!(RuntimeType::Native, RuntimeType::Native);
    assert_eq!(RuntimeType::Wasm, RuntimeType::Wasm);
    assert_eq!(RuntimeType::Container, RuntimeType::Container);
    assert_eq!(RuntimeType::Gpu, RuntimeType::Gpu);
    assert_eq!(RuntimeType::Python, RuntimeType::Python);
    assert_eq!(
        RuntimeType::from("my-runtime"),
        RuntimeType::from("my-runtime")
    );
}

#[test]
fn test_runtime_type_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(RuntimeType::Native);
    set.insert(RuntimeType::Wasm);
    set.insert(RuntimeType::Container);
    set.insert(RuntimeType::Gpu);
    set.insert(RuntimeType::Python);
    set.insert(RuntimeType::from("custom"));

    assert_eq!(set.len(), 6);
    assert!(set.contains(&RuntimeType::Native));
    assert!(set.contains(&RuntimeType::from("custom")));
}

#[test]
fn test_runtime_type_serde_roundtrip() {
    let types = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Gpu,
        RuntimeType::Python,
        RuntimeType::from("my-custom"),
    ];

    for runtime_type in types {
        let json = serde_json::to_string(&runtime_type).unwrap();
        let deserialized: RuntimeType = serde_json::from_str(&json).unwrap();
        assert_eq!(runtime_type, deserialized);
    }
}

#[test]
fn test_callback_event_serde_roundtrip() {
    let events = vec![
        CallbackEvent::Started,
        CallbackEvent::Completed,
        CallbackEvent::Failed,
        CallbackEvent::Progress,
    ];

    for event in events {
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: CallbackEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{event:?}"), format!("{:?}", deserialized));
    }
}

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
        data: Bytes::from("test data"),
        format: Some("text/plain".to_string()),
        metadata: HashMap::from([("key".to_string(), "value".to_string())]),
    };

    assert_eq!(input.data.as_ref(), b"test data");
    assert_eq!(input.format, Some("text/plain".to_string()));
    assert_eq!(input.metadata.get("key"), Some(&"value".to_string()));
}

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
fn test_execution_output_with_data() {
    let output = ExecutionOutput {
        data: Bytes::from("output"),
        stdout: Some("stdout text".to_string()),
        stderr: Some("stderr text".to_string()),
        exit_code: Some(0),
        format: Some("application/json".to_string()),
        result: HashMap::new(),
        metadata: HashMap::new(),
    };

    assert_eq!(output.data.as_ref(), b"output");
    assert_eq!(output.stdout, Some("stdout text".to_string()));
    assert_eq!(output.stderr, Some("stderr text".to_string()));
    assert_eq!(output.exit_code, Some(0));
}

#[test]
fn test_execution_response_default() {
    let response = ExecutionResponse::default();
    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.runtime_used, RuntimeType::Native);
    assert!(response.warnings.is_empty());
    assert_eq!(response.duration, Duration::from_secs(0));
}

#[test]
fn test_callback_config_serde_roundtrip() {
    let config = CallbackConfig {
        url: "https://example.com/callback".to_string(),
        auth_token: Some("secret-token".to_string()),
        events: vec![CallbackEvent::Started, CallbackEvent::Completed],
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: CallbackConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.url, deserialized.url);
    assert_eq!(config.auth_token, deserialized.auth_token);
    assert_eq!(config.events.len(), deserialized.events.len());
}

#[test]
fn test_runtime_config_default() {
    let config = RuntimeConfig::default();
    assert!(config.settings.is_empty());
    assert!(config.resource_limits.is_none());
    assert!(config.security_settings.is_none());
    assert!(config.logging.is_none());
}

#[test]
fn test_logging_config_serde_roundtrip() {
    let config = LoggingConfig {
        level: "info".to_string(),
        format: "json".to_string(),
        destination: "stdout".to_string(),
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: LoggingConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.level, deserialized.level);
    assert_eq!(config.format, deserialized.format);
    assert_eq!(config.destination, deserialized.destination);
}

#[test]
fn test_execution_request_has_new_id_each_time() {
    let req1 = ExecutionRequest::default();
    let req2 = ExecutionRequest::default();
    assert_ne!(req1.execution_id, req2.execution_id);
}
