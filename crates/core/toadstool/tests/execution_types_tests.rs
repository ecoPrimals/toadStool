//! Tests for execution types and RuntimeEngine trait.

use std::collections::HashMap;
use std::time::Duration;
use toadstool::execution::*;
use toadstool::workload::WorkloadType;
use uuid::Uuid;

// ============== ExecutionRequest tests ==============

#[test]
fn execution_request_default_construction() {
    let req = ExecutionRequest::default();
    assert!(req.runtime_hint.is_none());
    assert_eq!(req.environment.len(), 0);
    assert_eq!(req.timeout, Some(Duration::from_secs(300)));
    assert!(req.callback_config.is_none());
    assert!(req.encryption_config.is_none());
}

#[test]
fn execution_request_field_access() {
    let id = Uuid::new_v4();
    let mut environment = HashMap::new();
    environment.insert("FOO".to_string(), "bar".to_string());
    let req = ExecutionRequest {
        execution_id: id,
        runtime_hint: Some(RuntimeType::Wasm),
        timeout: Some(Duration::from_secs(60)),
        environment,
        ..Default::default()
    };

    assert_eq!(req.execution_id, id);
    assert_eq!(req.runtime_hint, Some(RuntimeType::Wasm));
    assert_eq!(req.timeout, Some(Duration::from_secs(60)));
    assert_eq!(req.environment.get("FOO"), Some(&"bar".to_string()));
}

#[test]
fn execution_request_with_callback_config() {
    let req = ExecutionRequest {
        callback_config: Some(CallbackConfig {
            url: "https://example.com/callback".to_string(),
            auth_token: Some("secret".to_string()),
            events: vec![CallbackEvent::Started, CallbackEvent::Completed],
        }),
        ..Default::default()
    };

    let config = req.callback_config.as_ref().unwrap();
    assert_eq!(config.url, "https://example.com/callback");
    assert_eq!(config.auth_token.as_deref(), Some("secret"));
    assert_eq!(config.events.len(), 2);
}

#[test]
fn execution_request_with_encryption_config() {
    let req = ExecutionRequest {
        encryption_config: Some(toadstool::encryption::EncryptionConfig::default()),
        ..Default::default()
    };

    assert!(req.encryption_config.is_some());
}

#[test]
fn execution_request_clone() {
    let req = ExecutionRequest::default();
    let cloned = req.clone();
    assert_eq!(req.execution_id, cloned.execution_id);
}

// ============== ExecutionResponse tests ==============

#[test]
fn execution_response_default_construction() {
    let resp = ExecutionResponse::default();
    assert_eq!(resp.status, ExecutionStatus::Success);
    assert_eq!(resp.runtime_used, RuntimeType::Native);
    assert_eq!(resp.duration, Duration::from_secs(0));
    assert!(resp.warnings.is_empty());
}

#[test]
fn execution_response_with_all_fields() {
    let resp = ExecutionResponse {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Failed {
            error: "oops".to_string(),
        },
        output: ExecutionOutput {
            stdout: Some("hello".to_string()),
            stderr: Some("err".to_string()),
            exit_code: Some(1),
            ..Default::default()
        },
        metrics: toadstool::resources::RuntimeMetrics::default(),
        duration: Duration::from_millis(1500),
        runtime_used: RuntimeType::Container,
        warnings: vec!["deprecated".to_string()],
    };

    assert!(matches!(resp.status, ExecutionStatus::Failed { .. }));
    assert_eq!(resp.output.stdout.as_deref(), Some("hello"));
    assert_eq!(resp.output.exit_code, Some(1));
    assert_eq!(resp.runtime_used, RuntimeType::Container);
    assert_eq!(resp.warnings, vec!["deprecated"]);
}

#[test]
fn execution_response_clone() {
    let resp = ExecutionResponse::default();
    let cloned = resp.clone();
    assert_eq!(resp.status, cloned.status);
}

// ============== ExecutionStatus tests ==============

#[test]
fn execution_status_all_variants() {
    let success = ExecutionStatus::Success;
    let failed = ExecutionStatus::Failed {
        error: "test error".to_string(),
    };
    let cancelled = ExecutionStatus::Cancelled;
    let timed_out = ExecutionStatus::TimedOut;
    let running = ExecutionStatus::Running;
    let pending = ExecutionStatus::Pending;

    assert_eq!(success, ExecutionStatus::Success);
    assert!(matches!(failed, ExecutionStatus::Failed { error } if error == "test error"));
    assert_eq!(cancelled, ExecutionStatus::Cancelled);
    assert_eq!(timed_out, ExecutionStatus::TimedOut);
    assert_eq!(running, ExecutionStatus::Running);
    assert_eq!(pending, ExecutionStatus::Pending);
}

#[test]
fn execution_status_comparisons() {
    assert_eq!(ExecutionStatus::Success, ExecutionStatus::Success);
    assert_ne!(
        ExecutionStatus::Success,
        ExecutionStatus::Failed {
            error: "x".to_string(),
        }
    );
    assert_ne!(
        ExecutionStatus::Failed {
            error: "a".to_string(),
        },
        ExecutionStatus::Failed {
            error: "b".to_string(),
        }
    );
    assert_eq!(
        ExecutionStatus::Failed {
            error: "same".to_string(),
        },
        ExecutionStatus::Failed {
            error: "same".to_string(),
        }
    );
}

#[test]
fn execution_status_debug() {
    let status = ExecutionStatus::Success;
    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("Success"));
}

// ============== RuntimeType tests ==============

#[test]
fn runtime_type_all_variants() {
    let _native = RuntimeType::Native;
    let _wasm = RuntimeType::Wasm;
    let _container = RuntimeType::Container;
    let _gpu = RuntimeType::Gpu;
    let _python = RuntimeType::Python;
    let custom = RuntimeType::Custom("my-runtime".to_string());

    assert_eq!(custom, RuntimeType::Custom("my-runtime".to_string()));
}

#[test]
fn runtime_type_comparisons() {
    assert_eq!(RuntimeType::Native, RuntimeType::Native);
    assert_ne!(RuntimeType::Native, RuntimeType::Wasm);
    assert_eq!(
        RuntimeType::Custom("x".to_string()),
        RuntimeType::Custom("x".to_string())
    );
    assert_ne!(
        RuntimeType::Custom("x".to_string()),
        RuntimeType::Custom("y".to_string())
    );
}

#[test]
fn runtime_type_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let t1 = RuntimeType::Native;
    let t2 = RuntimeType::Native;
    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    t1.hash(&mut h1);
    t2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn runtime_type_debug() {
    let rt = RuntimeType::Gpu;
    let debug_str = format!("{:?}", rt);
    assert!(debug_str.contains("Gpu"));
}

// ============== RuntimeConfig tests ==============

#[test]
fn runtime_config_default() {
    let config = RuntimeConfig::default();
    assert!(config.settings.is_empty());
    assert!(config.resource_limits.is_none());
    assert!(config.security_settings.is_none());
    assert!(config.logging.is_none());
}

#[test]
fn runtime_config_with_settings() {
    let mut settings = HashMap::new();
    settings.insert("foo".to_string(), serde_json::json!("bar"));
    let config = RuntimeConfig {
        settings,
        ..Default::default()
    };

    assert_eq!(config.settings.get("foo"), Some(&serde_json::json!("bar")));
}

#[test]
fn runtime_config_with_resource_limits() {
    let config = RuntimeConfig {
        resource_limits: Some(toadstool::resources::ResourceLimits::default()),
        ..Default::default()
    };

    assert!(config.resource_limits.is_some());
}

#[test]
fn runtime_config_clone() {
    let config = RuntimeConfig::default();
    let cloned = config.clone();
    assert_eq!(config.settings.len(), cloned.settings.len());
}

// ============== ExecutionInput tests ==============

#[test]
fn execution_input_default() {
    let input = ExecutionInput::default();
    assert!(input.data.is_empty());
    assert!(input.format.is_none());
    assert!(input.metadata.is_empty());
}

#[test]
fn execution_input_with_data() {
    let mut metadata = HashMap::new();
    metadata.insert("key".to_string(), "value".to_string());
    let input = ExecutionInput {
        data: vec![1, 2, 3],
        format: Some("json".to_string()),
        metadata,
    };

    assert_eq!(input.data, vec![1, 2, 3]);
    assert_eq!(input.format.as_deref(), Some("json"));
    assert_eq!(input.metadata.get("key"), Some(&"value".to_string()));
}

// ============== ExecutionOutput tests ==============

#[test]
fn execution_output_default() {
    let output = ExecutionOutput::default();
    assert!(output.data.is_empty());
    assert!(output.stdout.is_none());
    assert!(output.stderr.is_none());
    assert!(output.exit_code.is_none());
    assert!(output.result.is_empty());
}

#[test]
fn execution_output_with_fields() {
    let output = ExecutionOutput {
        data: vec![42u8],
        stdout: Some("out".to_string()),
        stderr: Some("err".to_string()),
        exit_code: Some(0),
        format: Some("binary".to_string()),
        result: HashMap::from([("k".to_string(), "v".to_string())]),
        metadata: HashMap::new(),
    };

    assert_eq!(output.data, vec![42]);
    assert_eq!(output.stdout.as_deref(), Some("out"));
    assert_eq!(output.exit_code, Some(0));
    assert_eq!(output.result.get("k"), Some(&"v".to_string()));
}

// ============== CallbackConfig and CallbackEvent tests ==============

#[test]
fn callback_event_variants() {
    let _started = CallbackEvent::Started;
    let _completed = CallbackEvent::Completed;
    let _failed = CallbackEvent::Failed;
    let _progress = CallbackEvent::Progress;
}

#[test]
fn callback_config_construction() {
    let config = CallbackConfig {
        url: "https://example.com".to_string(),
        auth_token: None,
        events: vec![CallbackEvent::Started, CallbackEvent::Failed],
    };

    assert_eq!(config.url, "https://example.com");
    assert!(config.auth_token.is_none());
    assert_eq!(config.events.len(), 2);
}

// ============== RuntimeCapabilities tests ==============

#[test]
fn runtime_capabilities_construction() {
    let caps = RuntimeCapabilities {
        supported_workloads: vec![WorkloadType::Native, WorkloadType::Wasm],
        max_concurrent_executions: Some(8),
        supported_architectures: vec!["x86_64".to_string()],
        platform_features: HashMap::from([("gpu".to_string(), true)]),
        version: "1.0".to_string(),
    };

    assert_eq!(caps.supported_workloads.len(), 2);
    assert_eq!(caps.max_concurrent_executions, Some(8));
    assert_eq!(caps.supported_architectures, vec!["x86_64"]);
    assert_eq!(caps.platform_features.get("gpu"), Some(&true));
    assert_eq!(caps.version, "1.0");
}

// ============== LoggingConfig tests ==============

#[test]
fn logging_config_construction() {
    let config = LoggingConfig {
        level: "info".to_string(),
        format: "json".to_string(),
        destination: "stderr".to_string(),
    };

    assert_eq!(config.level, "info");
    assert_eq!(config.format, "json");
    assert_eq!(config.destination, "stderr");
}

// ============== Serialization round-trip tests ==============

#[test]
fn execution_request_serialization_roundtrip() {
    let req = ExecutionRequest::default();
    let json = serde_json::to_string(&req).expect("serialize");
    let deserialized: ExecutionRequest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(req.execution_id, deserialized.execution_id);
    assert_eq!(req.runtime_hint, deserialized.runtime_hint);
}

#[test]
fn execution_response_serialization_roundtrip() {
    let resp = ExecutionResponse::default();
    let json = serde_json::to_string(&resp).expect("serialize");
    let deserialized: ExecutionResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(resp.execution_id, deserialized.execution_id);
    assert_eq!(resp.status, deserialized.status);
}

#[test]
fn execution_status_serialization_roundtrip() {
    let statuses = [
        ExecutionStatus::Success,
        ExecutionStatus::Failed {
            error: "err".to_string(),
        },
        ExecutionStatus::Cancelled,
        ExecutionStatus::TimedOut,
        ExecutionStatus::Running,
        ExecutionStatus::Pending,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: ExecutionStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, deserialized);
    }
}

#[test]
fn runtime_type_serialization_roundtrip() {
    let types = [
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Gpu,
        RuntimeType::Python,
        RuntimeType::Custom("custom-rt".to_string()),
    ];

    for rt in types {
        let json = serde_json::to_string(&rt).expect("serialize");
        let deserialized: RuntimeType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rt, deserialized);
    }
}

#[test]
fn runtime_config_serialization_roundtrip() {
    let config = RuntimeConfig::default();
    let json = serde_json::to_string(&config).expect("serialize");
    let deserialized: RuntimeConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config.settings.len(), deserialized.settings.len());
}

#[test]
fn execution_input_serialization_roundtrip() {
    let input = ExecutionInput {
        data: vec![1, 2, 3],
        format: Some("bin".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_string(&input).expect("serialize");
    let deserialized: ExecutionInput = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(input.data, deserialized.data);
    assert_eq!(input.format, deserialized.format);
}

#[test]
fn execution_output_serialization_roundtrip() {
    let output = ExecutionOutput {
        stdout: Some("hello".to_string()),
        exit_code: Some(0),
        ..Default::default()
    };
    let json = serde_json::to_string(&output).expect("serialize");
    let deserialized: ExecutionOutput = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(output.stdout, deserialized.stdout);
    assert_eq!(output.exit_code, deserialized.exit_code);
}

#[test]
fn callback_config_serialization_roundtrip() {
    let config = CallbackConfig {
        url: "https://cb.example.com".to_string(),
        auth_token: Some("token".to_string()),
        events: vec![CallbackEvent::Completed],
    };
    let json = serde_json::to_string(&config).expect("serialize");
    let deserialized: CallbackConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config.url, deserialized.url);
    assert_eq!(config.events.len(), deserialized.events.len());
}

#[test]
fn callback_event_serialization_roundtrip() {
    let events = [
        CallbackEvent::Started,
        CallbackEvent::Completed,
        CallbackEvent::Failed,
        CallbackEvent::Progress,
    ];
    for event in events {
        let json = serde_json::to_string(&event).expect("serialize");
        let deserialized: CallbackEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            std::mem::discriminant(&event),
            std::mem::discriminant(&deserialized)
        );
    }
}

#[test]
fn logging_config_serialization_roundtrip() {
    let config = LoggingConfig {
        level: "debug".to_string(),
        format: "text".to_string(),
        destination: "stdout".to_string(),
    };
    let json = serde_json::to_string(&config).expect("serialize");
    let deserialized: LoggingConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config.level, deserialized.level);
}

#[test]
fn runtime_capabilities_serialization_roundtrip() {
    let caps = RuntimeCapabilities {
        supported_workloads: vec![WorkloadType::Native],
        max_concurrent_executions: Some(4),
        supported_architectures: vec!["aarch64".to_string()],
        platform_features: HashMap::new(),
        version: "2.0".to_string(),
    };
    let json = serde_json::to_string(&caps).expect("serialize");
    let deserialized: RuntimeCapabilities = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(caps.version, deserialized.version);
}
