// SPDX-License-Identifier: AGPL-3.0-or-later
//! Circuit breakers and health monitoring extension
//!
//! Provides circuit breaker and health monitoring configuration.

use toadstool::error::ToadStoolResult;
use tracing::{debug, info, trace};

/// Reliability extension trait
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
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

impl ReliabilityExt for super::OrchestrationNetworkConfigurator {
    async fn apply_circuit_breaker_config(&self) -> ToadStoolResult<()> {
        info!("⚡ Applying circuit breaker configuration");

        let config = &self.config.circuit_breaker;
        debug!("Failure threshold: {}", config.failure_threshold);
        debug!("Circuit breaker timeout: {:?}", config.timeout);
        debug!(
            "configuration stored; runtime application deferred to orchestration layer (circuit breaker)"
        );

        Ok(())
    }

    async fn apply_health_monitoring_config(&self) -> ToadStoolResult<()> {
        info!("💊 Applying health monitoring configuration");

        let config = &self.config.health_monitoring;
        debug!("Health monitoring interval: {:?}", config.interval);
        debug!(
            "configuration stored; runtime application deferred to orchestration layer (health monitoring)"
        );

        Ok(())
    }

    fn validate_circuit_breaker_config(&self) -> ToadStoolResult<()> {
        trace!(
            "validate_circuit_breaker_config: structural checks (thresholds, durations); no runtime probe"
        );
        let config = &self.config.circuit_breaker;

        if config.enabled && config.failure_threshold == 0 {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Circuit breaker failure threshold cannot be 0".to_string(),
            ));
        }

        if config.enabled && config.success_threshold == 0 {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Circuit breaker success threshold cannot be 0".to_string(),
            ));
        }

        if config.enabled
            && (config.timeout.is_zero()
                || config.half_open_timeout.is_zero()
                || config.reset_timeout.is_zero())
        {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Circuit breaker timeouts must be non-zero when enabled".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_health_monitoring_config(&self) -> ToadStoolResult<()> {
        trace!(
            "validate_health_monitoring_config: structural checks (interval, endpoints, metrics); no HTTP probe"
        );
        let config = &self.config.health_monitoring;

        if config.enabled && config.endpoints.is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "At least one health endpoint must be configured".to_string(),
            ));
        }

        if config.enabled && config.interval.is_zero() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Health monitoring interval cannot be zero when enabled".to_string(),
            ));
        }

        if config.enabled {
            for ep in &config.endpoints {
                if ep.name.trim().is_empty() {
                    return Err(toadstool::error::ToadStoolError::configuration(
                        "Health endpoint name cannot be empty".to_string(),
                    ));
                }
                if ep.url.trim().is_empty() {
                    return Err(toadstool::error::ToadStoolError::configuration(
                        "Health endpoint URL cannot be empty".to_string(),
                    ));
                }
                if ep.health_check.path.trim().is_empty() {
                    return Err(toadstool::error::ToadStoolError::configuration(format!(
                        "Health check path cannot be empty for endpoint `{}`",
                        ep.name
                    )));
                }
                let status = ep.health_check.expected_status;
                if !(100..=599).contains(&status) {
                    return Err(toadstool::error::ToadStoolError::configuration(format!(
                        "Expected HTTP status for endpoint `{}` must be between 100 and 599",
                        ep.name
                    )));
                }
            }

            if config.metrics.enabled && config.metrics.endpoint.trim().is_empty() {
                return Err(toadstool::error::ToadStoolError::configuration(
                    "Metrics endpoint cannot be empty when metrics collection is enabled"
                        .to_string(),
                ));
            }

            if config.metrics.enabled && config.metrics.interval.is_zero() {
                return Err(toadstool::error::ToadStoolError::configuration(
                    "Metrics collection interval cannot be zero when metrics are enabled"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ReliabilityExt;
    use crate::network_config::{HealthEndpoint, OrchestrationNetworkConfigurator};
    use std::time::Duration;
    use toadstool_common::config_bases::{HealthCheckConfig, HttpHealthCheckConfig};

    #[test]
    fn validate_circuit_breaker_default_succeeds() {
        let c = OrchestrationNetworkConfigurator::new();
        assert!(c.validate_circuit_breaker_config().is_ok());
    }

    #[test]
    fn validate_circuit_breaker_skips_thresholds_when_disabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.circuit_breaker.enabled = false;
        c.config.circuit_breaker.failure_threshold = 0;
        c.config.circuit_breaker.success_threshold = 0;
        assert!(c.validate_circuit_breaker_config().is_ok());
    }

    #[test]
    fn validate_circuit_breaker_rejects_zero_failure_threshold_when_enabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.circuit_breaker.enabled = true;
        c.config.circuit_breaker.failure_threshold = 0;
        assert!(c.validate_circuit_breaker_config().is_err());
    }

    #[test]
    fn validate_circuit_breaker_rejects_zero_success_threshold_when_enabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.circuit_breaker.enabled = true;
        c.config.circuit_breaker.success_threshold = 0;
        assert!(c.validate_circuit_breaker_config().is_err());
    }

    #[test]
    fn validate_circuit_breaker_rejects_zero_timeout_when_enabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.circuit_breaker.enabled = true;
        c.config.circuit_breaker.timeout = Duration::ZERO;
        assert!(c.validate_circuit_breaker_config().is_err());
    }

    #[test]
    fn validate_health_monitoring_default_succeeds() {
        let c = OrchestrationNetworkConfigurator::new();
        assert!(c.validate_health_monitoring_config().is_ok());
    }

    #[test]
    fn validate_health_monitoring_skips_when_disabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.enabled = false;
        c.config.health_monitoring.endpoints.clear();
        c.config.health_monitoring.interval = Duration::ZERO;
        assert!(c.validate_health_monitoring_config().is_ok());
    }

    #[test]
    fn validate_health_monitoring_rejects_no_endpoints_when_enabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.enabled = true;
        c.config.health_monitoring.endpoints.clear();
        assert!(c.validate_health_monitoring_config().is_err());
    }

    #[test]
    fn validate_health_monitoring_rejects_zero_interval_when_enabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.enabled = true;
        c.config.health_monitoring.interval = Duration::ZERO;
        assert!(c.validate_health_monitoring_config().is_err());
    }

    #[test]
    fn validate_health_monitoring_rejects_empty_endpoint_name() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.enabled = true;
        c.config.health_monitoring.endpoints = vec![HealthEndpoint {
            name: "  ".to_string(),
            url: "http://localhost/health".to_string(),
            health_check: HttpHealthCheckConfig::default(),
        }];
        assert!(c.validate_health_monitoring_config().is_err());
    }

    #[test]
    fn validate_health_monitoring_rejects_empty_endpoint_url() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.enabled = true;
        c.config.health_monitoring.endpoints = vec![HealthEndpoint {
            name: "a".to_string(),
            url: "".to_string(),
            health_check: HttpHealthCheckConfig::default(),
        }];
        assert!(c.validate_health_monitoring_config().is_err());
    }

    #[test]
    fn validate_health_monitoring_rejects_empty_health_check_path() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.enabled = true;
        let mut hc = HttpHealthCheckConfig::default();
        hc.path = "".to_string();
        c.config.health_monitoring.endpoints = vec![HealthEndpoint {
            name: "svc".to_string(),
            url: "http://localhost/health".to_string(),
            health_check: hc,
        }];
        assert!(c.validate_health_monitoring_config().is_err());
    }

    #[test]
    fn validate_health_monitoring_rejects_invalid_expected_http_status() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.enabled = true;
        let mut hc = HttpHealthCheckConfig::default();
        hc.expected_status = 99;
        c.config.health_monitoring.endpoints = vec![HealthEndpoint {
            name: "svc".to_string(),
            url: "http://localhost/health".to_string(),
            health_check: hc,
        }];
        assert!(c.validate_health_monitoring_config().is_err());
    }

    #[test]
    fn validate_health_monitoring_rejects_metrics_enabled_with_empty_endpoint() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.metrics.enabled = true;
        c.config.health_monitoring.metrics.endpoint = "  ".to_string();
        assert!(c.validate_health_monitoring_config().is_err());
    }

    #[test]
    fn validate_health_monitoring_rejects_metrics_enabled_with_zero_interval() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.metrics.enabled = true;
        c.config.health_monitoring.metrics.interval = Duration::ZERO;
        assert!(c.validate_health_monitoring_config().is_err());
    }

    #[test]
    fn validate_health_monitoring_accepts_expected_status_in_range() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.health_monitoring.enabled = true;
        let mut hc = HttpHealthCheckConfig {
            base: HealthCheckConfig {
                enabled: true,
                interval: Duration::from_secs(30),
                timeout: Duration::from_secs(5),
                healthy_threshold: 2,
                unhealthy_threshold: 3,
                retry_count: 3,
            },
            path: "/health".to_string(),
            expected_status: 599,
            method: "GET".to_string(),
        };
        c.config.health_monitoring.endpoints = vec![HealthEndpoint {
            name: "edge".to_string(),
            url: "http://localhost/health".to_string(),
            health_check: hc.clone(),
        }];
        assert!(c.validate_health_monitoring_config().is_ok());

        hc.expected_status = 100;
        c.config.health_monitoring.endpoints = vec![HealthEndpoint {
            name: "edge".to_string(),
            url: "http://localhost/health".to_string(),
            health_check: hc,
        }];
        assert!(c.validate_health_monitoring_config().is_ok());
    }
}
