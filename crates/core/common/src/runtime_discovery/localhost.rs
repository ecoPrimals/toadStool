// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::error::ToadStoolResult;
use crate::primal_identity::{Capability, DiscoveredService, ServiceEndpoint};

use super::client::DiscoveryClient;

/// Localhost discovery client - fallback when no discovery service available
pub struct LocalhostDiscoveryClient {
    /// Known localhost services
    services: Vec<DiscoveredService>,
}

impl LocalhostDiscoveryClient {
    /// Create a new localhost discovery client.
    ///
    /// Starts with an empty service list. Register services via `add_service()`
    /// or use `with_local_compute()` to seed a local compute endpoint.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    /// Seed a local compute endpoint discovered from the environment.
    ///
    /// Reads `TOADSTOOL_LOCAL_PORT` (default: OS-assigned) to avoid hardcoded ports.
    #[must_use]
    pub fn with_local_compute(mut self) -> Self {
        let port: u16 = std::env::var("TOADSTOOL_LOCAL_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if port > 0 {
            self.services.push(DiscoveredService {
                id: Some("localhost-compute".to_string()),
                capabilities: vec![Capability::Compute(
                    crate::primal_identity::ComputeCapability::NativeExecution,
                )],
                endpoints: vec![ServiceEndpoint::http(
                    crate::constants::network::DEFAULT_HOSTNAME,
                    port,
                )],
                healthy: true,
                metadata: HashMap::new(),
            });
        }
        self
    }

    /// Add a service to the localhost registry
    pub fn add_service(&mut self, service: DiscoveredService) {
        self.services.push(service);
    }
}

impl Default for LocalhostDiscoveryClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryClient for LocalhostDiscoveryClient {
    fn discover_by_capability<'a>(
        &'a self,
        capability: &'a Capability,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>> {
        Box::pin(async move {
            Ok(self
                .services
                .iter()
                .filter(|s| s.has_capability(capability))
                .cloned()
                .collect())
        })
    }

    fn discover_all<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>> {
        Box::pin(async move { Ok(self.services.clone()) })
    }

    fn register_service<'a>(
        &'a self,
        _service: &'a DiscoveredService,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
            // Localhost client is read-only
            Ok(())
        })
    }

    fn deregister_service<'a>(
        &'a self,
        _service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
            // Localhost client is read-only
            Ok(())
        })
    }

    fn health_check<'a>(
        &'a self,
        service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<bool>> + Send + 'a>> {
        Box::pin(async move {
            Ok(self
                .services
                .iter()
                .any(|s| s.id.as_deref() == Some(service_id)))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::primal_identity::{
        Capability, ComputeCapability, DiscoveredService, ServiceEndpoint, StorageCapability,
    };

    use crate::runtime_discovery::client::DiscoveryClient;

    use super::LocalhostDiscoveryClient;

    fn seeded_compute_client() -> LocalhostDiscoveryClient {
        let mut client = LocalhostDiscoveryClient::new();
        client.add_service(DiscoveredService {
            id: Some("test-compute".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 9999)],
            healthy: true,
            metadata: HashMap::new(),
        });
        client
    }

    #[tokio::test]
    async fn new_and_default_start_with_no_services() {
        let a = LocalhostDiscoveryClient::new();
        let b = LocalhostDiscoveryClient::default();
        assert!(a.discover_all().await.unwrap().is_empty());
        assert!(b.discover_all().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn discover_all_returns_registered_services_in_order() {
        let mut client = LocalhostDiscoveryClient::new();
        client.add_service(DiscoveredService {
            id: Some("first".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 1)],
            healthy: true,
            metadata: HashMap::new(),
        });
        client.add_service(DiscoveredService {
            id: Some("second".to_string()),
            capabilities: vec![Capability::Storage(StorageCapability::ObjectStorage)],
            endpoints: vec![ServiceEndpoint::http("localhost", 2)],
            healthy: true,
            metadata: HashMap::new(),
        });
        let all = client.discover_all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id.as_deref(), Some("first"));
        assert_eq!(all[1].id.as_deref(), Some("second"));
    }

    #[tokio::test]
    async fn discover_by_capability_filters_out_non_matching_services() {
        let mut client = LocalhostDiscoveryClient::new();
        client.add_service(DiscoveredService {
            id: Some("compute-only".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 10)],
            healthy: true,
            metadata: HashMap::new(),
        });
        client.add_service(DiscoveredService {
            id: Some("storage-only".to_string()),
            capabilities: vec![Capability::Storage(StorageCapability::ObjectStorage)],
            endpoints: vec![ServiceEndpoint::http("localhost", 11)],
            healthy: true,
            metadata: HashMap::new(),
        });

        let storage_cap = Capability::Storage(StorageCapability::ObjectStorage);
        let found = client.discover_by_capability(&storage_cap).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id.as_deref(), Some("storage-only"));

        let compute_cap = Capability::Compute(ComputeCapability::NativeExecution);
        let compute_only = client.discover_by_capability(&compute_cap).await.unwrap();
        assert_eq!(compute_only.len(), 1);
        assert_eq!(compute_only[0].id.as_deref(), Some("compute-only"));
    }

    #[tokio::test]
    async fn discover_by_capability_empty_when_no_match() {
        let client = seeded_compute_client();
        let cap = Capability::Storage(StorageCapability::BlockStorage);
        let services = client.discover_by_capability(&cap).await.unwrap();
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn health_check_true_only_for_known_service_id() {
        let client = seeded_compute_client();
        assert!(client.health_check("test-compute").await.unwrap());
        assert!(!client.health_check("unknown-id").await.unwrap());
    }

    #[tokio::test]
    async fn register_and_deregister_are_no_ops_but_ok() {
        let client = LocalhostDiscoveryClient::new();
        let service = DiscoveredService {
            id: Some("register-test".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 9000)],
            healthy: true,
            metadata: HashMap::new(),
        };
        assert!(client.register_service(&service).await.is_ok());
        assert!(client.deregister_service("any").await.is_ok());
        assert!(client.discover_all().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn with_local_compute_adds_service_when_port_env_positive() {
        temp_env::async_with_vars([("TOADSTOOL_LOCAL_PORT", Some("18444"))], async {
            let client = LocalhostDiscoveryClient::new().with_local_compute();
            let all = client.discover_all().await.unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].id.as_deref(), Some("localhost-compute"));
            let ep = &all[0].endpoints[0];
            assert_eq!(ep.protocol, "http");
            assert_eq!(ep.port, 18444);
        })
        .await;
    }

    #[tokio::test]
    async fn with_local_compute_skips_service_when_port_env_missing_or_zero() {
        temp_env::async_with_vars([("TOADSTOOL_LOCAL_PORT", Some("0"))], async {
            let client = LocalhostDiscoveryClient::new().with_local_compute();
            assert!(client.discover_all().await.unwrap().is_empty());
        })
        .await;
    }
}
