//! Traffic management and load balancing extension
//!
//! Provides traffic management, load balancing, and routing configuration.

use toadstool::error::ToadStoolResult;
use tracing::{debug, info};

/// Traffic management extension trait
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

impl TrafficExt for super::SongbirdNetworkConfigurator {
    async fn apply_traffic_management_config(&self) -> ToadStoolResult<()> {
        info!("🚦 Applying traffic management configuration");

        let config = &self.config.traffic_management;
        debug!("Traffic management enabled: {}", config.enabled);

        // Configuration details...

        Ok(())
    }

    async fn apply_load_balancing_config(&self) -> ToadStoolResult<()> {
        info!("⚖️ Applying load balancing configuration");

        let config = &self.config.load_balancing;
        debug!("Load balancing algorithm: {}", config.algorithm);

        // Configuration details...

        Ok(())
    }

    fn validate_traffic_management_config(&self) -> ToadStoolResult<()> {
        // Validation logic...
        Ok(())
    }

    fn validate_load_balancing_config(&self) -> ToadStoolResult<()> {
        let config = &self.config.load_balancing;

        if config.enabled {
            // Validate load balancing algorithm
            match config.algorithm.as_str() {
                "round_robin" | "least_conn" | "random" | "ip_hash" => {}
                _ => {
                    return Err(toadstool::error::ToadStoolError::configuration(format!(
                        "Invalid load balancing algorithm: {}",
                        config.algorithm
                    )))
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
