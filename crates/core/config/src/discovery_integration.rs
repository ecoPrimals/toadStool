// SPDX-License-Identifier: AGPL-3.0-only
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
use toadstool_common::ToadStoolResult;
use toadstool_common::primal_identity::{Capability, DiscoveredService};
use toadstool_common::runtime_discovery::RuntimeDiscovery;

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
/// use toadstool_common::constants::network;
///
/// let discovery = RuntimeDiscovery::new()?;
/// let fallback = std::env::var("TOADSTOOL_COORDINATION_ENDPOINT")
///     .unwrap_or_else(|_| network::http_url(
///         network::DEFAULT_HOSTNAME,
///         network::COORDINATION_FALLBACK_PORT,
///     ));
/// let coord_endpoint = discover_or_fallback(
///     &discovery,
///     Capability::Coordination,
///     &fallback,
/// ).await?;
/// ```
///
/// # Errors
///
/// This function does not fail; it always returns the discovered endpoint or fallback.
pub async fn discover_or_fallback(
    discovery: &RuntimeDiscovery,
    capability: &Capability,
    fallback_endpoint: &str,
) -> ToadStoolResult<String> {
    match discovery.discover_capability(capability).await {
        Ok(services) if !services.is_empty() => {
            // Use the first available service with a valid endpoint
            services[0].endpoints.first().map_or_else(
                || {
                    // No endpoints available, use fallback
                    tracing::warn!(
                        "Service found for capability {:?} but has no endpoints, using fallback: {}",
                        capability,
                        fallback_endpoint
                    );
                    Ok(fallback_endpoint.to_string())
                },
                |endpoint| {
                    let url = format!(
                        "{}://{}:{}",
                        endpoint.protocol, endpoint.address, endpoint.port
                    );
                    Ok(url)
                },
            )
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
///
/// # Errors
///
/// Returns `Err` if discovery fails.
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
///
/// # Errors
///
/// This function does not fail; it always returns the discovered endpoint or fallback.
pub async fn discover_with_load_balancing(
    discovery: &RuntimeDiscovery,
    capability: &Capability,
    fallback_endpoint: &str,
) -> ToadStoolResult<String> {
    match discovery.discover_capability(capability).await {
        Ok(services) if !services.is_empty() => {
            // Simple round-robin: use hash of timestamp to select
            #[expect(clippy::cast_possible_truncation, reason = "endpoint count bounded")]
            let index = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                % services.len() as u64) as usize; // fits: index < services.len()

            services[index].endpoints.first().map_or_else(
                || {
                    // No endpoints available, use fallback
                    tracing::warn!(
                        "Service found for capability {:?} but has no endpoints, using fallback: {}",
                        capability,
                        fallback_endpoint
                    );
                    Ok(fallback_endpoint.to_string())
                },
                |endpoint| {
                    let url = format!(
                        "{}://{}:{}",
                        endpoint.protocol, endpoint.address, endpoint.port
                    );
                    Ok(url)
                },
            )
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

/// Create a new service discovery instance using localhost discovery
///
/// Creates a `RuntimeDiscovery` instance that discovers services on the
/// local machine via socket probing and environment variables. This is the
/// standard production discovery path — primals discover each other at
/// runtime using capability-based socket discovery.
///
/// For multi-machine deployments, use a network-aware discovery client
/// (e.g., mDNS, Birdsong UDP, or Songbird coordination).
///
/// # Returns
///
/// A configured `RuntimeDiscovery` instance with localhost discovery
///
/// # Examples
///
/// ```rust,ignore
/// use toadstool_config::discovery_integration::create_discovery;
///
/// let discovery = create_discovery()?;
/// // Discovers services on localhost via socket probing
/// ```
///
/// # Errors
///
/// This implementation does not fail; returns [`ToadStoolResult`] for API consistency.
pub fn create_discovery() -> ToadStoolResult<RuntimeDiscovery> {
    let client = Arc::new(toadstool_common::runtime_discovery::LocalhostDiscoveryClient::new());
    Ok(RuntimeDiscovery::new(client))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use toadstool_common::primal_identity::{CoordinationCapability, ServiceEndpoint};
    use toadstool_common::runtime_discovery::{DiscoveryClient, LocalhostDiscoveryClient};

    /// Test discovery client that returns configurable results
    struct TestDiscoveryClient {
        services: Vec<DiscoveredService>,
        fail: bool,
    }

    // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
    #[async_trait]
    impl DiscoveryClient for TestDiscoveryClient {
        async fn discover_by_capability(
            &self,
            _capability: &Capability,
        ) -> ToadStoolResult<Vec<DiscoveredService>> {
            if self.fail {
                return Err(toadstool_common::ToadStoolError::Integration(
                    toadstool_common::error::IntegrationError::ServiceUnavailable {
                        service: "test".to_string(),
                        reason: "forced failure".to_string(),
                    },
                ));
            }
            Ok(self.services.clone())
        }

        async fn discover_all(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
            Ok(self.services.clone())
        }

        async fn register_service(&self, _service: &DiscoveredService) -> ToadStoolResult<()> {
            Ok(())
        }

        async fn deregister_service(&self, _service_id: &str) -> ToadStoolResult<()> {
            Ok(())
        }

        async fn health_check(&self, _service_id: &str) -> ToadStoolResult<bool> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_discover_or_fallback_uses_fallback() {
        // When discovery fails or returns no results, should use fallback
        let client = Arc::new(LocalhostDiscoveryClient::new());
        let discovery = RuntimeDiscovery::new(client);
        let fallback = "http://localhost:50001";

        let result = discover_or_fallback(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
            fallback,
        )
        .await
        .unwrap();

        assert_eq!(result, fallback);
    }

    #[tokio::test]
    async fn test_discover_or_fallback_uses_discovered_service_with_endpoints() {
        let service_with_endpoint = DiscoveredService {
            id: Some("coord-1".to_string()),
            capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
            endpoints: vec![ServiceEndpoint::http("discovered.host", 9000)],
            healthy: true,
            metadata: std::collections::HashMap::new(),
        };
        let client = Arc::new(TestDiscoveryClient {
            services: vec![service_with_endpoint],
            fail: false,
        });
        let discovery = RuntimeDiscovery::new(client);
        let fallback = "http://localhost:50001";

        let result = discover_or_fallback(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
            fallback,
        )
        .await
        .unwrap();

        assert_eq!(result, "http://discovered.host:9000");
    }

    #[tokio::test]
    async fn test_discover_or_fallback_uses_fallback_when_service_has_no_endpoints() {
        let service_no_endpoints = DiscoveredService {
            id: Some("coord-1".to_string()),
            capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
            endpoints: vec![],
            healthy: true,
            metadata: std::collections::HashMap::new(),
        };
        let client = Arc::new(TestDiscoveryClient {
            services: vec![service_no_endpoints],
            fail: false,
        });
        let discovery = RuntimeDiscovery::new(client);
        let fallback = "http://localhost:9999";

        let result = discover_or_fallback(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
            fallback,
        )
        .await
        .unwrap();

        assert_eq!(result, fallback);
    }

    #[tokio::test]
    async fn test_discover_or_fallback_uses_fallback_when_no_services() {
        let client = Arc::new(TestDiscoveryClient {
            services: vec![],
            fail: false,
        });
        let discovery = RuntimeDiscovery::new(client);
        let fallback = "http://localhost:8888";

        let result = discover_or_fallback(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
            fallback,
        )
        .await
        .unwrap();

        assert_eq!(result, fallback);
    }

    #[tokio::test]
    async fn test_discover_or_fallback_uses_fallback_on_discovery_error() {
        let client = Arc::new(TestDiscoveryClient {
            services: vec![],
            fail: true,
        });
        let discovery = RuntimeDiscovery::new(client);
        let fallback = "http://localhost:7777";

        let result = discover_or_fallback(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
            fallback,
        )
        .await
        .unwrap();

        assert_eq!(result, fallback);
    }

    #[tokio::test]
    async fn test_discover_all_by_capability() {
        let service = DiscoveredService {
            id: Some("coord-1".to_string()),
            capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
            endpoints: vec![ServiceEndpoint::http("host1", 8080)],
            healthy: true,
            metadata: std::collections::HashMap::new(),
        };
        let client = Arc::new(TestDiscoveryClient {
            services: vec![service],
            fail: false,
        });
        let discovery = RuntimeDiscovery::new(client);

        let services = discover_all_by_capability(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
        )
        .await
        .unwrap();

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id.as_deref(), Some("coord-1"));
    }

    #[tokio::test]
    async fn test_discover_with_load_balancing_uses_discovered_service() {
        let service = DiscoveredService {
            id: Some("coord-1".to_string()),
            capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
            endpoints: vec![ServiceEndpoint::http("lb.host", 9001)],
            healthy: true,
            metadata: std::collections::HashMap::new(),
        };
        let client = Arc::new(TestDiscoveryClient {
            services: vec![service],
            fail: false,
        });
        let discovery = RuntimeDiscovery::new(client);

        let result = discover_with_load_balancing(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
            "http://fallback:5000",
        )
        .await
        .unwrap();

        assert_eq!(result, "http://lb.host:9001");
    }

    #[tokio::test]
    async fn test_discover_with_load_balancing_uses_fallback_when_no_endpoints() {
        let service = DiscoveredService {
            id: Some("coord-1".to_string()),
            capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
            endpoints: vec![],
            healthy: true,
            metadata: std::collections::HashMap::new(),
        };
        let client = Arc::new(TestDiscoveryClient {
            services: vec![service],
            fail: false,
        });
        let discovery = RuntimeDiscovery::new(client);

        let result = discover_with_load_balancing(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
            "http://fallback:6000",
        )
        .await
        .unwrap();

        assert_eq!(result, "http://fallback:6000");
    }

    #[tokio::test]
    async fn test_discover_with_load_balancing_uses_fallback_when_no_services() {
        let client = Arc::new(TestDiscoveryClient {
            services: vec![],
            fail: false,
        });
        let discovery = RuntimeDiscovery::new(client);

        let result = discover_with_load_balancing(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
            "http://fallback:7000",
        )
        .await
        .unwrap();

        assert_eq!(result, "http://fallback:7000");
    }

    #[tokio::test]
    async fn test_discover_with_load_balancing_uses_fallback_on_error() {
        let client = Arc::new(TestDiscoveryClient {
            services: vec![],
            fail: true,
        });
        let discovery = RuntimeDiscovery::new(client);

        let result = discover_with_load_balancing(
            &discovery,
            &Capability::Coordination(CoordinationCapability::default()),
            "http://fallback:8000",
        )
        .await
        .unwrap();

        assert_eq!(result, "http://fallback:8000");
    }

    #[tokio::test]
    async fn test_create_discovery() {
        let discovery = create_discovery();
        assert!(discovery.is_ok());
    }
}
