// SPDX-License-Identifier: AGPL-3.0-or-later
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
use toadstool_common::interned_strings::socket_env;
use tokio::io::{AsyncRead, AsyncWrite};

/// Universal IPC stream
///
/// **Deep Debt**: Agnostic wrapper over all transport types
#[derive(Debug)]
pub enum IpcStream {
    /// Filesystem Unix socket stream.
    Unix(tokio::net::UnixStream),
    /// Abstract Unix socket stream (Linux/Android).
    #[cfg(target_os = "linux")]
    Abstract(tokio::net::UnixStream),
    /// TCP stream (cross-device fallback).
    Tcp(tokio::net::TcpStream),
}

impl IpcStream {
    /// Get endpoint info for this stream
    pub const fn endpoint_type(&self) -> &'static str {
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

        if let Ok(port_str) = std::env::var(socket_env::BIOMEOS_IPC_PORT) {
            if let Ok(port) = port_str.parse::<u16>() {
                return port;
            }
        }

        // Self-knowledge: ToadStool's own default port
        platform::tcp::DEFAULT_PORT
    }

    /// Create client from a launcher-injected `TransportEndpoint`.
    ///
    /// Converts the sourDough-standard endpoint into the internal `Endpoint`
    /// representation and uses it as the sole connection target.
    pub fn from_transport_endpoint(
        te: &toadstool_common::TransportEndpoint,
    ) -> ToadStoolResult<Self> {
        let endpoint = match te {
            toadstool_common::TransportEndpoint::Uds { path } => {
                Endpoint::Unix { path: path.clone() }
            }
            toadstool_common::TransportEndpoint::Tcp { host, port } => Endpoint::Tcp {
                host: host.clone(),
                port: *port,
            },
            toadstool_common::TransportEndpoint::MeshRelay {
                peer_id,
                capability,
            } => {
                return Err(ToadStoolError::not_supported(format!(
                    "mesh_relay transport not yet supported (peer={peer_id}, cap={capability})"
                )));
            }
        };
        Ok(Self {
            endpoints: vec![endpoint],
        })
    }

    /// Create client with custom endpoints
    ///
    /// **Deep Debt**: Flexible, allows override
    pub const fn with_endpoints(endpoints: Vec<Endpoint>) -> Self {
        Self { endpoints }
    }

    /// Connect with smart fallback
    ///
    /// **Deep Debt**: Tries all transports in order, returns first success
    ///
    /// # Errors
    ///
    /// Returns error if every endpoint fails or no endpoints are configured.
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
#[path = "client_tests.rs"]
mod tests;
