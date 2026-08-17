// SPDX-License-Identifier: AGPL-3.0-or-later

use tracing::info;

use crate::{ToadStoolError, ToadStoolResult};
#[cfg(feature = "runtime")]
use toadstool_common::service_discovery::DiscoveredService;

use super::ServiceManager;
use crate::ecosystem::types::{ServiceInstance, ServiceStatus};

impl ServiceManager {
    /// Register a discovered service
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`.
    pub async fn register_service(&self, service: DiscoveredService) -> ToadStoolResult<()> {
        info!("📋 Registering service: {} ({})", service.name, service.id);

        let service_id = service.id.clone();
        let initial_status = if service.healthy {
            ServiceStatus::Discovered
        } else {
            ServiceStatus::Failed("Service reported unhealthy".to_string())
        };

        self.services
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(service_id.clone(), service);
        self.statuses
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(service_id.clone(), initial_status);

        info!("✅ Service registered: {}", service_id);
        Ok(())
    }

    /// Unregister a service
    ///
    /// # Errors
    ///
    /// Returns error if `service_id` is not registered.
    pub async fn unregister_service(&self, service_id: &str) -> ToadStoolResult<()> {
        info!("🗑️  Unregistering service: {}", service_id);

        self.services
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(service_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Service not found: {service_id}")))?;

        self.statuses
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(service_id);

        info!("✅ Service unregistered: {}", service_id);
        Ok(())
    }

    /// Get a service by ID
    pub async fn get_service(&self, service_id: &str) -> Option<ServiceInstance> {
        let services = self.services.read().unwrap_or_else(|e| e.into_inner());
        services.get(service_id).cloned()
    }

    /// Get all registered services
    pub async fn get_all_services(&self) -> Vec<ServiceInstance> {
        let services = self.services.read().unwrap_or_else(|e| e.into_inner());
        services.values().cloned().collect()
    }

    /// Clear all services
    pub async fn clear_all(&self) {
        info!("🗑️  Clearing all services");
        self.services
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        self.statuses
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Get service count
    pub async fn service_count(&self) -> usize {
        let services = self.services.read().unwrap_or_else(|e| e.into_inner());
        services.len()
    }
}
