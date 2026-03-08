// SPDX-License-Identifier: AGPL-3.0-or-later

use tracing::info;

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::service_discovery::DiscoveredService;

use super::ServiceManager;
use crate::ecosystem::types::{ServiceInstance, ServiceStatus};

impl ServiceManager {
    /// Register a discovered service
    pub async fn register_service(&self, service: DiscoveredService) -> ToadStoolResult<()> {
        info!("📋 Registering service: {} ({})", service.name, service.id);

        let service_id = service.id.clone();

        let mut services = self.services.write().await;
        services.insert(service_id.clone(), service.clone());

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
