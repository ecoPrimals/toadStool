// SPDX-License-Identifier: AGPL-3.0-only
//! Universal IPC client with smart transport selection
//!
//! **Deep Debt Principles**:
//! - ✅ Capability-based (auto-detect available transports)
//! - ✅ Self-knowledge (knows own capabilities)
//! - ✅ Smart fallback (Tier1 → Tier2)
//! - ✅ Agnostic (not hardcoded to one transport)
//! - ✅ Modern async (tokio patterns)
//!
//! ## Transport Selection Logic
//!
//! 1. **Tier 1** (Local, Preferred):
//!    - Linux: Try Abstract → Unix
//!    - macOS: Unix
//!    - Android: Abstract
//!
//! 2. **Tier 2** (Fallback):
//!    - TCP (localhost)
//!
//! 3. **Tier 3** (Remote):
//!    - TCP (network) - requires explicit host

use super::platform::{self, Endpoint};
use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::constants::network::LOCALHOST_IPV4;
use tokio::io::{AsyncRead, AsyncWrite};

/// Universal IPC stream
///
/// **Deep Debt**: Agnostic wrapper over all transport types
#[derive(Debug)]
pub enum IpcStream {
    Unix(tokio::net::UnixStream),
    #[cfg(target_os = "linux")]
    Abstract(tokio::net::UnixStream),
    Tcp(tokio::net::TcpStream),
}

impl IpcStream {
    /// Get endpoint info for this stream
    pub fn endpoint_type(&self) -> &'static str {
        match self {
            Self::Unix(_) => "unix",
            #[cfg(target_os = "linux")]
            Self::Abstract(_) => "abstract",
            Self::Tcp(_) => "tcp",
        }
    }
}

// Implement AsyncRead/AsyncWrite for IpcStream
impl AsyncRead for IpcStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            Self::Unix(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            #[cfg(target_os = "linux")]
            Self::Abstract(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            Self::Tcp(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for IpcStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match &mut *self {
            Self::Unix(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
            #[cfg(target_os = "linux")]
            Self::Abstract(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
            Self::Tcp(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            Self::Unix(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            #[cfg(target_os = "linux")]
            Self::Abstract(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            Self::Tcp(stream) => std::pin::Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            Self::Unix(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            #[cfg(target_os = "linux")]
            Self::Abstract(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            Self::Tcp(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
        }
    }
}

/// Universal IPC client with smart transport selection
///
/// **Deep Debt**: Capability-based, auto-fallback
pub struct IpcClient {
    /// Ordered list of endpoints to try (Tier1 → Tier2)
    endpoints: Vec<Endpoint>,
}

impl IpcClient {
    /// Create client for ToadStool with smart defaults
    ///
    /// **Deep Debt**: Runtime detection, platform-aware
    pub fn for_toadstool() -> Self {
        let mut endpoints = Vec::new();

        // Tier 1: Platform-specific preferred transport
        #[cfg(target_os = "linux")]
        {
            // Try abstract first (Android-friendly, SELinux-safe)
            endpoints.push(Endpoint::Abstract {
                name: "@biomeos_toadstool".to_string(),
            });
        }

        // Unix socket (Linux desktop, macOS)
        endpoints.push(Endpoint::for_toadstool());

        // Tier 2: TCP fallback (universal)
        #[allow(deprecated)]
        endpoints.push(Endpoint::Tcp {
            host: LOCALHOST_IPV4.to_string(),
            port: platform::tcp::DEFAULT_PORT,
        });

        Self { endpoints }
    }

    /// Create client for specific primal
    ///
    /// **Deep Debt**: Runtime discovery, no hardcoding
    pub fn for_primal(primal_name: &str) -> Self {
        let mut endpoints = Vec::new();

        // Tier 1: Platform-specific
        #[cfg(target_os = "linux")]
        {
            endpoints.push(Endpoint::Abstract {
                name: format!("@biomeos_{}", primal_name.to_lowercase()),
            });
        }

        // Unix socket with primal name (ecoBin v2.0 compliant)
        let socket_path = toadstool_common::platform_paths::biomeos_runtime_dir()
            .join(format!("{}.sock", primal_name.to_lowercase()));

        endpoints.push(Endpoint::Unix { path: socket_path });

        // Tier 2: TCP with environment-based port discovery
        // Deep Debt: No hardcoded ports for other primals.
        // Port resolved from environment at runtime.
        let port = Self::resolve_port(primal_name);

        endpoints.push(Endpoint::Tcp {
            host: LOCALHOST_IPV4.to_string(),
            port,
        });

        Self { endpoints }
    }

    /// Resolve TCP port for a primal using environment-first discovery
    ///
    /// **Deep Debt**: Self-knowledge pattern. Ports discovered from environment.
    fn resolve_port(primal_name: &str) -> u16 {
        let env_key = format!("{}_PORT", primal_name.to_uppercase().replace('-', "_"));
        if let Ok(port_str) = std::env::var(&env_key) {
            if let Ok(port) = port_str.parse::<u16>() {
                return port;
            }
        }

        if let Ok(port_str) = std::env::var("BIOMEOS_IPC_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                return port;
            }
        }

        // Self-knowledge: ToadStool's own default port
        #[allow(deprecated)]
        platform::tcp::DEFAULT_PORT
    }

    /// Create client with custom endpoints
    ///
    /// **Deep Debt**: Flexible, allows override
    pub fn with_endpoints(endpoints: Vec<Endpoint>) -> Self {
        Self { endpoints }
    }

    /// Connect with smart fallback
    ///
    /// **Deep Debt**: Tries all transports in order, returns first success
    pub async fn connect(&self) -> ToadStoolResult<IpcStream> {
        let mut last_error = None;

        for endpoint in &self.endpoints {
            match self.try_connect(endpoint).await {
                Ok(stream) => {
                    tracing::debug!("✅ Connected via {}", endpoint.display());
                    return Ok(stream);
                }
                Err(e) => {
                    tracing::debug!("⚠️ Failed to connect via {}: {}", endpoint.display(), e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| ToadStoolError::integration("No endpoints configured".to_string())))
    }

    /// Try to connect to specific endpoint
    async fn try_connect(&self, endpoint: &Endpoint) -> ToadStoolResult<IpcStream> {
        match endpoint {
            Endpoint::Unix { path } => {
                let stream = platform::unix::connect(path).await?;
                Ok(IpcStream::Unix(stream))
            }

            #[cfg(target_os = "linux")]
            Endpoint::Abstract { name } => {
                let stream = platform::abstract_socket::connect(name).await?;
                Ok(IpcStream::Abstract(stream))
            }

            Endpoint::Tcp { host, port } => {
                let stream = platform::tcp::connect(host, *port).await?;
                Ok(IpcStream::Tcp(stream))
            }
        }
    }

    /// Get list of endpoints this client will try
    pub fn endpoints(&self) -> &[Endpoint] {
        &self.endpoints
    }
}

#[cfg(test)]
mod tests {
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
        assert!(endpoints
            .iter()
            .any(|e| { matches!(e, Endpoint::Tcp { .. }) }));

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
    fn test_for_primal_tcp_endpoint_has_valid_port() {
        let client = IpcClient::for_primal("test-primal");
        let tcp_endpoint = client.endpoints().iter().find(|e| e.is_tcp());
        assert!(tcp_endpoint.is_some());
        if let Some(Endpoint::Tcp { port, .. }) = tcp_endpoint {
            assert!(*port > 0, "TCP port should be non-zero");
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
}
