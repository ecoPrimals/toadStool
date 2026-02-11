//! Integration adapter between primal_discovery and mDNS
//!
//! This module bridges the new `primal_discovery` module with the existing
//! mDNS discovery infrastructure in the config crate.

use crate::primal_discovery::{
    DiscoveryConfig, DiscoveryError, DiscoveryMethod, PrimalEndpoint, TrustLevel,
};
use std::sync::Arc;
use std::time::Instant;

/// Adapter to integrate mDNS with primal discovery
///
/// This will be the bridge once we wire up the config crate's MdnsDiscoveryClient
pub struct MdnsAdapter {
    // TODO: Add MdnsDiscoveryClient when we wire up config crate
    _config: Arc<DiscoveryConfig>,
}

impl MdnsAdapter {
    /// Create new mDNS adapter
    pub async fn new(config: DiscoveryConfig) -> Result<Self, DiscoveryError> {
        Ok(Self {
            _config: Arc::new(config),
        })
    }

    /// Discover services via mDNS
    ///
    /// mDNS discovery pending mdns-sd crate integration (network access required).
    /// TODO: Wire up to config crate's MdnsDiscoveryClient
    pub async fn discover(&self, _capability: &str) -> Result<Vec<PrimalEndpoint>, DiscoveryError> {
        tracing::debug!("mDNS discovery pending mdns-sd integration; returning empty");
        Ok(Vec::new())
    }
}

/// Helper to convert mDNS discovered services to PrimalEndpoints
///
/// This will be used once we wire up the actual mDNS integration
#[allow(dead_code)]
fn convert_mdns_service_to_endpoint(
    service_id: String,
    capabilities: Vec<String>,
    url: String,
) -> PrimalEndpoint {
    PrimalEndpoint {
        service_id,
        capabilities,
        url,
        trust_level: TrustLevel::Local, // mDNS is local network
        discovered_via: DiscoveryMethod::MDns,
        discovered_at: Instant::now(),
        last_seen: Instant::now(),
        latency_ms: 0, // TODO: Measure actual latency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mdns_adapter_creation() {
        let config = DiscoveryConfig::default();
        let adapter = MdnsAdapter::new(config).await;
        assert!(adapter.is_ok());
    }

    #[tokio::test]
    async fn test_mdns_adapter_discover_returns_empty() {
        let config = DiscoveryConfig::default();
        let adapter = MdnsAdapter::new(config)
            .await
            .expect("Failed to create adapter");

        // Should return empty vector (placeholder implementation)
        let endpoints = adapter.discover("storage").await.expect("Discovery failed");
        assert_eq!(endpoints.len(), 0);
    }

    #[tokio::test]
    async fn test_mdns_adapter_discover_with_different_capabilities() {
        let config = DiscoveryConfig::default();
        let adapter = MdnsAdapter::new(config)
            .await
            .expect("Failed to create adapter");

        // Test with various capability strings
        let endpoints1 = adapter.discover("compute").await.expect("Discovery failed");
        let endpoints2 = adapter
            .discover("security")
            .await
            .expect("Discovery failed");
        let endpoints3 = adapter
            .discover("coordination")
            .await
            .expect("Discovery failed");

        assert_eq!(endpoints1.len(), 0);
        assert_eq!(endpoints2.len(), 0);
        assert_eq!(endpoints3.len(), 0);
    }

    #[test]
    fn test_convert_mdns_service_to_endpoint() {
        let endpoint = convert_mdns_service_to_endpoint(
            "service-123".to_string(),
            vec!["storage".to_string(), "replication".to_string()],
            "http://192.168.1.100:8000".to_string(),
        );

        assert_eq!(endpoint.service_id, "service-123");
        assert_eq!(endpoint.capabilities.len(), 2);
        assert_eq!(endpoint.capabilities[0], "storage");
        assert_eq!(endpoint.capabilities[1], "replication");
        assert_eq!(endpoint.url, "http://192.168.1.100:8000");
        assert_eq!(endpoint.trust_level, TrustLevel::Local);
        assert_eq!(endpoint.discovered_via, DiscoveryMethod::MDns);
        assert_eq!(endpoint.latency_ms, 0);
    }

    #[test]
    fn test_convert_mdns_service_empty_capabilities() {
        let endpoint = convert_mdns_service_to_endpoint(
            "service-456".to_string(),
            vec![],
            "http://localhost:9000".to_string(),
        );

        assert_eq!(endpoint.service_id, "service-456");
        assert_eq!(endpoint.capabilities.len(), 0);
        assert_eq!(endpoint.url, "http://localhost:9000");
    }

    #[test]
    fn test_convert_mdns_service_single_capability() {
        let endpoint = convert_mdns_service_to_endpoint(
            "compute-1".to_string(),
            vec!["gpu-compute".to_string()],
            "http://10.0.0.5:7777".to_string(),
        );

        assert_eq!(endpoint.service_id, "compute-1");
        assert_eq!(endpoint.capabilities.len(), 1);
        assert_eq!(endpoint.capabilities[0], "gpu-compute");
    }

    #[test]
    fn test_convert_mdns_service_trust_level_always_local() {
        // mDNS services should always be Local trust level
        let endpoint1 = convert_mdns_service_to_endpoint(
            "svc1".to_string(),
            vec!["test".to_string()],
            "http://192.168.0.1:8080".to_string(),
        );
        let endpoint2 = convert_mdns_service_to_endpoint(
            "svc2".to_string(),
            vec!["test".to_string()],
            "http://10.0.0.1:8080".to_string(),
        );

        assert_eq!(endpoint1.trust_level, TrustLevel::Local);
        assert_eq!(endpoint2.trust_level, TrustLevel::Local);
    }

    #[test]
    fn test_convert_mdns_service_discovery_method_always_mdns() {
        let endpoint = convert_mdns_service_to_endpoint(
            "test-service".to_string(),
            vec!["capability".to_string()],
            "http://example.local:8080".to_string(),
        );

        assert_eq!(endpoint.discovered_via, DiscoveryMethod::MDns);
    }
}
