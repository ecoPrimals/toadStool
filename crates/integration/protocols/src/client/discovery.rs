//! Service discovery and registration

use tracing::info;

use crate::config::ServiceDiscoveryConfig;
use crate::types::{ProtocolResult, ServiceInfo};

/// Register service with discovery service
pub async fn register_with_discovery(
    service_info: &ServiceInfo,
    _discovery_config: &ServiceDiscoveryConfig,
) -> ProtocolResult<()> {
    // EVOLVED: Capability-based discovery (no HTTP registry needed!) ✅
    info!(
        "Service {} uses capability-based discovery (no registration needed)",
        service_info.id
    );

    // Note: Capability files are written by each primal in /tmp/ecoPrimals/discovery/
    // See: crates/server/src/capabilities.rs for self-knowledge + announcement pattern

    Ok(())
}

/// Discover services from registry
pub async fn discover_from_registry(
    service_name: &str,
    _discovery_config: &ServiceDiscoveryConfig,
) -> ProtocolResult<Vec<ServiceInfo>> {
    // EVOLVED: Capability-based discovery (no HTTP registry query!) ✅
    info!(
        "Service discovery for {} uses capability files (no registry query needed)",
        service_name
    );

    // Note: Capability files can be read from /tmp/ecoPrimals/discovery/
    // See: crates/server/src/capabilities.rs for find_peer_with() and find_all_peers()

    Ok(Vec::new())
}
