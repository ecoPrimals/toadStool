//! Discovery Integration for Configuration Layer
//!
//! This module provides integration between the legacy configuration system
//! and the new capability-based discovery architecture.
//!
//! # Architecture Evolution
//!
//! **Old Pattern** (Hardcoded):
//! ```rust,ignore
//! let songbird_url = config.endpoints.songbird;
//! ```
//!
//! **New Pattern** (Capability-Based):
//! ```rust,ignore
//! let discovery = ServiceDiscovery::new()?;
//! let coord_service = discovery.find_by_capability(Capability::Coordination).await?;
//! let songbird_url = coord_service.endpoint;
//! ```
//!
//! # Migration Helpers
//!
//! This module provides helpers to ease the transition:
//! ```rust,ignore
//! // Discover with fallback to legacy config
//! let endpoint = discover_or_fallback(
//!     &discovery,
//!     Capability::Coordination,
//!     &config.endpoints.songbird
//! ).await?;
//! ```

use std::sync::Arc;
use toadstool_common::primal_identity::{Capability, DiscoveredService};
use toadstool_common::runtime_discovery::{DiscoveryClient, RuntimeDiscovery};
use toadstool_common::ToadStoolResult;

/// Discover a service by capability with fallback to legacy endpoint
///
/// This helper function attempts to discover a service via capability-based
/// discovery. If discovery fails or no service is found, it falls back to
/// the provided legacy endpoint.
///
/// # Arguments
///
/// * `discovery` - The service discovery instance
/// * `capability` - The capability to search for
/// * `fallback_endpoint` - Legacy endpoint to use if discovery fails
///
/// # Returns
///
/// The discovered or fallback endpoint URL
///
/// # Examples
///
/// ```rust,ignore
/// use toadstool_config::discovery_integration::discover_or_fallback;
/// use toadstool_common::runtime_discovery::RuntimeDiscovery;
/// use toadstool_common::primal_identity::Capability;
///
/// let discovery = RuntimeDiscovery::new()?;
/// let coord_endpoint = discover_or_fallback(
///     &discovery,
///     Capability::Coordination,
///     "http://localhost:50001"
/// ).await?;
/// ```
pub async fn discover_or_fallback(
    discovery: &RuntimeDiscovery,
    capability: &Capability,
    fallback_endpoint: &str,
) -> ToadStoolResult<String> {
    match discovery.discover_capability(capability).await {
        Ok(services) if !services.is_empty() => {
            // Use the first available service with a valid endpoint
            if let Some(endpoint) = services[0].endpoints.first() {
                let url = format!(
                    "{}://{}:{}",
                    endpoint.protocol, endpoint.address, endpoint.port
                );
                Ok(url)
            } else {
                // No endpoints available, use fallback
                tracing::warn!(
                    "Service found for capability {:?} but has no endpoints, using fallback: {}",
                    capability,
                    fallback_endpoint
                );
                Ok(fallback_endpoint.to_string())
            }
        }
        Ok(_) => {
            // No services found, use fallback
            tracing::debug!(
                "No service found for capability {:?}, using fallback: {}",
                capability,
                fallback_endpoint
            );
            Ok(fallback_endpoint.to_string())
        }
        Err(e) => {
            // Discovery failed, use fallback
            tracing::warn!(
                "Service discovery failed for capability {:?}: {}. Using fallback: {}",
                capability,
                e,
                fallback_endpoint
            );
            Ok(fallback_endpoint.to_string())
        }
    }
}

/// Discover all services with specific capability
///
/// Returns all services that advertise the requested capability.
/// Unlike `discover_or_fallback`, this does not use a fallback and
/// returns an error if discovery fails.
///
/// # Arguments
///
/// * `discovery` - The service discovery instance
/// * `capability` - The capability to search for
///
/// # Returns
///
/// Vector of service information for all matching services
pub async fn discover_all_by_capability(
    discovery: &RuntimeDiscovery,
    capability: &Capability,
) -> ToadStoolResult<Vec<DiscoveredService>> {
    discovery.discover_capability(capability).await
}

/// Discover service with load balancing preference
///
/// Discovers services by capability and selects one based on load balancing.
/// Currently uses round-robin selection, but can be enhanced with more
/// sophisticated algorithms.
///
/// # Arguments
///
/// * `discovery` - The service discovery instance
/// * `capability` - The capability to search for
/// * `fallback_endpoint` - Legacy endpoint to use if discovery fails
///
/// # Returns
///
/// The selected endpoint URL
pub async fn discover_with_load_balancing(
    discovery: &RuntimeDiscovery,
    capability: &Capability,
    fallback_endpoint: &str,
) -> ToadStoolResult<String> {
    match discovery.discover_capability(capability).await {
        Ok(services) if !services.is_empty() => {
            // Simple round-robin: use hash of timestamp to select
            let index = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                % services.len() as u64) as usize;

            if let Some(endpoint) = services[index].endpoints.first() {
                let url = format!(
                    "{}://{}:{}",
                    endpoint.protocol, endpoint.address, endpoint.port
                );
                Ok(url)
            } else {
                // No endpoints available, use fallback
                tracing::warn!(
                    "Service found for capability {:?} but has no endpoints, using fallback: {}",
                    capability,
                    fallback_endpoint
                );
                Ok(fallback_endpoint.to_string())
            }
        }
        Ok(_) => {
            tracing::debug!(
                "No service found for capability {:?}, using fallback: {}",
                capability,
                fallback_endpoint
            );
            Ok(fallback_endpoint.to_string())
        }
        Err(e) => {
            tracing::warn!(
                "Service discovery failed for capability {:?}: {}. Using fallback: {}",
                capability,
                e,
                fallback_endpoint
            );
            Ok(fallback_endpoint.to_string())
        }
    }
}

/// Create a mock discovery client for testing
///
/// This creates a simple in-memory discovery client that can be used for testing
/// or as a fallback when no real discovery service is available.
struct MockDiscoveryClient {
    services: Arc<tokio::sync::RwLock<Vec<DiscoveredService>>>,
}

#[async_trait::async_trait]
impl DiscoveryClient for MockDiscoveryClient {
    async fn discover_by_capability(
        &self,
        capability: &Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        let services = self.services.read().await;
        Ok(services
            .iter()
            .filter(|s| s.capabilities.contains(capability))
            .cloned()
            .collect())
    }

    async fn discover_all(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        Ok(self.services.read().await.clone())
    }

    async fn register_service(&self, service: &DiscoveredService) -> ToadStoolResult<()> {
        self.services.write().await.push(service.clone());
        Ok(())
    }

    async fn deregister_service(&self, service_id: &str) -> ToadStoolResult<()> {
        self.services
            .write()
            .await
            .retain(|s| s.id.as_deref() != Some(service_id));
        Ok(())
    }

    async fn health_check(&self, _service_id: &str) -> ToadStoolResult<bool> {
        Ok(true)
    }
}

/// Create a new service discovery instance with mock client
///
/// This is a convenience function that creates a `RuntimeDiscovery` instance
/// with a mock discovery client for testing or fallback scenarios.
///
/// For production use, you should create a RuntimeDiscovery with a real
/// discovery client (e.g., mDNS, Consul, etcd).
///
/// # Returns
///
/// A configured `RuntimeDiscovery` instance with mock client
pub fn create_discovery() -> ToadStoolResult<RuntimeDiscovery> {
    let mock_client = Arc::new(MockDiscoveryClient {
        services: Arc::new(tokio::sync::RwLock::new(Vec::new())),
    });
    Ok(RuntimeDiscovery::new(mock_client))
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_common::runtime_discovery::LocalhostDiscoveryClient;

    #[tokio::test]
    async fn test_discover_or_fallback_uses_fallback() {
        // When discovery fails or returns no results, should use fallback
        // Use LocalhostDiscoveryClient for testing - it provides fallback behavior
        let client = Arc::new(LocalhostDiscoveryClient::new());
        let discovery = RuntimeDiscovery::new(client);
        let fallback = "http://localhost:50001";

        let result = discover_or_fallback(
            &discovery,
            &Capability::Coordination(Default::default()),
            fallback,
        )
        .await
        .unwrap();

        // Should return fallback since no services are registered in test environment
        assert_eq!(result, fallback);
    }

    #[tokio::test]
    async fn test_create_discovery() {
        let discovery = create_discovery();
        assert!(discovery.is_ok());
    }
}
