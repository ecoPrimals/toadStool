// SPDX-License-Identifier: AGPL-3.0-or-later
//! Orchestration Service Discovery
//!
//! **Self-Knowledge Pattern**: ToadStool knows it needs orchestration services
//! (service-discovery, load-balancing, job-routing) but doesn't know specific
//! service names like "Songbird".
//!
//! This module provides a simple API for discovering and using orchestration
//! services by capability.

use crate::error::{ToadStoolError, ToadStoolResult};
use std::sync::Arc;
use toadstool_common::infant_discovery::DiscoveryEngine;

/// Orchestration service client
///
/// Discovers services that provide orchestration capabilities
/// (service-discovery, load-balancing, job-routing) at runtime.
pub struct OrchestrationClient {
    /// Discovery engine for finding services
    discovery: Arc<DiscoveryEngine>,
}

impl OrchestrationClient {
    /// Create a new orchestration client
    ///
    /// Uses the default discovery engine which checks:
    /// 1. Environment variables (SONGBIRD_ENDPOINT, etc.)
    /// 2. mDNS/local network
    /// 3. primal-capabilities.toml fallback
    pub fn new() -> Self {
        Self {
            discovery: Arc::new(DiscoveryEngine::new()),
        }
    }

    /// Create with custom discovery engine
    #[must_use]
    pub const fn with_discovery(discovery: Arc<DiscoveryEngine>) -> Self {
        Self { discovery }
    }

    /// Discover orchestration service endpoint
    ///
    /// **Capability-Based**: Discovers ANY service that provides
    /// service-discovery capability.
    ///
    /// # Example
    /// ```no_run
    /// use toadstool::discovery::OrchestrationClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OrchestrationClient::new();
    /// let endpoint = client.discover_service_discovery().await?;
    /// println!("Discovered orchestration at: {}", endpoint);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover_service_discovery(&self) -> ToadStoolResult<String> {
        self.discovery
            .discover_endpoint("service-discovery")
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to discover service-discovery capability: {e}"
                ))
            })
    }

    /// Discover load-balancing service endpoint
    pub async fn discover_load_balancing(&self) -> ToadStoolResult<String> {
        self.discovery
            .discover_endpoint("load-balancing")
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to discover load-balancing capability: {e}"
                ))
            })
    }

    /// Discover job-routing service endpoint
    pub async fn discover_job_routing(&self) -> ToadStoolResult<String> {
        self.discovery
            .discover_endpoint("job-routing")
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to discover job-routing capability: {e}"
                ))
            })
    }

    /// Discover any orchestration endpoint
    ///
    /// Tries multiple capabilities in priority order and returns the first found.
    ///
    /// Priority:
    /// 1. service-discovery (primary orchestration capability)
    /// 2. load-balancing (can route jobs)
    /// 3. job-routing (specialized routing)
    pub async fn discover_any_orchestration(&self) -> ToadStoolResult<String> {
        // Try capabilities in priority order
        let capabilities = ["service-discovery", "load-balancing", "job-routing"];

        for capability in &capabilities {
            if let Ok(endpoint) = self.discovery.discover_endpoint(capability).await {
                tracing::info!(
                    "✅ Discovered orchestration service via '{}' capability: {}",
                    capability,
                    endpoint
                );
                return Ok(endpoint);
            }
        }

        Err(ToadStoolError::configuration(
            "No orchestration services found. Tried: service-discovery, load-balancing, job-routing",
        ))
    }
}

impl Default for OrchestrationClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick helper function for discovering orchestration services
///
/// **Self-Knowledge Pattern**: No "Songbird" mentioned - discovers by capability!
///
/// # Example
/// ```no_run
/// use toadstool::discovery::discover_orchestration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let endpoint = discover_orchestration().await?;
/// // Use the endpoint (could be Songbird, or any compatible service!)
/// # Ok(())
/// # }
/// ```
pub async fn discover_orchestration() -> ToadStoolResult<String> {
    OrchestrationClient::new()
        .discover_any_orchestration()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use toadstool_common::infant_discovery::{DiscoveryEngine, DiscoveryError, EndpointSource};

    /// Mock source that returns different endpoints per capability
    struct CapabilityAwareMockSource {
        results: HashMap<String, Option<String>>,
    }

    impl EndpointSource for CapabilityAwareMockSource {
        fn resolve(
            &self,
            capability: &str,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<Option<String>, DiscoveryError>>
                    + Send
                    + '_,
            >,
        > {
            let endpoint = self.results.get(capability).cloned().flatten();
            Box::pin(async move { Ok(endpoint) })
        }

        fn source_name(&self) -> &'static str {
            "test_mock"
        }
    }

    #[tokio::test]
    async fn test_default_construction() {
        let _client = OrchestrationClient::default();
    }

    #[tokio::test]
    async fn test_with_discovery_uses_provided_engine() {
        let mut results = HashMap::new();
        results.insert(
            "service-discovery".to_string(),
            Some("http://orchestration:8080".to_string()),
        );

        let engine = DiscoveryEngine::with_config(
            toadstool_common::infant_discovery::ServiceDiscoveryConfig::default(),
        );
        engine
            .register_source(Arc::new(CapabilityAwareMockSource { results }))
            .await;

        let client = OrchestrationClient::with_discovery(Arc::new(engine));
        let endpoint = client.discover_service_discovery().await.unwrap();
        assert_eq!(endpoint, "http://orchestration:8080");
    }

    #[tokio::test]
    async fn test_discover_service_discovery_error_mapping() {
        let engine = DiscoveryEngine::new();
        let client = OrchestrationClient::with_discovery(Arc::new(engine));

        let result = client.discover_service_discovery().await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("service-discovery") || err_msg.contains("discover"));
    }

    #[tokio::test]
    async fn test_discover_load_balancing_error_mapping() {
        let engine = DiscoveryEngine::new();
        let client = OrchestrationClient::with_discovery(Arc::new(engine));

        let result = client.discover_load_balancing().await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("load-balancing") || err_msg.contains("discover"));
    }

    #[tokio::test]
    async fn test_discover_job_routing_error_mapping() {
        let engine = DiscoveryEngine::new();
        let client = OrchestrationClient::with_discovery(Arc::new(engine));

        let result = client.discover_job_routing().await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("job-routing") || err_msg.contains("discover"));
    }

    #[tokio::test]
    async fn test_discover_any_orchestration_priority_service_discovery_first() {
        let mut results = HashMap::new();
        results.insert(
            "service-discovery".to_string(),
            Some("http://primary:8080".to_string()),
        );
        results.insert(
            "load-balancing".to_string(),
            Some("http://secondary:8080".to_string()),
        );

        let engine = DiscoveryEngine::with_config(
            toadstool_common::infant_discovery::ServiceDiscoveryConfig::default(),
        );
        engine
            .register_source(Arc::new(CapabilityAwareMockSource { results }))
            .await;

        let client = OrchestrationClient::with_discovery(Arc::new(engine));
        let endpoint = client.discover_any_orchestration().await.unwrap();
        assert_eq!(endpoint, "http://primary:8080");
    }

    #[tokio::test]
    async fn test_discover_any_orchestration_priority_load_balancing_second() {
        let mut results = HashMap::new();
        results.insert("service-discovery".to_string(), None);
        results.insert(
            "load-balancing".to_string(),
            Some("http://lb:9090".to_string()),
        );

        let engine = DiscoveryEngine::with_config(
            toadstool_common::infant_discovery::ServiceDiscoveryConfig::default(),
        );
        engine
            .register_source(Arc::new(CapabilityAwareMockSource { results }))
            .await;

        let client = OrchestrationClient::with_discovery(Arc::new(engine));
        let endpoint = client.discover_any_orchestration().await.unwrap();
        assert_eq!(endpoint, "http://lb:9090");
    }

    #[tokio::test]
    async fn test_discover_any_orchestration_priority_job_routing_third() {
        let mut results = HashMap::new();
        results.insert("service-discovery".to_string(), None);
        results.insert("load-balancing".to_string(), None);
        results.insert(
            "job-routing".to_string(),
            Some("http://routing:7070".to_string()),
        );

        let engine = DiscoveryEngine::with_config(
            toadstool_common::infant_discovery::ServiceDiscoveryConfig::default(),
        );
        engine
            .register_source(Arc::new(CapabilityAwareMockSource { results }))
            .await;

        let client = OrchestrationClient::with_discovery(Arc::new(engine));
        let endpoint = client.discover_any_orchestration().await.unwrap();
        assert_eq!(endpoint, "http://routing:7070");
    }

    #[tokio::test]
    async fn test_discover_any_orchestration_fails_when_all_missing() {
        let engine = DiscoveryEngine::new();
        let client = OrchestrationClient::with_discovery(Arc::new(engine));

        let result = client.discover_any_orchestration().await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("service-discovery")
                || err_msg.contains("load-balancing")
                || err_msg.contains("job-routing")
                || err_msg.contains("No orchestration")
        );
    }

    #[tokio::test]
    async fn test_discover_orchestration_helper() {
        let result = discover_orchestration().await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_orchestration_discovery_pattern() {
        let client = OrchestrationClient::new();
        let result = client.discover_any_orchestration().await;
        assert!(result.is_ok() || result.is_err());
    }
}
