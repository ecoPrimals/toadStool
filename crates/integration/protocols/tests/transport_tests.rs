// SPDX-License-Identifier: AGPL-3.0-or-later
//! Transport layer tests for protocol communication
//!
//! Tests for HTTP transport, message routing, and protocol handling.

use std::collections::HashMap;
use std::time::Duration;
use toadstool_integration_protocols::transport::*;
use toadstool_integration_protocols::types::*;
use uuid::Uuid;

#[test]
fn test_http_transport_creation() {
    let transport = HttpTransport::new();
    assert_eq!(transport.transport_type(), TransportType::Http);
}

#[test]
fn test_http_transport_default() {
    let transport = HttpTransport::default();
    assert_eq!(transport.transport_type(), TransportType::Http);
}

// WebSocket removed — use JSON-RPC 2.0 (biomeOS/songbird)

#[test]
fn test_trpc_transport_creation() {
    let transport = TRpcTransport::new();
    assert_eq!(transport.transport_type(), TransportType::TRpc);
}

#[test]
fn test_trpc_transport_default() {
    let transport = TRpcTransport::default();
    assert_eq!(transport.transport_type(), TransportType::TRpc);
}

#[test]
fn test_http_transport_supports_http_endpoint() {
    let transport = HttpTransport::new();
    let endpoint = ServiceEndpoint {
        id: "endpoint-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: Some("/api".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert!(transport.supports_endpoint(&endpoint));
}

#[test]
fn test_http_transport_does_not_support_trpc_endpoint() {
    let transport = HttpTransport::new();
    let endpoint = ServiceEndpoint {
        id: "endpoint-2".to_string(),
        transport: TransportType::TRpc,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert!(!transport.supports_endpoint(&endpoint));
}

#[test]
fn test_trpc_transport_supports_trpc_endpoint() {
    let transport = TRpcTransport::new();
    let endpoint = ServiceEndpoint {
        id: "endpoint-3".to_string(),
        transport: TransportType::TRpc,
        address: "localhost".to_string(),
        port: 9000,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert!(transport.supports_endpoint(&endpoint));
}

#[test]
fn test_trpc_transport_supports_trpc_endpoint_with_path() {
    let transport = TRpcTransport::new();
    let endpoint = ServiceEndpoint {
        id: "endpoint-4".to_string(),
        transport: TransportType::TRpc,
        address: "api.example.com".to_string(),
        port: 443,
        path: Some("/trpc".to_string()),
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    assert!(transport.supports_endpoint(&endpoint));
}

#[test]
fn test_transport_manager_creation() {
    let manager = TransportManager::new();
    let transports = manager.get_supported_transports();

    assert_eq!(transports.len(), 2); // Http, TRpc (WebSocket removed)
    assert!(transports.contains(&TransportType::Http));
    assert!(transports.contains(&TransportType::TRpc));
    assert!(transports.contains(&TransportType::TRpc));
}

#[test]
fn test_transport_manager_default() {
    let manager = TransportManager::default();
    let transports = manager.get_supported_transports();

    assert!(!transports.is_empty());
}

#[test]
fn test_connection_info() {
    let endpoint = ServiceEndpoint {
        id: "test-endpoint".to_string(),
        transport: TransportType::Http,
        address: "127.0.0.1".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let connection = Connection {
        service_id: "service-1".to_string(),
        endpoint: endpoint.clone(),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 5,
    };

    assert_eq!(connection.service_id, "service-1");
    assert_eq!(connection.active_requests, 5);
    assert_eq!(connection.endpoint.id, endpoint.id);
}

#[test]
fn test_service_endpoint_with_tls() {
    let endpoint = ServiceEndpoint {
        id: "secure-endpoint".to_string(),
        transport: TransportType::Http,
        address: "secure.example.com".to_string(),
        port: 443,
        path: Some("/api/v1".to_string()),
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    assert!(endpoint.tls_enabled);
    assert_eq!(endpoint.port, 443);
}

#[test]
fn test_service_endpoint_without_tls() {
    let endpoint = ServiceEndpoint {
        id: "insecure-endpoint".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert!(!endpoint.tls_enabled);
    assert_eq!(endpoint.port, 8080);
}

#[test]
fn test_transport_enum_http() {
    let transport = Transport::Http(HttpTransport::new());
    assert_eq!(transport.transport_type(), TransportType::Http);
}

#[test]
fn test_transport_enum_websocket() {
    let transport = Transport::TRpc(TRpcTransport::new());
    assert_eq!(transport.transport_type(), TransportType::TRpc);
}

#[test]
fn test_transport_enum_trpc() {
    let transport = Transport::TRpc(TRpcTransport::new());
    assert_eq!(transport.transport_type(), TransportType::TRpc);
}

// ============================================================================
// Extended Transport Tests - Error Handling and Edge Cases
// ============================================================================

#[test]
fn test_http_transport_with_custom_port() {
    let endpoint = ServiceEndpoint {
        id: "custom-port".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 9999,
        path: Some("/custom".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let transport = HttpTransport::new();
    assert!(transport.supports_endpoint(&endpoint));
    assert_eq!(endpoint.port, 9999);
}

#[test]
fn test_http_transport_with_tls() {
    let endpoint = ServiceEndpoint {
        id: "tls-endpoint".to_string(),
        transport: TransportType::Http,
        address: "secure.example.com".to_string(),
        port: 443,
        path: Some("/api/secure".to_string()),
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    let transport = HttpTransport::new();
    assert!(transport.supports_endpoint(&endpoint));
    assert!(endpoint.tls_enabled);
}

#[test]
fn test_websocket_transport_with_path() {
    let endpoint = ServiceEndpoint {
        id: "ws-path".to_string(),
        transport: TransportType::TRpc,
        address: "localhost".to_string(),
        port: 8080,
        path: Some("/socket.io".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let transport = TRpcTransport::new();
    assert!(transport.supports_endpoint(&endpoint));
    assert_eq!(endpoint.path, Some("/socket.io".to_string()));
}

#[test]
fn test_websocket_transport_secure() {
    let endpoint = ServiceEndpoint {
        id: "wss-endpoint".to_string(),
        transport: TransportType::TRpc,
        address: "wss.example.com".to_string(),
        port: 443,
        path: None,
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    let transport = TRpcTransport::new();
    assert!(transport.supports_endpoint(&endpoint));
    assert!(endpoint.tls_enabled);
}

#[test]
fn test_trpc_transport_with_custom_path() {
    let endpoint = ServiceEndpoint {
        id: "trpc-custom".to_string(),
        transport: TransportType::TRpc,
        address: "trpc.example.com".to_string(),
        port: 4000,
        path: Some("/trpc/v1".to_string()),
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    let transport = TRpcTransport::new();
    assert!(transport.supports_endpoint(&endpoint));
}

#[test]
fn test_endpoint_with_unhealthy_status() {
    let endpoint = ServiceEndpoint {
        id: "unhealthy".to_string(),
        transport: TransportType::Http,
        address: "down.example.com".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Unhealthy,
    };

    assert_eq!(endpoint.health_status, HealthStatus::Unhealthy);
}

#[test]
fn test_endpoint_with_degraded_status() {
    let endpoint = ServiceEndpoint {
        id: "degraded".to_string(),
        transport: TransportType::Http,
        address: "slow.example.com".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Degraded,
    };

    assert_eq!(endpoint.health_status, HealthStatus::Degraded);
}

#[test]
fn test_endpoint_with_unknown_status() {
    let endpoint = ServiceEndpoint {
        id: "unknown".to_string(),
        transport: TransportType::Http,
        address: "new.example.com".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Unknown,
    };

    assert_eq!(endpoint.health_status, HealthStatus::Unknown);
}

#[test]
fn test_connection_active_requests() {
    let endpoint = ServiceEndpoint {
        id: "test-ep".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let connection = Connection {
        service_id: "service-1".to_string(),
        endpoint,
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 10,
    };

    assert_eq!(connection.active_requests, 10);
}

#[test]
fn test_connection_timing() {
    let endpoint = ServiceEndpoint {
        id: "timing-test".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let created = std::time::Instant::now();
    let connection = Connection {
        service_id: "timing-service".to_string(),
        endpoint,
        created_at: created,
        last_used: created,
        active_requests: 0,
    };

    assert_eq!(connection.created_at, created);
    assert_eq!(connection.last_used, created);
}

#[test]
fn test_transport_manager_http_support() {
    let manager = TransportManager::new();
    let transports = manager.get_supported_transports();

    assert!(transports.contains(&TransportType::Http));
}

#[test]
fn test_transport_manager_websocket_support() {
    let manager = TransportManager::new();
    let transports = manager.get_supported_transports();

    assert!(transports.contains(&TransportType::TRpc));
}

#[test]
fn test_transport_manager_trpc_support() {
    let manager = TransportManager::new();
    let transports = manager.get_supported_transports();

    assert!(transports.contains(&TransportType::TRpc));
}

#[test]
fn test_service_endpoint_clone() {
    let endpoint = ServiceEndpoint {
        id: "clone-test".to_string(),
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
    assert_eq!(endpoint.address, cloned.address);
}

#[test]
fn test_service_endpoint_with_ipv4() {
    let endpoint = ServiceEndpoint {
        id: "ipv4".to_string(),
        transport: TransportType::Http,
        address: "192.168.1.100".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.address, "192.168.1.100");
}

#[test]
fn test_service_endpoint_with_ipv6() {
    let endpoint = ServiceEndpoint {
        id: "ipv6".to_string(),
        transport: TransportType::Http,
        address: "::1".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.address, "::1");
}

#[test]
fn test_service_endpoint_with_domain() {
    let endpoint = ServiceEndpoint {
        id: "domain".to_string(),
        transport: TransportType::Http,
        address: "api.toadstool.example.com".to_string(),
        port: 443,
        path: Some("/v2".to_string()),
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    assert_eq!(endpoint.address, "api.toadstool.example.com");
    assert!(endpoint.tls_enabled);
}

#[test]
fn test_connection_with_zero_active_requests() {
    let endpoint = ServiceEndpoint {
        id: "idle".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let connection = Connection {
        service_id: "idle-service".to_string(),
        endpoint,
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(connection.active_requests, 0);
}

#[test]
fn test_http_transport_multiple_endpoints() {
    let transport = HttpTransport::new();

    let ep1 = ServiceEndpoint {
        id: "ep1".to_string(),
        transport: TransportType::Http,
        address: "server1.example.com".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let ep2 = ServiceEndpoint {
        id: "ep2".to_string(),
        transport: TransportType::Http,
        address: "server2.example.com".to_string(),
        port: 8081,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert!(transport.supports_endpoint(&ep1));
    assert!(transport.supports_endpoint(&ep2));
}

#[test]
fn test_websocket_transport_mixed_security() {
    let transport = TRpcTransport::new();

    let secure_ep = ServiceEndpoint {
        id: "secure".to_string(),
        transport: TransportType::TRpc,
        address: "wss.example.com".to_string(),
        port: 443,
        path: None,
        tls_enabled: true,
        health_status: HealthStatus::Healthy,
    };

    let insecure_ep = ServiceEndpoint {
        id: "insecure".to_string(),
        transport: TransportType::TRpc,
        address: "ws.example.com".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert!(transport.supports_endpoint(&secure_ep));
    assert!(transport.supports_endpoint(&insecure_ep));
}

#[test]
fn test_transport_manager_consistency() {
    let manager1 = TransportManager::new();
    let manager2 = TransportManager::new();

    let transports1 = manager1.get_supported_transports();
    let transports2 = manager2.get_supported_transports();

    assert_eq!(transports1.len(), transports2.len());
}

#[test]
fn test_endpoint_path_optional() {
    let with_path = ServiceEndpoint {
        id: "with-path".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: Some("/api/v1".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let without_path = ServiceEndpoint {
        id: "without-path".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8081,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    assert!(with_path.path.is_some());
    assert!(without_path.path.is_none());
}

#[test]
fn test_transport_type_equality() {
    assert_eq!(TransportType::Http, TransportType::Http);
    assert_eq!(TransportType::TRpc, TransportType::TRpc);
    assert_eq!(TransportType::TRpc, TransportType::TRpc);
    assert_ne!(TransportType::Http, TransportType::TRpc);
}

#[test]
fn test_health_status_in_endpoint() {
    let healthy = ServiceEndpoint {
        id: "healthy".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let unhealthy = ServiceEndpoint {
        id: "unhealthy".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8081,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Unhealthy,
    };

    assert_ne!(healthy.health_status, unhealthy.health_status);
}

#[test]
fn test_connection_service_id() {
    let endpoint = ServiceEndpoint {
        id: "ep".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let connection = Connection {
        service_id: "unique-service-123".to_string(),
        endpoint,
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 5,
    };

    assert_eq!(connection.service_id, "unique-service-123");
}

// ============================================================================
// Async Transport send_message Tests
// ============================================================================

fn make_test_message() -> ProtocolMessage {
    ProtocolMessage {
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
        ttl: Some(Duration::from_secs(60)),
        priority: MessagePriority::Normal,
    }
}

#[tokio::test]
async fn test_http_transport_send_message_returns_error() {
    let transport = HttpTransport::new();
    let msg = make_test_message();
    let endpoint = ServiceEndpoint {
        id: "ep".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };
    let result = transport.send_message(&msg, &endpoint).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("deprecated") || err.to_string().contains("Unix"));
}

#[tokio::test]
async fn test_trpc_transport_send_message_returns_error() {
    let transport = TRpcTransport::new();
    let msg = make_test_message();
    let endpoint = ServiceEndpoint {
        id: "ep".to_string(),
        transport: TransportType::TRpc,
        address: "localhost".to_string(),
        port: 9000,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };
    let result = transport.send_message(&msg, &endpoint).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not yet implemented"));
}

#[tokio::test]
async fn test_transport_enum_http_send_message() {
    let transport = Transport::Http(HttpTransport::new());
    let msg = make_test_message();
    let endpoint = ServiceEndpoint {
        id: "ep".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };
    let result = transport.send_message(&msg, &endpoint).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_transport_enum_trpc_send_message() {
    let transport = Transport::TRpc(TRpcTransport::new());
    let msg = make_test_message();
    let endpoint = ServiceEndpoint {
        id: "ep".to_string(),
        transport: TransportType::TRpc,
        address: "localhost".to_string(),
        port: 9000,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };
    let result = transport.send_message(&msg, &endpoint).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_transport_manager_send_message_http_error() {
    let manager = TransportManager::new();
    let msg = make_test_message();
    let endpoint = ServiceEndpoint {
        id: "ep".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };
    let result = manager.send_message(&msg, &endpoint).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_transport_manager_send_message_trpc_error() {
    let manager = TransportManager::new();
    let msg = make_test_message();
    let endpoint = ServiceEndpoint {
        id: "ep".to_string(),
        transport: TransportType::TRpc,
        address: "localhost".to_string(),
        port: 9000,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };
    let result = manager.send_message(&msg, &endpoint).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_transport_manager_send_message_unknown_transport() {
    let manager = TransportManager::new();
    let msg = make_test_message();
    let endpoint = ServiceEndpoint {
        id: "ep".to_string(),
        transport: TransportType::Tcp,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };
    let result = manager.send_message(&msg, &endpoint).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("No transport handler") || err.to_string().contains("Tcp"));
}

#[tokio::test]
async fn test_transport_manager_register_transport() {
    let mut manager = TransportManager::new();
    manager.register_transport(Transport::Http(HttpTransport::new()));
    let transports = manager.get_supported_transports();
    assert!(transports.contains(&TransportType::Http));
}
