//! Comprehensive tests for protocol integration types

use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use toadstool_integration_protocols::types::*;
use uuid::Uuid;

// ============================================================================
// MessageFormat Tests
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
        panic!("Expected Custom variant");
    }
}

#[test]
fn test_message_format_clone() {
    let format = MessageFormat::Json;
    let cloned = format.clone();
    assert_eq!(format, cloned);
}

// ============================================================================
// TransportType Tests
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
    let transport = TransportType::Custom("quic".to_string());

    if let TransportType::Custom(name) = transport {
        assert_eq!(name, "quic");
    } else {
        panic!("Expected Custom variant");
    }
}

// ============================================================================
// AuthType Tests
// ============================================================================

#[test]
fn test_auth_type_none() {
    let auth = AuthType::None;
    assert!(matches!(auth, AuthType::None));
}

#[test]
fn test_auth_type_bearer() {
    let auth = AuthType::Bearer;
    assert!(matches!(auth, AuthType::Bearer));
}

#[test]
fn test_auth_type_apikey() {
    let auth = AuthType::ApiKey;
    assert!(matches!(auth, AuthType::ApiKey));
}

#[test]
fn test_auth_type_mutual_tls() {
    let auth = AuthType::MutualTls;
    assert!(matches!(auth, AuthType::MutualTls));
}

#[test]
fn test_auth_type_jwt() {
    let auth = AuthType::Jwt;
    assert!(matches!(auth, AuthType::Jwt));
}

#[test]
fn test_auth_type_custom() {
    let auth = AuthType::Custom("oauth2".to_string());

    if let AuthType::Custom(name) = auth {
        assert_eq!(name, "oauth2");
    } else {
        panic!("Expected Custom variant");
    }
}

// ============================================================================
// MessagePriority Tests
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
// HealthStatus Tests
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
fn test_health_status_clone() {
    let status = HealthStatus::Healthy;
    let cloned = status.clone();
    assert_eq!(status, cloned);
}

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
        payload: serde_json::json!({"action": "ping"}),
        headers: HashMap::new(),
        timestamp: Utc::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.message_type, "request");
    assert_eq!(message.source, "service-a");
}

#[test]
fn test_protocol_message_with_correlation() {
    let correlation = Uuid::new_v4();
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "response".to_string(),
        source: "service-b".to_string(),
        destination: Some("service-a".to_string()),
        payload: serde_json::json!({"result": "pong"}),
        headers: HashMap::new(),
        timestamp: Utc::now(),
        format: MessageFormat::Json,
        correlation_id: Some(correlation),
        reply_to: Some("service-a".to_string()),
        ttl: Some(Duration::from_secs(60)),
        priority: MessagePriority::High,
    };

    assert_eq!(message.correlation_id, Some(correlation));
    assert_eq!(message.priority, MessagePriority::High);
}

#[test]
fn test_protocol_message_with_ttl() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "notification".to_string(),
        source: "service-c".to_string(),
        destination: None,
        payload: serde_json::json!({"event": "update"}),
        headers: HashMap::new(),
        timestamp: Utc::now(),
        format: MessageFormat::MessagePack,
        correlation_id: None,
        reply_to: None,
        ttl: Some(Duration::from_secs(300)),
        priority: MessagePriority::Low,
    };

    assert!(message.ttl.is_some());
    assert_eq!(message.destination, None);
}

// ============================================================================
// ServiceEndpoint Tests
// ============================================================================

#[test]
fn test_service_endpoint_http() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Http,
        address: "api.example.com".to_string(),
        port: 443,
        path: Some("/v1/api".to_string()),
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.transport, TransportType::Http);
    assert!(endpoint.tls_enabled);
}

#[test]
fn test_service_endpoint_websocket() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-2".to_string(),
        transport: TransportType::TRpc,
        address: "ws.example.com".to_string(),
        port: 8080,
        path: Some("/ws".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.transport, TransportType::TRpc);
    assert!(!endpoint.tls_enabled);
}

#[test]
fn test_service_endpoint_tcp() {
    let endpoint = ServiceEndpoint {
        id: "endpoint-3".to_string(),
        transport: TransportType::Tcp,
        address: "10.0.0.5".to_string(),
        port: 9000,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Degraded,
    };

    assert_eq!(endpoint.health_status, HealthStatus::Degraded);
}

// ============================================================================
// ServiceInfo Tests
// ============================================================================

#[test]
fn test_service_info_creation() {
    let service = ServiceInfo {
        id: "service-1".to_string(),
        name: "API Gateway".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: Utc::now(),
        capabilities: vec!["http".to_string(), "websocket".to_string()],
    };

    assert_eq!(service.name, "API Gateway");
    assert_eq!(service.capabilities.len(), 2);
}

#[test]
fn test_service_info_with_endpoints() {
    let endpoint = ServiceEndpoint {
        id: "ep-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let service = ServiceInfo {
        id: "service-2".to_string(),
        name: "Data Service".to_string(),
        version: "2.0.0".to_string(),
        endpoints: vec![endpoint],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: Utc::now(),
        capabilities: vec![],
    };

    assert_eq!(service.endpoints.len(), 1);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_all_message_formats() {
    let formats = [
        MessageFormat::Json,
        MessageFormat::MessagePack,
        MessageFormat::Cbor,
        MessageFormat::Custom("avro".to_string()),
    ];

    assert_eq!(formats.len(), 4);
}

#[test]
fn test_all_transport_types() {
    let transports = [
        TransportType::Http,
        TransportType::TRpc,
        TransportType::TRpc,
        TransportType::Tcp,
        TransportType::Udp,
    ];

    assert_eq!(transports.len(), 5);
}

#[test]
fn test_all_auth_types() {
    let auth_types = [
        AuthType::None,
        AuthType::Bearer,
        AuthType::ApiKey,
        AuthType::MutualTls,
        AuthType::Jwt,
    ];

    assert_eq!(auth_types.len(), 5);
}

#[test]
fn test_all_message_priorities() {
    let priorities = [
        MessagePriority::Low,
        MessagePriority::Normal,
        MessagePriority::High,
        MessagePriority::Critical,
        MessagePriority::Emergency,
    ];

    assert_eq!(priorities.len(), 5);
}

#[test]
fn test_all_health_statuses() {
    let statuses = [
        HealthStatus::Healthy,
        HealthStatus::Degraded,
        HealthStatus::Unhealthy,
        HealthStatus::Unknown,
    ];

    assert_eq!(statuses.len(), 4);
}

#[test]
fn test_message_serialization() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: "test".to_string(),
        source: "test-source".to_string(),
        destination: None,
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: Utc::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    let json = serde_json::to_string(&message).expect("Failed to serialize");
    let deserialized: ProtocolMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(message.message_type, deserialized.message_type);
}
