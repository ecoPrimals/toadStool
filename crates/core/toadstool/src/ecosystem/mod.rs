// SPDX-License-Identifier: AGPL-3.0-only
//! # Ecosystem Coordination - Capability-Based Discovery
//!
//! This module handles integration with services in the ecosystem using
//! **capability-based discovery** rather than hardcoded primal names.
//!
//! ## Philosophy
//!
//! - **Self-Knowledge Only**: ToadStool knows only itself
//! - **Runtime Discovery**: Services discovered by capabilities, not names
//! - **Zero Hardcoding**: No primal-specific code or configuration
//! - **Capability-Based**: Services matched by what they can do, not who they are
//!
//! ## Architecture
//!
//! The ecosystem module is organized into domain-driven submodules:
//!
//! - **`types`**: Core type definitions and builders
//! - **`discovery`**: Capability-based service discovery
//! - **`communication`**: Multi-protocol messaging and channels
//! - **`management`**: Service lifecycle and status tracking
//! - **`legacy`**: Deprecated code (backward compatibility only)
//!
//! ## Example
//!
//! ```rust,ignore
//! // Create ecosystem coordinator
//! let config = EcosystemConfig::builder()
//!     .auto_discovery(true)
//!     .require_capability(Capability::Coordination(...))
//!     .build();
//!
//! let coordinator = EcosystemCoordinator::with_config(config).await?;
//!
//! // Find service by capability (modern approach)
//! let coordinator_service = coordinator
//!     .find_service_by_capability(
//!         Capability::Coordination(CoordinationCapability::ServiceDiscovery)
//!     )
//!     .await?;
//!
//! // Send message via appropriate protocol (tarpc/JSON-RPC/HTTP)
//! let response = coordinator.send_ecosystem_message(message).await?;
//! ```

mod communication;
mod discovery;
mod management;
mod types;

// Re-export public API
pub use communication::CommunicationManager;
pub use discovery::DiscoveryManager;
pub use management::ServiceManager;
pub use types::{
    DiscoveryMethodConfig, EcosystemConfig, EcosystemConfigBuilder, EcosystemMessage,
    EcosystemMessageType, ServiceChannel, ServiceClient, ServiceInstance, ServiceStatus,
};

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{error, info};

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::primal_identity::Capability;
use toadstool_common::service_discovery::{DiscoveredService, DiscoveryMethod, ServiceDiscovery};

/// Ecosystem coordinator for capability-based service integration
///
/// This is the main entry point for ecosystem integration. It orchestrates
/// discovery, communication, and lifecycle management of ecosystem services.
pub struct EcosystemCoordinator {
    /// Discovery manager
    discovery: Arc<DiscoveryManager>,
    /// Communication manager
    communication: Arc<CommunicationManager>,
    /// Service manager
    management: Arc<ServiceManager>,
    /// Service discovery client (stored for potential direct use)
    #[allow(dead_code, reason = "reserved for direct discovery fallback/reconnect")]
    discovery_client: Arc<ServiceDiscovery>,
    /// Configuration
    config: EcosystemConfig,
}

impl EcosystemCoordinator {
    /// Create a new ecosystem coordinator with capability-based discovery
    pub async fn new() -> ToadStoolResult<Self> {
        Self::with_config(EcosystemConfig::default()).await
    }

    /// Create a new ecosystem coordinator with custom config
    pub async fn with_config(config: EcosystemConfig) -> ToadStoolResult<Self> {
        info!("🌐 Creating Ecosystem Coordinator (capability-based discovery)");

        // Initialize service discovery client
        let discovery_method = match &config.discovery_method {
            DiscoveryMethodConfig::Auto => DiscoveryMethod::Auto,
            DiscoveryMethodConfig::Environment => DiscoveryMethod::Environment,
            DiscoveryMethodConfig::Mdns => DiscoveryMethod::Mdns,
            DiscoveryMethodConfig::ConfigFile { path } => {
                DiscoveryMethod::ConfigFile { path: path.clone() }
            }
            DiscoveryMethodConfig::Registry { endpoint } => DiscoveryMethod::Registry {
                endpoint: endpoint.clone(),
            },
        };

        let discovery_client = ServiceDiscovery::new(discovery_method)
            .await
            .map_err(|e| ToadStoolError::other(format!("Failed to initialize discovery: {e}")))?;

        let discovery_client = Arc::new(discovery_client);

        // Create managers
        let discovery = Arc::new(DiscoveryManager::new(discovery_client.clone()));
        let communication = Arc::new(CommunicationManager::new());
        let management = Arc::new(ServiceManager::new());

        Ok(Self {
            discovery,
            communication,
            management,
            discovery_client,
            config,
        })
    }

    // ========================================================================
    // Discovery Methods (Modern API)
    // ========================================================================

    /// Discover services by capability
    ///
    /// This is the **modern way** to find services - by what they can do, not who they are.
    pub async fn find_service_by_capability(
        &self,
        capability: Capability,
    ) -> ToadStoolResult<DiscoveredService> {
        let service = self.discovery.find_by_capability(capability).await?;

        // Register with management
        self.management.register_service(service.clone()).await?;

        Ok(service)
    }

    /// Discover all services for configured capabilities
    pub async fn discover_services(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        info!("🔍 Discovering ecosystem services");

        let services = self.discovery.discover_all_required(&self.config).await?;

        // Register all discovered services (must clone: we return services and register needs ownership)
        for service in &services {
            if let Err(e) = self.management.register_service(service.clone()).await {
                error!("Failed to register service {}: {}", service.id, e);
            }
        }

        info!("✅ Discovered {} services", services.len());
        Ok(services)
    }

    /// Get all discovered services
    pub async fn get_discovered_services(&self) -> Vec<DiscoveredService> {
        self.management.get_all_services().await
    }

    /// Check if a capability is available in the ecosystem
    pub async fn is_capability_available(&self, capability: &Capability) -> bool {
        self.management.is_capability_available(capability).await
    }

    // ========================================================================
    // Communication Methods
    // ========================================================================

    /// Integrate with discovered services
    ///
    /// This creates communication channels for all services.
    pub async fn integrate_services(
        &self,
        services: Vec<DiscoveredService>,
    ) -> ToadStoolResult<()> {
        info!("🔗 Integrating with {} services", services.len());

        for service in services {
            // Register service first (clone needed: we use original for create_channel)
            if let Err(e) = self.management.register_service(service.clone()).await {
                error!("Failed to register service {}: {}", service.id, e);
                continue;
            }

            // Create communication channel (uses original service by ref)
            match self.communication.create_channel(&service).await {
                Ok(channel) => {
                    info!("✅ Created channel for service: {}", service.id);

                    // Test connection
                    match self.communication.check_health(&channel).await {
                        Ok(()) => {
                            self.management.mark_connected(&service.id).await;
                            info!("✅ Service connected: {}", service.id);
                        }
                        Err(e) => {
                            self.management
                                .mark_failed(&service.id, format!("Health check failed: {e}"))
                                .await;
                            error!("❌ Health check failed for {}: {}", service.id, e);
                        }
                    }
                }
                Err(e) => {
                    self.management
                        .mark_failed(&service.id, format!("Channel creation failed: {e}"))
                        .await;
                    error!("❌ Failed to create channel for {}: {}", service.id, e);
                }
            }
        }

        info!("✅ Service integration complete");
        Ok(())
    }

    /// Send a message to a service via ecosystem messaging
    pub async fn send_ecosystem_message(
        &self,
        service_id: &str,
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        // Get channel
        let channel = self
            .communication
            .get_channel(service_id)
            .await
            .ok_or_else(|| {
                ToadStoolError::not_found(format!("No channel for service: {service_id}"))
            })?;

        // Send message
        self.communication.send_message(&channel, message).await
    }

    /// Send heartbeat to a service
    pub async fn send_heartbeat(&self, service_id: &str) -> ToadStoolResult<()> {
        self.communication.send_heartbeat(service_id).await
    }

    // ========================================================================
    // Status and Management Methods
    // ========================================================================

    /// Get service capabilities by service ID
    pub async fn get_service_capabilities(
        &self,
        service_id: &str,
    ) -> ToadStoolResult<Vec<Capability>> {
        self.management.get_service_capabilities(service_id).await
    }

    /// Get service status by ID
    pub async fn get_service_status(&self, service_id: &str) -> Option<ServiceStatus> {
        self.management.get_service_status(service_id).await
    }

    /// Get all service statuses
    pub async fn get_service_statuses(&self) -> HashMap<String, ServiceStatus> {
        self.management.get_all_statuses().await
    }

    /// Get count of healthy services
    pub async fn healthy_count(&self) -> usize {
        self.management.healthy_count().await
    }

    /// Get total service count
    pub async fn service_count(&self) -> usize {
        self.management.service_count().await
    }

    // ========================================================================
    // Legacy Compatibility (Deprecated)
    // ========================================================================

    /// Get primal status (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// Use `get_service_statuses()` instead.
    #[deprecated(since = "0.4.0", note = "Use get_service_statuses()")]
    pub async fn get_primal_status(&self) -> ToadStoolResult<HashMap<String, ServiceStatus>> {
        Ok(self.get_service_statuses().await)
    }

    /// Check if a primal is available (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// Use `is_capability_available()` instead.
    #[deprecated(since = "0.4.0", note = "Use is_capability_available()")]
    pub async fn is_primal_available(&self, primal_name: &str) -> bool {
        let services = self.management.get_all_services().await;
        services.iter().any(|s| s.name == primal_name && s.healthy)
    }

    /// Get primal capabilities (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// Use `get_service_capabilities()` instead.
    #[deprecated(since = "0.4.0", note = "Use get_service_capabilities()")]
    pub async fn get_primal_capabilities(&self, primal_name: &str) -> ToadStoolResult<Vec<String>> {
        let services = self.management.get_all_services().await;
        let service = services
            .iter()
            .find(|s| s.name == primal_name)
            .ok_or_else(|| {
                ToadStoolError::not_found(format!("Service not found: {primal_name}"))
            })?;

        Ok(service
            .capabilities
            .iter()
            .map(|c| format!("{c:?}"))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let coordinator = EcosystemCoordinator::new().await;
        assert!(coordinator.is_ok());
    }

    #[tokio::test]
    async fn test_coordinator_with_config() {
        let config = EcosystemConfig::builder()
            .auto_discovery(true)
            .discovery_timeout(std::time::Duration::from_secs(60))
            .build();

        let coordinator = EcosystemCoordinator::with_config(config).await;
        assert!(coordinator.is_ok());
    }

    #[tokio::test]
    async fn test_service_count() {
        let coordinator = EcosystemCoordinator::new().await.unwrap();
        let count = coordinator.service_count().await;
        assert_eq!(count, 0); // No services initially
    }

    #[tokio::test]
    async fn test_healthy_count_initially_zero() {
        let coordinator = EcosystemCoordinator::new().await.unwrap();
        let healthy = coordinator.healthy_count().await;
        assert_eq!(healthy, 0);
    }

    #[tokio::test]
    async fn test_get_discovered_services_initially_empty() {
        let coordinator = EcosystemCoordinator::new().await.unwrap();
        let services = coordinator.get_discovered_services().await;
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_get_service_statuses_initially_empty() {
        let coordinator = EcosystemCoordinator::new().await.unwrap();
        let statuses = coordinator.get_service_statuses().await;
        assert!(statuses.is_empty());
    }

    #[tokio::test]
    async fn test_send_heartbeat_no_channel() {
        let coordinator = EcosystemCoordinator::new().await.unwrap();
        let result = coordinator.send_heartbeat("unknown-service-id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_ecosystem_message_no_channel() {
        let coordinator = EcosystemCoordinator::new().await.unwrap();
        let msg =
            EcosystemMessage::heartbeat("sender".to_string(), "unknown-service-id".to_string());
        let result = coordinator
            .send_ecosystem_message("unknown-service-id", msg)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_service_status_unknown() {
        let coordinator = EcosystemCoordinator::new().await.unwrap();
        let status = coordinator.get_service_status("unknown-service-id").await;
        assert!(status.is_none());
    }

    #[tokio::test]
    async fn test_coordinator_with_environment_method() {
        let config = EcosystemConfig::builder()
            .discovery_method(DiscoveryMethodConfig::Environment)
            .build();
        let coordinator = EcosystemCoordinator::with_config(config).await;
        assert!(coordinator.is_ok());
    }

    #[allow(deprecated)]
    #[tokio::test]
    async fn test_deprecated_primal_available() {
        let coordinator = EcosystemCoordinator::new().await.unwrap();
        let available = coordinator.is_primal_available("unknown-primal").await;
        assert!(!available);
    }
}
