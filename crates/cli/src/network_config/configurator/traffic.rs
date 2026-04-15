// SPDX-License-Identifier: AGPL-3.0-or-later
//! Traffic management and load balancing extension
//!
//! Provides traffic management, load balancing, and routing configuration.

use toadstool::error::ToadStoolResult;
use tracing::{debug, info, trace};

/// Traffic management extension trait
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) trait TrafficExt {
    /// Apply traffic management configuration
    async fn apply_traffic_management_config(&self) -> ToadStoolResult<()>;

    /// Apply load balancing configuration
    async fn apply_load_balancing_config(&self) -> ToadStoolResult<()>;

    /// Validate traffic management configuration
    fn validate_traffic_management_config(&self) -> ToadStoolResult<()>;

    /// Validate load balancing configuration
    fn validate_load_balancing_config(&self) -> ToadStoolResult<()>;
}

impl TrafficExt for super::OrchestrationNetworkConfigurator {
    async fn apply_traffic_management_config(&self) -> ToadStoolResult<()> {
        info!("🚦 Applying traffic management configuration");

        let config = &self.config.traffic_management;
        debug!("Traffic management enabled: {}", config.enabled);
        debug!(
            "configuration stored; runtime application deferred to orchestration layer (traffic management)"
        );

        Ok(())
    }

    async fn apply_load_balancing_config(&self) -> ToadStoolResult<()> {
        info!("⚖️ Applying load balancing configuration");

        let config = &self.config.load_balancing;
        debug!("Load balancing algorithm: {}", config.algorithm);
        debug!(
            "configuration stored; runtime application deferred to orchestration layer (load balancing)"
        );

        Ok(())
    }

    fn validate_traffic_management_config(&self) -> ToadStoolResult<()> {
        trace!(
            "validate_traffic_management_config: structural checks (percentages, rate limits, strategies); no traffic plane probe"
        );
        let config = &self.config.traffic_management;

        if !config.enabled {
            return Ok(());
        }

        if config.canary.percentage > 100 {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Canary percentage cannot exceed 100".to_string(),
            ));
        }

        if config.traffic_mirroring.enabled && config.traffic_mirroring.percentage > 100 {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Traffic mirroring percentage cannot exceed 100".to_string(),
            ));
        }

        if config.traffic_splitting.enabled && config.traffic_splitting.strategy.trim().is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Traffic splitting strategy cannot be empty when traffic splitting is enabled"
                    .to_string(),
            ));
        }

        if config.rate_limiting.enabled {
            if let Some(ref global) = config.rate_limiting.global_limit {
                if global.requests_per_second == 0 || global.burst_size == 0 {
                    return Err(toadstool::error::ToadStoolError::configuration(
                        "Global rate limit requests_per_second and burst_size must be non-zero when set"
                            .to_string(),
                    ));
                }
                if global.window_size.is_zero() {
                    return Err(toadstool::error::ToadStoolError::configuration(
                        "Global rate limit window_size cannot be zero".to_string(),
                    ));
                }
            }
        }

        if config.blue_green.enabled && config.blue_green.switch_strategy.trim().is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Blue-green switch strategy cannot be empty when blue-green is enabled".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_load_balancing_config(&self) -> ToadStoolResult<()> {
        trace!(
            "validate_load_balancing_config: structural checks (algorithm, health thresholds); no backend reachability test"
        );
        let config = &self.config.load_balancing;

        if config.enabled {
            // Validate load balancing algorithm
            match config.algorithm.as_str() {
                "round_robin" | "least_conn" | "random" | "ip_hash" => {}
                _ => {
                    return Err(toadstool::error::ToadStoolError::configuration(format!(
                        "Invalid load balancing algorithm: {}",
                        config.algorithm
                    )));
                }
            }

            // Validate health check configuration
            if config.health_check.base.enabled
                && (config.health_check.base.healthy_threshold == 0
                    || config.health_check.base.unhealthy_threshold == 0)
            {
                return Err(toadstool::error::ToadStoolError::configuration(
                    "Health check thresholds cannot be zero".to_string(),
                ));
            }
        }

        Ok(())
    }
}
