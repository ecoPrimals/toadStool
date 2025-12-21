//! Comprehensive tests for ToadStoolClient

use super::config::{AuthConfig, ClientConfig};
use super::core::ServerEvent;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

// Helper to create test client without connecting
fn create_test_config() -> ClientConfig {
    ClientConfig {
        base_url: "http://localhost:3000".to_string(),
        request_timeout: Duration::from_secs(10),
        retry_backoff: Duration::from_millis(100),
        max_retries: 3,
        enable_websocket: false, // Disable WebSocket for unit tests
        websocket_timeout: Duration::from_secs(30),
        auth: None,
        custom_headers: HashMap::new(),
    }
}

#[test]
fn test_server_event_deserialization_execution_started() {
    let json = r#"{
        "type": "execution_started",
        "execution_id": "123e4567-e89b-12d3-a456-426614174000",
        "runtime_type": "native",
        "timestamp": "2025-01-01T00:00:00Z"
    }"#;

    let event: ServerEvent =
        serde_json::from_str(json).expect("Valid JSON should deserialize to ServerEvent");
    match event {
        ServerEvent::ExecutionStarted {
            execution_id,
            runtime_type,
            ..
        } => {
            assert_eq!(
                execution_id,
                Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000")
                    .expect("Valid UUID string should parse")
            );
            assert_eq!(runtime_type, "native");
        }
        _ => panic!("Expected ExecutionStarted event"),
    }
}

#[test]
fn test_server_event_deserialization_execution_completed() {
    let json = r#"{
        "type": "execution_completed",
        "execution_id": "123e4567-e89b-12d3-a456-426614174000",
        "status": "success",
        "duration_ms": 1500,
        "timestamp": "2025-01-01T00:00:01Z"
    }"#;

    let event: ServerEvent =
        serde_json::from_str(json).expect("Valid ExecutionCompleted JSON should deserialize");
    match event {
        ServerEvent::ExecutionCompleted {
            duration_ms,
            status,
            ..
        } => {
            assert_eq!(duration_ms, 1500);
            assert_eq!(status, "success");
        }
        _ => panic!("Expected ExecutionCompleted event"),
    }
}

#[test]
fn test_server_event_deserialization_resource_usage() {
    let json = r#"{
        "type": "resource_usage_update",
        "cpu_usage_percent": 45.5,
        "memory_usage_percent": 60.2,
        "active_executions": 5,
        "timestamp": "2025-01-01T00:00:01Z"
    }"#;

    let event: ServerEvent = serde_json::from_str(json).expect("Failed to deserialize");
    match event {
        ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent,
            memory_usage_percent,
            active_executions,
            ..
        } => {
            assert_eq!(cpu_usage_percent, 45.5);
            assert_eq!(memory_usage_percent, 60.2);
            assert_eq!(active_executions, 5);
        }
        _ => panic!("Expected ResourceUsageUpdate event"),
    }
}

#[test]
fn test_server_event_deserialization_error() {
    let json = r#"{
        "type": "error_occurred",
        "error_type": "timeout",
        "message": "Execution timed out",
        "execution_id": "123e4567-e89b-12d3-a456-426614174000",
        "timestamp": "2025-01-01T00:00:01Z"
    }"#;

    let event: ServerEvent = serde_json::from_str(json).expect("Failed to deserialize");
    match event {
        ServerEvent::ErrorOccurred {
            error_type,
            message,
            execution_id,
            ..
        } => {
            assert_eq!(error_type, "timeout");
            assert_eq!(message, "Execution timed out");
            assert_eq!(
                execution_id,
                Some(Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap())
            );
        }
        _ => panic!("Expected ErrorOccurred event"),
    }
}

#[test]
fn test_client_config_creation() {
    let config = create_test_config();
    assert_eq!(config.base_url, "http://localhost:3000");
    assert_eq!(config.request_timeout, Duration::from_secs(10));
    assert_eq!(config.max_retries, 3);
    assert!(!config.enable_websocket);
}

#[test]
fn test_client_config_with_api_key_auth() {
    let auth = AuthConfig::ApiKey {
        key: "test-key-123".to_string(),
        header_name: "X-API-Key".to_string(),
    };
    let config = ClientConfig {
        auth: Some(auth),
        ..create_test_config()
    };

    assert!(config.auth.is_some());
}

#[test]
fn test_client_config_with_bearer_token() {
    let auth = AuthConfig::BearerToken {
        token: "bearer-token-abc".to_string(),
    };
    let config = ClientConfig {
        auth: Some(auth),
        ..create_test_config()
    };

    assert!(config.auth.is_some());
}

#[test]
fn test_client_config_with_basic_auth() {
    let auth = AuthConfig::Basic {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    let config = ClientConfig {
        auth: Some(auth),
        ..create_test_config()
    };

    assert!(config.auth.is_some());
}

#[test]
fn test_client_config_with_custom_headers() {
    let mut custom_headers = HashMap::new();
    custom_headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

    let config = ClientConfig {
        custom_headers,
        ..create_test_config()
    };

    assert_eq!(config.custom_headers.len(), 1);
}

#[test]
fn test_client_config_api_url() {
    let config = create_test_config();
    let api_url = config.api_url("health");
    assert_eq!(api_url, "http://localhost:3000/api/v1/health");
}

#[test]
fn test_client_config_api_url_with_path() {
    let config = create_test_config();
    let api_url = config.api_url("executions/123");
    assert_eq!(api_url, "http://localhost:3000/api/v1/executions/123");
}

// Note: Integration tests that actually connect to a server
// should be in tests/ directory with #[tokio::test]
