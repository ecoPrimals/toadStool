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
    pub async fn mark_failed(&self, service_id: &str, reason: impl Into<String>) {
        let reason = reason.into();
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
    use std::collections::HashMap;

    use super::*;
    use toadstool_common::primal_identity::{
        ComputeCapability, CoordinationCapability, ServiceEndpoint, StorageCapability,
    };

    fn create_test_service(name: &str, healthy: bool) -> DiscoveredService {
        create_test_service_with_id(uuid::Uuid::new_v4().to_string().as_str(), name, healthy)
    }

    fn create_test_service_with_id(id: &str, name: &str, healthy: bool) -> DiscoveredService {
        DiscoveredService {
            id: id.to_string(),
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

    fn create_test_service_with_capabilities(
        id: &str,
        name: &str,
        healthy: bool,
        capabilities: Vec<Capability>,
    ) -> DiscoveredService {
        DiscoveredService {
            id: id.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            capabilities,
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
    async fn test_service_registration_unhealthy_initial_status() {
        let manager = ServiceManager::new();
        let service = create_test_service("unhealthy-service", false);
        let service_id = service.id.clone();

        manager.register_service(service).await.unwrap();

        let status = manager.get_service_status(&service_id).await;
        assert!(matches!(status, Some(ServiceStatus::Failed(_))));
        if let Some(ServiceStatus::Failed(reason)) = status {
            assert!(reason.contains("unhealthy"));
        }
    }

    #[tokio::test]
    async fn test_duplicate_registration_overwrites() {
        let manager = ServiceManager::new();
        let service = create_test_service_with_id("fixed-id", "original", true);

        manager.register_service(service).await.unwrap();

        let updated = create_test_service_with_id("fixed-id", "updated", true);
        manager.register_service(updated).await.unwrap();

        let retrieved = manager.get_service("fixed-id").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "updated");
        assert_eq!(manager.service_count().await, 1);
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
    async fn test_unregister_unknown_service_returns_err() {
        let manager = ServiceManager::new();

        let result = manager.unregister_service("nonexistent-service-id").await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Service not found"));
        assert!(err_msg.contains("nonexistent-service-id"));
    }

    #[tokio::test]
    async fn test_get_service_unknown_returns_none() {
        let manager = ServiceManager::new();

        let retrieved = manager.get_service("unknown-id").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_all_services() {
        let manager = ServiceManager::new();
        let service1 = create_test_service("service-a", true);
        let service2 = create_test_service("service-b", true);

        manager.register_service(service1).await.unwrap();
        manager.register_service(service2).await.unwrap();

        let all = manager.get_all_services().await;
        assert_eq!(all.len(), 2);
        let names: Vec<_> = all.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"service-a"));
        assert!(names.contains(&"service-b"));
    }

    #[tokio::test]
    async fn test_get_all_services_empty() {
        let manager = ServiceManager::new();

        let all = manager.get_all_services().await;
        assert!(all.is_empty());
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
    async fn test_mark_disconnected() {
        let manager = ServiceManager::new();
        let service = create_test_service("test-service", true);
        let service_id = service.id.clone();

        manager.register_service(service).await.unwrap();
        manager.mark_connected(&service_id).await;
        manager.mark_disconnected(&service_id).await;

        let status = manager.get_service_status(&service_id).await;
        assert!(matches!(status, Some(ServiceStatus::Disconnected)));
    }

    #[tokio::test]
    async fn test_update_service_status_direct() {
        let manager = ServiceManager::new();
        let service = create_test_service("test-service", true);
        let service_id = service.id.clone();

        manager.register_service(service).await.unwrap();
        manager
            .update_service_status(&service_id, ServiceStatus::Connecting)
            .await;

        let status = manager.get_service_status(&service_id).await;
        assert!(matches!(status, Some(ServiceStatus::Connecting)));
    }

    #[tokio::test]
    async fn test_get_service_status_unknown_returns_none() {
        let manager = ServiceManager::new();

        let status = manager.get_service_status("unknown-id").await;
        assert!(status.is_none());
    }

    #[tokio::test]
    async fn test_get_all_statuses() {
        let manager = ServiceManager::new();
        let service1 = create_test_service("service-a", true);
        let service2 = create_test_service("service-b", true);
        let id1 = service1.id.clone();
        let id2 = service2.id.clone();

        manager.register_service(service1).await.unwrap();
        manager.register_service(service2).await.unwrap();
        manager.mark_connected(&id1).await;

        let all = manager.get_all_statuses().await;
        assert_eq!(all.len(), 2);
        assert!(all.get(&id1).map(|s| s.is_usable()).unwrap_or(false));
        assert!(matches!(all.get(&id2), Some(ServiceStatus::Discovered)));
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
    async fn test_capability_search_multiple_capabilities() {
        let manager = ServiceManager::new();
        let compute = create_test_service_with_capabilities(
            "compute-1",
            "compute-svc",
            true,
            vec![Capability::Compute(ComputeCapability::NativeExecution)],
        );
        let storage = create_test_service_with_capabilities(
            "storage-1",
            "storage-svc",
            true,
            vec![Capability::Storage(StorageCapability::ObjectStorage)],
        );
        let coordinator = create_test_service_with_capabilities(
            "coord-1",
            "coord-svc",
            true,
            vec![Capability::Coordination(
                CoordinationCapability::ServiceDiscovery,
            )],
        );

        manager.register_service(compute).await.unwrap();
        manager.register_service(storage).await.unwrap();
        manager.register_service(coordinator).await.unwrap();

        let compute_found = manager
            .find_services_by_capability(&Capability::Compute(ComputeCapability::NativeExecution))
            .await;
        assert_eq!(compute_found.len(), 1);
        assert_eq!(compute_found[0].name, "compute-svc");

        let storage_found = manager
            .find_services_by_capability(&Capability::Storage(StorageCapability::ObjectStorage))
            .await;
        assert_eq!(storage_found.len(), 1);
        assert_eq!(storage_found[0].name, "storage-svc");

        let coord_found = manager
            .find_services_by_capability(&Capability::Coordination(
                CoordinationCapability::ServiceDiscovery,
            ))
            .await;
        assert_eq!(coord_found.len(), 1);
        assert_eq!(coord_found[0].name, "coord-svc");
    }

    #[tokio::test]
    async fn test_capability_search_empty_when_no_match() {
        let manager = ServiceManager::new();
        let service = create_test_service_with_capabilities(
            "compute-1",
            "compute-only",
            true,
            vec![Capability::Compute(ComputeCapability::NativeExecution)],
        );

        manager.register_service(service).await.unwrap();

        let found = manager
            .find_services_by_capability(&Capability::Storage(StorageCapability::ObjectStorage))
            .await;
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn test_is_capability_available_requires_usable_status() {
        let manager = ServiceManager::new();
        let service = create_test_service("test-service", true);
        let service_id = service.id.clone();

        manager.register_service(service).await.unwrap();

        // Discovered but not connected - not usable
        let available = manager
            .is_capability_available(&Capability::Compute(ComputeCapability::NativeExecution))
            .await;
        assert!(!available);

        manager.mark_connected(&service_id).await;

        let available = manager
            .is_capability_available(&Capability::Compute(ComputeCapability::NativeExecution))
            .await;
        assert!(available);
    }

    #[tokio::test]
    async fn test_is_capability_available_false_when_no_service() {
        let manager = ServiceManager::new();

        let available = manager
            .is_capability_available(&Capability::Compute(ComputeCapability::NativeExecution))
            .await;
        assert!(!available);
    }

    #[tokio::test]
    async fn test_get_service_capabilities() {
        let manager = ServiceManager::new();
        let capabilities = vec![
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Storage(StorageCapability::ObjectStorage),
        ];
        let service = create_test_service_with_capabilities(
            "svc-1",
            "multi-cap-svc",
            true,
            capabilities.clone(),
        );

        manager.register_service(service).await.unwrap();

        let caps = manager.get_service_capabilities("svc-1").await.unwrap();
        assert_eq!(caps.len(), 2);
        assert!(caps.contains(&Capability::Compute(ComputeCapability::NativeExecution)));
        assert!(caps.contains(&Capability::Storage(StorageCapability::ObjectStorage)));
    }

    #[tokio::test]
    async fn test_get_service_capabilities_unknown_returns_error() {
        let manager = ServiceManager::new();

        let result = manager.get_service_capabilities("unknown-svc").await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Service not found"));
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

    #[tokio::test]
    async fn test_healthy_count_zero_when_empty() {
        let manager = ServiceManager::new();

        assert_eq!(manager.healthy_count().await, 0);
    }

    #[tokio::test]
    async fn test_healthy_count_all_connected() {
        let manager = ServiceManager::new();
        let service1 = create_test_service("service1", true);
        let service2 = create_test_service("service2", true);
        let id1 = service1.id.clone();
        let id2 = service2.id.clone();

        manager.register_service(service1).await.unwrap();
        manager.register_service(service2).await.unwrap();
        manager.mark_connected(&id1).await;
        manager.mark_connected(&id2).await;

        assert_eq!(manager.healthy_count().await, 2);
    }

    #[tokio::test]
    async fn test_count_by_status() {
        let manager = ServiceManager::new();
        let s1 = create_test_service("s1", true);
        let s2 = create_test_service("s2", true);
        let s3 = create_test_service("s3", true);
        let id1 = s1.id.clone();
        let id2 = s2.id.clone();

        manager.register_service(s1).await.unwrap();
        manager.register_service(s2).await.unwrap();
        manager.register_service(s3).await.unwrap();

        manager.mark_connected(&id1).await;
        manager.mark_failed(&id2, "failure-reason").await;

        assert_eq!(manager.count_by_status(ServiceStatus::Connected).await, 1);
        assert_eq!(manager.count_by_status(ServiceStatus::Discovered).await, 1);
        assert_eq!(
            manager
                .count_by_status(ServiceStatus::Failed("failure-reason".to_string()))
                .await,
            1
        );
        assert_eq!(
            manager.count_by_status(ServiceStatus::Disconnected).await,
            0
        );
    }

    #[tokio::test]
    async fn test_get_unhealthy_services() {
        let manager = ServiceManager::new();
        let s1 = create_test_service("s1", true);
        let s2 = create_test_service("s2", true);
        let id1 = s1.id.clone();
        let id2 = s2.id.clone();

        manager.register_service(s1).await.unwrap();
        manager.register_service(s2).await.unwrap();
        manager.mark_connected(&id1).await;
        manager.mark_failed(&id2, "connection refused").await;

        let unhealthy = manager.get_unhealthy_services().await;
        assert_eq!(unhealthy.len(), 1);
        assert_eq!(unhealthy[0], id2);
    }

    #[tokio::test]
    async fn test_get_unhealthy_services_empty_when_all_ok() {
        let manager = ServiceManager::new();
        let s1 = create_test_service("s1", true);
        let id1 = s1.id.clone();

        manager.register_service(s1).await.unwrap();
        manager.mark_connected(&id1).await;

        let unhealthy = manager.get_unhealthy_services().await;
        assert!(unhealthy.is_empty());
    }

    #[tokio::test]
    async fn test_service_count() {
        let manager = ServiceManager::new();
        assert_eq!(manager.service_count().await, 0);

        let s1 = create_test_service("s1", true);
        let s2 = create_test_service("s2", true);
        manager.register_service(s1).await.unwrap();
        manager.register_service(s2).await.unwrap();

        assert_eq!(manager.service_count().await, 2);
    }

    #[tokio::test]
    async fn test_clear_all() {
        let manager = ServiceManager::new();
        let s1 = create_test_service("s1", true);
        let s2 = create_test_service("s2", true);

        manager.register_service(s1).await.unwrap();
        manager.register_service(s2).await.unwrap();
        assert_eq!(manager.service_count().await, 2);

        manager.clear_all().await;

        assert_eq!(manager.service_count().await, 0);
        assert!(manager.get_all_services().await.is_empty());
        assert!(manager.get_all_statuses().await.is_empty());
    }

    #[tokio::test]
    async fn test_clear_all_empty_manager() {
        let manager = ServiceManager::new();

        manager.clear_all().await;

        assert_eq!(manager.service_count().await, 0);
    }

    #[tokio::test]
    async fn test_mark_failed_with_string_slice() {
        let manager = ServiceManager::new();
        let service = create_test_service("test", true);
        let service_id = service.id.clone();

        manager.register_service(service).await.unwrap();
        manager.mark_failed(&service_id, "static-str-reason").await;

        let status = manager.get_service_status(&service_id).await;
        assert!(matches!(status, Some(ServiceStatus::Failed(_))));
        assert_eq!(status.unwrap().error_message(), Some("static-str-reason"));
    }

    #[tokio::test]
    async fn test_default_impl() {
        let manager = ServiceManager::default();
        assert_eq!(manager.service_count().await, 0);

        let service = create_test_service("test", true);
        manager.register_service(service).await.unwrap();
        assert_eq!(manager.service_count().await, 1);
    }
}
