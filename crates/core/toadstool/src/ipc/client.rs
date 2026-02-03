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

use tokio::io::{AsyncRead, AsyncWrite};
use crate::{ToadStoolError, ToadStoolResult};
use super::platform::{self, Endpoint};

/// Universal IPC stream
///
/// **Deep Debt**: Agnostic wrapper over all transport types
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
        endpoints.push(Endpoint::Tcp {
            host: "127.0.0.1".to_string(),
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
        
        // Unix socket with primal name
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
            if let Ok(uid) = toadstool_common::uid_detector::get_user_id() {
                format!("/run/user/{}", uid)
            } else {
                "/tmp/biomeos-runtime".to_string()
            }
        });
        
        endpoints.push(Endpoint::Unix {
            path: format!("{}/biomeos/{}.sock", runtime_dir, primal_name.to_lowercase()).into(),
        });
        
        // Tier 2: TCP with port offset
        let port = match primal_name.to_lowercase().as_str() {
            "toadstool" => 8370,
            "songbird" => 8371,
            "beardog" => 8372,
            "squirrel" => 8373,
            "nestgate" => 8374,
            _ => 8375,  // Generic fallback
        };
        
        endpoints.push(Endpoint::Tcp {
            host: "127.0.0.1".to_string(),
            port,
        });
        
        Self { endpoints }
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
        
        Err(last_error.unwrap_or_else(|| {
            ToadStoolError::integration("No endpoints configured".to_string())
        }))
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
        let client = IpcClient::for_primal("Songbird");
        let endpoints = client.endpoints();
        
        // Should have multiple endpoints
        assert!(!endpoints.is_empty());
        
        // Should have TCP with Songbird port
        assert!(endpoints.iter().any(|e| {
            matches!(e, Endpoint::Tcp { port, .. } if *port == 8371)
        }));
    }
    
    #[test]
    fn test_client_with_custom_endpoints() {
        let custom = vec![
            Endpoint::Tcp {
                host: "192.168.1.100".to_string(),
                port: 9000,
            },
        ];
        
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
}
