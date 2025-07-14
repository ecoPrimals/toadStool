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
#[derive(Debug, Clone)]
pub enum Transport {
    Http(HttpTransport),
    WebSocket(WebSocketTransport),
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
            Transport::WebSocket(transport) => transport.send_message(message, endpoint).await,
            Transport::TRpc(transport) => transport.send_message(message, endpoint).await,
        }
    }

    /// Check if this transport supports the given endpoint
    pub fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        match self {
            Transport::Http(transport) => transport.supports_endpoint(endpoint),
            Transport::WebSocket(transport) => transport.supports_endpoint(endpoint),
            Transport::TRpc(transport) => transport.supports_endpoint(endpoint),
        }
    }

    /// Get transport type
    pub fn transport_type(&self) -> TransportType {
        match self {
            Transport::Http(transport) => transport.transport_type(),
            Transport::WebSocket(transport) => transport.transport_type(),
            Transport::TRpc(transport) => transport.transport_type(),
        }
    }
}

/// HTTP transport implementation
#[derive(Debug, Clone)]
pub struct HttpTransport {
    client: reqwest::Client,
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn send_message(
        &self,
        message: &ProtocolMessage,
        endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        let url = if endpoint.tls_enabled {
            format!(
                "https://{}:{}{}",
                endpoint.address,
                endpoint.port,
                endpoint.path.as_deref().unwrap_or("")
            )
        } else {
            format!(
                "http://{}:{}{}",
                endpoint.address,
                endpoint.port,
                endpoint.path.as_deref().unwrap_or("")
            )
        };

        let response = self
            .client
            .post(&url)
            .json(message)
            .send()
            .await
            .map_err(|e| ProtocolError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProtocolError::Transport(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let response_message: ProtocolMessage = response
            .json()
            .await
            .map_err(|e| ProtocolError::Serialization(e.to_string()))?;

        Ok(response_message)
    }

    pub fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        matches!(endpoint.transport, TransportType::Http)
    }

    pub fn transport_type(&self) -> TransportType {
        TransportType::Http
    }
}

/// WebSocket transport implementation
#[derive(Debug, Clone)]
pub struct WebSocketTransport {
    // WebSocket client would be implemented here
}

impl Default for WebSocketTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketTransport {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn send_message(
        &self,
        _message: &ProtocolMessage,
        _endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        // WebSocket implementation would go here
        // For now, return a placeholder response
        Err(ProtocolError::Transport(
            "WebSocket not implemented".to_string(),
        ))
    }

    pub fn supports_endpoint(&self, endpoint: &ServiceEndpoint) -> bool {
        matches!(endpoint.transport, TransportType::WebSocket)
    }

    pub fn transport_type(&self) -> TransportType {
        TransportType::WebSocket
    }
}

/// Pure Rust tRPC transport implementation (HTTP+WebSocket hybrid)
#[derive(Debug, Clone)]
pub struct TRpcTransport {
    http_client: reqwest::Client,
    // WebSocket connections would be managed here
}

impl Default for TRpcTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl TRpcTransport {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn send_message(
        &self,
        message: &ProtocolMessage,
        endpoint: &ServiceEndpoint,
    ) -> ProtocolResult<ProtocolMessage> {
        // tRPC implementation using HTTP POST with JSON
        let url = if endpoint.tls_enabled {
            format!(
                "https://{}:{}/trpc{}",
                endpoint.address,
                endpoint.port,
                endpoint.path.as_deref().unwrap_or("")
            )
        } else {
            format!(
                "http://{}:{}/trpc{}",
                endpoint.address,
                endpoint.port,
                endpoint.path.as_deref().unwrap_or("")
            )
        };

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(message)
            .send()
            .await
            .map_err(|e| ProtocolError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProtocolError::Transport(format!(
                "tRPC request failed with status: {}",
                response.status()
            )));
        }

        let response_message: ProtocolMessage = response
            .json()
            .await
            .map_err(|e| ProtocolError::Serialization(e.to_string()))?;

        Ok(response_message)
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

        // Register default transports
        transports.insert(TransportType::Http, Transport::Http(HttpTransport::new()));
        transports.insert(
            TransportType::WebSocket,
            Transport::WebSocket(WebSocketTransport::new()),
        );
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
