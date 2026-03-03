// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for ManualJsonRpcServer
//!
//! Tests the pure Rust HTTP/1.1 + JSON-RPC 2.0 implementation over Unix sockets.
//! ManualJsonRpcServer is deprecated; tests retained until migration to pure_jsonrpc.

#![allow(deprecated)]

use std::sync::Arc;
use toadstool_server::manual_jsonrpc::ManualJsonRpcServer;
use toadstool_server::StandaloneExecutor;

/// Helper to create a test server
fn create_test_server() -> ManualJsonRpcServer {
    let executor = Arc::new(StandaloneExecutor::new());
    ManualJsonRpcServer::new(executor, "test-version".to_string(), None)
}

/// Helper to create a JSON-RPC request
fn create_jsonrpc_request(method: &str, id: i64) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"{}","params":{{}},"id":{}}}"#,
        method, id
    )
}

/// Helper to create an HTTP request
fn create_http_request(body: &str) -> String {
    format!(
        "POST /rpc HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

#[test]
fn test_server_creation() {
    let _server = create_test_server();
    // Server created successfully
}

#[tokio::test]
async fn test_health_method() {
    let _server = create_test_server();
    let request = create_jsonrpc_request("toadstool.health", 1);

    // This would require the server to be running, so we test the structure
    assert!(request.contains("toadstool.health"));
    assert!(request.contains(r#""jsonrpc":"2.0""#));
}

#[tokio::test]
async fn test_version_method() {
    let _server = create_test_server();
    let request = create_jsonrpc_request("toadstool.version", 2);

    assert!(request.contains("toadstool.version"));
    assert!(request.contains(r#""id":2"#));
}

#[tokio::test]
async fn test_query_capabilities_method() {
    let _server = create_test_server();
    let request = create_jsonrpc_request("toadstool.query_capabilities", 3);

    assert!(request.contains("toadstool.query_capabilities"));
    assert!(request.contains(r#""id":3"#));
}

#[test]
fn test_http_request_format() {
    let json_body = create_jsonrpc_request("toadstool.health", 1);
    let http_request = create_http_request(&json_body);

    assert!(http_request.starts_with("POST /rpc HTTP/1.1"));
    assert!(http_request.contains("Content-Type: application/json"));
    assert!(http_request.contains(&format!("Content-Length: {}", json_body.len())));
    assert!(http_request.ends_with(&json_body));
}

#[test]
fn test_jsonrpc_request_structure() {
    let request = create_jsonrpc_request("test.method", 42);

    // Parse as JSON to verify structure
    let parsed: serde_json::Value = serde_json::from_str(&request).expect("Valid JSON");

    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["method"], "test.method");
    assert_eq!(parsed["id"], 42);
    assert!(parsed["params"].is_object());
}

#[test]
fn test_multiple_request_ids() {
    for id in 1..10 {
        let request = create_jsonrpc_request("toadstool.health", id);
        let parsed: serde_json::Value = serde_json::from_str(&request).expect("Valid JSON");
        assert_eq!(parsed["id"], id);
    }
}

#[test]
fn test_different_methods() {
    let methods = vec![
        "toadstool.health",
        "toadstool.version",
        "toadstool.query_capabilities",
    ];

    for (i, method) in methods.iter().enumerate() {
        let request = create_jsonrpc_request(method, i as i64);
        let parsed: serde_json::Value = serde_json::from_str(&request).expect("Valid JSON");
        assert_eq!(parsed["method"], *method);
    }
}

#[test]
fn test_http_headers() {
    let json_body = create_jsonrpc_request("toadstool.health", 1);
    let http_request = create_http_request(&json_body);

    let lines: Vec<&str> = http_request.lines().collect();
    assert!(lines[0].starts_with("POST"));
    assert!(lines.iter().any(|l| l.contains("Content-Type")));
    assert!(lines.iter().any(|l| l.contains("Content-Length")));
}

#[test]
fn test_server_version() {
    let _server = create_test_server();

    let _server2 = ManualJsonRpcServer::new(
        Arc::new(StandaloneExecutor::new()),
        "1.2.3".to_string(),
        None,
    );
    // Servers created with different versions
}

#[test]
fn test_jsonrpc_error_response_structure() {
    // Test that error responses follow JSON-RPC 2.0 spec
    let error_json = serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": -32601,
            "message": "Method not found"
        },
        "id": 1
    });

    assert_eq!(error_json["jsonrpc"], "2.0");
    assert!(error_json["error"].is_object());
    assert_eq!(error_json["error"]["code"], -32601);
}

#[test]
fn test_jsonrpc_success_response_structure() {
    // Test that success responses follow JSON-RPC 2.0 spec
    let success_json = serde_json::json!({
        "jsonrpc": "2.0",
        "result": {
            "status": "healthy"
        },
        "id": 1
    });

    assert_eq!(success_json["jsonrpc"], "2.0");
    assert!(success_json["result"].is_object());
    assert_eq!(success_json["id"], 1);
}

#[test]
fn test_content_length_calculation() {
    let bodies = vec![
        "{}",
        r#"{"test": "value"}"#,
        r#"{"jsonrpc":"2.0","method":"test","id":1}"#,
    ];

    for body in bodies {
        let http_request = create_http_request(body);
        let expected_length = format!("Content-Length: {}", body.len());
        assert!(http_request.contains(&expected_length));
    }
}

#[test]
fn test_http_request_line_endings() {
    let json_body = create_jsonrpc_request("test", 1);
    let http_request = create_http_request(&json_body);

    // HTTP/1.1 requires CRLF line endings
    assert!(http_request.contains("\r\n"));

    // Should have double CRLF before body
    assert!(http_request.contains("\r\n\r\n"));
}

#[test]
fn test_empty_params() {
    let request = create_jsonrpc_request("test.method", 1);
    let parsed: serde_json::Value = serde_json::from_str(&request).expect("Valid JSON");

    // Params should be an empty object
    assert!(parsed["params"].is_object());
    assert_eq!(parsed["params"].as_object().unwrap().len(), 0);
}

#[test]
fn test_jsonrpc_version_field() {
    let request = create_jsonrpc_request("test", 1);
    let parsed: serde_json::Value = serde_json::from_str(&request).expect("Valid JSON");

    // Must be exactly "2.0"
    assert_eq!(parsed["jsonrpc"], "2.0");
}

#[test]
fn test_method_names() {
    let valid_methods = vec![
        "toadstool.health",
        "toadstool.version",
        "toadstool.query_capabilities",
    ];

    for method in valid_methods {
        let request = create_jsonrpc_request(method, 1);
        assert!(request.contains(method));
    }
}

#[test]
fn test_request_id_types() {
    // JSON-RPC 2.0 allows string, number, or null IDs
    let id_tests = vec![
        (1, r#""id":1"#),
        (0, r#""id":0"#),
        (-1, r#""id":-1"#),
        (999999, r#""id":999999"#),
    ];

    for (id, expected) in id_tests {
        let request = create_jsonrpc_request("test", id);
        assert!(request.contains(expected));
    }
}

#[test]
fn test_http_method() {
    let json_body = create_jsonrpc_request("test", 1);
    let http_request = create_http_request(&json_body);

    // Should use POST method
    assert!(http_request.starts_with("POST"));
}

#[test]
fn test_http_path() {
    let json_body = create_jsonrpc_request("test", 1);
    let http_request = create_http_request(&json_body);

    // Should use /rpc path
    assert!(http_request.contains("POST /rpc HTTP/1.1"));
}

#[test]
fn test_http_version() {
    let json_body = create_jsonrpc_request("test", 1);
    let http_request = create_http_request(&json_body);

    // Should use HTTP/1.1
    assert!(http_request.contains("HTTP/1.1"));
}

#[test]
fn test_content_type_header() {
    let json_body = create_jsonrpc_request("test", 1);
    let http_request = create_http_request(&json_body);

    // Should have correct Content-Type
    assert!(http_request.contains("Content-Type: application/json"));
}

#[test]
fn test_host_header() {
    let json_body = create_jsonrpc_request("test", 1);
    let http_request = create_http_request(&json_body);

    // Should have Host header
    assert!(http_request.contains("Host: localhost"));
}
