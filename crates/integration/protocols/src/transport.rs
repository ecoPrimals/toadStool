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
        }
    }

    /// Check if this transport supports the given endpoint
    pub const fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        match self {
            Self::Http(transport) => transport.supports_endpoint(endpoint),
            Self::TRpc(transport) => transport.supports_endpoint(endpoint),
        }
    }

    /// Get transport type
    pub const fn transport_type(&self) -> TransportType {
        match self {
            Self::Http(transport) => transport.transport_type(),
            Self::TRpc(transport) => transport.transport_type(),
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
                ProtocolError::Transport(format!("coordination service comms.http_forward: {e}"))
            })?;

        serde_json::from_value(response)
            .map_err(|e| ProtocolError::Transport(format!("response deserialization: {e}")))
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

    /// Check if endpoint uses tRPC transport.
    pub const fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        matches!(endpoint.transport, TransportType::TRpc)
    }

    /// Return transport type.
    pub const fn transport_type(&self) -> TransportType {
        TransportType::TRpc
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
