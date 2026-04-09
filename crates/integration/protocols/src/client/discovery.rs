// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service discovery and registration

use tracing::info;

use crate::config::ServiceDiscoveryConfig;
use crate::types::{ProtocolResult, ServiceInfo};

#[expect(
    clippy::unused_async,
    reason = "async signature kept for when discovery performs real I/O"
)]
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

#[expect(
    clippy::unused_async,
    reason = "async signature kept for when discovery performs real I/O"
)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceDiscoveryConfig;
    use crate::types::ServiceInfo;
    use std::sync::Arc;
    use std::time::Duration;

    fn make_service_info() -> ServiceInfo {
        ServiceInfo {
            id: Arc::from("test-service-1"),
            name: Arc::from("Test Service"),
            version: "1.0.0".to_string(),
            endpoints: vec![],
            metadata: std::collections::HashMap::new(),
            health_status: crate::types::HealthStatus::Healthy,
            last_seen: std::time::SystemTime::now(),
            capabilities: vec![],
        }
    }

    fn make_discovery_config() -> ServiceDiscoveryConfig {
        ServiceDiscoveryConfig {
            discovery_type: crate::config::DiscoveryType::Static,
            registry_endpoint: None,
            registration_ttl: Duration::from_secs(300),
            refresh_interval: Duration::from_secs(60),
            auto_register: false,
        }
    }

    #[tokio::test]
    async fn test_register_with_discovery() {
        let service = make_service_info();
        let config = make_discovery_config();
        let result = register_with_discovery(&service, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discover_from_registry() {
        let config = make_discovery_config();
        let result = discover_from_registry("test-service", &config).await;
        assert!(result.is_ok());
        let services = result.unwrap();
        assert!(services.is_empty());
    }
}
