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
        let services = self.services.read().await;
        services
            .values()
            .filter(|s| s.has_capability(capability))
            .cloned()
            .collect()
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
}
