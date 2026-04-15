// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_request_creation() {
    let request = Request::new(
        "verify",
        serde_json::json!({
            "signature": "abc123"
        }),
    );

    assert_eq!(request.operation, "verify");
    assert!(request.payload.is_object());
}

#[test]
fn test_response_success() {
    let response = Response {
        status: ResponseStatus::Success,
        data: Some(serde_json::json!({"result": true})),
        error: None,
    };

    assert!(response.is_success());
    assert!(response.data.is_some());
}

#[test]
fn test_protocol_selection_prefers_jsonrpc() {
    // jsonrpc and unix-socket preferred over http/grpc
    let protocols = vec![
        "grpc".to_string(),
        "http".to_string(),
        "jsonrpc".to_string(),
    ];
    let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
    assert_eq!(selected, "jsonrpc");
}

#[test]
fn test_protocol_selection_prefers_unix_over_http() {
    let protocols = vec!["http".to_string(), "unix".to_string()];
    let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
    assert_eq!(selected, "unix");
}

#[test]
fn test_protocol_selection_unix_socket_alias_normalizes_to_unix() {
    let protocols = vec!["http".to_string(), "unix-socket".to_string()];
    let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
    assert_eq!(
        selected, "unix",
        "unix-socket alias should normalize to canonical unix"
    );
}

#[test]
fn test_protocol_selection_http_over_grpc() {
    let protocols = vec!["grpc".to_string(), "http".to_string()];
    let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
    assert_eq!(selected, "http");
}

#[test]
fn test_protocol_selection_grpc_fallback() {
    let protocols = vec!["grpc".to_string()];
    let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
    assert_eq!(selected, "grpc");
}

#[test]
fn test_protocol_selection_empty() {
    let protocols: Vec<String> = vec![];
    let result = UniversalServiceAdapter::select_protocol(&protocols);
    assert!(result.is_err());
}

#[test]
fn test_socket_path_from_endpoint_unix_prefix() {
    let path = UniversalServiceAdapter::socket_path_from_endpoint("unix:///var/run/toadstool.sock")
        .unwrap();
    assert_eq!(path, std::path::Path::new("/var/run/toadstool.sock"));
}

#[test]
fn test_socket_path_from_endpoint_bare_path() {
    let path = UniversalServiceAdapter::socket_path_from_endpoint("/tmp/service.sock").unwrap();
    assert_eq!(path, std::path::Path::new("/tmp/service.sock"));
}

#[test]
fn test_socket_path_from_endpoint_http_fails() {
    let result = UniversalServiceAdapter::socket_path_from_endpoint("http://localhost:8080");
    assert!(result.is_err());
}

#[test]
fn test_protocol_selection_json_rpc_alias() {
    let protocols = vec!["json-rpc".to_string()];
    let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
    assert_eq!(selected, "jsonrpc");
}

#[test]
fn test_protocol_selection_first_available_fallback() {
    let protocols = vec!["custom".to_string(), "other".to_string()];
    let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
    assert_eq!(selected, "custom");
}

#[test]
fn test_response_data_success_with_data() {
    let response = Response {
        status: ResponseStatus::Success,
        data: Some(serde_json::json!({"result": true})),
        error: None,
    };
    let data = response.data().unwrap();
    assert!(data.get("result").and_then(|v| v.as_bool()).unwrap());
}

#[test]
fn test_response_data_success_no_data_returns_err() {
    let response = Response {
        status: ResponseStatus::Success,
        data: None,
        error: None,
    };
    let result = response.data();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No data"));
}

#[test]
fn test_response_data_error_status_returns_err() {
    let response = Response {
        status: ResponseStatus::Error,
        data: None,
        error: Some("Service error message".to_string()),
    };
    let result = response.data();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Service error"));
}

#[test]
fn test_response_data_error_unknown_message() {
    let response = Response {
        status: ResponseStatus::Error,
        data: None,
        error: None,
    };
    let result = response.data();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown"));
}

#[test]
fn test_response_is_success() {
    let success = Response {
        status: ResponseStatus::Success,
        data: Some(serde_json::json!({})),
        error: None,
    };
    let error = Response {
        status: ResponseStatus::Error,
        data: None,
        error: Some("err".to_string()),
    };
    assert!(success.is_success());
    assert!(!error.is_success());
}

#[test]
fn test_request_operation_into_string() {
    let request = Request::new(String::from("encrypt"), serde_json::json!({"key": "value"}));
    assert_eq!(request.operation, "encrypt");
}

#[test]
fn test_universal_adapter_builder() {
    use crate::ecosystem::capabilities::CapabilityResolver;
    use std::sync::Arc;
    use toadstool_common::infant_discovery::DiscoveryEngine;

    let discovery = Arc::new(DiscoveryEngine::new());
    let registry = Arc::new(crate::ecosystem::capabilities::CapabilityRegistry::new());
    let resolver = Arc::new(CapabilityResolver::new(discovery, registry));

    let adapter = UniversalServiceAdapter::new(resolver)
        .with_timeout(Duration::from_secs(10))
        .with_logging(true);

    // Verify construction
    drop(adapter);
}
