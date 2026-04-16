// SPDX-License-Identifier: AGPL-3.0-or-later

//! Enum dispatch for [`toadstool_common::runtime_discovery::DiscoveryClient`] implementations
//! that live in `toadstool-common` and this crate.

use toadstool_common::ToadStoolResult;
use toadstool_common::primal_identity::{Capability, DiscoveredService};
use toadstool_common::runtime_discovery::{DiscoveryClient, LocalhostDiscoveryClient};

use crate::mdns_discovery::MdnsDiscoveryClient;

/// Known [`DiscoveryClient`] implementations (static dispatch, no `dyn`).
pub enum DiscoveryClientDispatch {
    /// Localhost-only discovery (in-memory / env seeding).
    Localhost(LocalhostDiscoveryClient),
    /// Cache-based mDNS-oriented client from this crate.
    Mdns(MdnsDiscoveryClient),
}

impl DiscoveryClient for DiscoveryClientDispatch {
    async fn discover_by_capability<'a>(
        &'a self,
        capability: &'a Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        match self {
            Self::Localhost(c) => c.discover_by_capability(capability).await,
            Self::Mdns(c) => c.discover_by_capability(capability).await,
        }
    }

    async fn discover_all(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        match self {
            Self::Localhost(c) => c.discover_all().await,
            Self::Mdns(c) => c.discover_all().await,
        }
    }

    async fn register_service<'a>(&'a self, service: &'a DiscoveredService) -> ToadStoolResult<()> {
        match self {
            Self::Localhost(c) => c.register_service(service).await,
            Self::Mdns(c) => c.register_service(service).await,
        }
    }

    async fn deregister_service<'a>(&'a self, service_id: &'a str) -> ToadStoolResult<()> {
        match self {
            Self::Localhost(c) => c.deregister_service(service_id).await,
            Self::Mdns(c) => c.deregister_service(service_id).await,
        }
    }

    async fn health_check<'a>(&'a self, service_id: &'a str) -> ToadStoolResult<bool> {
        match self {
            Self::Localhost(c) => c.health_check(service_id).await,
            Self::Mdns(c) => c.health_check(service_id).await,
        }
    }
}
