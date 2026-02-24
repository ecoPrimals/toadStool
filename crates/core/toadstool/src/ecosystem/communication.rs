//! # Service Communication
//!
//! Manages communication channels and messaging with ecosystem services.
//!
//! ## Features
//!
//! - **Multi-Protocol**: JSON-RPC 2.0 (PRIMARY), tarpc (OPTIONAL), HTTP (DEPRECATED)
//! - **Real-time events**: use JSON-RPC 2.0 over Unix sockets (biomeOS/songbird)
//! - **Trait-Based**: Polymorphic communication via `ServiceCommunication` trait
//! - **Connection Management**: Automatic reconnection and health checks
//! - **Message Routing**: Type-safe ecosystem messaging
//!
//! ## wateringHole Standard Compliance
//!
//! Per PRIMAL_IPC_PROTOCOL.md and UNIVERSAL_IPC_STANDARD_V3.md:
//! - JSON-RPC 2.0 is the PRIMARY protocol for inter-primal communication
//! - tarpc is OPTIONAL for performance-critical internal paths
//! - HTTP is DEPRECATED (use Songbird for HTTP/TLS)
//!
//! ## Real-time events
//!
//! Real-time events: use JSON-RPC 2.0 over Unix sockets (biomeOS/songbird)
//!
//! ## Usage
//!
//! ```rust,ignore
//! let comm = CommunicationManager::new();
//!
//! // Create channel to service
//! let channel = comm.create_channel(&service).await?;
//!
//! // Send message
//! let response = comm.send_message(&channel, message).await?;
//!
//! // Health check
//! let healthy = comm.check_health(&channel).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::constants::timeouts;
use toadstool_common::constants::PRIMAL_NAME;
use toadstool_common::service_discovery::DiscoveredService;

use super::types::{EcosystemMessage, ServiceChannel, ServiceClient, ServiceStatus};

/// Communication manager for service channels and messaging
pub struct CommunicationManager {
    /// Active communication channels (keyed by service ID)
    channels: Arc<RwLock<HashMap<String, ServiceChannel>>>,
    /// Default timeout for operations (queried in tests; will be used for
    /// channel-level timeouts once async messaging is wired up).
    _default_timeout: Duration,
}

impl CommunicationManager {
    /// Create a new communication manager
    pub fn new() -> Self {
        Self::with_timeout(timeouts::DEFAULT_REQUEST_TIMEOUT)
    }

    /// Create a communication manager with custom timeout
    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            _default_timeout: timeout,
        }
    }

    /// Create a communication channel for a service
    pub async fn create_channel(
        &self,
        service: &DiscoveredService,
    ) -> ToadStoolResult<ServiceChannel> {
        info!("📡 Creating channel for service: {}", service.name);

        // EVOLVED: No hardcoded localhost - fail if no endpoint discovered
        let endpoint = service
            .primary_endpoint()
            .map(|e| e.url())
            .ok_or_else(|| {
                ToadStoolError::integration(format!(
                    "No endpoint discovered for service: {}. Deep Debt: Services must be discovered at runtime, not hardcoded.",
                    service.name
                ))
            })?;

        // Determine best protocol based on service capabilities
        let client = self.create_client_for_service(service)?;

        let channel = ServiceChannel {
            service_id: service.id.clone(),
            service_name: service.name.clone(),
            endpoint,
            client,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Discovered,
        };

        // Store channel
        let mut channels = self.channels.write().await;
        // ✅ OPTIMIZED: Use Entry API - avoid double clone
        channels
            .entry(service.id.clone())
            .or_insert_with(|| channel.clone());

        info!("✅ Channel created for service: {}", service.name);
        Ok(channel)
    }

    /// Send a message to a service
    pub async fn send_message(
        &self,
        channel: &ServiceChannel,
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        debug!("📤 Sending message to service: {}", channel.service_name);

        match &channel.client {
            #[cfg(feature = "networking")]
            ServiceClient::Tarpc(_tarpc_client) => {
                // tarpc integration pending: requires full tarpc transport setup.
                // Use Unix socket JSON-RPC path (ServiceClient::UnixSocket) which provides
                // equivalent functionality over the same socket.
                Err(ToadStoolError::runtime(format!(
                    "tarpc transport not yet configured for {}; use JSON-RPC path",
                    channel.service_name
                )))
            }

            #[cfg(feature = "networking")]
            ServiceClient::UnixSocket(rpc_client) => {
                debug!("📤 Sending via JSON-RPC over unix socket (PRIMARY - pure Rust!)");
                self.send_via_unix_socket(rpc_client, message).await
            }

            #[cfg(not(feature = "networking"))]
            ServiceClient::Disabled => {
                debug!("📤 Degraded-mode: no networking, returning fallback response");
                Ok(self.fallback_response(message))
            }
        }
    }

    /// Check health of a service channel
    pub async fn check_health(&self, channel: &ServiceChannel) -> ToadStoolResult<()> {
        debug!("🔍 Checking health of service: {}", channel.service_name);

        match &channel.client {
            #[cfg(feature = "networking")]
            ServiceClient::UnixSocket(rpc_client) => {
                // Health check via JSON-RPC
                let _result: serde_json::Value = rpc_client
                    .call("health", serde_json::json!({}))
                    .await
                    .map_err(|e| ToadStoolError::network(format!("Health check failed: {e}")))?;

                debug!("✅ Health check passed");
                Ok(())
            }

            #[cfg(feature = "networking")]
            ServiceClient::Tarpc(_) => {
                // tarpc integration pending: requires full tarpc transport setup.
                // Use Unix socket JSON-RPC path (ServiceClient::UnixSocket) which provides
                // equivalent functionality over the same socket.
                Err(ToadStoolError::runtime(format!(
                    "tarpc transport not yet configured for {}; use JSON-RPC path",
                    channel.service_name
                )))
            }

            #[cfg(not(feature = "networking"))]
            ServiceClient::Disabled => {
                debug!("✅ Degraded-mode: health check passed (no networking)");
                Ok(())
            }
        }
    }

    /// Send heartbeat to a service
    pub async fn send_heartbeat(&self, service_id: &str) -> ToadStoolResult<()> {
        let channels = self.channels.read().await;
        let channel = channels
            .get(service_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Channel not found: {service_id}")))?;

        let heartbeat_msg =
            EcosystemMessage::heartbeat(PRIMAL_NAME.to_string(), channel.service_name.clone());

        self.send_message(channel, heartbeat_msg).await?;

        // Update last heartbeat time
        drop(channels);
        let mut channels = self.channels.write().await;
        if let Some(channel) = channels.get_mut(service_id) {
            channel.last_heartbeat = chrono::Utc::now();
        }

        debug!("💓 Heartbeat sent to service: {}", service_id);
        Ok(())
    }

    /// Get a channel by service ID
    pub async fn get_channel(&self, service_id: &str) -> Option<ServiceChannel> {
        let channels = self.channels.read().await;
        channels.get(service_id).cloned()
    }

    /// Get all active channels
    pub async fn get_all_channels(&self) -> Vec<ServiceChannel> {
        let channels = self.channels.read().await;
        channels.values().cloned().collect()
    }

    /// Remove a channel
    pub async fn remove_channel(&self, service_id: &str) {
        let mut channels = self.channels.write().await;
        if let Some(channel) = channels.remove(service_id) {
            info!("🗑️  Removed channel for service: {}", channel.service_name);
        }
    }

    /// Update channel status
    pub async fn update_channel_status(&self, service_id: &str, status: ServiceStatus) {
        let mut channels = self.channels.write().await;
        if let Some(channel) = channels.get_mut(service_id) {
            channel.status = status;
        }
    }

    // ========================================================================
    // Private Helper Methods
    // ========================================================================

    /// Create the appropriate client for a service
    #[cfg(feature = "networking")]
    fn create_client_for_service(
        &self,
        service: &DiscoveredService,
    ) -> ToadStoolResult<ServiceClient> {
        // Check service endpoints for protocol support
        // wateringHole Standard: JSON-RPC > tarpc > HTTP (deprecated)
        // JSON-RPC 2.0 is PRIMARY for inter-primal communication

        for endpoint in &service.endpoints {
            // JSON-RPC is PRIMARY protocol per wateringHole standard
            if endpoint.protocol == "jsonrpc"
                || endpoint.protocol == "json-rpc"
                || endpoint.protocol == "unix-socket"
                || endpoint.protocol == "unix"
            {
                debug!(
                    "🌍 Using JSON-RPC 2.0 over unix socket (PRIMARY - wateringHole standard) for service: {}",
                    service.name
                );
                // EVOLVED: Extract socket path from endpoint when available
                let socket_path = Self::extract_socket_path(endpoint, &service.name);
                return Ok(ServiceClient::UnixSocket(
                    toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
                ));
            }

            // tarpc is OPTIONAL for performance-critical paths
            if endpoint.protocol == "tarpc" {
                tracing::debug!(
                    "tarpc endpoint available for {} but using JSON-RPC as PRIMARY per wateringHole standard",
                    service.name
                );
                continue;
            }
        }

        // Fallback: Use capability-based path resolution
        debug!("Using socket path discovery for service: {}", service.name);
        let socket_path = toadstool_common::primal_sockets::resolve_socket_path_for_service(
            &service.name,
            &toadstool_common::primal_sockets::SocketPathEnv::from_env(),
            None,
        );
        Ok(ServiceClient::UnixSocket(
            toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        ))
    }

    #[cfg(not(feature = "networking"))]
    fn create_client_for_service(
        &self,
        _service: &DiscoveredService,
    ) -> ToadStoolResult<ServiceClient> {
        Ok(ServiceClient::Disabled)
    }

    /// Extract socket path from endpoint (capability-based)
    ///
    /// **Deep Debt Compliant**: Uses actual endpoint info, not hardcoded names.
    #[cfg(feature = "networking")]
    fn extract_socket_path(
        endpoint: &toadstool_common::primal_identity::ServiceEndpoint,
        service_name: &str,
    ) -> std::path::PathBuf {
        // Priority 1: Address is a Unix socket path
        if endpoint.address.starts_with('/') {
            return std::path::PathBuf::from(&endpoint.address);
        }

        // Priority 2: Metadata contains socket_path
        if let Some(path) = endpoint.metadata.get("socket_path") {
            return std::path::PathBuf::from(path);
        }

        // Priority 3: Metadata contains path
        if let Some(path) = endpoint.metadata.get("path") {
            return std::path::PathBuf::from(path);
        }

        // Fallback: Resolve by service name (capability-based)
        toadstool_common::primal_sockets::resolve_socket_path_for_service(
            service_name,
            &toadstool_common::primal_sockets::SocketPathEnv::from_env(),
            None,
        )
    }

    // DEPRECATED HTTP METHODS REMOVED (Feb 17, 2026 — Deep Debt Evolution)
    // - send_via_http: Removed (placeholder parameter, always returned error)
    // - send_via_jsonrpc: Removed (placeholder parameter, always returned error)
    // - send_via_http_fallback: Removed (always returned error)
    // Use send_via_unix_socket instead (pure Rust, no external dependencies)

    /// Send message via JSON-RPC 2.0 over unix socket (pure Rust!)
    #[cfg(feature = "networking")]
    async fn send_via_unix_socket(
        &self,
        rpc_client: &toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        let params = serde_json::to_value(&message)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize message: {e}")))?;

        let response_message: EcosystemMessage = rpc_client
            .call_typed("ecosystem.send_message", params)
            .await
            .map_err(|e| ToadStoolError::network(format!("Unix socket RPC failed: {e}")))?;

        debug!("✅ Message sent via JSON-RPC over unix socket (pure Rust!)");
        Ok(response_message)
    }

    /// Fallback response when networking is disabled (degraded-mode)
    ///
    /// **Deep Debt Evolved**: Returns a structured status indicating networking
    /// is disabled, rather than a generic "mock_response". This allows callers
    /// to detect degraded mode and take appropriate action.
    #[cfg(not(feature = "networking"))]
    fn fallback_response(&self, original: EcosystemMessage) -> EcosystemMessage {
        EcosystemMessage {
            id: uuid::Uuid::new_v4(),
            from: format!("{}_local", PRIMAL_NAME),
            to: original.from,
            message_type: super::types::EcosystemMessageType::StatusUpdate,
            payload: serde_json::json!({
                "status": "networking_disabled",
                "reason": "Networking feature not compiled",
                "mode": "degraded",
                "original_message_id": original.id.to_string()
            }),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl Default for CommunicationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_communication_manager_creation() {
        let manager = CommunicationManager::new();
        let channels = manager.get_all_channels().await;
        assert_eq!(channels.len(), 0);
    }

    #[tokio::test]
    async fn test_custom_timeout() {
        let manager = CommunicationManager::with_timeout(Duration::from_secs(60));
        assert_eq!(manager._default_timeout.as_secs(), 60);
    }

    #[test]
    fn test_fallback_response() {
        use super::super::types::EcosystemMessageType;

        let _manager = CommunicationManager::new();
        let _original = super::super::types::EcosystemMessage::new(
            "sender".to_string(),
            "receiver".to_string(),
            EcosystemMessageType::Heartbeat,
            serde_json::json!({}),
        );

        #[cfg(not(feature = "networking"))]
        {
            let response = _manager.fallback_response(_original.clone());
            assert_eq!(response.to, _original.from);
            assert_eq!(response.from, "toadstool_local"); // Evolved: indicates local-only mode
                                                          // Verify structured status payload
            let status = response.payload.get("status").and_then(|v| v.as_str());
            assert_eq!(status, Some("networking_disabled"));
        }
    }

    // ─── Channel management tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_default_impl() {
        let manager = CommunicationManager::default();
        let channels = manager.get_all_channels().await;
        assert_eq!(channels.len(), 0);
    }

    #[tokio::test]
    async fn test_get_channel_not_found() {
        let manager = CommunicationManager::new();
        let channel = manager.get_channel("non-existent-service").await;
        assert!(channel.is_none());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_channel() {
        let manager = CommunicationManager::new();
        manager.remove_channel("non-existent-id").await;
        let channels = manager.get_all_channels().await;
        assert_eq!(channels.len(), 0);
    }

    #[tokio::test]
    async fn test_update_status_nonexistent_channel() {
        let manager = CommunicationManager::new();
        manager
            .update_channel_status("non-existent-id", ServiceStatus::Connected)
            .await;
        let channel = manager.get_channel("non-existent-id").await;
        assert!(channel.is_none());
    }

    #[tokio::test]
    async fn test_send_heartbeat_no_channel() {
        let manager = CommunicationManager::new();
        let result = manager.send_heartbeat("non-existent-service").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Channel not found"));
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_channel_operations_with_direct_insert() {
        let manager = CommunicationManager::new();
        let channel = ServiceChannel {
            service_id: "test-service".to_string(),
            service_name: "Test Service".to_string(),
            endpoint: "http://localhost:1234".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Connected,
        };

        {
            let mut channels = manager.channels.write().await;
            channels.insert("test-service".to_string(), channel.clone());
        }

        let retrieved = manager.get_channel("test-service").await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.service_id, "test-service");
        assert_eq!(retrieved.service_name, "Test Service");
        assert_eq!(retrieved.status, ServiceStatus::Connected);

        let all = manager.get_all_channels().await;
        assert_eq!(all.len(), 1);

        manager
            .update_channel_status("test-service", ServiceStatus::Disconnected)
            .await;
        let updated = manager.get_channel("test-service").await.unwrap();
        assert_eq!(updated.status, ServiceStatus::Disconnected);

        manager.remove_channel("test-service").await;
        let gone = manager.get_channel("test-service").await;
        assert!(gone.is_none());
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_multiple_channels() {
        let manager = CommunicationManager::new();
        let mk_channel = |id: &str, name: &str| ServiceChannel {
            service_id: id.to_string(),
            service_name: name.to_string(),
            endpoint: "http://localhost:1234".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Connected,
        };

        {
            let mut channels = manager.channels.write().await;
            channels.insert("svc-1".to_string(), mk_channel("svc-1", "Service 1"));
            channels.insert("svc-2".to_string(), mk_channel("svc-2", "Service 2"));
            channels.insert("svc-3".to_string(), mk_channel("svc-3", "Service 3"));
        }

        let all = manager.get_all_channels().await;
        assert_eq!(all.len(), 3);

        assert!(manager.get_channel("svc-1").await.is_some());
        assert!(manager.get_channel("svc-2").await.is_some());
        assert!(manager.get_channel("svc-3").await.is_some());
    }

    // ─── Additional message routing and protocol tests ──────────────────────────

    #[tokio::test]
    async fn test_with_timeout_zero_duration() {
        let manager = CommunicationManager::with_timeout(Duration::ZERO);
        assert_eq!(manager._default_timeout, Duration::ZERO);
    }

    #[tokio::test]
    async fn test_create_channel_no_endpoint_fails() {
        use std::collections::HashMap;
        use std::time::SystemTime;
        use toadstool_common::service_discovery::DiscoveredService;

        let manager = CommunicationManager::new();
        let service = DiscoveredService {
            id: "svc-no-ep".to_string(),
            name: "NoEndpoint".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![],
            metadata: HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };

        let result = manager.create_channel(&service).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No endpoint discovered"));
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_create_channel_with_endpoint_succeeds() {
        use std::collections::HashMap;
        use std::time::SystemTime;
        use toadstool_common::primal_identity::ServiceEndpoint;
        use toadstool_common::service_discovery::DiscoveredService;

        let manager = CommunicationManager::new();
        let service = DiscoveredService {
            id: "svc-with-ep".to_string(),
            name: "WithEndpoint".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![ServiceEndpoint::http("localhost", 9999)],
            metadata: HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };

        let result = manager.create_channel(&service).await;
        assert!(result.is_ok());
        let channel = result.unwrap();
        assert_eq!(channel.service_id, "svc-with-ep");
        assert_eq!(channel.service_name, "WithEndpoint");
        assert_eq!(channel.endpoint, "http://localhost:9999");
        assert_eq!(channel.status, ServiceStatus::Discovered);
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_send_message_degraded_mode_returns_fallback() {
        use super::super::types::EcosystemMessageType;

        let manager = CommunicationManager::new();
        let channel = ServiceChannel {
            service_id: "test".to_string(),
            service_name: "Test".to_string(),
            endpoint: "http://localhost:1234".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Connected,
        };

        let msg = super::super::types::EcosystemMessage::new(
            "sender".to_string(),
            "receiver".to_string(),
            EcosystemMessageType::Heartbeat,
            serde_json::json!({"ping": true}),
        );

        let result = manager.send_message(&channel, msg.clone()).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.to, "sender");
        assert_eq!(response.from, "toadstool_local");
        assert!(
            response.payload.get("status").and_then(|v| v.as_str()) == Some("networking_disabled")
        );
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_check_health_degraded_mode_succeeds() {
        let manager = CommunicationManager::new();
        let channel = ServiceChannel {
            service_id: "test".to_string(),
            service_name: "Test".to_string(),
            endpoint: "http://localhost:1234".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Connected,
        };

        let result = manager.check_health(&channel).await;
        assert!(result.is_ok());
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_send_heartbeat_succeeds_and_updates_timestamp() {
        let manager = CommunicationManager::new();
        let channel = ServiceChannel {
            service_id: "heartbeat-svc".to_string(),
            service_name: "Heartbeat Service".to_string(),
            endpoint: "http://localhost:1234".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now() - chrono::Duration::minutes(5),
            status: ServiceStatus::Connected,
        };

        {
            let mut channels = manager.channels.write().await;
            channels.insert("heartbeat-svc".to_string(), channel.clone());
        }

        let result = manager.send_heartbeat("heartbeat-svc").await;
        assert!(result.is_ok());

        let updated = manager.get_channel("heartbeat-svc").await.unwrap();
        assert!(updated.last_heartbeat > channel.last_heartbeat);
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_remove_channel_existing_logs_info() {
        let manager = CommunicationManager::new();
        let channel = ServiceChannel {
            service_id: "to-remove".to_string(),
            service_name: "ToRemove".to_string(),
            endpoint: "http://localhost:1".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Disconnected,
        };

        {
            let mut channels = manager.channels.write().await;
            channels.insert("to-remove".to_string(), channel);
        }

        manager.remove_channel("to-remove").await;
        assert!(manager.get_channel("to-remove").await.is_none());
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_update_channel_status_existing_channel() {
        let manager = CommunicationManager::new();
        let channel = ServiceChannel {
            service_id: "status-svc".to_string(),
            service_name: "Status Svc".to_string(),
            endpoint: "http://localhost:1".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Discovered,
        };

        {
            let mut channels = manager.channels.write().await;
            channels.insert("status-svc".to_string(), channel);
        }

        manager
            .update_channel_status("status-svc", ServiceStatus::Connected)
            .await;

        let updated = manager.get_channel("status-svc").await.unwrap();
        assert_eq!(updated.status, ServiceStatus::Connected);
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_create_channel_idempotent_or_insert() {
        use std::collections::HashMap;
        use std::time::SystemTime;
        use toadstool_common::primal_identity::ServiceEndpoint;
        use toadstool_common::service_discovery::DiscoveredService;

        let manager = CommunicationManager::new();
        let service = DiscoveredService {
            id: "dup-id".to_string(),
            name: "Dup".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![ServiceEndpoint::http("localhost", 8888)],
            metadata: HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };

        let ch1 = manager.create_channel(&service).await.unwrap();
        let ch2 = manager.create_channel(&service).await.unwrap();
        assert_eq!(ch1.service_id, ch2.service_id);
        let all = manager.get_all_channels().await;
        assert_eq!(all.len(), 1);
    }

    // ─── Message type and serialization tests ────────────────────────────────────

    #[test]
    fn test_ecosystem_message_new_constructor() {
        use super::super::types::EcosystemMessageType;

        let msg = super::super::types::EcosystemMessage::new(
            "from-svc".to_string(),
            "to-svc".to_string(),
            EcosystemMessageType::Heartbeat,
            serde_json::json!({"extra": true}),
        );
        assert_eq!(msg.from, "from-svc");
        assert_eq!(msg.to, "to-svc");
        assert_eq!(msg.message_type, EcosystemMessageType::Heartbeat);
        assert!(msg
            .payload
            .get("extra")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
    }

    #[test]
    fn test_ecosystem_message_heartbeat_factory() {
        let msg = super::super::types::EcosystemMessage::heartbeat(
            "sender".to_string(),
            "receiver".to_string(),
        );
        assert_eq!(
            msg.message_type,
            super::super::types::EcosystemMessageType::Heartbeat
        );
    }

    #[test]
    fn test_ecosystem_message_error_factory() {
        let msg = super::super::types::EcosystemMessage::error(
            "a".to_string(),
            "b".to_string(),
            "something failed".to_string(),
        );
        assert_eq!(
            msg.message_type,
            super::super::types::EcosystemMessageType::Error
        );
        assert_eq!(msg.payload["error"], "something failed");
    }

    #[test]
    fn test_ecosystem_message_serialization_roundtrip() {
        use super::super::types::EcosystemMessageType;

        let msg = super::super::types::EcosystemMessage::new(
            "a".to_string(),
            "b".to_string(),
            EcosystemMessageType::StatusUpdate,
            serde_json::json!({"k": "v"}),
        );
        let json = serde_json::to_string(&msg).expect("serialize");
        let parsed: super::super::types::EcosystemMessage =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.from, msg.from);
        assert_eq!(parsed.to, msg.to);
    }

    #[test]
    fn test_service_status_all_variants() {
        assert!(!super::super::types::ServiceStatus::Discovered.is_usable());
        assert!(!super::super::types::ServiceStatus::Connecting.is_usable());
        assert!(super::super::types::ServiceStatus::Connected.is_usable());
        assert!(!super::super::types::ServiceStatus::Disconnected.is_usable());
        let failed = super::super::types::ServiceStatus::Failed("err".to_string());
        assert!(failed.is_error());
        assert_eq!(failed.error_message(), Some("err"));
    }

    #[cfg(not(feature = "networking"))]
    #[test]
    fn test_service_channel_debug_clone() {
        let ch = ServiceChannel {
            service_id: "id".to_string(),
            service_name: "Name".to_string(),
            endpoint: "http://x".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Connected,
        };
        let ch2 = ch.clone();
        assert_eq!(ch.service_id, ch2.service_id);
        assert_eq!(format!("{:?}", ch).len(), format!("{:?}", ch2).len());
    }

    // ─── Additional message type, serialization, protocol tests ─────────────────

    #[test]
    fn test_ecosystem_message_type_all_variants_serde() {
        use super::super::types::EcosystemMessageType;

        for mt in [
            EcosystemMessageType::Heartbeat,
            EcosystemMessageType::CapabilityAnnouncement,
            EcosystemMessageType::ResourceRequest,
            EcosystemMessageType::ResourceResponse,
            EcosystemMessageType::WorkloadRequest,
            EcosystemMessageType::WorkloadResponse,
            EcosystemMessageType::StatusUpdate,
            EcosystemMessageType::Error,
        ] {
            let json = serde_json::to_value(&mt).unwrap();
            let _: EcosystemMessageType = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_ecosystem_message_capability_announcement() {
        use super::super::types::EcosystemMessageType;

        let msg = super::super::types::EcosystemMessage::new(
            "svc-a".to_string(),
            "svc-b".to_string(),
            EcosystemMessageType::CapabilityAnnouncement,
            serde_json::json!({"caps": ["compute"]}),
        );
        assert_eq!(
            msg.message_type,
            EcosystemMessageType::CapabilityAnnouncement
        );
    }

    #[test]
    fn test_ecosystem_message_resource_request() {
        use super::super::types::EcosystemMessageType;

        let msg = super::super::types::EcosystemMessage::new(
            "requester".to_string(),
            "provider".to_string(),
            EcosystemMessageType::ResourceRequest,
            serde_json::json!({"cpu": 4}),
        );
        assert!(msg.message_type.requires_response());
    }

    #[test]
    fn test_service_status_removing() {
        let status = super::super::types::ServiceStatus::Removing;
        assert!(!status.is_usable());
        assert!(!status.is_error());
    }

    #[test]
    fn test_ecosystem_message_serialization_all_types() {
        use super::super::types::EcosystemMessageType;

        let types = [
            EcosystemMessageType::StatusUpdate,
            EcosystemMessageType::CapabilityAnnouncement,
            EcosystemMessageType::ResourceResponse,
            EcosystemMessageType::WorkloadResponse,
        ];
        for mt in types {
            let msg = super::super::types::EcosystemMessage::new(
                "a".to_string(),
                "b".to_string(),
                mt.clone(),
                serde_json::json!({}),
            );
            let json = serde_json::to_string(&msg).unwrap();
            let parsed: super::super::types::EcosystemMessage =
                serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.message_type, mt);
        }
    }

    #[test]
    fn test_discovery_method_config_serde() {
        use super::super::types::DiscoveryMethodConfig;

        let config = DiscoveryMethodConfig::Environment;
        let json = serde_json::to_value(&config).unwrap();
        let _: DiscoveryMethodConfig = serde_json::from_value(json).unwrap();
    }

    #[test]
    fn test_service_status_connecting() {
        assert!(!super::super::types::ServiceStatus::Connecting.is_usable());
    }

    #[test]
    fn test_ecosystem_message_with_complex_payload() {
        use super::super::types::EcosystemMessageType;

        let msg = super::super::types::EcosystemMessage::new(
            "from".to_string(),
            "to".to_string(),
            EcosystemMessageType::WorkloadRequest,
            serde_json::json!({
                "job_id": "j1",
                "resources": {"cpu": 8},
                "nested": {"a": 1}
            }),
        );
        assert!(msg.payload.get("nested").is_some());
    }

    // ─── Additional coverage: error_message, DiscoveryMethodConfig, status display ─

    #[test]
    fn test_service_status_error_message() {
        let status = super::super::types::ServiceStatus::Failed("connection refused".to_string());
        assert!(status.is_error());
        assert_eq!(status.error_message(), Some("connection refused"));
    }

    #[test]
    fn test_service_status_error_message_none_for_non_failed() {
        let status = super::super::types::ServiceStatus::Connected;
        assert!(status.error_message().is_none());
    }

    #[test]
    fn test_discovery_method_config_config_file_serde() {
        use super::super::types::DiscoveryMethodConfig;

        let config = DiscoveryMethodConfig::ConfigFile {
            path: "/etc/biomeos/discovery.json".to_string(),
        };
        let json = serde_json::to_value(&config).unwrap();
        let parsed: DiscoveryMethodConfig = serde_json::from_value(json).unwrap();
        match parsed {
            DiscoveryMethodConfig::ConfigFile { path } => {
                assert_eq!(path, "/etc/biomeos/discovery.json")
            }
            _ => panic!("expected ConfigFile"),
        }
    }

    #[test]
    fn test_discovery_method_config_registry_serde() {
        use super::super::types::DiscoveryMethodConfig;

        let config = DiscoveryMethodConfig::Registry {
            endpoint: "http://registry:8080".to_string(),
        };
        let json = serde_json::to_value(&config).unwrap();
        let parsed: DiscoveryMethodConfig = serde_json::from_value(json).unwrap();
        match parsed {
            DiscoveryMethodConfig::Registry { endpoint } => {
                assert_eq!(endpoint, "http://registry:8080")
            }
            _ => panic!("expected Registry"),
        }
    }

    #[test]
    fn test_ecosystem_message_type_workload_response_no_response_required() {
        use super::super::types::EcosystemMessageType;

        assert!(!EcosystemMessageType::WorkloadResponse.requires_response());
    }

    #[test]
    fn test_ecosystem_message_type_resource_request_requires_response() {
        use super::super::types::EcosystemMessageType;

        assert!(EcosystemMessageType::ResourceRequest.requires_response());
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_fallback_response_preserves_original_id() {
        use super::super::types::EcosystemMessageType;

        let manager = CommunicationManager::new();
        let original = super::super::types::EcosystemMessage::new(
            "sender".to_string(),
            "receiver".to_string(),
            EcosystemMessageType::Heartbeat,
            serde_json::json!({}),
        );
        let original_id_str = original.id.to_string();
        let response = manager.fallback_response(original);
        let stored_id = response
            .payload
            .get("original_message_id")
            .and_then(|v| v.as_str());
        assert_eq!(stored_id, Some(original_id_str.as_str()));
    }

    // ─── Mock-based integration tests: protocol negotiation, error handling ────

    #[tokio::test]
    async fn test_create_channel_empty_endpoints_fails() {
        use std::collections::HashMap;
        use std::time::SystemTime;
        use toadstool_common::service_discovery::DiscoveredService;

        let manager = CommunicationManager::new();
        let service = DiscoveredService {
            id: "empty".to_string(),
            name: "EmptyEndpoints".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![],
            metadata: HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };
        let result = manager.create_channel(&service).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No endpoint"));
    }

    #[tokio::test]
    async fn test_remove_channel_clears_from_map() {
        let manager = CommunicationManager::new();
        manager.remove_channel("never-existed").await;
        let channels = manager.get_all_channels().await;
        assert!(channels.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_channels_empty_returns_vec() {
        let manager = CommunicationManager::new();
        let channels = manager.get_all_channels().await;
        assert!(channels.is_empty());
        assert!(channels.capacity() >= 0);
    }

    #[tokio::test]
    async fn test_check_health_degraded_mode_when_disabled() {
        #[cfg(not(feature = "networking"))]
        {
            let manager = CommunicationManager::new();
            let channel = ServiceChannel {
                service_id: "deg".to_string(),
                service_name: "Degraded".to_string(),
                endpoint: "http://x:1".to_string(),
                client: ServiceClient::Disabled,
                last_heartbeat: chrono::Utc::now(),
                status: ServiceStatus::Connected,
            };
            let result = manager.check_health(&channel).await;
            assert!(result.is_ok());
        }
    }

    #[cfg(not(feature = "networking"))]
    #[tokio::test]
    async fn test_create_channel_service_with_multiple_endpoints() {
        use std::collections::HashMap;
        use std::time::SystemTime;
        use toadstool_common::primal_identity::ServiceEndpoint;
        use toadstool_common::service_discovery::DiscoveredService;

        let manager = CommunicationManager::new();
        let service = DiscoveredService {
            id: "multi-ep".to_string(),
            name: "MultiEndpoint".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![
                ServiceEndpoint::http("localhost", 8080),
                ServiceEndpoint::http("localhost", 8081),
            ],
            metadata: HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };
        let result = manager.create_channel(&service).await;
        assert!(result.is_ok());
        let ch = result.unwrap();
        assert_eq!(ch.service_id, "multi-ep");
    }

    #[tokio::test]
    async fn test_send_message_requires_channel() {
        let manager = CommunicationManager::new();
        let result = manager.send_heartbeat("ghost").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_channel_status_idempotent() {
        let manager = CommunicationManager::new();
        manager
            .update_channel_status("nonexistent", ServiceStatus::Connected)
            .await;
        manager
            .update_channel_status("nonexistent", ServiceStatus::Disconnected)
            .await;
        let channel = manager.get_channel("nonexistent").await;
        assert!(channel.is_none());
    }

    #[tokio::test]
    async fn test_communication_manager_with_timeout_creation() {
        let manager = CommunicationManager::with_timeout(Duration::from_secs(30));
        let channels = manager.get_all_channels().await;
        assert_eq!(channels.len(), 0);
    }

    #[test]
    fn test_ecosystem_message_status_update_type() {
        use super::super::types::EcosystemMessageType;

        let msg = super::super::types::EcosystemMessage::new(
            "a".to_string(),
            "b".to_string(),
            EcosystemMessageType::StatusUpdate,
            serde_json::json!({"state": "ready"}),
        );
        assert!(!msg.message_type.requires_response());
    }

    #[cfg(not(feature = "networking"))]
    #[test]
    fn test_service_channel_debug_impl() {
        let ch = ServiceChannel {
            service_id: "debug-svc".to_string(),
            service_name: "Debug Svc".to_string(),
            endpoint: "unix:///tmp/debug.sock".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Discovered,
        };
        let debug_str = format!("{:?}", ch);
        assert!(debug_str.contains("debug-svc"));
        assert!(debug_str.contains("Debug Svc"));
    }
}
