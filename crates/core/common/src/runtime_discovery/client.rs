// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;

use crate::error::ToadStoolResult;
use crate::primal_identity::{Capability, DiscoveredService};

/// Discovery client trait - implement for different discovery mechanisms
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait DiscoveryClient: Send + Sync {
    /// Discover services by capability
    async fn discover_by_capability(
        &self,
        capability: &Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>>;

    /// Discover all available services
    async fn discover_all(&self) -> ToadStoolResult<Vec<DiscoveredService>>;

    /// Register a service (for service registration)
    async fn register_service(&self, service: &DiscoveredService) -> ToadStoolResult<()>;

    /// Deregister a service
    async fn deregister_service(&self, service_id: &str) -> ToadStoolResult<()>;

    /// Health check a service
    async fn health_check(&self, service_id: &str) -> ToadStoolResult<bool>;
}
