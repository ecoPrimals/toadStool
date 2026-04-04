// SPDX-License-Identifier: AGPL-3.0-only
//! BYOB executor configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// BYOB executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByobExecutorConfig {
    /// Maximum concurrent deployments
    pub max_concurrent_deployments: u32,
    /// Default network subnet
    pub default_network_subnet: String,
    /// Resource monitoring interval
    pub resource_monitoring_interval: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Deployment timeout
    pub deployment_timeout: Duration,
    /// Default host port for service mappings
    pub default_host_port: u16,
    /// Common web service ports for external IP allocation
    pub web_service_ports: Vec<u16>,
    /// Graceful shutdown timeout in seconds
    pub graceful_shutdown_timeout_secs: u64,
}

impl Default for ByobExecutorConfig {
    #[allow(deprecated)] // Using deprecated field during migration to capability-based discovery
    fn default() -> Self {
        #[allow(deprecated)]
        let config = toadstool_config::env_config::EnvironmentConfig::from_env();

        #[allow(deprecated)]
        let coordinator_port = config.network.coordination_port;

        Self {
            max_concurrent_deployments: 50,
            default_network_subnet: "10.0.0.0/24".to_string(),
            resource_monitoring_interval: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            deployment_timeout: Duration::from_secs(600), // 10 minutes
            default_host_port: coordinator_port,
            web_service_ports: vec![80, 443, coordinator_port, 8443, 3000, 8000, 9000],
            graceful_shutdown_timeout_secs: 30, // 30 second graceful shutdown
        }
    }
}

impl ByobExecutorConfig {
    /// Create a new configuration with custom values
    pub fn new(
        max_concurrent_deployments: u32,
        default_network_subnet: String,
        default_host_port: u16,
    ) -> Self {
        Self {
            max_concurrent_deployments,
            default_network_subnet,
            default_host_port,
            ..Default::default()
        }
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns an error if `max_concurrent_deployments` is 0 or `deployment_timeout` is less than 60 seconds.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_concurrent_deployments == 0 {
            return Err("max_concurrent_deployments must be greater than 0".to_string());
        }

        if self.deployment_timeout.as_secs() < 60 {
            return Err("deployment_timeout must be at least 60 seconds".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ByobExecutorConfig::default();

        assert_eq!(config.max_concurrent_deployments, 50);
        assert_eq!(config.default_network_subnet, "10.0.0.0/24");
        assert_eq!(config.resource_monitoring_interval, Duration::from_secs(30));
        assert_eq!(config.health_check_interval, Duration::from_secs(10));
        assert_eq!(config.deployment_timeout, Duration::from_secs(600));
        assert!(config.web_service_ports.contains(&80));
        assert!(config.web_service_ports.contains(&443));
    }

    #[test]
    fn test_custom_config() {
        let config = ByobExecutorConfig::new(100, "192.168.1.0/24".to_string(), 9000);

        assert_eq!(config.max_concurrent_deployments, 100);
        assert_eq!(config.default_network_subnet, "192.168.1.0/24");
        assert_eq!(config.default_host_port, 9000);
        // Should inherit defaults for other fields
        assert_eq!(config.resource_monitoring_interval, Duration::from_secs(30));
    }

    #[test]
    fn test_validate_success() {
        let config = ByobExecutorConfig::default();
        assert!(config.validate().is_ok());

        let custom = ByobExecutorConfig::new(10, "10.1.0.0/16".to_string(), 8080);
        assert!(custom.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_concurrent_deployments() {
        let config = ByobExecutorConfig {
            max_concurrent_deployments: 0,
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "max_concurrent_deployments must be greater than 0"
        );
    }

    #[test]
    fn test_validate_zero_host_port_is_valid() {
        let config = ByobExecutorConfig {
            default_host_port: 0,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_short_timeout() {
        let config = ByobExecutorConfig {
            deployment_timeout: Duration::from_secs(30),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "deployment_timeout must be at least 60 seconds"
        );
    }

    #[test]
    fn test_validate_minimum_timeout() {
        let config = ByobExecutorConfig {
            deployment_timeout: Duration::from_secs(60),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_clone() {
        let config = ByobExecutorConfig::default();
        let cloned = config.clone();

        assert_eq!(
            config.max_concurrent_deployments,
            cloned.max_concurrent_deployments
        );
        assert_eq!(config.default_network_subnet, cloned.default_network_subnet);
        assert_eq!(config.default_host_port, cloned.default_host_port);
    }

    #[test]
    fn test_config_serialization() {
        let config = ByobExecutorConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("max_concurrent_deployments"));
        assert!(json.contains("default_network_subnet"));
    }

    #[test]
    fn test_config_deserialization() {
        let json = r#"{
            "max_concurrent_deployments": 25,
            "default_network_subnet": "172.16.0.0/16",
            "resource_monitoring_interval": {"secs": 15, "nanos": 0},
            "health_check_interval": {"secs": 5, "nanos": 0},
            "deployment_timeout": {"secs": 300, "nanos": 0},
            "default_host_port": 7000,
            "web_service_ports": [80, 443, 8080],
            "graceful_shutdown_timeout_secs": 45
        }"#;

        let config: ByobExecutorConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_concurrent_deployments, 25);
        assert_eq!(config.default_network_subnet, "172.16.0.0/16");
        assert_eq!(config.default_host_port, 7000);
        assert_eq!(config.web_service_ports, vec![80, 443, 8080]);
        assert_eq!(config.graceful_shutdown_timeout_secs, 45);
    }

    #[test]
    fn test_web_service_ports_default() {
        let config = ByobExecutorConfig::default();
        assert!(config.web_service_ports.len() >= 5);
        assert!(config.web_service_ports.contains(&80));
        assert!(config.web_service_ports.contains(&443));
    }

    #[test]
    fn test_custom_web_service_ports() {
        let config = ByobExecutorConfig {
            web_service_ports: vec![8080, 8443, 9000],
            ..Default::default()
        };
        assert_eq!(config.web_service_ports.len(), 3);
        assert!(config.web_service_ports.contains(&8080));
    }

    #[test]
    fn test_large_concurrent_deployments() {
        let config = ByobExecutorConfig::new(1000, "10.0.0.0/8".to_string(), 8000);
        assert!(config.validate().is_ok());
        assert_eq!(config.max_concurrent_deployments, 1000);
    }

    #[test]
    fn test_various_network_subnets() {
        let subnets = vec![
            "10.0.0.0/24",
            "192.168.1.0/24",
            "172.16.0.0/16",
            "10.0.0.0/8",
        ];

        for subnet in subnets {
            let config = ByobExecutorConfig::new(10, subnet.to_string(), 8080);
            assert_eq!(config.default_network_subnet, subnet);
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_config_debug_format() {
        let config = ByobExecutorConfig::default();
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("ByobExecutorConfig"));
        assert!(debug_str.contains("max_concurrent_deployments"));
    }
}
