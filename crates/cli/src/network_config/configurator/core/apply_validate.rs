// SPDX-License-Identifier: AGPL-3.0-only
//! `apply_configuration` / `validate_configuration` orchestration for the network configurator.

use super::super::*;
use super::super::{DiscoveryExt, ReliabilityExt, SecurityExt, ServiceMeshExt, TrafficExt};
use super::ConfiguratorCore;
use super::defaults;
use tracing::info;

impl ConfiguratorCore for SongbirdNetworkConfigurator {
    fn new() -> Self {
        Self {
            config: Self::default_config(),
        }
    }

    fn default_config() -> SongbirdNetworkConfig {
        defaults::songbird_default_network_config()
    }

    async fn apply_configuration(&self) -> ToadStoolResult<()> {
        info!("🔧 Applying Songbird network configuration");

        // Apply service mesh configuration
        if self.config.service_mesh.enabled {
            self.apply_service_mesh_config().await?;
        }

        // Apply DNS discovery configuration
        if self.config.dns_discovery.enabled {
            self.apply_dns_discovery_config().await?;
        }

        // Apply cross-primal security configuration
        if self.config.cross_primal_security.enabled {
            self.apply_cross_primal_security_config().await?;
        }

        // Apply network policies
        if self.config.network_policies.enabled {
            self.apply_network_policies_config().await?;
        }

        // Apply traffic management configuration
        if self.config.traffic_management.enabled {
            self.apply_traffic_management_config().await?;
        }

        // Apply load balancing configuration
        if self.config.load_balancing.enabled {
            self.apply_load_balancing_config().await?;
        }

        // Apply circuit breaker configuration
        if self.config.circuit_breaker.enabled {
            self.apply_circuit_breaker_config().await?;
        }

        // Apply health monitoring configuration
        if self.config.health_monitoring.enabled {
            self.apply_health_monitoring_config().await?;
        }

        info!("✅ Songbird network configuration applied successfully");
        Ok(())
    }

    fn validate_configuration(&self) -> ToadStoolResult<()> {
        info!("🔍 Validating Songbird network configuration");

        // Validate service mesh configuration
        self.validate_service_mesh_config()?;

        // Validate DNS discovery configuration
        self.validate_dns_discovery_config()?;

        // Validate cross-primal security configuration
        self.validate_cross_primal_security_config()?;

        // Validate network policies configuration
        self.validate_network_policies_config()?;

        // Validate traffic management configuration
        self.validate_traffic_management_config()?;

        // Validate load balancing configuration
        self.validate_load_balancing_config()?;

        // Validate circuit breaker configuration
        self.validate_circuit_breaker_config()?;

        // Validate health monitoring configuration
        self.validate_health_monitoring_config()?;

        info!("✅ Songbird network configuration validation completed");
        Ok(())
    }
}
