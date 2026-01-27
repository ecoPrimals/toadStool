//! # Ecosystem Types
//!
//! Core type definitions for ecosystem coordination and service integration.

#[cfg(any(feature = "networking", feature = "websocket"))]
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use toadstool_common::primal_identity::Capability;
use toadstool_common::service_discovery::DiscoveredService;

/// Discovered service instance (type alias for clarity)
pub type ServiceInstance = DiscoveredService;

/// Configuration for ecosystem integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConfig {
    /// Enable auto-discovery of services
    pub auto_discovery: bool,
    /// Discovery timeout
    pub discovery_timeout: std::time::Duration,
    /// Discovery method to use
    pub discovery_method: DiscoveryMethodConfig,
    /// Required capabilities for operation
    pub required_capabilities: Vec<Capability>,
    /// Optional capabilities for enhanced functionality
    pub optional_capabilities: Vec<Capability>,
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            auto_discovery: true,
            discovery_timeout: std::time::Duration::from_secs(30),
            discovery_method: DiscoveryMethodConfig::Auto,
            // No hardcoded primal names - discover by capability instead
            required_capabilities: vec![],
            optional_capabilities: vec![],
        }
    }
}

impl EcosystemConfig {
    /// Create a new config builder
    pub fn builder() -> EcosystemConfigBuilder {
        EcosystemConfigBuilder::default()
    }
}

/// Builder for EcosystemConfig (fluent API)
#[derive(Default)]
pub struct EcosystemConfigBuilder {
    auto_discovery: bool,
    discovery_timeout: std::time::Duration,
    discovery_method: DiscoveryMethodConfig,
    required_capabilities: Vec<Capability>,
    optional_capabilities: Vec<Capability>,
}

impl EcosystemConfigBuilder {
    /// Enable or disable auto-discovery
    pub fn auto_discovery(mut self, enabled: bool) -> Self {
        self.auto_discovery = enabled;
        self
    }

    /// Set discovery timeout
    pub fn discovery_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.discovery_timeout = timeout;
        self
    }

    /// Set discovery method
    pub fn discovery_method(mut self, method: DiscoveryMethodConfig) -> Self {
        self.discovery_method = method;
        self
    }

    /// Add a required capability
    pub fn require_capability(mut self, capability: Capability) -> Self {
        self.required_capabilities.push(capability);
        self
    }

    /// Add an optional capability
    pub fn optional_capability(mut self, capability: Capability) -> Self {
        self.optional_capabilities.push(capability);
        self
    }

    /// Build the configuration
    pub fn build(self) -> EcosystemConfig {
        EcosystemConfig {
            auto_discovery: self.auto_discovery,
            discovery_timeout: self.discovery_timeout,
            discovery_method: self.discovery_method,
            required_capabilities: self.required_capabilities,
            optional_capabilities: self.optional_capabilities,
        }
    }
}

/// Discovery method configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum DiscoveryMethodConfig {
    /// Automatic selection
    #[default]
    Auto,
    /// Environment variables only
    Environment,
    /// mDNS discovery
    Mdns,
    /// Configuration file
    ConfigFile { path: String },
    /// Registry service
    Registry { endpoint: String },
}


/// Status of a service instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Discovered but not connected
    Discovered,
    /// Connecting to service
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection failed
    Failed(String),
    /// Disconnected
    Disconnected,
    /// Service is being removed
    Removing,
}

impl ServiceStatus {
    /// Check if service is usable
    pub fn is_usable(&self) -> bool {
        matches!(self, ServiceStatus::Connected)
    }

    /// Check if service is in error state
    pub fn is_error(&self) -> bool {
        matches!(self, ServiceStatus::Failed(_))
    }

    /// Get error message if in failed state
    pub fn error_message(&self) -> Option<&str> {
        match self {
            ServiceStatus::Failed(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Communication channel with a service
#[derive(Debug, Clone)]
pub struct ServiceChannel {
    /// Service identifier
    pub service_id: String,
    /// Service name (for logging/debugging only)
    pub service_name: String,
    /// Service endpoint
    pub endpoint: String,
    /// Client type
    pub client: ServiceClient,
    /// Last successful heartbeat
    pub last_heartbeat: DateTime<Utc>,
    /// Current status
    pub status: ServiceStatus,
}

/// Client for communicating with services
///
/// This enum supports multiple protocols following the ecosystem pattern:
/// - tarpc (PRIMARY): High-performance binary RPC
/// - JSON-RPC (PRIMARY): Universal language-agnostic access
/// - HTTP (FALLBACK): Legacy/debugging
#[derive(Debug, Clone)]
pub enum ServiceClient {
    /// tarpc client (PRIMARY - high performance)
    #[cfg(feature = "networking")]
    Tarpc(Arc<tokio::sync::Mutex<Option<TarpcClientWrapper>>>),

    /// JSON-RPC 2.0 over unix sockets (PRIMARY - pure Rust!)
    #[cfg(feature = "networking")]
    UnixSocket(toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient),

    /// WebSocket client (real-time bidirectional)
    #[cfg(feature = "websocket")]
    WebSocket(
        Arc<
            tokio::sync::Mutex<
                Option<
                    tokio_tungstenite::WebSocketStream<
                        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                    >,
                >,
            >,
        >,
    ),

    /// Mock client for testing without networking
    #[cfg(not(feature = "networking"))]
    Mock,
}

/// Wrapper for tarpc client to make it cloneable
#[cfg(feature = "networking")]
#[derive(Debug)]
pub struct TarpcClientWrapper {
    // Placeholder for actual tarpc client
    // Will be populated when we wire up the tarpc integration
    _marker: std::marker::PhantomData<()>,
}

#[cfg(feature = "networking")]
impl Clone for TarpcClientWrapper {
    fn clone(&self) -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

/// Ecosystem message for primal communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemMessage {
    /// Unique message identifier
    pub id: Uuid,
    /// Source service ID
    pub from: String,
    /// Destination service ID
    pub to: String,
    /// Message type
    pub message_type: EcosystemMessageType,
    /// Message payload (JSON)
    pub payload: serde_json::Value,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
}

impl EcosystemMessage {
    /// Create a new message
    pub fn new(
        from: String,
        to: String,
        message_type: EcosystemMessageType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to,
            message_type,
            payload,
            timestamp: Utc::now(),
        }
    }

    /// Create a heartbeat message
    pub fn heartbeat(from: String, to: String) -> Self {
        Self::new(
            from,
            to,
            EcosystemMessageType::Heartbeat,
            serde_json::json!({}),
        )
    }

    /// Create an error message
    pub fn error(from: String, to: String, error: String) -> Self {
        Self::new(
            from,
            to,
            EcosystemMessageType::Error,
            serde_json::json!({ "error": error }),
        )
    }
}

/// Types of ecosystem messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EcosystemMessageType {
    /// Heartbeat message
    Heartbeat,
    /// Capability announcement
    CapabilityAnnouncement,
    /// Resource request
    ResourceRequest,
    /// Resource response
    ResourceResponse,
    /// Workload request
    WorkloadRequest,
    /// Workload response
    WorkloadResponse,
    /// Status update
    StatusUpdate,
    /// Error message
    Error,
}

impl EcosystemMessageType {
    /// Check if this message requires a response
    pub fn requires_response(&self) -> bool {
        matches!(
            self,
            EcosystemMessageType::ResourceRequest | EcosystemMessageType::WorkloadRequest
        )
    }

    /// Check if this is a response message
    pub fn is_response(&self) -> bool {
        matches!(
            self,
            EcosystemMessageType::ResourceResponse | EcosystemMessageType::WorkloadResponse
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = EcosystemConfig::builder()
            .auto_discovery(true)
            .discovery_timeout(std::time::Duration::from_secs(60))
            .build();

        assert!(config.auto_discovery);
        assert_eq!(config.discovery_timeout.as_secs(), 60);
    }

    #[test]
    fn test_service_status() {
        let status = ServiceStatus::Connected;
        assert!(status.is_usable());
        assert!(!status.is_error());

        let failed = ServiceStatus::Failed("test error".to_string());
        assert!(!failed.is_usable());
        assert!(failed.is_error());
        assert_eq!(failed.error_message(), Some("test error"));
    }

    #[test]
    fn test_message_creation() {
        let msg = EcosystemMessage::new(
            "service-1".to_string(),
            "service-2".to_string(),
            EcosystemMessageType::Heartbeat,
            serde_json::json!({}),
        );

        assert_eq!(msg.from, "service-1");
        assert_eq!(msg.to, "service-2");
        assert_eq!(msg.message_type, EcosystemMessageType::Heartbeat);
    }

    #[test]
    fn test_message_type_properties() {
        assert!(EcosystemMessageType::ResourceRequest.requires_response());
        assert!(!EcosystemMessageType::Heartbeat.requires_response());

        assert!(EcosystemMessageType::ResourceResponse.is_response());
        assert!(!EcosystemMessageType::ResourceRequest.is_response());
    }
}
