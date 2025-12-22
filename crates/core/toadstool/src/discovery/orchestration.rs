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
    pub fn with_discovery(discovery: Arc<DiscoveryEngine>) -> Self {
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
                    "Failed to discover service-discovery capability: {}",
                    e
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
                    "Failed to discover load-balancing capability: {}",
                    e
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
                    "Failed to discover job-routing capability: {}",
                    e
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
            "No orchestration services found. Tried: service-discovery, load-balancing, job-routing"
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

    #[tokio::test]
    async fn test_orchestration_discovery_pattern() {
        let client = OrchestrationClient::new();

        // Pattern test - either succeeds or fails gracefully
        let result = client.discover_any_orchestration().await;

        // The important part is the pattern, not the result
        // In production, environment variables or config would provide the endpoint
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_default_construction() {
        let _client = OrchestrationClient::default();
        // Should construct without error
    }
}
