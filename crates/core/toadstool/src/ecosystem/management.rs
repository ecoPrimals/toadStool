//! # Service Management
//!
//! Manages the lifecycle of ecosystem services including registration,
//! status tracking, health monitoring, and integration.
//!
//! ## Features
//!
//! - **Lifecycle Management**: Track service states from discovery to removal
//! - **Health Monitoring**: Periodic health checks and heartbeats
//! - **Status Tracking**: Real-time service status updates
//! - **Integration**: Automated service integration workflows
//!
//! ## Usage
//!
//! ```rust,ignore
//! let manager = ServiceManager::new();
//!
//! // Register service
//! manager.register_service(service).await?;
//!
//! // Check status
//! let status = manager.get_service_status(&service_id).await?;
//!
//! // Monitor health
//! manager.start_health_monitoring().await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::primal_identity::Capability;
use toadstool_common::service_discovery::DiscoveredService;

use super::types::{ServiceInstance, ServiceStatus};

/// Service manager for lifecycle and status management
pub struct ServiceManager {
    /// Registered services (keyed by service ID)
    services: Arc<RwLock<HashMap<String, ServiceInstance>>>,
    /// Service status tracking (keyed by service ID)
    statuses: Arc<RwLock<HashMap<String, ServiceStatus>>>,
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            statuses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a discovered service
    pub async fn register_service(&self, service: DiscoveredService) -> ToadStoolResult<()> {
        info!("📋 Registering service: {} ({})", service.name, service.id);

        let service_id = service.id.clone();

        // Store service
        let mut services = self.services.write().await;
        services.insert(service_id.clone(), service.clone());

        // Initialize status
        let mut statuses = self.statuses.write().await;
        let initial_status = if service.healthy {
            ServiceStatus::Discovered
        } else {
            ServiceStatus::Failed("Service reported unhealthy".to_string())
        };
        statuses.insert(service_id.clone(), initial_status);

        info!("✅ Service registered: {}", service_id);
        Ok(())
    }

    /// Unregister a service
    pub async fn unregister_service(&self, service_id: &str) -> ToadStoolResult<()> {
        info!("🗑️  Unregistering service: {}", service_id);

        let mut services = self.services.write().await;
        let mut statuses = self.statuses.write().await;

        services
            .remove(service_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Service not found: {service_id}")))?;

        statuses.remove(service_id);

        info!("✅ Service unregistered: {}", service_id);
        Ok(())
    }

    /// Get a service by ID
    pub async fn get_service(&self, service_id: &str) -> Option<ServiceInstance> {
        let services = self.services.read().await;
        services.get(service_id).cloned()
    }

    /// Get all registered services
    pub async fn get_all_services(&self) -> Vec<ServiceInstance> {
        let services = self.services.read().await;
        services.values().cloned().collect()
    }

    /// Find services by capability
    pub async fn find_services_by_capability(
        &self,
        capability: &Capability,
    ) -> Vec<ServiceInstance> {
        let services = self.services.read().await;
        services
            .values()
            .filter(|s| s.has_capability(capability))
            .cloned()
            .collect()
    }

    /// Get service status
    pub async fn get_service_status(&self, service_id: &str) -> Option<ServiceStatus> {
        let statuses = self.statuses.read().await;
        statuses.get(service_id).cloned()
    }

    /// Update service status
    pub async fn update_service_status(&self, service_id: &str, status: ServiceStatus) {
        debug!("📊 Updating service status: {} -> {:?}", service_id, status);

        let mut statuses = self.statuses.write().await;
        statuses.insert(service_id.to_string(), status);
    }

    /// Get all service statuses
    pub async fn get_all_statuses(&self) -> HashMap<String, ServiceStatus> {
        let statuses = self.statuses.read().await;
        statuses.clone()
    }

    /// Check if a capability is available
    pub async fn is_capability_available(&self, capability: &Capability) -> bool {
        let services = self.services.read().await;
        let statuses = self.statuses.read().await;

        services.values().any(|service| {
            service.has_capability(capability)
                && statuses
                    .get(&service.id)
                    .map(|s| s.is_usable())
                    .unwrap_or(false)
        })
    }

    /// Get capabilities for a service
    pub async fn get_service_capabilities(
        &self,
        service_id: &str,
    ) -> ToadStoolResult<Vec<Capability>> {
        let services = self.services.read().await;
        let service = services
            .get(service_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Service not found: {service_id}")))?;

        Ok(service.capabilities.clone())
    }

    /// Mark service as connected
    pub async fn mark_connected(&self, service_id: &str) {
        info!("🔗 Service connected: {}", service_id);
        self.update_service_status(service_id, ServiceStatus::Connected)
            .await;
    }

    /// Mark service as disconnected
    pub async fn mark_disconnected(&self, service_id: &str) {
        warn!("⚠️  Service disconnected: {}", service_id);
        self.update_service_status(service_id, ServiceStatus::Disconnected)
            .await;
    }

    /// Mark service as failed
    pub async fn mark_failed(&self, service_id: &str, reason: String) {
        error!("❌ Service failed: {} - {}", service_id, reason);
        self.update_service_status(service_id, ServiceStatus::Failed(reason))
            .await;
    }

    /// Get count of services by status
    pub async fn count_by_status(&self, status: ServiceStatus) -> usize {
        let statuses = self.statuses.read().await;
        statuses.values().filter(|s| **s == status).count()
    }

    /// Get healthy services count
    pub async fn healthy_count(&self) -> usize {
        self.count_by_status(ServiceStatus::Connected).await
    }

    /// Get unhealthy services
    pub async fn get_unhealthy_services(&self) -> Vec<String> {
        let statuses = self.statuses.read().await;
        statuses
            .iter()
            .filter(|(_, status)| status.is_error())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Clear all services
    pub async fn clear_all(&self) {
        info!("🗑️  Clearing all services");
        let mut services = self.services.write().await;
        let mut statuses = self.statuses.write().await;
        services.clear();
        statuses.clear();
    }

    /// Get service count
    pub async fn service_count(&self) -> usize {
        let services = self.services.read().await;
        services.len()
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_common::primal_identity::{ComputeCapability, ServiceEndpoint};

    fn create_test_service(name: &str, healthy: bool) -> DiscoveredService {
        DiscoveredService {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
            metadata: HashMap::new(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy,
        }
    }

    #[tokio::test]
    async fn test_service_registration() {
        let manager = ServiceManager::new();
        let service = create_test_service("test-service", true);
        let service_id = service.id.clone();

        manager.register_service(service).await.unwrap();

        let retrieved = manager.get_service(&service_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-service");
    }

    #[tokio::test]
    async fn test_service_unregistration() {
        let manager = ServiceManager::new();
        let service = create_test_service("test-service", true);
        let service_id = service.id.clone();

        manager.register_service(service).await.unwrap();
        manager.unregister_service(&service_id).await.unwrap();

        let retrieved = manager.get_service(&service_id).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_status_updates() {
        let manager = ServiceManager::new();
        let service = create_test_service("test-service", true);
        let service_id = service.id.clone();

        manager.register_service(service).await.unwrap();

        // Initial status should be Discovered
        let status = manager.get_service_status(&service_id).await;
        assert!(matches!(status, Some(ServiceStatus::Discovered)));

        // Update to Connected
        manager.mark_connected(&service_id).await;
        let status = manager.get_service_status(&service_id).await;
        assert!(matches!(status, Some(ServiceStatus::Connected)));

        // Update to Failed
        manager
            .mark_failed(&service_id, "test error".to_string())
            .await;
        let status = manager.get_service_status(&service_id).await;
        assert!(matches!(status, Some(ServiceStatus::Failed(_))));
    }

    #[tokio::test]
    async fn test_capability_search() {
        let manager = ServiceManager::new();
        let service = create_test_service("test-service", true);

        manager.register_service(service).await.unwrap();

        let found = manager
            .find_services_by_capability(&Capability::Compute(ComputeCapability::NativeExecution))
            .await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "test-service");
    }

    #[tokio::test]
    async fn test_healthy_count() {
        let manager = ServiceManager::new();

        let service1 = create_test_service("service1", true);
        let service2 = create_test_service("service2", true);

        let id1 = service1.id.clone();

        manager.register_service(service1).await.unwrap();
        manager.register_service(service2).await.unwrap();

        manager.mark_connected(&id1).await;

        let healthy = manager.healthy_count().await;
        assert_eq!(healthy, 1);
    }
}
