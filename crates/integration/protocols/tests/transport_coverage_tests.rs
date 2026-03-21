// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive Transport Coverage Tests
//!
//! This test suite provides thorough coverage of the transport module to improve
//! coverage from 36.67% towards the 60%+ target.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use toadstool_integration_protocols::transport::*;
use toadstool_integration_protocols::types::*;
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_message() -> ProtocolMessage {
    ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("test"),
        source: Arc::from("test-service"),
        destination: Some(Arc::from("target-service")),
        payload: serde_json::json!({"test": "data"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    }
}

fn create_test_endpoint(transport_type: TransportType) -> ServiceEndpoint {
    ServiceEndpoint {
        id: "test-endpoint".to_string(),
        transport: transport_type,
        address: "localhost".to_string(),
        port: 59999, // Non-existent port for testing
        path: Some("/api".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    }
}

// ============================================================================
// Connection Tests
// ============================================================================

#[test]
fn test_connection_creation() {
    let endpoint = create_test_endpoint(TransportType::Http);
    let connection = Connection {
        service_id: "test-service".to_string(),
        endpoint,
        created_at: Instant::now(),
        last_used: Instant::now(),
        active_requests: 0,
    };

    assert_eq!(connection.service_id, "test-service");
    assert_eq!(connection.active_requests, 0);
}

#[test]
fn test_connection_with_active_requests() {
    let endpoint = create_test_endpoint(TransportType::Http);
    let connection = Connection {
        service_id: "busy-service".to_string(),
        endpoint,
        created_at: Instant::now(),
        last_used: Instant::now(),
        active_requests: 10,
    };

    assert_eq!(connection.active_requests, 10);
}

#[test]
fn test_connection_clone() {
    let endpoint = create_test_endpoint(TransportType::Http);
    let connection1 = Connection {
        service_id: "test".to_string(),
        endpoint,
        created_at: Instant::now(),
        last_used: Instant::now(),
        active_requests: 5,
    };

    let connection2 = connection1.clone();
    assert_eq!(connection1.service_id, connection2.service_id);
    assert_eq!(connection1.active_requests, connection2.active_requests);
}

#[test]
fn test_connection_debug() {
    let endpoint = create_test_endpoint(TransportType::Http);
    let connection = Connection {
        service_id: "test".to_string(),
        endpoint,
        created_at: Instant::now(),
        last_used: Instant::now(),
        active_requests: 0,
    };

    let debug_str = format!("{connection:?}");
    assert!(debug_str.contains("Connection"));
}

// ============================================================================
// HttpTransport Tests
// ============================================================================

#[test]
fn test_http_transport_new() {
    let transport = HttpTransport::new();
    let _ = format!("{transport:?}");
}

#[test]
fn test_http_transport_default() {
    let transport = HttpTransport::default();
    let _ = format!("{transport:?}");
}

#[test]
fn test_http_transport_supports_http_endpoint() {
    let transport = HttpTransport::new();
    let endpoint = create_test_endpoint(TransportType::Http);
    assert!(transport.supports_endpoint(&endpoint));
}

#[test]
fn test_http_transport_rejects_websocket_endpoint() {
    let transport = HttpTransport::new();
    let endpoint = create_test_endpoint(TransportType::TRpc);
    assert!(!transport.supports_endpoint(&endpoint));
}

#[test]
fn test_http_transport_type() {
    let transport = HttpTransport::new();
    assert_eq!(transport.transport_type(), TransportType::Http);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_http_transport_send_message_no_server() {
    let transport = HttpTransport::new();
    let endpoint = create_test_endpoint(TransportType::Http);
    let message = create_test_message();

    let result = transport.send_message(&message, &endpoint).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_http_transport_with_tls() {
    let transport = HttpTransport::new();
    let mut endpoint = create_test_endpoint(TransportType::Http);
    endpoint.tls_enabled = true;
    let message = create_test_message();

    let result = transport.send_message(&message, &endpoint).await;
    assert!(result.is_err());
}

// ============================================================================
// TRpcTransport Tests (WebSocket removed — use JSON-RPC 2.0)
// ============================================================================

#[test]
fn test_trpc_transport_new() {
    let transport = TRpcTransport::new();
    let _ = format!("{transport:?}");
}

#[test]
fn test_trpc_transport_default() {
    let transport = TRpcTransport::default();
    let _ = format!("{transport:?}");
}

#[test]
fn test_trpc_transport_supports_trpc_endpoint() {
    let transport = TRpcTransport::new();
    let endpoint = create_test_endpoint(TransportType::TRpc);
    assert!(transport.supports_endpoint(&endpoint));
}

#[test]
fn test_trpc_transport_rejects_http_endpoint() {
    let transport = TRpcTransport::new();
    let endpoint = create_test_endpoint(TransportType::Http);
    assert!(!transport.supports_endpoint(&endpoint));
}

#[test]
fn test_trpc_transport_type() {
    let transport = TRpcTransport::new();
    assert_eq!(transport.transport_type(), TransportType::TRpc);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_trpc_transport_send_message_no_server() {
    let transport = TRpcTransport::new();
    let endpoint = create_test_endpoint(TransportType::TRpc);
    let message = create_test_message();

    let result = transport.send_message(&message, &endpoint).await;
    assert!(result.is_err());
    if let Err(e) = result {
        let msg = e.to_string().to_ascii_lowercase();
        assert!(
            msg.contains("trpc transport not available") || msg.contains("pure_jsonrpc"),
            "unexpected error message: {msg}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_trpc_transport_with_tls() {
    let transport = TRpcTransport::new();
    let mut endpoint = create_test_endpoint(TransportType::TRpc);
    endpoint.tls_enabled = true;
    let message = create_test_message();

    let result = transport.send_message(&message, &endpoint).await;
    assert!(result.is_err());
}

// ============================================================================
// Transport Enum Tests
// ============================================================================

#[test]
fn test_transport_enum_http_variant() {
    let transport = Transport::Http(HttpTransport::new());
    assert_eq!(transport.transport_type(), TransportType::Http);
}

#[test]
fn test_transport_enum_trpc_variant_alias() {
    let transport = Transport::TRpc(TRpcTransport::new());
    assert_eq!(transport.transport_type(), TransportType::TRpc);
}

#[test]
fn test_transport_enum_trpc_variant() {
    let transport = Transport::TRpc(TRpcTransport::new());
    assert_eq!(transport.transport_type(), TransportType::TRpc);
}

#[test]
fn test_transport_enum_supports_endpoint_http() {
    let transport = Transport::Http(HttpTransport::new());
    let endpoint = create_test_endpoint(TransportType::Http);
    assert!(transport.supports_endpoint(&endpoint));
}

#[test]
fn test_transport_enum_clone() {
    let transport1 = Transport::Http(HttpTransport::new());
    let transport2 = transport1.clone();
    assert_eq!(transport1.transport_type(), transport2.transport_type());
}

#[test]
fn test_transport_enum_debug() {
    let transport = Transport::Http(HttpTransport::new());
    let debug_str = format!("{transport:?}");
    assert!(debug_str.contains("Http"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_transport_enum_send_message_http() {
    let transport = Transport::Http(HttpTransport::new());
    let endpoint = create_test_endpoint(TransportType::Http);
    let message = create_test_message();

    let result = transport.send_message(&message, &endpoint).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_transport_enum_send_message_trpc() {
    let transport = Transport::TRpc(TRpcTransport::new());
    let endpoint = create_test_endpoint(TransportType::TRpc);
    let message = create_test_message();

    let result = transport.send_message(&message, &endpoint).await;
    assert!(result.is_err());
}

// ============================================================================
// TransportManager Tests
// ============================================================================

#[test]
fn test_transport_manager_new() {
    let manager = TransportManager::new();
    let supported = manager.get_supported_transports();

    // Http and TRpc are always registered (WebSocket removed in favour of JSON-RPC 2.0)
    assert!(supported.len() >= 2);
    assert!(supported.contains(&TransportType::Http));
    assert!(supported.contains(&TransportType::TRpc));
}

#[test]
fn test_transport_manager_default() {
    let manager = TransportManager::default();
    let supported = manager.get_supported_transports();
    assert!(supported.len() >= 2);
}

#[test]
fn test_transport_manager_register_transport() {
    let mut manager = TransportManager::new();
    let http_transport = Transport::Http(HttpTransport::new());

    manager.register_transport(http_transport);

    let supported = manager.get_supported_transports();
    assert!(supported.contains(&TransportType::Http));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_transport_manager_send_message_http() {
    let manager = TransportManager::new();
    let endpoint = create_test_endpoint(TransportType::Http);
    let message = create_test_message();

    let result = manager.send_message(&message, &endpoint).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_transport_manager_send_message_unsupported_transport() {
    let manager = TransportManager::new();
    let endpoint = create_test_endpoint(TransportType::Tcp);
    let message = create_test_message();

    let result = manager.send_message(&message, &endpoint).await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("No transport handler"));
    }
}

#[test]
fn test_transport_manager_get_supported_transports() {
    let manager = TransportManager::new();
    let transports = manager.get_supported_transports();

    assert!(!transports.is_empty());
    assert!(transports.contains(&TransportType::Http));
}

#[test]
fn test_transport_manager_debug() {
    let manager = TransportManager::new();
    let debug_str = format!("{manager:?}");
    assert!(debug_str.contains("TransportManager"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_http_transport_with_no_path() {
    let transport = HttpTransport::new();
    let mut endpoint = create_test_endpoint(TransportType::Http);
    endpoint.path = None;
    let message = create_test_message();

    let result = transport.send_message(&message, &endpoint).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_trpc_transport_with_no_path() {
    let transport = TRpcTransport::new();
    let mut endpoint = create_test_endpoint(TransportType::TRpc);
    endpoint.path = None;
    let message = create_test_message();

    let result = transport.send_message(&message, &endpoint).await;
    assert!(result.is_err());
}

#[test]
fn test_connection_timestamps() {
    let endpoint = create_test_endpoint(TransportType::Http);
    let now = Instant::now();
    let connection = Connection {
        service_id: "test".to_string(),
        endpoint,
        created_at: now,
        last_used: now,
        active_requests: 0,
    };

    assert!(connection.created_at <= connection.last_used);
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_transport_coverage_summary() {
    println!("========================================");
    println!("Transport Module Coverage Tests");
    println!("========================================");
    println!("Connection Tests:             4 tests");
    println!("HttpTransport Tests:          7 tests");
    println!("TRpcTransport Tests:          7 tests");
    println!("Transport Enum Tests:         8 tests");
    println!("TransportManager Tests:       7 tests");
    println!("Edge Cases:                   3 tests");
    println!("========================================");
    println!("Total New Tests:             36 tests");
    println!("========================================");
    println!();
    println!("🎯 Target: Increase transport.rs coverage");
    println!("   From: 36.67% → Target: 55%+");
    println!("========================================");
}
