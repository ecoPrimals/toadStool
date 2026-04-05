// SPDX-License-Identifier: AGPL-3.0-or-later
//! Daemon configuration
//!
//! Configuration for ToadStool daemon mode including ports, paths, and resource limits.

use crate::{CliContextExt, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// JSON-RPC TCP port (0 = OS-assigned; non-zero enables TCP alongside Unix socket)
    pub port: u16,

    /// Register with biomeOS capability registry
    pub register_with_biomeos: bool,

    /// Unix socket path for IPC (optional)
    pub socket_path: Option<PathBuf>,

    /// Configuration file path (optional)
    pub config_file: Option<PathBuf>,

    /// Maximum concurrent workloads
    pub max_concurrent_workloads: usize,

    /// Default workload timeout
    pub default_workload_timeout: Duration,

    /// Resource monitoring interval
    pub resource_monitor_interval: Duration,

    /// Heartbeat interval (if registered with biomeOS)
    pub heartbeat_interval: Duration,

    /// biomeOS registry socket path
    pub biomeos_socket: Option<PathBuf>,

    /// Health check interval
    pub health_check_interval: Duration,
}

impl DaemonConfig {
    /// Load daemon configuration
    ///
    /// Priority:
    /// 1. Command-line arguments (highest priority)
    /// 2. Config file (if provided)
    /// 3. Environment variables
    /// 4. Defaults (lowest priority)
    pub async fn load(
        port: u16,
        register_with_biomeos: bool,
        socket_path: Option<PathBuf>,
        config_file: Option<PathBuf>,
        max_workloads: usize,
        biomeos_socket: Option<PathBuf>,
    ) -> Result<Self> {
        // If config file provided, load it
        let mut config = if let Some(ref path) = config_file {
            Self::load_from_file(path).await?
        } else {
            Self::default()
        };

        // Override with command-line arguments
        config.port = port;
        config.register_with_biomeos = register_with_biomeos;

        if socket_path.is_some() {
            config.socket_path = socket_path;
        }

        config.max_concurrent_workloads = max_workloads;

        if biomeos_socket.is_some() {
            config.biomeos_socket = biomeos_socket;
        }

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from file
    async fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context(format!("Failed to read config file: {}", path.display()))?;

        let config: Self = toml::from_str(&content)
            .context(format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        if self.port != 0 && self.port < 1024 {
            return Err(crate::CliError::Other(
                "Port must be 0 (OS-assigned) or >= 1024 (non-privileged)".to_string(),
            ));
        }

        if self.max_concurrent_workloads == 0 {
            return Err(crate::CliError::Other(
                "max_concurrent_workloads must be > 0".to_string(),
            ));
        }

        if self.max_concurrent_workloads > 1000 {
            return Err(crate::CliError::Other(
                "max_concurrent_workloads too high (max: 1000)".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            port: toadstool_config::ports::daemon_port(),
            register_with_biomeos: false,
            socket_path: None,
            config_file: None,
            max_concurrent_workloads: 10,
            default_workload_timeout: Duration::from_secs(3600), // 1 hour
            resource_monitor_interval: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(30),
            biomeos_socket: None,
            health_check_interval: Duration::from_secs(10),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_config() {
        let config = DaemonConfig::default();
        assert_eq!(config.port, toadstool_config::ports::daemon_port());
        assert_eq!(config.max_concurrent_workloads, 10);
        assert!(!config.register_with_biomeos);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let mut config = DaemonConfig::default();

        // Valid config (port 0 = OS-assigned is valid)
        assert!(config.validate().is_ok());

        // Invalid port (privileged range, non-zero)
        config.port = 80;
        assert!(config.validate().is_err());

        // Port 0 (OS-assigned) is valid
        config.port = 0;
        assert!(config.validate().is_ok());

        // Valid explicit port
        config.port = 8084;
        assert!(config.validate().is_ok());

        // Invalid max workloads
        config.max_concurrent_workloads = 0;
        assert!(config.validate().is_err());

        config.max_concurrent_workloads = 2000;
        assert!(config.validate().is_err());
    }
}
