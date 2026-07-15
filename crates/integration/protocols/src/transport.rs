// SPDX-License-Identifier: AGPL-3.0-or-later
//! Transport implementations for protocol communication

use std::collections::HashMap;
use std::time::Instant;

use crate::types::{
    ProtocolError, ProtocolMessage, ProtocolResult, ServiceEndpoint, TransportType,
};

/// Connection information for active connections
#[derive(Debug, Clone)]
pub struct Connection {
    /// Connected service identifier
    pub service_id: String,
    /// Endpoint details
    pub endpoint: ServiceEndpoint,
    /// Connection creation time
    pub created_at: Instant,
    /// Last activity timestamp
    pub last_used: Instant,
    /// Number of in-flight requests
    pub active_requests: u32,
}

/// Transport implementations enum.
/// WebSocket removed — use JSON-RPC 2.0 (biomeOS/coordination)
#[derive(Debug, Clone)]
pub enum Transport {
    /// HTTP transport (deprecated; delegated to Coordination)
    Http(HttpTransport),
    /// tRPC over Unix sockets
    TRpc(TRpcTransport),
    /// Binary-framed MessagePack (Rust-to-Rust), with JSON-RPC fallback when negotiation fails
    #[cfg(all(feature = "tarpc-transport", feature = "binary-transport"))]
    BinaryTrpc(BinaryTrpcTransport),
}

impl Transport {
    /// Send message through this transport
    pub async fn send_message(
        &self,
        message: &ProtocolMessage,
        endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        match self {
            Self::Http(transport) => transport.send_message(message, endpoint).await,
            Self::TRpc(transport) => transport.send_message(message, endpoint).await,
            #[cfg(all(feature = "tarpc-transport", feature = "binary-transport"))]
            Self::BinaryTrpc(transport) => transport.send_message(message, endpoint).await,
        }
    }

    /// Check if this transport supports the given endpoint
    pub const fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        match self {
            Self::Http(transport) => transport.supports_endpoint(endpoint),
            Self::TRpc(transport) => transport.supports_endpoint(endpoint),
            #[cfg(all(feature = "tarpc-transport", feature = "binary-transport"))]
            Self::BinaryTrpc(transport) => transport.supports_endpoint(endpoint),
        }
    }

    /// Get transport type
    pub const fn transport_type(&self) -> TransportType {
        match self {
            Self::Http(transport) => transport.transport_type(),
            Self::TRpc(transport) => transport.transport_type(),
            #[cfg(all(feature = "tarpc-transport", feature = "binary-transport"))]
            Self::BinaryTrpc(transport) => transport.transport_type(),
        }
    }
}

/// HTTP transport — delegates to the coordination service primal.
///
/// ToadStool does not perform outbound HTTP itself. When a message needs
/// HTTP transport, we forward it to Coordination's `comms.http_forward` method
/// over the coordination Unix socket. This keeps all network I/O behind
/// the network primal's sovereignty boundary.
#[derive(Debug, Clone)]
pub struct HttpTransport {}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport {
    /// Create HTTP transport handle.
    pub const fn new() -> Self {
        Self {}
    }

    /// Forward message to Coordination for HTTP delivery.
    ///
    /// Returns `HttpTransportNotAvailable` when Coordination's coordination
    /// socket cannot be discovered at runtime.
    pub async fn send_message(
        &self,
        message: &ProtocolMessage,
        endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        #[cfg(not(unix))]
        {
            let _ = (message, endpoint);
            return Err(ProtocolError::HttpTransportNotAvailable);
        }
        #[cfg(unix)]
        {
            use toadstool_common::primal_sockets::get_socket_path_for_capability;
            use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

            let socket = get_socket_path_for_capability("coordination");
            if !socket.exists() {
                tracing::debug!(
                    "No coordination primal socket at {}, HTTP transport unavailable",
                    socket.display(),
                );
                return Err(ProtocolError::HttpTransportNotAvailable);
            }

            let client = UnixJsonRpcClient::new(socket);

            let url = if let Some(ref path) = endpoint.path {
                format!("http://{}:{}{path}", endpoint.address, endpoint.port)
            } else {
                format!("http://{}:{}", endpoint.address, endpoint.port)
            };

            let params = serde_json::json!({
                "url": url,
                "method": "POST",
                "body": serde_json::to_value(message)
                    .map_err(|e| ProtocolError::Transport(format!("serialization: {e}")))?,
                "headers": {
                    "Content-Type": "application/json",
                },
            });

            let response = client
                .call("comms.http_forward", params)
                .await
                .map_err(|e| {
                    ProtocolError::Transport(format!(
                        "coordination service comms.http_forward: {e}"
                    ))
                })?;

            serde_json::from_value(response)
                .map_err(|e| ProtocolError::Transport(format!("response deserialization: {e}")))
        }
    }

    /// Check if endpoint uses HTTP transport
    pub const fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        matches!(endpoint.transport, TransportType::Http)
    }

    /// Return transport type
    pub const fn transport_type(&self) -> TransportType {
        TransportType::Http
    }
}

/// tRPC transport (primal-to-primal over Unix sockets).
///
/// Per `wateringHole/PRIMAL_IPC_PROTOCOL.md`, JSON-RPC is the **required** IPC
/// protocol — all primals must accept it. tarpc is the **optional** high-perf
/// secondary for Rust-to-Rust paths.
///
/// This transport resolves the target primal's Unix socket and sends messages
/// via JSON-RPC 2.0 (NDJSON). When the `tarpc-transport` feature is enabled,
/// future evolution may negotiate tarpc binary framing for eligible peers.
#[derive(Debug, Clone)]
pub struct TRpcTransport {}

impl Default for TRpcTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl TRpcTransport {
    /// Create a new tRPC transport.
    pub const fn new() -> Self {
        Self {}
    }

    /// Send message to a primal via its Unix socket.
    ///
    /// Resolves the socket path from the endpoint's `path` field (explicit socket)
    /// or falls back to capability-based discovery using the endpoint `address`
    /// as the capability domain name.
    ///
    /// The message is forwarded as a JSON-RPC `protocol.forward` call so the
    /// receiving primal can route it by `message_type`.
    pub async fn send_message(
        &self,
        message: &ProtocolMessage,
        endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        #[cfg(not(unix))]
        {
            let _ = (message, endpoint);
            return Err(ProtocolError::TRpcTransportNotAvailable);
        }
        #[cfg(unix)]
        {
            use std::path::PathBuf;
            use toadstool_common::primal_sockets::get_socket_path_for_capability;
            use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

            let socket = if let Some(ref path) = endpoint.path {
                PathBuf::from(path)
            } else {
                get_socket_path_for_capability(&endpoint.address)
            };

            if !socket.exists() {
                tracing::debug!(
                    socket = %socket.display(),
                    address = %endpoint.address,
                    "tRPC target socket not found — primal may not be running",
                );
                return Err(ProtocolError::TRpcTransportNotAvailable);
            }

            let client = UnixJsonRpcClient::new(socket);

            let params = serde_json::to_value(message)
                .map_err(|e| ProtocolError::Transport(format!("message serialization: {e}")))?;

            let method = format!("protocol.forward.{}", message.message_type);

            let response = client.call(&method, params).await.map_err(|e| {
                ProtocolError::Transport(format!("Unix JSON-RPC to {}: {e}", endpoint.address))
            })?;

            serde_json::from_value(response)
                .map_err(|e| ProtocolError::Transport(format!("response deserialization: {e}")))
        }
    }

    /// Check if endpoint uses tRPC transport.
    pub const fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        matches!(endpoint.transport, TransportType::TRpc)
    }

    /// Return transport type.
    pub const fn transport_type(&self) -> TransportType {
        TransportType::TRpc
    }
}

/// Binary primal transport: `TSB1` + version handshake, then MessagePack frames (length-delimited),
/// matching the codec stack used by [`tarpc::serde_transport`] over TCP/Unix.
///
/// If the peer does not complete the handshake (e.g. speaks JSON-RPC only), falls back to
/// [`TRpcTransport`] on the same socket path / host:port.
#[cfg(all(feature = "tarpc-transport", feature = "binary-transport"))]
#[derive(Debug, Clone)]
pub struct BinaryTrpcTransport {}

#[cfg(all(feature = "tarpc-transport", feature = "binary-transport"))]
impl Default for BinaryTrpcTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(feature = "tarpc-transport", feature = "binary-transport"))]
const BINARY_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(all(feature = "tarpc-transport", feature = "binary-transport"))]
impl BinaryTrpcTransport {
    /// Create a binary tRPC transport handle.
    pub const fn new() -> Self {
        Self {}
    }

    /// Send message using binary framing, or JSON-RPC if the peer rejects the handshake.
    pub async fn send_message(
        &self,
        message: &ProtocolMessage,
        endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        use std::net::SocketAddr;
        use std::path::PathBuf;
        use std::time::Duration;

        use futures::{SinkExt, StreamExt};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;
        use tokio::net::UnixStream;
        use tokio_serde::formats::SymmetricalMessagePack;
        use tokio_util::codec::LengthDelimitedCodec;

        const HANDSHAKE_MAGIC: &[u8; 4] = b"TSB1";
        const HANDSHAKE_VERSION: u32 = 1;

        async fn run_binary_roundtrip<S>(
            mut stream: S,
            message: &ProtocolMessage,
        ) -> ProtocolResult<ProtocolMessage>
        where
            S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        {
            let mut hdr = [0u8; 8];
            hdr[0..4].copy_from_slice(HANDSHAKE_MAGIC);
            hdr[4..8].copy_from_slice(&HANDSHAKE_VERSION.to_be_bytes());
            stream
                .write_all(&hdr)
                .await
                .map_err(|e| ProtocolError::Negotiation(format!("binary handshake write: {e}")))?;
            stream
                .flush()
                .await
                .map_err(|e| ProtocolError::Negotiation(format!("binary handshake flush: {e}")))?;
            stream
                .read_exact(&mut hdr)
                .await
                .map_err(|e| ProtocolError::Negotiation(format!("binary handshake read: {e}")))?;
            if &hdr[0..4] != HANDSHAKE_MAGIC {
                return Err(ProtocolError::Negotiation(
                    "peer binary handshake magic mismatch".to_string(),
                ));
            }
            let ver = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]);
            if ver != HANDSHAKE_VERSION {
                return Err(ProtocolError::Negotiation(format!(
                    "unsupported binary protocol version {ver}"
                )));
            }

            let framed = LengthDelimitedCodec::builder()
                .max_frame_length(16 * 1024 * 1024)
                .new_framed(stream);
            let mut t = tarpc::serde_transport::new(
                framed,
                SymmetricalMessagePack::<ProtocolMessage>::default(),
            );
            t.send(message.clone())
                .await
                .map_err(|e| ProtocolError::Transport(format!("binary MessagePack send: {e}")))?;
            t.next()
                .await
                .transpose()
                .map_err(|e| ProtocolError::Transport(format!("binary MessagePack recv: {e}")))?
                .ok_or_else(|| {
                    ProtocolError::Transport("binary peer closed before response".to_string())
                })
        }

        let try_binary = async {
            if let Some(ref path) = endpoint.path {
                let socket = PathBuf::from(path);
                if !socket.exists() {
                    return Err(ProtocolError::TRpcTransportNotAvailable);
                }
                let stream = UnixStream::connect(&socket).await.map_err(|e| {
                    ProtocolError::Transport(format!("Unix connect {}: {e}", socket.display()))
                })?;
                tokio::time::timeout(
                    BINARY_HANDSHAKE_TIMEOUT,
                    run_binary_roundtrip(stream, message),
                )
                .await
                .map_err(|_| ProtocolError::Timeout("binary handshake".to_string()))?
            } else {
                let addr: SocketAddr = format!("{}:{}", endpoint.address, endpoint.port)
                    .parse()
                    .map_err(|e| ProtocolError::Transport(format!("invalid TCP address: {e}")))?;
                let stream = TcpStream::connect(addr)
                    .await
                    .map_err(|e| ProtocolError::Transport(format!("TCP connect: {e}")))?;
                tokio::time::timeout(
                    BINARY_HANDSHAKE_TIMEOUT,
                    run_binary_roundtrip(stream, message),
                )
                .await
                .map_err(|_| ProtocolError::Timeout("binary handshake".to_string()))?
            }
        };

        match try_binary.await {
            Ok(m) => Ok(m),
            Err(e) => {
                tracing::debug!(error = %e, "binary primal transport failed; falling back to JSON-RPC");
                let mut json_endpoint = endpoint.clone();
                json_endpoint.transport = TransportType::TRpc;
                TRpcTransport::new()
                    .send_message(message, &json_endpoint)
                    .await
            }
        }
    }

    /// Binary transport is selected only for [`TransportType::Binary`] endpoints.
    pub const fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        matches!(endpoint.transport, TransportType::Binary)
    }

    /// Return transport type.
    pub const fn transport_type(&self) -> TransportType {
        TransportType::Binary
    }
}

/// Transport manager for handling multiple transport types
#[derive(Debug)]
pub struct TransportManager {
    transports: HashMap<TransportType, Transport>,
}

impl Default for TransportManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TransportManager {
    /// Create transport manager with default HTTP and tRPC transports
    pub fn new() -> Self {
        let mut transports = HashMap::new();

        // Register default transports (WebSocket removed — use JSON-RPC 2.0)
        transports.insert(TransportType::Http, Transport::Http(HttpTransport::new()));
        transports.insert(TransportType::TRpc, Transport::TRpc(TRpcTransport::new()));
        #[cfg(all(feature = "tarpc-transport", feature = "binary-transport"))]
        transports.insert(
            TransportType::Binary,
            Transport::BinaryTrpc(BinaryTrpcTransport::new()),
        );

        Self { transports }
    }

    /// Register a transport for its type
    pub fn register_transport(&mut self, transport: Transport) {
        self.transports
            .insert(transport.transport_type(), transport);
    }

    /// Send message via appropriate transport for endpoint
    pub async fn send_message(
        &self,
        message: &ProtocolMessage,
        endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        let transport = self.transports.get(&endpoint.transport).ok_or_else(|| {
            ProtocolError::Transport(format!("No transport handler for {:?}", endpoint.transport))
        })?;

        if !transport.supports_endpoint(endpoint) {
            return Err(ProtocolError::Transport(format!(
                "Transport {:?} does not support endpoint",
                endpoint.transport
            )));
        }

        transport.send_message(message, endpoint).await
    }

    /// Get list of registered transport types
    pub fn get_supported_transports(&self) -> Vec<TransportType> {
        self.transports.keys().cloned().collect()
    }
}
