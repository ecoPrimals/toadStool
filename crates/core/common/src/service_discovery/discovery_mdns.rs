//! mDNS discovery backend

use std::collections::HashMap;
use std::time::SystemTime;

use tracing::info;

use crate::primal_identity::ServiceEndpoint;

use super::config::capability_from_str;
use super::types::{DiscoveredService, DiscoveryError, DiscoveryResult};

/// Discover services via mDNS.
pub async fn discover_via_mdns() -> DiscoveryResult<Vec<DiscoveredService>> {
    use crate::primal_discovery::DiscoveryConfig as PrimalDiscoveryConfig;
    use crate::primal_discovery_mdns::MdnsAdapter;

    let mdns_config = PrimalDiscoveryConfig {
        enable_mdns: true,
        ..Default::default()
    };

    // MdnsAdapter::discover_all() uses blocking recv_timeout internally;
    // run on the blocking thread pool to avoid starving the async executor.
    let endpoints = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            MdnsAdapter::new(mdns_config)
                .await
                .map_err(|e| DiscoveryError::MethodUnavailable {
                    method: format!("mDNS init failed: {e}"),
                })?
                .discover_all()
                .await
                .map_err(|e| DiscoveryError::MethodUnavailable {
                    method: format!("mDNS browse failed: {e}"),
                })
        })
    })
    .await
    .map_err(|e| DiscoveryError::MethodUnavailable {
        method: format!("spawn_blocking failed: {e}"),
    })??;

    let now = SystemTime::now();
    let services: Vec<DiscoveredService> = endpoints
        .into_iter()
        .map(|ep| {
            let caps: Vec<_> = ep
                .capabilities
                .iter()
                .map(|s| capability_from_str(s))
                .collect();
            let endpoint =
                ServiceEndpoint::from_url_string(&ep.url).unwrap_or_else(|_| ServiceEndpoint {
                    protocol: "http".to_string(),
                    address: ep.url.clone(),
                    port: 80,
                    path: None,
                    metadata: HashMap::new(),
                });
            DiscoveredService {
                id: ep.service_id.clone(),
                name: ep.service_id,
                version: "mdns".to_string(),
                capabilities: caps,
                endpoints: vec![endpoint],
                metadata: HashMap::new(),
                discovered_at: now,
                last_seen: now,
                healthy: true,
            }
        })
        .collect();

    info!("mDNS discovery: found {} services", services.len());
    Ok(services)
}
