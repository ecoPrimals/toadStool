// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::path::PathBuf;

#[test]
fn test_client_for_toadstool() {
    let client = IpcClient::for_toadstool();
    let endpoints = client.endpoints();

    // Should have multiple endpoints
    assert!(!endpoints.is_empty());

    // Should include TCP fallback
    assert!(endpoints.iter().any(|e| e.is_tcp()));

    #[cfg(target_os = "linux")]
    {
        // On Linux, should have abstract socket
        assert!(endpoints.iter().any(|e| e.is_abstract()));
    }
}

#[test]
fn test_client_for_primal() {
    // Use env var or constant - no hardcoded other-primal names (self-knowledge)
    let primal = std::env::var("TOADSTOOL_TEST_PRIMAL")
        .unwrap_or_else(|_| "coordination-service".to_string());
    let client = IpcClient::for_primal(&primal);
    let endpoints = client.endpoints();

    // Should have multiple endpoints (abstract, unix, tcp)
    assert!(!endpoints.is_empty());

    // Should have TCP endpoint with a resolved port
    assert!(
        endpoints
            .iter()
            .any(|e| { matches!(e, Endpoint::Tcp { .. }) })
    );

    // Should have Unix socket endpoint containing primal name
    assert!(endpoints.iter().any(|e| {
        matches!(e, Endpoint::Unix { path } if path.to_string_lossy().contains(&primal.to_lowercase()))
    }));
}

#[test]
fn test_client_with_custom_endpoints() {
    let custom = vec![Endpoint::Tcp {
        host: "192.168.1.100".to_string(),
        port: 9000,
    }];

    let client = IpcClient::with_endpoints(custom.clone());
    assert_eq!(client.endpoints().len(), 1);
    assert_eq!(client.endpoints()[0], custom[0]);
}

#[test]
fn test_endpoint_display() {
    let endpoint = Endpoint::Tcp {
        host: "127.0.0.1".to_string(),
        port: 8370,
    };

    assert_eq!(endpoint.display(), "tcp://127.0.0.1:8370");
}

#[tokio::test]
async fn test_connect_no_server() {
    // Try to connect with no server running
    let client = IpcClient::for_toadstool();
    let result = client.connect().await;

    // Should fail (no server listening)
    assert!(result.is_err());
}

// =========================================================================
// Client configuration tests
// =========================================================================

#[test]
fn test_client_configuration_primal_name_normalization() {
    let client = IpcClient::for_primal("Coordination-Service");
    let endpoints = client.endpoints();

    assert!(endpoints.iter().any(|e| {
        matches!(e, Endpoint::Unix { path } if path.to_string_lossy().contains("coordination-service"))
    }));
}

#[test]
fn test_client_configuration_multiple_custom_endpoints() {
    let custom = vec![
        Endpoint::Unix {
            path: PathBuf::from("/tmp/a.sock"),
        },
        Endpoint::Tcp {
            host: "127.0.0.1".to_string(),
            port: 12345,
        },
    ];
    let client = IpcClient::with_endpoints(custom);
    assert_eq!(client.endpoints().len(), 2);
    assert!(client.endpoints()[0].is_unix());
    assert!(client.endpoints()[1].is_tcp());
}

#[test]
fn test_for_primal_tcp_endpoint_uses_env_or_ephemeral() {
    let client = IpcClient::for_primal("test-primal");
    let tcp_endpoint = client.endpoints().iter().find(|e| e.is_tcp());
    assert!(tcp_endpoint.is_some());
    if let Some(Endpoint::Tcp { port, .. }) = tcp_endpoint {
        // Port 0 = OS-assigned ephemeral (unless overridden by env)
        assert_eq!(*port, 0, "TCP port defaults to ephemeral (0)");
    }
}

#[test]
fn test_endpoint_ordering_tcp_is_last() {
    let client = IpcClient::for_toadstool();
    let endpoints = client.endpoints();
    let last = endpoints.last().expect("should have endpoints");
    assert!(
        last.is_tcp(),
        "TCP fallback should be last in endpoint list"
    );
}

// =========================================================================
// Error handling tests
// =========================================================================

#[tokio::test]
async fn test_connect_empty_endpoints() {
    let client = IpcClient::with_endpoints(vec![]);
    let result = client.connect().await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.to_lowercase().contains("no endpoints")
            || err_msg.to_lowercase().contains("configured"),
        "Expected 'no endpoints' or 'configured' in error, got: {err_msg}"
    );
}

#[tokio::test]
async fn test_connect_all_endpoints_fail_returns_last_error() {
    let client = IpcClient::with_endpoints(vec![
        Endpoint::Unix {
            path: PathBuf::from("/nonexistent/path/xyz123.sock"),
        },
        Endpoint::Tcp {
            host: "127.0.0.1".to_string(),
            port: 1, // Port 1 typically has no listener
        },
    ]);
    let result = client.connect().await;

    assert!(result.is_err());
}

// =========================================================================
// IpcStream and connection state tests
// =========================================================================

#[tokio::test]
async fn test_connect_success_via_tcp_returns_stream() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    let client = IpcClient::with_endpoints(vec![Endpoint::Tcp {
        host: "127.0.0.1".to_string(),
        port,
    }]);

    let server_accept = tokio::spawn(async move { listener.accept().await });

    let stream_result = client.connect().await;
    assert!(stream_result.is_ok());

    let stream = stream_result.unwrap();
    assert_eq!(stream.endpoint_type(), "tcp");

    server_accept.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_connect_fallback_to_second_endpoint() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let client = IpcClient::with_endpoints(vec![
        Endpoint::Unix {
            path: PathBuf::from("/nonexistent/does/not/exist.sock"),
        },
        Endpoint::Tcp {
            host: "127.0.0.1".to_string(),
            port,
        },
    ]);

    let _server_handle = tokio::spawn(async move { listener.accept().await });

    let result = client.connect().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().endpoint_type(), "tcp");
}

#[test]
fn test_endpoint_display_unix_and_tcp() {
    let unix_endpoint = Endpoint::Unix {
        path: PathBuf::from("/tmp/test.sock"),
    };
    assert_eq!(unix_endpoint.display(), "unix:/tmp/test.sock");

    let tcp_endpoint = Endpoint::Tcp {
        host: "localhost".to_string(),
        port: 8080,
    };
    assert_eq!(tcp_endpoint.display(), "tcp://localhost:8080");
}

#[tokio::test]
async fn test_connect_unix_stream_endpoint_type() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test.sock");
    let _listener = crate::ipc::platform::bind_unix(&socket_path).await.unwrap();

    let client = IpcClient::with_endpoints(vec![Endpoint::Unix {
        path: socket_path.clone(),
    }]);

    let result = client.connect().await;
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert_eq!(stream.endpoint_type(), "unix");
}
