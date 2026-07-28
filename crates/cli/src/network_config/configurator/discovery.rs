// SPDX-License-Identifier: AGPL-3.0-or-later
//! DNS and service discovery extension
//!
//! Provides DNS discovery configuration and validation.

use toadstool::error::ToadStoolResult;
use tracing::{debug, info, trace};

/// Discovery extension trait
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) trait DiscoveryExt {
    /// Persist DNS discovery settings for the orchestration layer.
    ///
    /// This is the **configure** stage of the configurator lifecycle: values are
    /// validated via [`DiscoveryExt::validate_dns_discovery_config`] and stored on
    /// [`OrchestrationNetworkConfigurator`]. Live DNS resolver wiring and service
    /// discovery registration happen later when the orchestration runtime starts.
    async fn apply_dns_discovery_config(&self) -> ToadStoolResult<()>;

    /// Validate DNS discovery configuration
    fn validate_dns_discovery_config(&self) -> ToadStoolResult<()>;
}

impl DiscoveryExt for super::OrchestrationNetworkConfigurator {
    async fn apply_dns_discovery_config(&self) -> ToadStoolResult<()> {
        info!("🔍 Applying DNS discovery configuration");

        let config = &self.config.dns_discovery;
        debug!("DNS servers: {:?}", config.dns_servers);
        debug!("Search domains: {:?}", config.search_domains);
        // Intentional no-op at this stage: see trait doc for configure → runtime lifecycle.
        debug!(
            "configuration stored; runtime application deferred to orchestration layer (DNS discovery)"
        );

        Ok(())
    }

    fn validate_dns_discovery_config(&self) -> ToadStoolResult<()> {
        trace!(
            "validate_dns_discovery_config: structural checks (servers, domains, timeouts); no live DNS query"
        );
        let config = &self.config.dns_discovery;

        if config.enabled && config.dns_servers.is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "At least one DNS server must be configured".to_string(),
            ));
        }

        if config.enabled {
            for server in &config.dns_servers {
                if server.trim().is_empty() {
                    return Err(toadstool::error::ToadStoolError::configuration(
                        "DNS server entries cannot be empty strings".to_string(),
                    ));
                }
            }

            if config.resolution_timeout.is_zero() {
                return Err(toadstool::error::ToadStoolError::configuration(
                    "DNS resolution timeout cannot be zero when DNS discovery is enabled"
                        .to_string(),
                ));
            }

            let sd = &config.service_domains;
            if sd.compute.trim().is_empty()
                || sd.coordination.trim().is_empty()
                || sd.security.trim().is_empty()
                || sd.storage.trim().is_empty()
                || sd.ai_processing.trim().is_empty()
                || sd.biomeos.trim().is_empty()
            {
                return Err(toadstool::error::ToadStoolError::configuration(
                    "All capability service domain fields must be non-empty when DNS discovery is enabled"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}
