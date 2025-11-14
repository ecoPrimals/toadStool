//! Additional transport layer tests for comprehensive coverage
//!
//! This file expands test coverage for transport functionality including:
//! - Connection management edge cases
//! - Error handling scenarios
//! - Multiple transport types
//! - Endpoint validation
//! - Transport selection logic

use toadstool_integration_protocols::transport::*;
use toadstool_integration_protocols::types::*;

// Helper function to create a default healthy endpoint
fn create_test_endpoint(
    id: &str,
    transport: TransportType,
    address: &str,
    port: u16,
    path: Option<String>,
    tls_enabled: bool,
) -> ServiceEndpoint {
    ServiceEndpoint {
        id: id.to_string(),
        transport,
        address: address.to_string(),
        port,
        path,
        tls_enabled,
        health_status: HealthStatus::Healthy,
    }
}

// ============================================================================
// Connection Tests (20 tests)
// ============================================================================

#[test]
fn test_connection_with_localhost() {
    let conn = Connection {
        service_id: "test-service".to_string(),
        endpoint: create_test_endpoint(
            "endpoint-1",
            TransportType::Http,
            "localhost",
            8080,
            Some("/api".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(conn.endpoint.address, "localhost");
}

#[test]
fn test_connection_with_ipv4() {
    let conn = Connection {
        service_id: "test-service".to_string(),
        endpoint: create_test_endpoint(
            "endpoint-2",
            TransportType::Http,
            "192.168.1.1",
            8080,
            Some("/api".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(conn.endpoint.address, "192.168.1.1");
}

#[test]
fn test_connection_with_domain_name() {
    let conn = Connection {
        service_id: "test-service".to_string(),
        endpoint: create_test_endpoint(
            "endpoint-3",
            TransportType::Http,
            "api.example.com",
            443,
            Some("/v1".to_string()),
            true,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(conn.endpoint.address, "api.example.com");
    assert!(conn.endpoint.tls_enabled);
}

#[test]
fn test_connection_active_requests_zero() {
    let conn = Connection {
        service_id: "test-service".to_string(),
        endpoint: create_test_endpoint(
            "endpoint-4",
            TransportType::Http,
            "localhost",
            8080,
            Some("/".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(conn.active_requests, 0);
}

#[test]
fn test_connection_active_requests_multiple() {
    let conn = Connection {
        service_id: "test-service".to_string(),
        endpoint: create_test_endpoint(
            "endpoint-5",
            TransportType::Http,
            "localhost",
            8080,
            Some("/".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 10,
    };

    assert_eq!(conn.active_requests, 10);
}

#[test]
fn test_connection_timestamps() {
    let now = std::time::Instant::now();
    let conn = Connection {
        service_id: "test-service".to_string(),
        endpoint: create_test_endpoint(
            "endpoint-6",
            TransportType::Http,
            "localhost",
            8080,
            Some("/".to_string()),
            false,
        ),
        created_at: now,
        last_used: now,
        active_requests: 0,
    };

    assert!(conn.created_at <= conn.last_used);
}

#[test]
fn test_connection_with_http_transport() {
    let conn = Connection {
        service_id: "http-service".to_string(),
        endpoint: create_test_endpoint(
            "http-endpoint",
            TransportType::Http,
            "localhost",
            80,
            Some("/".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(conn.endpoint.transport, TransportType::Http);
}

#[test]
fn test_connection_with_websocket_transport() {
    let conn = Connection {
        service_id: "ws-service".to_string(),
        endpoint: create_test_endpoint(
            "ws-endpoint",
            TransportType::WebSocket,
            "localhost",
            8080,
            Some("/ws".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(conn.endpoint.transport, TransportType::WebSocket);
}

#[test]
fn test_connection_with_trpc_transport() {
    let conn = Connection {
        service_id: "trpc-service".to_string(),
        endpoint: create_test_endpoint(
            "trpc-endpoint",
            TransportType::TRpc,
            "localhost",
            9000,
            Some("/trpc".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(conn.endpoint.transport, TransportType::TRpc);
}

#[test]
fn test_connection_with_tcp_transport() {
    let conn = Connection {
        service_id: "tcp-service".to_string(),
        endpoint: create_test_endpoint(
            "tcp-endpoint",
            TransportType::Tcp,
            "localhost",
            9090,
            None,
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(conn.endpoint.transport, TransportType::Tcp);
}

#[test]
fn test_connection_service_id_formatting() {
    let ids = vec![
        "service-1",
        "my_service",
        "Service123",
        "service.name",
        "service:8080",
    ];

    for id in ids {
        let conn = Connection {
            service_id: id.to_string(),
            endpoint: create_test_endpoint(
                &format!("{}-endpoint", id),
                TransportType::Http,
                "localhost",
                8080,
                Some("/".to_string()),
                false,
            ),
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
            active_requests: 0,
        };

        assert_eq!(conn.service_id, id);
    }
}

#[test]
fn test_connection_with_standard_ports() {
    let ports = vec![80, 443, 8080, 3000, 5000, 9000];

    for port in ports {
        let conn = Connection {
            service_id: format!("service-{}", port),
            endpoint: create_test_endpoint(
                &format!("endpoint-{}", port),
                TransportType::Http,
                "localhost",
                port,
                Some("/".to_string()),
                port == 443,
            ),
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
            active_requests: 0,
        };

        assert_eq!(conn.endpoint.port, port);
    }
}

#[test]
fn test_connection_with_custom_path() {
    let paths = vec!["/", "/api", "/v1", "/api/v2", "/health", "/metrics"];

    for path in paths {
        let conn = Connection {
            service_id: "test-service".to_string(),
            endpoint: create_test_endpoint(
                "endpoint",
                TransportType::Http,
                "localhost",
                8080,
                Some(path.to_string()),
                false,
            ),
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
            active_requests: 0,
        };

        assert_eq!(conn.endpoint.path, Some(path.to_string()));
    }
}

#[test]
fn test_connection_health_status() {
    // Test that endpoints are created with healthy status
    let conn = Connection {
        service_id: "test-service".to_string(),
        endpoint: create_test_endpoint(
            "endpoint-healthy",
            TransportType::Http,
            "localhost",
            8080,
            Some("/".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert_eq!(conn.endpoint.health_status, HealthStatus::Healthy);
}

#[test]
fn test_connection_tls_enabled() {
    let conn = Connection {
        service_id: "secure-service".to_string(),
        endpoint: create_test_endpoint(
            "secure-endpoint",
            TransportType::Http,
            "api.example.com",
            443,
            Some("/api".to_string()),
            true,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert!(conn.endpoint.tls_enabled);
}

#[test]
fn test_connection_tls_disabled() {
    let conn = Connection {
        service_id: "insecure-service".to_string(),
        endpoint: create_test_endpoint(
            "insecure-endpoint",
            TransportType::Http,
            "localhost",
            80,
            Some("/api".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    assert!(!conn.endpoint.tls_enabled);
}

#[test]
fn test_connection_clone() {
    let conn = Connection {
        service_id: "test-service".to_string(),
        endpoint: create_test_endpoint(
            "endpoint",
            TransportType::Http,
            "localhost",
            8080,
            Some("/".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 5,
    };

    let cloned = conn.clone();
    assert_eq!(conn.service_id, cloned.service_id);
    assert_eq!(conn.active_requests, cloned.active_requests);
}

#[test]
fn test_connection_debug_format() {
    let conn = Connection {
        service_id: "debug-test".to_string(),
        endpoint: create_test_endpoint(
            "endpoint",
            TransportType::Http,
            "localhost",
            8080,
            Some("/".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 0,
    };

    let debug_str = format!("{:?}", conn);
    assert!(debug_str.contains("debug-test"));
}

#[test]
fn test_connection_with_high_active_requests() {
    let conn = Connection {
        service_id: "busy-service".to_string(),
        endpoint: create_test_endpoint(
            "endpoint",
            TransportType::Http,
            "localhost",
            8080,
            Some("/".to_string()),
            false,
        ),
        created_at: std::time::Instant::now(),
        last_used: std::time::Instant::now(),
        active_requests: 1000,
    };

    assert_eq!(conn.active_requests, 1000);
}

#[test]
fn test_connection_timestamp_ordering() {
    let created = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let used = std::time::Instant::now();

    let conn = Connection {
        service_id: "time-test".to_string(),
        endpoint: create_test_endpoint(
            "endpoint",
            TransportType::Http,
            "localhost",
            8080,
            Some("/".to_string()),
            false,
        ),
        created_at: created,
        last_used: used,
        active_requests: 0,
    };

    assert!(conn.last_used > conn.created_at);
}

// ============================================================================
// Transport Enum Tests (15 tests)
// ============================================================================

#[test]
fn test_transport_http_variant() {
    let transport = Transport::Http(HttpTransport::new());
    assert_eq!(transport.transport_type(), TransportType::Http);
}

#[test]
fn test_transport_websocket_variant() {
    let transport = Transport::WebSocket(WebSocketTransport::new());
    assert_eq!(transport.transport_type(), TransportType::WebSocket);
}

#[test]
fn test_transport_trpc_variant() {
    let transport = Transport::TRpc(TRpcTransport::new());
    assert_eq!(transport.transport_type(), TransportType::TRpc);
}

#[test]
fn test_transport_clone_http() {
    let transport = Transport::Http(HttpTransport::new());
    let cloned = transport.clone();
    assert_eq!(cloned.transport_type(), TransportType::Http);
}

#[test]
fn test_transport_clone_websocket() {
    let transport = Transport::WebSocket(WebSocketTransport::new());
    let cloned = transport.clone();
    assert_eq!(cloned.transport_type(), TransportType::WebSocket);
}

#[test]
fn test_transport_clone_trpc() {
    let transport = Transport::TRpc(TRpcTransport::new());
    let cloned = transport.clone();
    assert_eq!(cloned.transport_type(), TransportType::TRpc);
}

#[test]
fn test_transport_debug_http() {
    let transport = Transport::Http(HttpTransport::new());
    let debug_str = format!("{:?}", transport);
    assert!(debug_str.contains("Http"));
}

#[test]
fn test_transport_debug_websocket() {
    let transport = Transport::WebSocket(WebSocketTransport::new());
    let debug_str = format!("{:?}", transport);
    assert!(debug_str.contains("WebSocket"));
}

#[test]
fn test_transport_debug_trpc() {
    let transport = Transport::TRpc(TRpcTransport::new());
    let debug_str = format!("{:?}", transport);
    assert!(debug_str.contains("TRpc"));
}

#[test]
fn test_transport_supports_matching_endpoint() {
    let transport = Transport::Http(HttpTransport::new());
    let endpoint = create_test_endpoint(
        "test",
        TransportType::Http,
        "localhost",
        8080,
        Some("/".to_string()),
        false,
    );

    assert!(transport.supports_endpoint(&endpoint));
}

#[test]
fn test_transport_rejects_mismatched_endpoint() {
    let transport = Transport::Http(HttpTransport::new());
    let endpoint = create_test_endpoint(
        "test",
        TransportType::WebSocket,
        "localhost",
        8080,
        Some("/".to_string()),
        false,
    );

    assert!(!transport.supports_endpoint(&endpoint));
}

#[test]
fn test_transport_http_supports_http_only() {
    let transport = Transport::Http(HttpTransport::new());

    let http_endpoint = create_test_endpoint(
        "http",
        TransportType::Http,
        "localhost",
        8080,
        Some("/".to_string()),
        false,
    );

    let ws_endpoint = create_test_endpoint(
        "ws",
        TransportType::WebSocket,
        "localhost",
        8080,
        Some("/".to_string()),
        false,
    );

    assert!(transport.supports_endpoint(&http_endpoint));
    assert!(!transport.supports_endpoint(&ws_endpoint));
}

#[test]
fn test_transport_websocket_supports_ws_only() {
    let transport = Transport::WebSocket(WebSocketTransport::new());

    let http_endpoint = create_test_endpoint(
        "http",
        TransportType::Http,
        "localhost",
        8080,
        Some("/".to_string()),
        false,
    );

    let ws_endpoint = create_test_endpoint(
        "ws",
        TransportType::WebSocket,
        "localhost",
        8080,
        Some("/".to_string()),
        false,
    );

    assert!(!transport.supports_endpoint(&http_endpoint));
    assert!(transport.supports_endpoint(&ws_endpoint));
}

#[test]
fn test_transport_type_consistency() {
    let http = Transport::Http(HttpTransport::new());
    let ws = Transport::WebSocket(WebSocketTransport::new());
    let trpc = Transport::TRpc(TRpcTransport::new());

    assert_eq!(http.transport_type(), TransportType::Http);
    assert_eq!(ws.transport_type(), TransportType::WebSocket);
    assert_eq!(trpc.transport_type(), TransportType::TRpc);
}

#[test]
fn test_transport_manager_creation() {
    let manager = TransportManager::new();
    // TransportManager should be created successfully
    let _ = format!("{:?}", manager); // Test debug formatting
}

// Total tests in this file: 20 + 15 = 35 tests
// These expand the transport test coverage incrementally
