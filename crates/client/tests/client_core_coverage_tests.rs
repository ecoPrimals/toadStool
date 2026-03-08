// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
//! Comprehensive tests for `ToadStoolClient` (client/core.rs)
//!
//! Target: crates/client/src/client/core.rs — 90% coverage
//! Tests client methods, error paths. `resolve_socket_path` tested via `new_for_testing`.
//! No real TCP/HTTP connections — uses `new_for_testing` and error paths.

use std::collections::HashMap;

use toadstool_client::{
    AuthConfig, ClientConfig, ToadStoolClient, ToadStoolEvent, WorkloadSubmission, WorkloadType,
};
use uuid::Uuid;

// ============================================================================
// ToadStoolClient::new_for_testing tests
// ============================================================================

#[expect(clippy::expect_used, reason = "test helper; expect is intentional")]
fn test_client() -> ToadStoolClient {
    let config = ClientConfig {
        base_url: "unix:///tmp/test-toadstool-core.sock".to_string(),
        ..Default::default()
    };
    ToadStoolClient::new_for_testing(config).expect("test client")
}

#[test]
fn new_for_testing_unix_succeeds() {
    let config = ClientConfig {
        base_url: "unix:///tmp/test.sock".to_string(),
        ..Default::default()
    };
    let result = ToadStoolClient::new_for_testing(config);
    assert!(result.is_ok());
}

#[test]
fn new_for_testing_invalid_url_fails() {
    let config = ClientConfig {
        base_url: "not-a-valid-url!!!".to_string(),
        ..Default::default()
    };
    let result = ToadStoolClient::new_for_testing(config);
    assert!(result.is_err());
}

#[test]
fn new_for_testing_http_url_succeeds() {
    let config = ClientConfig {
        base_url: "http://localhost:8080".to_string(),
        ..Default::default()
    };
    let result = ToadStoolClient::new_for_testing(config);
    assert!(result.is_ok());
}

// ============================================================================
// submit_workload tests
// ============================================================================

#[tokio::test]
async fn submit_workload_returns_error_with_message() {
    let client = test_client();
    let workload = WorkloadSubmission {
        workload_type: WorkloadType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            working_dir: None,
        },
        runtime_hint: None,
        priority: None,
        timeout: None,
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };
    let result = client.submit_workload(workload).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("compute.submit") || err.to_string().contains("JSON-RPC"),
        "err: {err}"
    );
}

// ============================================================================
// get_execution_status tests
// ============================================================================

#[tokio::test]
async fn get_execution_status_returns_error() {
    let client = test_client();
    let result = client.get_execution_status(Uuid::new_v4()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("compute.status"));
}

// ============================================================================
// cancel_execution tests (error path - no server)
// ============================================================================

#[tokio::test]
async fn cancel_execution_fails_without_server() {
    let client = test_client();
    let result = client.cancel_execution(Uuid::new_v4()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("compute.cancel"));
}

// ============================================================================
// wait_for_completion tests
// ============================================================================

#[tokio::test]
async fn wait_for_completion_fails_without_server() {
    let client = test_client();
    let result = client.wait_for_completion(Uuid::new_v4()).await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("wait_for_completion")
            || err_str.contains("compute.status")
            || err_str.contains("JSON-RPC"),
        "expected wait_for_completion or compute.status error, got: {err_str}"
    );
}

// ============================================================================
// get_cluster_status tests
// ============================================================================

#[tokio::test]
async fn get_cluster_status_fails_without_server() {
    let client = test_client();
    let result = client.get_cluster_status().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("toadstool.health"));
}

// ============================================================================
// health_check tests
// ============================================================================

#[tokio::test]
async fn health_check_fails_without_server() {
    let client = test_client();
    let result = client.health_check().await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("Health check") || err_str.contains("toadstool.health"),
        "expected health check error, got: {err_str}"
    );
}

// ============================================================================
// list_executions tests
// ============================================================================

#[tokio::test]
async fn list_executions_returns_empty() {
    let client = test_client();
    let result = client.list_executions().await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

// ============================================================================
// add_event_handler tests
// ============================================================================

#[tokio::test]
async fn add_event_handler_accepts_closure() {
    let client = test_client();
    client.add_event_handler(|_: ToadStoolEvent| {}).await;
}

#[tokio::test]
async fn add_event_handler_multiple_handlers() {
    let client = test_client();
    client.add_event_handler(|_: ToadStoolEvent| {}).await;
    client.add_event_handler(|_: ToadStoolEvent| {}).await;
}

// ============================================================================
// subscribe_to_events tests
// ============================================================================

#[tokio::test]
async fn subscribe_to_events_returns_error() {
    let client = test_client();
    let result = client.subscribe_to_events().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("polling"));
}

// ============================================================================
// start_event_stream tests
// ============================================================================

#[test]
fn start_event_stream_returns_ok() {
    let client = test_client();
    let result = client.start_event_stream();
    assert!(result.is_ok());
}

// ============================================================================
// ClientConfig with auth/custom headers (debug branch)
// ============================================================================

#[test]
fn new_for_testing_with_auth_config() {
    let config = ClientConfig {
        base_url: "unix:///tmp/sock".to_string(),
        auth: Some(AuthConfig::BearerToken {
            token: "test-token".to_string(),
        }),
        ..Default::default()
    };
    let result = ToadStoolClient::new_for_testing(config);
    assert!(result.is_ok());
}

#[test]
fn new_for_testing_with_custom_headers() {
    let mut headers = HashMap::new();
    headers.insert("X-Custom".to_string(), "value".to_string());
    let config = ClientConfig {
        base_url: "unix:///tmp/sock".to_string(),
        custom_headers: headers,
        ..Default::default()
    };
    let result = ToadStoolClient::new_for_testing(config);
    assert!(result.is_ok());
}

// ============================================================================
// ClientConfig timeout and connection setup
// ============================================================================

#[test]
fn new_for_testing_with_request_timeout() {
    let config = ClientConfig {
        base_url: "unix:///tmp/sock".to_string(),
        request_timeout: std::time::Duration::from_secs(60),
        ..Default::default()
    };
    let result = ToadStoolClient::new_for_testing(config);
    assert!(result.is_ok());
}

#[test]
fn new_for_testing_unix_double_slash_path() {
    let config = ClientConfig {
        base_url: "unix:///run/toadstool.sock".to_string(),
        ..Default::default()
    };
    let result = ToadStoolClient::new_for_testing(config);
    assert!(result.is_ok());
}

#[test]
fn new_for_testing_unix_single_colon_path() {
    let config = ClientConfig {
        base_url: "unix:/tmp/toadstool.sock".to_string(),
        ..Default::default()
    };
    let result = ToadStoolClient::new_for_testing(config);
    assert!(result.is_ok());
}

#[test]
fn new_for_testing_empty_unix_prefix() {
    let config = ClientConfig {
        base_url: "unix:".to_string(),
        ..Default::default()
    };
    let result = ToadStoolClient::new_for_testing(config);
    assert!(result.is_ok());
}
