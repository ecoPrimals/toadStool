// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_common::primal_identity::Capability;

use crate::{ToadStoolError, ToadStoolResult};

use super::ServiceManager;

impl ServiceManager {
    /// Find services by capability
    pub async fn find_services_by_capability(
        &self,
        capability: &Capability,
    ) -> Vec<crate::ecosystem::types::ServiceInstance> {
        let services = self.services.read().unwrap_or_else(|e| e.into_inner());
        services
            .values()
            .filter(|s| s.has_capability(capability))
            .cloned()
            .collect()
    }

    /// Check if a capability is available
    pub async fn is_capability_available(&self, capability: &Capability) -> bool {
        let services = self.services.read().unwrap_or_else(|e| e.into_inner());
        let statuses = self.statuses.read().unwrap_or_else(|e| e.into_inner());

        services.values().any(|service| {
            service.has_capability(capability)
                && statuses
                    .get(&service.id)
                    .map(|s| s.is_usable())
                    .unwrap_or(false)
        })
    }

    /// Get capabilities for a service
    ///
    /// # Errors
    ///
    /// Returns error if `service_id` is not registered.
    pub async fn get_service_capabilities(
        &self,
        service_id: &str,
    ) -> ToadStoolResult<Vec<Capability>> {
        let service = self
            .services
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(service_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Service not found: {service_id}")))?
            .capabilities
            .clone();

        Ok(service)
    }
}
