//! Transport implementations for protocol communication

use std::collections::HashMap;
use std::time::Instant;

use crate::types::{
    ProtocolError, ProtocolMessage, ProtocolResult, ServiceEndpoint, TransportType,
};

/// Connection information for active connections
#[derive(Debug, Clone)]
pub struct Connection {
    pub service_id: String,
    pub endpoint: ServiceEndpoint,
    pub created_at: Instant,
    pub last_used: Instant,
    pub active_requests: u32,
}

/// Transport implementations enum
/// WebSocket removed — use JSON-RPC 2.0 (biomeOS/songbird)
#[derive(Debug, Clone)]
pub enum Transport {
    Http(HttpTransport),
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
            Transport::Http(transport) => transport.send_message(message, endpoint).await,
            Transport::TRpc(transport) => transport.send_message(message, endpoint).await,
        }
    }

    /// Check if this transport supports the given endpoint
    pub fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        match self {
            Transport::Http(transport) => transport.supports_endpoint(endpoint),
            Transport::TRpc(transport) => transport.supports_endpoint(endpoint),
        }
    }

    /// Get transport type
    pub fn transport_type(&self) -> TransportType {
        match self {
            Transport::Http(transport) => transport.transport_type(),
            Transport::TRpc(transport) => transport.transport_type(),
        }
    }
}

/// HTTP transport implementation
///
/// EVOLVED: Deprecated! HTTP is handled by Songbird (architectural inversion!)
/// ToadStool uses Unix sockets for inter-primal communication.
#[derive(Debug, Clone)]
pub struct HttpTransport {
    // No HTTP client! Delegated to Songbird! ✅
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn send_message(
        &self,
        _message: &ProtocolMessage,
        _endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        // EVOLVED: HTTP delegated to Songbird!
        // ToadStool communicates with Songbird via Unix socket
        // Songbird handles external HTTP
        Err(ProtocolError::Transport(
            "HTTP transport deprecated - use Unix sockets to Songbird".to_string(),
        ))
    }

    pub fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        matches!(endpoint.transport, TransportType::Http)
    }

    pub fn transport_type(&self) -> TransportType {
        TransportType::Http
    }
}

/// Pure Rust tRPC transport implementation (Unix socket-based)
///
/// EVOLVED: Uses Unix sockets instead of HTTP! Pure Rust! ✅
#[derive(Debug, Clone)]
pub struct TRpcTransport {
    // Pure Rust Unix sockets! ✅
}

impl Default for TRpcTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl TRpcTransport {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn send_message(
        &self,
        _message: &ProtocolMessage,
        _endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        // EVOLVED: Use UnixStream for inter-primal communication!
        // Implementation would use tokio::net::UnixStream
        Err(ProtocolError::Transport(
            "tRPC over Unix sockets - not yet implemented".to_string(),
        ))
    }

    pub fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        matches!(endpoint.transport, TransportType::TRpc)
    }

    pub fn transport_type(&self) -> TransportType {
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
    pub fn new() -> Self {
        let mut transports = HashMap::new();

        // Register default transports (WebSocket removed — use JSON-RPC 2.0)
        transports.insert(TransportType::Http, Transport::Http(HttpTransport::new()));
        transports.insert(TransportType::TRpc, Transport::TRpc(TRpcTransport::new()));

        Self { transports }
    }

    pub fn register_transport(&mut self, transport: Transport) {
        self.transports
            .insert(transport.transport_type(), transport);
    }

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

    pub fn get_supported_transports(&self) -> Vec<TransportType> {
        self.transports.keys().cloned().collect()
    }
}
