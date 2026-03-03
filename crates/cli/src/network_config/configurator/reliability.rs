// SPDX-License-Identifier: AGPL-3.0-or-later
//! Circuit breakers and health monitoring extension
//!
//! Provides circuit breaker and health monitoring configuration.

use toadstool::error::ToadStoolResult;
use tracing::{debug, info};

/// Reliability extension trait
pub(crate) trait ReliabilityExt {
    /// Apply circuit breaker configuration
    async fn apply_circuit_breaker_config(&self) -> ToadStoolResult<()>;

    /// Apply health monitoring configuration
    async fn apply_health_monitoring_config(&self) -> ToadStoolResult<()>;

    /// Validate circuit breaker configuration
    fn validate_circuit_breaker_config(&self) -> ToadStoolResult<()>;

    /// Validate health monitoring configuration
    fn validate_health_monitoring_config(&self) -> ToadStoolResult<()>;
}

impl ReliabilityExt for super::SongbirdNetworkConfigurator {
    async fn apply_circuit_breaker_config(&self) -> ToadStoolResult<()> {
        info!("⚡ Applying circuit breaker configuration");

        let config = &self.config.circuit_breaker;
        debug!("Failure threshold: {}", config.failure_threshold);
        debug!("Circuit breaker timeout: {:?}", config.timeout);

        // Configuration details...

        Ok(())
    }

    async fn apply_health_monitoring_config(&self) -> ToadStoolResult<()> {
        info!("💊 Applying health monitoring configuration");

        let config = &self.config.health_monitoring;
        debug!("Health monitoring interval: {:?}", config.interval);

        // Configuration details...

        Ok(())
    }

    fn validate_circuit_breaker_config(&self) -> ToadStoolResult<()> {
        let config = &self.config.circuit_breaker;

        if config.enabled && config.failure_threshold == 0 {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Circuit breaker failure threshold cannot be 0".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_health_monitoring_config(&self) -> ToadStoolResult<()> {
        let config = &self.config.health_monitoring;

        if config.enabled && config.endpoints.is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "At least one health endpoint must be configured".to_string(),
            ));
        }

        Ok(())
    }
}
