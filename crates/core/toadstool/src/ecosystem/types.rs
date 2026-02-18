//! # Ecosystem Types
//!
//! Core type definitions for ecosystem coordination and service integration.

#[cfg(feature = "networking")]
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use toadstool_common::constants::timeouts;
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
            discovery_timeout: timeouts::DEFAULT_REQUEST_TIMEOUT,
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
/// This enum supports multiple protocols following the wateringHole standard:
/// - JSON-RPC 2.0 (PRIMARY): Universal language-agnostic access
/// - tarpc (OPTIONAL): High-performance binary RPC for internal paths
/// - HTTP (DEPRECATED): Use Songbird for HTTP/TLS
#[derive(Debug, Clone)]
pub enum ServiceClient {
    /// tarpc client (OPTIONAL - for performance-critical internal paths)
    #[cfg(feature = "networking")]
    Tarpc(Arc<tokio::sync::Mutex<Option<TarpcClientWrapper>>>),

    /// JSON-RPC 2.0 over unix sockets (PRIMARY - wateringHole standard!)
    #[cfg(feature = "networking")]
    UnixSocket(toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient),

    /// No-op client when networking feature is disabled.
    /// Intentional degraded-mode fallback for builds without networking.
    #[cfg(not(feature = "networking"))]
    Disabled,
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

    #[test]
    fn test_default_ecosystem_config() {
        let config = EcosystemConfig::default();
        assert!(config.auto_discovery);
        assert_eq!(config.discovery_timeout, std::time::Duration::from_secs(30));
        assert!(matches!(
            config.discovery_method,
            DiscoveryMethodConfig::Auto
        ));
        assert!(config.required_capabilities.is_empty());
        assert!(config.optional_capabilities.is_empty());
    }

    #[test]
    fn test_config_builder_with_all_options() {
        use toadstool_common::primal_identity::{Capability, ComputeCapability};

        let config = EcosystemConfig::builder()
            .auto_discovery(false)
            .discovery_timeout(std::time::Duration::from_secs(120))
            .discovery_method(DiscoveryMethodConfig::Mdns)
            .require_capability(Capability::Compute(ComputeCapability::NativeExecution))
            .optional_capability(Capability::Compute(ComputeCapability::GpuCompute))
            .build();

        assert!(!config.auto_discovery);
        assert_eq!(config.discovery_timeout.as_secs(), 120);
        assert!(matches!(
            config.discovery_method,
            DiscoveryMethodConfig::Mdns
        ));
        assert_eq!(config.required_capabilities.len(), 1);
        assert_eq!(config.optional_capabilities.len(), 1);
    }

    #[test]
    fn test_discovery_method_config_variants() {
        let _auto = DiscoveryMethodConfig::Auto;
        let _env = DiscoveryMethodConfig::Environment;
        let _mdns = DiscoveryMethodConfig::Mdns;
        let _config_file = DiscoveryMethodConfig::ConfigFile {
            path: "/etc/config.yaml".to_string(),
        };
        let _registry = DiscoveryMethodConfig::Registry {
            endpoint: "http://localhost:8080".to_string(),
        };
    }

    #[test]
    fn test_service_status_discovered_not_usable() {
        let status = ServiceStatus::Discovered;
        assert!(!status.is_usable());
        assert!(!status.is_error());
    }

    #[test]
    fn test_service_status_connecting_not_usable() {
        let status = ServiceStatus::Connecting;
        assert!(!status.is_usable());
        assert!(!status.is_error());
    }

    #[test]
    fn test_service_status_disconnected_not_usable() {
        let status = ServiceStatus::Disconnected;
        assert!(!status.is_usable());
        assert!(!status.is_error());
    }

    #[test]
    fn test_service_status_removing_not_usable() {
        let status = ServiceStatus::Removing;
        assert!(!status.is_usable());
        assert!(!status.is_error());
    }

    #[test]
    fn test_service_status_non_failed_no_error_message() {
        assert_eq!(ServiceStatus::Discovered.error_message(), None);
        assert_eq!(ServiceStatus::Connecting.error_message(), None);
        assert_eq!(ServiceStatus::Connected.error_message(), None);
        assert_eq!(ServiceStatus::Disconnected.error_message(), None);
        assert_eq!(ServiceStatus::Removing.error_message(), None);
    }

    #[test]
    fn test_heartbeat_message() {
        let msg = EcosystemMessage::heartbeat("sender".to_string(), "receiver".to_string());
        assert_eq!(msg.message_type, EcosystemMessageType::Heartbeat);
        assert_eq!(msg.payload, serde_json::json!({}));
        assert_eq!(msg.from, "sender");
        assert_eq!(msg.to, "receiver");
    }

    #[test]
    fn test_error_message() {
        let msg = EcosystemMessage::error(
            "sender".to_string(),
            "receiver".to_string(),
            "oops".to_string(),
        );
        assert_eq!(msg.message_type, EcosystemMessageType::Error);
        assert_eq!(msg.payload["error"], "oops");
        assert_eq!(msg.from, "sender");
        assert_eq!(msg.to, "receiver");
    }

    #[test]
    fn test_all_message_types_requires_response() {
        assert!(
            EcosystemMessageType::ResourceRequest.requires_response(),
            "ResourceRequest should require response"
        );
        assert!(
            EcosystemMessageType::WorkloadRequest.requires_response(),
            "WorkloadRequest should require response"
        );
        assert!(
            !EcosystemMessageType::Heartbeat.requires_response(),
            "Heartbeat should not require response"
        );
        assert!(
            !EcosystemMessageType::CapabilityAnnouncement.requires_response(),
            "CapabilityAnnouncement should not require response"
        );
        assert!(
            !EcosystemMessageType::ResourceResponse.requires_response(),
            "ResourceResponse should not require response"
        );
        assert!(
            !EcosystemMessageType::WorkloadResponse.requires_response(),
            "WorkloadResponse should not require response"
        );
        assert!(
            !EcosystemMessageType::StatusUpdate.requires_response(),
            "StatusUpdate should not require response"
        );
        assert!(
            !EcosystemMessageType::Error.requires_response(),
            "Error should not require response"
        );
    }

    #[test]
    fn test_all_message_types_is_response() {
        assert!(
            EcosystemMessageType::ResourceResponse.is_response(),
            "ResourceResponse should be response"
        );
        assert!(
            EcosystemMessageType::WorkloadResponse.is_response(),
            "WorkloadResponse should be response"
        );
        assert!(
            !EcosystemMessageType::ResourceRequest.is_response(),
            "ResourceRequest should not be response"
        );
        assert!(
            !EcosystemMessageType::WorkloadRequest.is_response(),
            "WorkloadRequest should not be response"
        );
        assert!(
            !EcosystemMessageType::Heartbeat.is_response(),
            "Heartbeat should not be response"
        );
        assert!(
            !EcosystemMessageType::CapabilityAnnouncement.is_response(),
            "CapabilityAnnouncement should not be response"
        );
        assert!(
            !EcosystemMessageType::StatusUpdate.is_response(),
            "StatusUpdate should not be response"
        );
        assert!(
            !EcosystemMessageType::Error.is_response(),
            "Error should not be response"
        );
    }

    #[test]
    fn test_ecosystem_config_serialization() {
        let config = EcosystemConfig {
            auto_discovery: true,
            discovery_timeout: std::time::Duration::from_secs(45),
            discovery_method: DiscoveryMethodConfig::ConfigFile {
                path: "/tmp/config.yaml".to_string(),
            },
            required_capabilities: vec![],
            optional_capabilities: vec![],
        };

        let serialized = serde_json::to_string(&config).expect("serialize config");
        let deserialized: EcosystemConfig =
            serde_json::from_str(&serialized).expect("deserialize config");

        assert_eq!(config.auto_discovery, deserialized.auto_discovery);
        assert_eq!(
            config.discovery_timeout.as_secs(),
            deserialized.discovery_timeout.as_secs()
        );
        match (&config.discovery_method, &deserialized.discovery_method) {
            (
                DiscoveryMethodConfig::ConfigFile { path: p1 },
                DiscoveryMethodConfig::ConfigFile { path: p2 },
            ) => assert_eq!(p1, p2),
            _ => panic!("discovery_method variant mismatch"),
        }
    }

    #[test]
    fn test_ecosystem_message_serialization() {
        let msg = EcosystemMessage::heartbeat("service-a".to_string(), "service-b".to_string());

        let serialized = serde_json::to_string(&msg).expect("serialize message");
        let deserialized: EcosystemMessage =
            serde_json::from_str(&serialized).expect("deserialize message");

        assert_eq!(msg.id, deserialized.id);
        assert_eq!(msg.from, deserialized.from);
        assert_eq!(msg.to, deserialized.to);
        assert_eq!(msg.message_type, deserialized.message_type);
        assert_eq!(msg.payload, deserialized.payload);
    }

    #[test]
    fn test_service_status_serialization() {
        let status = ServiceStatus::Failed("connection refused".to_string());

        let serialized = serde_json::to_string(&status).expect("serialize status");
        let deserialized: ServiceStatus =
            serde_json::from_str(&serialized).expect("deserialize status");

        assert_eq!(status, deserialized);
        assert_eq!(deserialized.error_message(), Some("connection refused"));
    }
}
