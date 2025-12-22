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
    /// TODO: Wire up to config crate's MdnsDiscoveryClient
    pub async fn discover(&self, _capability: &str) -> Result<Vec<PrimalEndpoint>, DiscoveryError> {
        // Placeholder - will integrate with config::mdns_discovery::MdnsDiscoveryClient
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
}
