// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type system tests for protocol integration
//!
//! Tests for message types, health status, and service information structures.

use std::collections::HashMap;
use std::time::Duration;
use toadstool_integration_protocols::types::*;
use uuid::Uuid;

// ============================================================================
// Health Status Tests
// ============================================================================

#[test]
fn test_health_status_healthy() {
    let status = HealthStatus::Healthy;
    assert_eq!(status, HealthStatus::Healthy);
}

#[test]
fn test_health_status_degraded() {
    let status = HealthStatus::Degraded;
    assert_eq!(status, HealthStatus::Degraded);
}

#[test]
fn test_health_status_unhealthy() {
    let status = HealthStatus::Unhealthy;
    assert_eq!(status, HealthStatus::Unhealthy);
}

#[test]
fn test_health_status_unknown() {
    let status = HealthStatus::Unknown;
    assert_eq!(status, HealthStatus::Unknown);
}

#[test]
fn test_health_status_comparison() {
    assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
    assert_ne!(HealthStatus::Degraded, HealthStatus::Unhealthy);
    assert_ne!(HealthStatus::Unhealthy, HealthStatus::Unknown);
}

#[test]
fn test_health_status_serialization() {
    let status = HealthStatus::Healthy;
    let serialized = serde_json::to_string(&status).expect("Failed to serialize");
    let deserialized: HealthStatus =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(status, deserialized);
}

// ============================================================================
// Message Priority Tests
// ============================================================================

#[test]
fn test_message_priority_low() {
    let priority = MessagePriority::Low;
    assert_eq!(priority, MessagePriority::Low);
}

#[test]
fn test_message_priority_normal() {
    let priority = MessagePriority::Normal;
    assert_eq!(priority, MessagePriority::Normal);
}

#[test]
fn test_message_priority_high() {
    let priority = MessagePriority::High;
    assert_eq!(priority, MessagePriority::High);
}

#[test]
fn test_message_priority_critical() {
    let priority = MessagePriority::Critical;
    assert_eq!(priority, MessagePriority::Critical);
}

#[test]
fn test_message_priority_emergency() {
    let priority = MessagePriority::Emergency;
    assert_eq!(priority, MessagePriority::Emergency);
}

#[test]
fn test_message_priority_ordering() {
    assert!(MessagePriority::Low < MessagePriority::Normal);
    assert!(MessagePriority::Normal < MessagePriority::High);
    assert!(MessagePriority::High < MessagePriority::Critical);
    assert!(MessagePriority::Critical < MessagePriority::Emergency);
}

#[test]
fn test_message_priority_default() {
    let priority = MessagePriority::default();
    assert_eq!(priority, MessagePriority::Normal);
}

// ============================================================================
// Message Format Tests
// ============================================================================

#[test]
fn test_message_format_json() {
    let format = MessageFormat::Json;
    assert_eq!(format, MessageFormat::Json);
}

#[test]
fn test_message_format_messagepack() {
    let format = MessageFormat::MessagePack;
    assert_eq!(format, MessageFormat::MessagePack);
}

#[test]
fn test_message_format_cbor() {
    let format = MessageFormat::Cbor;
    assert_eq!(format, MessageFormat::Cbor);
}

#[test]
fn test_message_format_custom() {
    let format = MessageFormat::Custom("protobuf".to_string());
    if let MessageFormat::Custom(name) = format {
        assert_eq!(name, "protobuf");
    } else {
        panic!("Expected Custom format");
    }
}

// ============================================================================
// Transport Type Tests
// ============================================================================

#[test]
fn test_transport_type_http() {
    let transport = TransportType::Http;
    assert_eq!(transport, TransportType::Http);
}

#[test]
fn test_transport_type_websocket() {
    let transport = TransportType::TRpc;
    assert_eq!(transport, TransportType::TRpc);
}

#[test]
fn test_transport_type_trpc() {
    let transport = TransportType::TRpc;
    assert_eq!(transport, TransportType::TRpc);
}

#[test]
fn test_transport_type_tcp() {
    let transport = TransportType::Tcp;
    assert_eq!(transport, TransportType::Tcp);
}

#[test]
fn test_transport_type_udp() {
    let transport = TransportType::Udp;
    assert_eq!(transport, TransportType::Udp);
}

#[test]
fn test_transport_type_custom() {
    let transport = TransportType::Custom("grpc".to_string());
    if let TransportType::Custom(name) = transport {
        assert_eq!(name, "grpc");
    } else {
        panic!("Expected Custom transport");
    }
}

// ============================================================================
// Protocol Message Tests
// ============================================================================

#[test]
fn test_protocol_message_creation() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "request".to_string(),
        source: "service-a".to_string(),
        destination: Some("service-b".to_string()),
        payload: serde_json::json!({"action": "compute"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: Some(Uuid::new_v4()),
        reply_to: None,
        ttl: Some(Duration::from_secs(300)),
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.source, "service-a");
    assert_eq!(message.destination, Some("service-b".to_string()));
    assert_eq!(message.message_type, "request");
    assert_eq!(message.priority, MessagePriority::Normal);
}

#[test]
fn test_protocol_message_with_headers() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token123".to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "auth_request".to_string(),
        source: "client".to_string(),
        destination: Some("auth-server".to_string()),
        payload: serde_json::json!({}),
        headers,
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::High,
    };

    assert_eq!(message.headers.len(), 2);
    assert!(message.headers.contains_key("Authorization"));
}

// ============================================================================
// Service Info Tests
// ============================================================================

#[test]
fn test_service_info_creation() {
    let endpoint = ServiceEndpoint {
        id: "ep-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: Some("/api".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let service = ServiceInfo {
        id: "svc-123".to_string(),
        name: "ToadStool Compute".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![endpoint],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec!["execute".to_string(), "schedule".to_string()],
    };

    assert_eq!(service.id, "svc-123");
    assert_eq!(service.name, "ToadStool Compute");
    assert_eq!(service.health_status, HealthStatus::Healthy);
    assert_eq!(service.capabilities.len(), 2);
    assert_eq!(service.endpoints.len(), 1);
}

#[test]
fn test_service_info_multiple_endpoints() {
    let ep1 = ServiceEndpoint {
        id: "ep-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let ep2 = ServiceEndpoint {
        id: "ep-2".to_string(),
        transport: TransportType::TRpc,
        address: "localhost".to_string(),
        port: 9000,
        path: Some("/ws".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let service = ServiceInfo {
        id: "multi-endpoint-service".to_string(),
        name: "Multi-Endpoint Service".to_string(),
        version: "2.0.0".to_string(),
        endpoints: vec![ep1, ep2],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert_eq!(service.endpoints.len(), 2);
    assert_eq!(service.endpoints[0].transport, TransportType::Http);
    assert_eq!(service.endpoints[1].transport, TransportType::TRpc);
}

// ============================================================================
// Service Endpoint Tests
// ============================================================================

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

// ============================================================================
// Auth Type Tests
// ============================================================================

#[test]
fn test_auth_type_none() {
    let auth = AuthType::None;
    if let AuthType::None = auth {
        // Success
    } else {
        panic!("Expected None auth type");
    }
}

#[test]
fn test_auth_type_bearer() {
    let auth = AuthType::Bearer;
    if let AuthType::Bearer = auth {
        // Success
    } else {
        panic!("Expected Bearer auth type");
    }
}

#[test]
fn test_auth_type_custom() {
    let auth = AuthType::Custom("oauth2".to_string());
    if let AuthType::Custom(name) = auth {
        assert_eq!(name, "oauth2");
    } else {
        panic!("Expected Custom auth type");
    }
}

// ============================================================================
// Protocol Error Tests
// ============================================================================

#[test]
fn test_protocol_error_connection() {
    let error = ProtocolError::Connection("Connection refused".to_string());
    assert!(error.to_string().contains("Connection failed"));
}

#[test]
fn test_protocol_error_authentication() {
    let error = ProtocolError::Authentication("Invalid credentials".to_string());
    assert!(error.to_string().contains("Authentication failed"));
}

#[test]
fn test_protocol_error_timeout() {
    let error = ProtocolError::Timeout("Request timeout after 30s".to_string());
    assert!(error.to_string().contains("Timeout"));
}

// ============================================================================
// Extended Types Tests - Edge Cases and Validation
// ============================================================================

#[test]
fn test_protocol_message_with_ttl() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "command".to_string(),
        source: "controller".to_string(),
        destination: Some("worker".to_string()),
        payload: serde_json::json!({"cmd": "execute"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: Some(Duration::from_secs(60)),
        priority: MessagePriority::High,
    };

    assert!(message.ttl.is_some());
    assert_eq!(message.ttl.unwrap(), Duration::from_secs(60));
}

#[test]
fn test_protocol_message_without_ttl() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "notification".to_string(),
        source: "notifier".to_string(),
        destination: None,
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert!(message.ttl.is_none());
}

#[test]
fn test_protocol_message_correlation_id() {
    let corr_id = Uuid::new_v4();
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "reply".to_string(),
        source: "responder".to_string(),
        destination: Some("requester".to_string()),
        payload: serde_json::json!({"status": "ok"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: Some(corr_id),
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.correlation_id, Some(corr_id));
}

#[test]
fn test_protocol_message_reply_to() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "request".to_string(),
        source: "client".to_string(),
        destination: Some("server".to_string()),
        payload: serde_json::json!({"query": "data"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: Some("client-queue".to_string()),
        ttl: Some(Duration::from_secs(300)),
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.reply_to, Some("client-queue".to_string()));
}

#[test]
fn test_protocol_message_broadcast() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "broadcast".to_string(),
        source: "broadcaster".to_string(),
        destination: None, // No specific destination = broadcast
        payload: serde_json::json!({"announcement": "System update"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::High,
    };

    assert!(message.destination.is_none());
    assert_eq!(message.message_type, "broadcast");
}

#[test]
fn test_service_info_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("region".to_string(), "us-west-2".to_string());
    metadata.insert("zone".to_string(), "a".to_string());

    let service = ServiceInfo {
        id: "svc-meta".to_string(),
        name: "Metadata Service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata,
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert_eq!(service.metadata.len(), 2);
    assert_eq!(
        service.metadata.get("region"),
        Some(&"us-west-2".to_string())
    );
}

#[test]
fn test_service_info_with_capabilities() {
    let service = ServiceInfo {
        id: "capable-svc".to_string(),
        name: "Capable Service".to_string(),
        version: "2.1.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![
            "compute".to_string(),
            "storage".to_string(),
            "ml".to_string(),
        ],
    };

    assert_eq!(service.capabilities.len(), 3);
    assert!(service.capabilities.contains(&"compute".to_string()));
}

#[test]
fn test_service_info_version_parsing() {
    let service = ServiceInfo {
        id: "versioned".to_string(),
        name: "Versioned Service".to_string(),
        version: "3.2.1-beta".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert!(service.version.contains("beta"));
    assert!(service.version.starts_with("3.2.1"));
}

#[test]
fn test_message_format_messagepack_serialization() {
    let format = MessageFormat::MessagePack;
    let serialized = serde_json::to_string(&format).expect("Failed to serialize");
    let deserialized: MessageFormat =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(format, deserialized);
}

#[test]
fn test_message_format_cbor_serialization() {
    let format = MessageFormat::Cbor;
    let serialized = serde_json::to_string(&format).expect("Failed to serialize");
    let deserialized: MessageFormat =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(format, deserialized);
}

#[test]
fn test_transport_type_tcp_serialization() {
    let transport = TransportType::Tcp;
    let serialized = serde_json::to_string(&transport).expect("Failed to serialize");
    let deserialized: TransportType =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(transport, deserialized);
}

#[test]
fn test_transport_type_udp_serialization() {
    let transport = TransportType::Udp;
    let serialized = serde_json::to_string(&transport).expect("Failed to serialize");
    let deserialized: TransportType =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(transport, deserialized);
}

#[test]
fn test_auth_type_api_key() {
    let auth = AuthType::ApiKey;
    match auth {
        AuthType::ApiKey => (), // Success
        _ => panic!("Expected ApiKey auth type"),
    }
}

#[test]
fn test_auth_type_mutual_tls() {
    let auth = AuthType::MutualTls;
    match auth {
        AuthType::MutualTls => (), // Success
        _ => panic!("Expected MutualTls auth type"),
    }
}

#[test]
fn test_auth_type_jwt() {
    let auth = AuthType::Jwt;
    match auth {
        AuthType::Jwt => (), // Success
        _ => panic!("Expected JWT auth type"),
    }
}

#[test]
fn test_protocol_error_authorization() {
    let error = ProtocolError::Authorization("Access denied".to_string());
    assert!(error.to_string().contains("Authorization failed"));
}

#[test]
fn test_protocol_error_negotiation() {
    let error = ProtocolError::Negotiation("Protocol version mismatch".to_string());
    assert!(error.to_string().contains("Protocol negotiation failed"));
}

#[test]
fn test_protocol_error_serialization() {
    let error = ProtocolError::Serialization("Invalid JSON".to_string());
    assert!(error.to_string().contains("Serialization error"));
}

#[test]
fn test_protocol_error_transport() {
    let error = ProtocolError::Transport("Network unreachable".to_string());
    assert!(error.to_string().contains("Transport error"));
}

#[test]
fn test_protocol_error_discovery() {
    let error = ProtocolError::Discovery("Service not found".to_string());
    assert!(error.to_string().contains("Service discovery error"));
}

#[test]
fn test_protocol_error_routing() {
    let error = ProtocolError::Routing("No route to destination".to_string());
    assert!(error.to_string().contains("Message routing error"));
}

#[test]
fn test_protocol_error_internal() {
    let error = ProtocolError::Internal("Unexpected condition".to_string());
    assert!(error.to_string().contains("Internal error"));
}

#[test]
fn test_message_priority_comparison() {
    assert!(MessagePriority::Emergency > MessagePriority::Critical);
    assert!(MessagePriority::Critical > MessagePriority::High);
    assert!(MessagePriority::High > MessagePriority::Normal);
    assert!(MessagePriority::Normal > MessagePriority::Low);
}

#[test]
fn test_message_priority_serialization() {
    let priority = MessagePriority::Critical;
    let serialized = serde_json::to_string(&priority).expect("Failed to serialize");
    let deserialized: MessagePriority =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(priority, deserialized);
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
fn test_protocol_message_empty_payload() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "ping".to_string(),
        source: "pinger".to_string(),
        destination: Some("target".to_string()),
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: Some(Duration::from_secs(30)),
        priority: MessagePriority::Low,
    };

    assert_eq!(message.payload, serde_json::json!({}));
}

#[test]
fn test_protocol_message_complex_payload() {
    let payload = serde_json::json!({
        "workload": {
            "type": "computation",
            "params": {
                "iterations": 1000,
                "precision": "high"
            }
        },
        "resources": {
            "cpu": 4,
            "memory": "8GB"
        }
    });

    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "workload_request".to_string(),
        source: "scheduler".to_string(),
        destination: Some("worker-01".to_string()),
        payload,
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: Some("scheduler-queue".to_string()),
        ttl: Some(Duration::from_secs(600)),
        priority: MessagePriority::Normal,
    };

    assert!(message.payload.is_object());
    assert!(message.payload["workload"]["type"].is_string());
}

#[test]
fn test_service_info_no_endpoints() {
    let service = ServiceInfo {
        id: "no-endpoints".to_string(),
        name: "Configuring Service".to_string(),
        version: "0.1.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Unknown,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert!(service.endpoints.is_empty());
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

#[test]
fn test_transport_type_custom_serialization() {
    let transport = TransportType::Custom("quic".to_string());
    let serialized = serde_json::to_string(&transport).expect("Failed to serialize");
    let deserialized: TransportType =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let TransportType::Custom(name) = deserialized {
        assert_eq!(name, "quic");
    } else {
        panic!("Expected Custom transport type");
    }
}

#[test]
fn test_message_format_custom_serialization() {
    let format = MessageFormat::Custom("avro".to_string());
    let serialized = serde_json::to_string(&format).expect("Failed to serialize");
    let deserialized: MessageFormat =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let MessageFormat::Custom(name) = deserialized {
        assert_eq!(name, "avro");
    } else {
        panic!("Expected Custom format");
    }
}

#[test]
fn test_auth_type_custom_serialization() {
    let auth = AuthType::Custom("saml".to_string());
    let serialized = serde_json::to_string(&auth).expect("Failed to serialize");
    let deserialized: AuthType = serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let AuthType::Custom(name) = deserialized {
        assert_eq!(name, "saml");
    } else {
        panic!("Expected Custom auth type");
    }
}

#[test]
fn test_protocol_message_unique_ids() {
    let msg1 = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "test".to_string(),
        source: "source".to_string(),
        destination: None,
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    let msg2 = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "test".to_string(),
        source: "source".to_string(),
        destination: None,
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert_ne!(msg1.id, msg2.id);
}

#[test]
fn test_service_info_timestamp() {
    let now = std::time::SystemTime::now();
    let service = ServiceInfo {
        id: "timestamp-test".to_string(),
        name: "Timestamp Test".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: now,
        capabilities: vec![],
    };

    assert_eq!(service.last_seen, now);
}
