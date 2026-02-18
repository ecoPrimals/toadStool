//! Core types and data structures for protocol integration

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Protocol integration errors
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Protocol negotiation failed: {0}")]
    Negotiation(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Service discovery error: {0}")]
    Discovery(String),

    #[error("Message routing error: {0}")]
    Routing(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type ProtocolResult<T> = Result<T, ProtocolError>;

/// Message formats supported by the protocol system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageFormat {
    /// JSON format (human-readable)
    Json,
    /// MessagePack (binary, compact)
    MessagePack,
    /// CBOR (binary, extensible)
    Cbor,
    /// Custom format
    Custom(String),
}

/// Transport types for message delivery
/// WebSocket removed — use JSON-RPC 2.0 (biomeOS/songbird)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TransportType {
    /// HTTP/HTTPS transport
    Http,
    /// Pure Rust tRPC transport
    TRpc,
    /// TCP socket transport
    Tcp,
    /// UDP socket transport
    Udp,
    /// Custom transport
    Custom(String),
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    /// No authentication
    None,
    /// Bearer token authentication
    Bearer,
    /// API key authentication
    ApiKey,
    /// Mutual TLS authentication
    MutualTls,
    /// JWT authentication
    Jwt,
    /// Custom authentication
    Custom(String),
}

/// Message priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum MessagePriority {
    /// Low priority message
    Low,
    /// Normal priority message
    #[default]
    Normal,
    /// High priority message
    High,
    /// Critical priority message
    Critical,
    /// Emergency priority message
    Emergency,
}

/// Health status for services and endpoints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is unhealthy
    Unhealthy,
    /// Service status is unknown
    Unknown,
}

/// Protocol message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// Message ID
    pub id: Uuid,

    /// Message type
    pub message_type: String,

    /// Source service ID
    pub source: String,

    /// Destination service ID
    pub destination: Option<String>,

    /// Message payload
    pub payload: serde_json::Value,

    /// Message headers
    pub headers: HashMap<String, String>,

    /// Message timestamp
    pub timestamp: DateTime<Utc>,

    /// Message format
    pub format: MessageFormat,

    /// Correlation ID for request-response patterns
    pub correlation_id: Option<Uuid>,

    /// Reply-to address for responses
    pub reply_to: Option<String>,

    /// Message TTL
    pub ttl: Option<Duration>,

    /// Message priority
    pub priority: MessagePriority,
}

/// Service information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service ID
    pub id: String,

    /// Service name
    pub name: String,

    /// Service version
    pub version: String,

    /// Service endpoints
    pub endpoints: Vec<ServiceEndpoint>,

    /// Service metadata
    pub metadata: HashMap<String, String>,

    /// Service health status
    pub health_status: HealthStatus,

    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,

    /// Service capabilities
    pub capabilities: Vec<String>,
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Endpoint ID
    pub id: String,

    /// Transport type
    pub transport: TransportType,

    /// Endpoint address
    pub address: String,

    /// Endpoint port
    pub port: u16,

    /// Endpoint path
    pub path: Option<String>,

    /// TLS enabled
    pub tls_enabled: bool,

    /// Endpoint health status
    pub health_status: HealthStatus,
}

/// Protocol events for monitoring and notifications
#[derive(Debug, Clone)]
pub enum ProtocolEvent {
    /// Service registered
    ServiceRegistered { service: ServiceInfo },

    /// Service deregistered
    ServiceDeregistered { service_id: String },

    /// Service health changed
    ServiceHealthChanged {
        service_id: String,
        status: HealthStatus,
    },

    /// Message sent
    MessageSent {
        message_id: Uuid,
        destination: String,
    },

    /// Message received
    MessageReceived { message_id: Uuid, source: String },

    /// Connection established
    ConnectionEstablished {
        service_id: String,
        endpoint: String,
    },

    /// Connection lost
    ConnectionLost { service_id: String, error: String },
}

/// Message handler trait for processing incoming messages
pub trait MessageHandler: Send + Sync {
    /// Handle incoming message
    fn handle_message(
        &self,
        message: ProtocolMessage,
    ) -> Result<Option<ProtocolMessage>, ProtocolError>;
}
