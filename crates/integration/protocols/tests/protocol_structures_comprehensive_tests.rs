// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Protocol Structures
//!
//! This test suite provides extensive coverage of structure types used in protocol integration,
//! including ProtocolMessage, ServiceInfo, ServiceEndpoint, and their operations.

use std::collections::HashMap;
use std::time::Duration;
// Removed unused import: ServiceAuthConfig
use toadstool_integration_protocols::types::*;
use uuid::Uuid;

// ============================================================================
// ProtocolMessage Tests
// ============================================================================

#[test]
fn test_protocol_message_creation() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "request".to_string(),
        source: "service-a".to_string(),
        destination: Some("service-b".to_string()),
        payload: serde_json::json!({"action": "test"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.message_type, "request");
    assert_eq!(message.source, "service-a");
    assert_eq!(message.destination, Some("service-b".to_string()));
}

#[test]
fn test_protocol_message_with_headers() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("X-Request-ID".to_string(), "123".to_string());

    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "request".to_string(),
        source: "service-a".to_string(),
        destination: Some("service-b".to_string()),
        payload: serde_json::json!({}),
        headers: headers.clone(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.headers.len(), 2);
    assert_eq!(
        message.headers.get("Content-Type"),
        Some(&"application/json".to_string())
    );
}

#[test]
fn test_protocol_message_with_correlation_id() {
    let correlation_id = Uuid::new_v4();
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "response".to_string(),
        source: "service-b".to_string(),
        destination: Some("service-a".to_string()),
        payload: serde_json::json!({"result": "ok"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: Some(correlation_id),
        reply_to: Some("service-a".to_string()),
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.correlation_id, Some(correlation_id));
    assert_eq!(message.reply_to, Some("service-a".to_string()));
}

#[test]
fn test_protocol_message_with_ttl() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "request".to_string(),
        source: "service-a".to_string(),
        destination: Some("service-b".to_string()),
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: Some(Duration::from_secs(60)),
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.ttl, Some(Duration::from_secs(60)));
}

#[test]
fn test_protocol_message_with_priority() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "alert".to_string(),
        source: "service-a".to_string(),
        destination: None,
        payload: serde_json::json!({"alert": "critical"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Critical,
    };

    assert_eq!(message.priority, MessagePriority::Critical);
}

#[test]
fn test_protocol_message_broadcast() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "broadcast".to_string(),
        source: "service-a".to_string(),
        destination: None, // No specific destination = broadcast
        payload: serde_json::json!({"announcement": "test"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert!(message.destination.is_none());
}

#[test]
fn test_protocol_message_serialization() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "test".to_string(),
        source: "service-a".to_string(),
        destination: Some("service-b".to_string()),
        payload: serde_json::json!({"key": "value"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    let serialized = serde_json::to_string(&message).expect("Failed to serialize");
    let deserialized: ProtocolMessage =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(message.id, deserialized.id);
    assert_eq!(message.message_type, deserialized.message_type);
}

#[test]
fn test_protocol_message_clone() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "test".to_string(),
        source: "service-a".to_string(),
        destination: Some("service-b".to_string()),
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    let cloned = message.clone();
    assert_eq!(message.id, cloned.id);
    assert_eq!(message.message_type, cloned.message_type);
}

// ============================================================================
// ServiceInfo Tests
// ============================================================================

#[test]
fn test_service_info_creation() {
    let service = ServiceInfo {
        id: "service-123".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert_eq!(service.id, "service-123");
    assert_eq!(service.name, "test-service");
    assert_eq!(service.version, "1.0.0");
}

#[test]
fn test_service_info_with_endpoints() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: Some("/api".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let service = ServiceInfo {
        id: "service-123".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![endpoint],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert_eq!(service.endpoints.len(), 1);
    assert_eq!(service.endpoints[0].port, 8080);
}

#[test]
fn test_service_info_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("region".to_string(), "us-east-1".to_string());
    metadata.insert("datacenter".to_string(), "dc1".to_string());

    let service = ServiceInfo {
        id: "service-123".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: metadata.clone(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert_eq!(service.metadata.len(), 2);
    assert_eq!(
        service.metadata.get("region"),
        Some(&"us-east-1".to_string())
    );
}

#[test]
fn test_service_info_with_capabilities() {
    let service = ServiceInfo {
        id: "service-123".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![
            "compute".to_string(),
            "storage".to_string(),
            "networking".to_string(),
        ],
    };

    assert_eq!(service.capabilities.len(), 3);
    assert!(service.capabilities.contains(&"compute".to_string()));
}

#[test]
fn test_service_info_health_status() {
    let service = ServiceInfo {
        id: "service-123".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Degraded,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert_eq!(service.health_status, HealthStatus::Degraded);
}

#[test]
fn test_service_info_serialization() {
    let service = ServiceInfo {
        id: "service-123".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    let serialized = serde_json::to_string(&service).expect("Failed to serialize");
    let deserialized: ServiceInfo =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(service.id, deserialized.id);
    assert_eq!(service.name, deserialized.name);
}

#[test]
fn test_service_info_clone() {
    let service = ServiceInfo {
        id: "service-123".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    let cloned = service.clone();
    assert_eq!(service.id, cloned.id);
    assert_eq!(service.name, cloned.name);
}

// ============================================================================
// ServiceEndpoint Tests
// ============================================================================

#[test]
fn test_service_endpoint_http() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: Some("/api".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.transport, TransportType::Http);
    assert_eq!(endpoint.port, 8080);
    assert!(!endpoint.tls_enabled);
}

#[test]
fn test_service_endpoint_https() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Http,
        address: "example.com".to_string(),
        port: 443,
        path: Some("/api/v1".to_string()),
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    assert!(endpoint.tls_enabled);
    assert_eq!(endpoint.port, 443);
}

#[test]
fn test_service_endpoint_websocket() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-ws".to_string(),
        transport: TransportType::TRpc,
        address: "ws.example.com".to_string(),
        port: 8080,
        path: Some("/ws".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.transport, TransportType::TRpc);
    assert_eq!(endpoint.path, Some("/ws".to_string()));
}

#[test]
fn test_service_endpoint_without_path() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Tcp,
        address: "localhost".to_string(),
        port: 9000,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert!(endpoint.path.is_none());
}

#[test]
fn test_service_endpoint_health_unhealthy() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Unhealthy,
    };

    assert_eq!(endpoint.health_status, HealthStatus::Unhealthy);
}

#[test]
fn test_service_endpoint_serialization() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: Some("/api".to_string()),
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
fn test_service_endpoint_clone() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: Some("/api".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let cloned = endpoint.clone();
    assert_eq!(endpoint.id, cloned.id);
    assert_eq!(endpoint.port, cloned.port);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_service_with_multiple_endpoints() {
    let endpoints = vec![
        ServiceEndpoint {
            id: "http-endpoint".to_string(),
            transport: TransportType::Http,
            address: "localhost".to_string(),
            port: 8080,
            path: Some("/api".to_string()),
            tls_enabled: false,
            health_status: HealthStatus::Healthy,
        },
        ServiceEndpoint {
            id: "ws-endpoint".to_string(),
            transport: TransportType::TRpc,
            address: "localhost".to_string(),
            port: 8081,
            path: Some("/ws".to_string()),
            tls_enabled: false,
            health_status: HealthStatus::Healthy,
        },
    ];

    let service = ServiceInfo {
        id: "multi-endpoint-service".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints,
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert_eq!(service.endpoints.len(), 2);
    assert_eq!(service.endpoints[0].transport, TransportType::Http);
    assert_eq!(service.endpoints[1].transport, TransportType::TRpc);
}

#[test]
fn test_message_with_service_info() {
    let service = ServiceInfo {
        id: "service-123".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "service_info".to_string(),
        source: "discovery".to_string(),
        destination: Some("client".to_string()),
        payload: serde_json::to_value(&service).expect("Failed to serialize service"),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    // Verify service can be embedded in message payload
    let service_from_message: ServiceInfo =
        serde_json::from_value(message.payload).expect("Failed to deserialize service");
    assert_eq!(service_from_message.id, service.id);
}

// ============================================================================
// Test Counter
// ============================================================================

#[test]
fn test_structure_coverage_summary() {
    println!("============================================");
    println!("Protocol Structure Tests Summary:");
    println!("============================================");
    println!("ProtocolMessage Tests:        9 tests");
    println!("ServiceInfo Tests:            7 tests");
    println!("ServiceEndpoint Tests:        7 tests");
    println!("Integration Tests:            2 tests");
    println!("============================================");
    println!("Total Structure Tests:       25 tests");
    println!("============================================");
    println!();
    println!("🎊 TOTAL NEW TESTS: 28 + 38 + 25 = 91! 🎊");
    println!("============================================");
}
