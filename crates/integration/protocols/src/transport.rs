// SPDX-License-Identifier: AGPL-3.0-only
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
/// WebSocket removed — use JSON-RPC 2.0 (biomeOS/songbird)
#[derive(Debug, Clone)]
pub enum Transport {
    /// HTTP transport (deprecated; delegated to Songbird)
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

/// HTTP transport — delegates to the Songbird coordination primal.
///
/// ToadStool does not perform outbound HTTP itself. When a message needs
/// HTTP transport, we forward it to Songbird's `comms.http_forward` method
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

    /// Forward message to Songbird for HTTP delivery.
    ///
    /// Returns `HttpTransportNotAvailable` when Songbird's coordination
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
            .map_err(|e| ProtocolError::Transport(format!("Songbird comms.http_forward: {e}")))?;

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

/// tRPC transport (tarpc over Unix sockets)
///
/// Phase 3 transport: JSON-RPC is the required primary protocol per
/// `wateringHole/PRIMAL_IPC_PROTOCOL.md`; tarpc is the optional high-performance
/// secondary for primal-to-primal Rust-to-Rust paths.
#[derive(Debug, Clone)]
pub struct TRpcTransport {}

impl Default for TRpcTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl TRpcTransport {
    /// Create a new tRPC transport
    pub const fn new() -> Self {
        Self {}
    }

    /// Send message via tarpc over Unix sockets.
    ///
    /// Requires the `tarpc-transport` feature. Without it, returns
    /// [`ProtocolError::TRpcTransportNotAvailable`] at runtime.
    /// JSON-RPC (primary) via `pure_jsonrpc` is the required IPC path;
    /// tarpc is the optional high-performance secondary.
    pub async fn send_message(
        &self,
        _message: &ProtocolMessage,
        _endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        #[cfg(feature = "tarpc-transport")]
        {
            // TODO(tarpc-phase3): wire real tarpc call here once
            //   toadstool_common::tarpc_service is stabilized.
            let _ = (_message, _endpoint);
            Err(ProtocolError::TRpcTransportNotAvailable)
        }
        #[cfg(not(feature = "tarpc-transport"))]
        {
            Err(ProtocolError::TRpcTransportNotAvailable)
        }
    }

    /// Check if endpoint uses tRPC transport
    pub const fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        matches!(endpoint.transport, TransportType::TRpc)
    }

    /// Return transport type
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
