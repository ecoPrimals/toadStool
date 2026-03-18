// SPDX-License-Identifier: AGPL-3.0-or-later
//! DNS and service discovery extension
//!
//! Provides DNS discovery configuration and validation.

use toadstool::error::ToadStoolResult;
use tracing::{debug, info};

/// Discovery extension trait
#[allow(clippy::redundant_pub_crate)]
pub(crate) trait DiscoveryExt {
    /// Apply DNS discovery configuration
    async fn apply_dns_discovery_config(&self) -> ToadStoolResult<()>;

    /// Validate DNS discovery configuration
    fn validate_dns_discovery_config(&self) -> ToadStoolResult<()>;
}

impl DiscoveryExt for super::SongbirdNetworkConfigurator {
    async fn apply_dns_discovery_config(&self) -> ToadStoolResult<()> {
        info!("🔍 Applying DNS discovery configuration");

        let config = &self.config.dns_discovery;
        debug!("DNS servers: {:?}", config.dns_servers);
        debug!("Search domains: {:?}", config.search_domains);

        // Configuration details...

        Ok(())
    }

    fn validate_dns_discovery_config(&self) -> ToadStoolResult<()> {
        let config = &self.config.dns_discovery;

        if config.enabled && config.dns_servers.is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "At least one DNS server must be configured".to_string(),
            ));
        }

        Ok(())
    }
}
