// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn test_service_endpoint_basic() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Http,
        address: "api.example.com".to_string(),
        port: 443,
        path: Some("/v1/api".to_string()),
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.id, "endpoint-1");
    assert_eq!(endpoint.address, "api.example.com");
    assert_eq!(endpoint.port, 443);
    assert!(endpoint.tls_enabled);
}

#[test]
fn test_service_endpoint_serialization() {
    let endpoint = ServiceEndpoint {
        id: "test-endpoint".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let serialized = serde_json::to_string(&endpoint).expect("Failed to serialize");
    let deserialized: ServiceEndpoint =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(endpoint.id, deserialized.id);
    assert_eq!(endpoint.port, deserialized.port);
}

#[test]
fn test_service_endpoint_equality() {
    let ep1 = ServiceEndpoint {
        id: "same".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let ep2 = ServiceEndpoint {
        id: "same".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(ep1.id, ep2.id);
    assert_eq!(ep1.port, ep2.port);
}

#[test]
fn test_service_endpoint_standard_http_port() {
    let endpoint = ServiceEndpoint {
        id: "standard-http".to_string(),
        transport: TransportType::Http,
        address: "example.com".to_string(),
        port: 80,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.port, 80);
    assert!(!endpoint.tls_enabled);
}

#[test]
fn test_service_endpoint_standard_https_port() {
    let endpoint = ServiceEndpoint {
        id: "standard-https".to_string(),
        transport: TransportType::Http,
        address: "secure.example.com".to_string(),
        port: 443,
        path: None,
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.port, 443);
    assert!(endpoint.tls_enabled);
}
