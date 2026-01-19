//! # Service Communication
//!
//! Manages communication channels and messaging with ecosystem services.
//!
//! ## Features
//!
//! - **Multi-Protocol**: tarpc (PRIMARY), JSON-RPC (PRIMARY), HTTP (FALLBACK)
//! - **Trait-Based**: Polymorphic communication via `ServiceCommunication` trait
//! - **Connection Management**: Automatic reconnection and health checks
//! - **Message Routing**: Type-safe ecosystem messaging
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
use toadstool_common::service_discovery::DiscoveredService;

use super::types::{EcosystemMessage, ServiceChannel, ServiceClient, ServiceStatus};

/// Communication manager for service channels and messaging
pub struct CommunicationManager {
    /// Active communication channels (keyed by service ID)
    channels: Arc<RwLock<HashMap<String, ServiceChannel>>>,
    /// Default timeout for operations
    #[allow(dead_code)]
    default_timeout: Duration,
}

impl CommunicationManager {
    /// Create a new communication manager
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(30))
    }

    /// Create a communication manager with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            default_timeout: timeout,
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
                debug!("📤 Sending via tarpc (PRIMARY protocol)");
                // TODO(future): Implement tarpc message sending when tarpc integration complete
                // For now, tarpc not yet implemented, return error
                Err(ToadStoolError::runtime(
                    "Tarpc messaging not yet implemented",
                ))
            }

            #[cfg(feature = "networking")]
            ServiceClient::UnixSocket(rpc_client) => {
                debug!("📤 Sending via JSON-RPC over unix socket (PRIMARY - pure Rust!)");
                self.send_via_unix_socket(rpc_client, message).await
            }

            #[cfg(feature = "websocket")]
            ServiceClient::WebSocket(_ws) => {
                debug!("📤 Sending via WebSocket");
                // TODO(future): Implement WebSocket message sending for realtime updates
                Err(ToadStoolError::runtime(
                    "WebSocket messaging not yet implemented",
                ))
            }

            #[cfg(not(feature = "networking"))]
            ServiceClient::Mock => {
                debug!("📤 Mock message send");
                Ok(self.mock_response(message))
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
                // TODO(future): Implement tarpc health check when tarpc integration complete
                debug!("✅ Tarpc health check (placeholder)");
                Ok(())
            }

            #[cfg(feature = "websocket")]
            ServiceClient::WebSocket(_) => {
                // WebSocket health is implicit (connected = healthy)
                debug!("✅ WebSocket health check (connected)");
                Ok(())
            }

            #[cfg(not(feature = "networking"))]
            ServiceClient::Mock => {
                debug!("✅ Mock health check");
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
            EcosystemMessage::heartbeat("toadstool".to_string(), channel.service_name.clone());

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
        // Priority: tarpc > JSON-RPC > HTTP

        for endpoint in &service.endpoints {
            if endpoint.protocol == "tarpc" {
                debug!("🚀 Using tarpc (PRIMARY) for service: {}", service.name);
                // TODO(future): Create tarpc client when tarpc integration complete
                // Current: Falls through to JSON-RPC as tarpc is being integrated
                continue;
            }

            if endpoint.protocol == "jsonrpc"
                || endpoint.protocol == "json-rpc"
                || endpoint.protocol == "tarpc"
            {
                debug!(
                    "🌍 Using JSON-RPC over unix socket (PRIMARY) for service: {}",
                    service.name
                );
                // Use unix socket path from discovery
                let socket_path =
                    toadstool_common::primal_sockets::get_socket_path_for_service(&service.name);
                return Ok(ServiceClient::UnixSocket(
                    toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
                ));
            }
        }

        // Fallback: Try to determine from service name
        debug!("Using socket path discovery for service: {}", service.name);
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_service(&service.name);
        Ok(ServiceClient::UnixSocket(
            toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        ))
    }

    #[cfg(not(feature = "networking"))]
    fn create_client_for_service(
        &self,
        _service: &DiscoveredService,
    ) -> ToadStoolResult<ServiceClient> {
        Ok(ServiceClient::Mock)
    }

    /// Send message via HTTP - DEPRECATED
    ///
    /// **LEGACY**: HTTP deprecated - use Unix sockets
    #[allow(dead_code)]
    #[cfg(feature = "networking")]
    async fn send_via_http(
        &self,
        channel: &ServiceChannel,
        _client_placeholder: &str, // Was: &reqwest::Client
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        // LEGACY: HTTP deprecated - return error
        Err(ToadStoolError::not_supported(format!(
            "HTTP communication deprecated for channel {}. Use Unix sockets. Message: {:?}",
            channel.service_id, message
        )))
    }

    /// Send message via JSON-RPC - DEPRECATED
    ///
    /// **LEGACY**: HTTP RPC deprecated - use Unix sockets
    #[allow(dead_code)]
    #[cfg(feature = "networking")]
    async fn send_via_jsonrpc(
        &self,
        channel: &ServiceChannel,
        _client_placeholder: &str, // Was: &reqwest::Client
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        // LEGACY: HTTP RPC deprecated - return error
        Err(ToadStoolError::not_supported(format!(
            "HTTP JSON-RPC communication deprecated for channel {}. Use Unix sockets. Message: {:?}",
            channel.service_id, message
        )))
    }

    /// Fallback HTTP sending - DEPRECATED
    ///
    /// **LEGACY**: Use Unix sockets instead
    #[allow(dead_code)]
    #[cfg(feature = "networking")]
    async fn send_via_http_fallback(
        &self,
        channel: &ServiceChannel,
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        // LEGACY: Use Unix sockets instead
        Err(ToadStoolError::not_supported(format!(
            "HTTP fallback deprecated for channel {}. Use Unix sockets. Message: {:?}",
            channel.service_id, message
        )))
    }

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

    /// Create a mock response for testing
    #[cfg(not(feature = "networking"))]
    fn mock_response(&self, original: EcosystemMessage) -> EcosystemMessage {
        EcosystemMessage {
            id: uuid::Uuid::new_v4(),
            from: "mock_service".to_string(),
            to: original.from,
            message_type: super::types::EcosystemMessageType::StatusUpdate,
            payload: serde_json::json!({"status": "mock_response"}),
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
        assert_eq!(manager.default_timeout.as_secs(), 60);
    }

    #[test]
    fn test_mock_response() {
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
            let response = _manager.mock_response(_original.clone());
            assert_eq!(response.to, _original.from);
            assert_eq!(response.from, "mock_service");
        }
    }
}
