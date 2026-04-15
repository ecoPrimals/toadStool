// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::*;

#[test]
fn test_service_endpoint_url() {
    let endpoint = ServiceEndpoint::http("localhost", 8080).with_path("/api/v1");

    assert_eq!(endpoint.url(), "http://localhost:8080/api/v1");
}

#[test]
fn test_service_endpoint_http() {
    let endpoint = ServiceEndpoint::http("example.com", 8080);
    assert_eq!(endpoint.protocol, "http");
    assert_eq!(endpoint.address, "example.com");
    assert_eq!(endpoint.port, 8080);
    assert_eq!(endpoint.url(), "http://example.com:8080");
}

#[test]
fn test_service_endpoint_https() {
    let endpoint = ServiceEndpoint::https("secure.example.com", 443);
    assert_eq!(endpoint.protocol, "https");
    assert_eq!(endpoint.port, 443);
    assert_eq!(endpoint.url(), "https://secure.example.com:443");
}

#[test]
fn test_service_endpoint_grpc() {
    let endpoint = ServiceEndpoint::grpc("grpc.example.com", 9090);
    assert_eq!(endpoint.protocol, "grpc");
    assert_eq!(endpoint.url(), "grpc://grpc.example.com:9090");
}

#[test]
fn test_service_endpoint_jsonrpc_polling() {
    // JSON-RPC 2.0 polling (replacement for deprecated WebSocket)
    let endpoint = ServiceEndpoint::http("api.example.com", 8081).with_path("/jsonrpc");
    assert_eq!(endpoint.protocol, "http");
    assert_eq!(endpoint.url(), "http://api.example.com:8081/jsonrpc");
}

#[test]
fn test_service_endpoint_with_path() {
    let endpoint = ServiceEndpoint::http("api.example.com", 8080).with_path("/v2/compute");
    assert_eq!(endpoint.url(), "http://api.example.com:8080/v2/compute");
}

#[test]
fn test_service_endpoint_with_metadata() {
    let endpoint = ServiceEndpoint::http("api.example.com", 8080)
        .with_metadata("region", "us-west")
        .with_metadata("tier", "production");

    assert_eq!(
        endpoint.metadata.get("region"),
        Some(&"us-west".to_string())
    );
    assert_eq!(
        endpoint.metadata.get("tier"),
        Some(&"production".to_string())
    );
}

#[test]
fn test_service_endpoint_clone() {
    let endpoint1 = ServiceEndpoint::http("localhost", 8080);
    let endpoint2 = endpoint1.clone();

    assert_eq!(endpoint1.protocol, endpoint2.protocol);
    assert_eq!(endpoint1.address, endpoint2.address);
    assert_eq!(endpoint1.port, endpoint2.port);
}

#[test]
fn test_service_endpoint_debug_formatting() {
    let ep = ServiceEndpoint::http("localhost", 8080);
    let debug_str = format!("{ep:?}");
    assert!(!debug_str.is_empty());
    assert!(debug_str.contains("localhost"));
}

#[test]
fn test_service_endpoint_with_path_none_remains_none() {
    let endpoint = ServiceEndpoint::http("localhost", 8080);
    assert!(endpoint.path.is_none());
}

#[test]
fn test_service_endpoint_grpc_with_path() {
    let endpoint = ServiceEndpoint::grpc("grpc.example.com", 50051).with_path("/service.Greeter");
    assert_eq!(
        endpoint.url(),
        "grpc://grpc.example.com:50051/service.Greeter"
    );
}

#[test]
fn test_service_endpoint_https_with_path() {
    let endpoint = ServiceEndpoint::https("api.secure.com", 443).with_path("/v2/graphql");
    assert_eq!(endpoint.url(), "https://api.secure.com:443/v2/graphql");
}

#[test]
fn test_service_endpoint_eq() {
    let ep1 = ServiceEndpoint::http("localhost", 8080);
    let ep2 = ServiceEndpoint::http("localhost", 8080);
    assert_eq!(ep1, ep2);
}
