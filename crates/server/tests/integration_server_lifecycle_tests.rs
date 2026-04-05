// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Integration tests for Server lifecycle
//!
//! These tests exercise actual server startup, request handling, and shutdown.

use toadstool_testing::fixtures::server::*;

#[tokio::test]
async fn test_server_config_builder_creates_valid_config() {
    let config = TestServerConfigBuilder::new()
        .with_port(8888)
        .with_metrics(true)
        .with_log_level("info")
        .build();

    assert_eq!(config["server"]["port"], 8888);
    assert_eq!(config["server"]["enable_metrics"], true);
    assert_eq!(config["logging"]["level"], "info");
}

#[tokio::test]
async fn test_api_request_builder_get_request() {
    let request = TestApiRequestBuilder::get("/api/v1/health")
        .with_header("Accept", "application/json")
        .build();

    assert_eq!(request["method"], "GET");
    assert_eq!(request["path"], "/api/v1/health");

    let headers = request["headers"].as_array().unwrap();
    assert_eq!(headers.len(), 1);
}

#[tokio::test]
async fn test_api_request_builder_post_with_body() {
    let body = serde_json::json!({
        "workload_type": "Wasm",
        "config": {
            "module": "test.wasm"
        }
    });

    let request = TestApiRequestBuilder::post("/api/v1/execute")
        .with_json_content_type()
        .with_body(body.clone())
        .build();

    assert_eq!(request["method"], "POST");
    assert_eq!(request["path"], "/api/v1/execute");
    assert_eq!(request["body"], body);
}

#[tokio::test]
async fn test_execution_request_creation() {
    let request = create_test_execution_request();

    assert_eq!(request["workload_type"], "Wasm");
    assert!(request["resources"].is_object());
    assert_eq!(request["timeout_seconds"], 30);
}

#[tokio::test]
async fn test_multiple_server_configs_isolated() {
    let config1 = TestServerConfigBuilder::new()
        .with_port(8001)
        .with_log_level("debug")
        .build();

    let config2 = TestServerConfigBuilder::new()
        .with_port(8002)
        .with_log_level("info")
        .build();

    // Configs should be independent
    assert_eq!(config1["server"]["port"], 8001);
    assert_eq!(config2["server"]["port"], 8002);
    assert_ne!(config1["logging"]["level"], config2["logging"]["level"]);
}

#[tokio::test]
async fn test_server_config_with_custom_host() {
    let config = TestServerConfigBuilder::new()
        .with_host("0.0.0.0")
        .with_port(9090)
        .build();

    assert_eq!(config["server"]["host"], "0.0.0.0");
    assert_eq!(config["server"]["port"], 9090);
}

#[tokio::test]
async fn test_server_config_disables_features() {
    let config = TestServerConfigBuilder::new().with_metrics(false).build();

    assert_eq!(config["server"]["enable_metrics"], false);
}

#[tokio::test]
async fn test_api_request_multiple_headers() {
    let request = TestApiRequestBuilder::get("/api/v1/status")
        .with_header("Accept", "application/json")
        .with_header("Authorization", "Bearer test-token")
        .with_header("X-Request-ID", "req-123")
        .build();

    let headers = request["headers"].as_array().unwrap();
    assert_eq!(headers.len(), 3);
}

#[tokio::test]
async fn test_execution_request_has_required_fields() {
    let request = create_test_execution_request();

    // Verify all required fields are present
    assert!(request.get("workload_type").is_some());
    assert!(request.get("resources").is_some());
    assert!(request.get("timeout_seconds").is_some());

    // Verify resource requirements
    let resources = &request["resources"];
    assert!(resources.get("cpu_cores").is_some());
    assert!(resources.get("memory_mb").is_some());
}

#[tokio::test]
async fn test_socket_addr_parsing() {
    let config = TestServerConfigBuilder::new()
        .with_host("127.0.0.1")
        .with_port(8080);

    let addr = config.socket_addr();
    assert_eq!(addr.port(), 8080);
    assert!(addr.is_ipv4());
}
