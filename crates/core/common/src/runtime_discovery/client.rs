// SPDX-License-Identifier: AGPL-3.0-or-later

use std::future::Future;
use std::pin::Pin;

use crate::error::ToadStoolResult;
use crate::primal_identity::{Capability, DiscoveredService};

/// Discovery client trait - implement for different discovery mechanisms
pub trait DiscoveryClient: Send + Sync {
    /// Discover services by capability
    fn discover_by_capability<'a>(
        &'a self,
        capability: &'a Capability,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>>;

    /// Discover all available services
    fn discover_all<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>>;

    /// Register a service (for service registration)
    fn register_service<'a>(
        &'a self,
        service: &'a DiscoveredService,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Deregister a service
    fn deregister_service<'a>(
        &'a self,
        service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Health check a service
    fn health_check<'a>(
        &'a self,
        service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<bool>> + Send + 'a>>;
}
