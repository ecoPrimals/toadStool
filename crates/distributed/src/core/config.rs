// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};
use toadstool::{IsolationLevel, RuntimeType};

/// Configuration for `ToadStool` distributed integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// Instance identifier
    pub instance_id: String,
    /// Standalone execution configuration
    pub standalone: StandaloneConfig,
    /// Songbird integration (optional for standalone operation)
    pub songbird_integration: Option<SongbirdConfig>,
}

/// Standalone execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandaloneConfig {
    /// Maximum concurrent executions
    pub max_concurrent_executions: u32,
    /// Default execution timeout
    pub default_timeout_secs: u64,
    /// Enable local job queue
    pub enable_job_queue: bool,
    /// Job queue size
    pub max_queue_size: usize,
}

/// Songbird integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdConfig {
    /// Songbird endpoint
    pub endpoint: String,
    /// Authentication token
    pub auth_token: Option<String>,
    /// Health reporting interval in seconds
    pub health_reporting_interval_secs: u64,
}

/// `ToadStool` capabilities reported to Songbird
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToadStoolCapabilities {
    /// Available execution environments
    pub execution_environments: Vec<ExecutionEnvironment>,
    /// Supported runtime technologies
    pub supported_runtimes: Vec<RuntimeType>,
    /// Platform-specific capabilities
    pub platform_capabilities: PlatformCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEnvironment {
    Container { runtime: String },
    Wasm { runtime: String },
    Native { isolation: IsolationLevel },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    /// Operating system
    pub os: String,
    /// CPU architecture
    pub architecture: String,
    /// Available CPU cores
    pub cpu_cores: u32,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            instance_id: uuid::Uuid::new_v4().to_string(),
            standalone: StandaloneConfig {
                max_concurrent_executions: 10,
                default_timeout_secs: 3600,
                enable_job_queue: true,
                max_queue_size: 1000,
            },
            songbird_integration: None,
        }
    }
}

impl ToadStoolCapabilities {
    pub async fn detect_current() -> toadstool::ToadStoolResult<Self> {
        use toadstool::RuntimeType;

        let execution_environments = vec![
            ExecutionEnvironment::Native {
                isolation: toadstool::IsolationLevel::Standard,
            },
            ExecutionEnvironment::Container {
                runtime: "docker".to_string(),
            },
            ExecutionEnvironment::Wasm {
                runtime: "wasmtime".to_string(),
            },
        ];

        let supported_runtimes = vec![
            RuntimeType::Native,
            RuntimeType::Container,
            RuntimeType::Wasm,
        ];

        let platform_capabilities = PlatformCapabilities {
            os: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            cpu_cores: std::thread::available_parallelism()
                .map(|p| u32::try_from(p.get()).unwrap_or(4))
                .unwrap_or(4),
        };

        Ok(Self {
            execution_environments,
            supported_runtimes,
            platform_capabilities,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distributed_config_default_values() {
        let config = DistributedConfig::default();
        assert!(!config.instance_id.is_empty());
        assert_eq!(config.standalone.max_concurrent_executions, 10);
        assert_eq!(config.standalone.default_timeout_secs, 3600);
        assert!(config.standalone.enable_job_queue);
        assert_eq!(config.standalone.max_queue_size, 1000);
        assert!(config.songbird_integration.is_none());
    }

    #[test]
    fn standalone_config_fields() {
        let standalone = StandaloneConfig {
            max_concurrent_executions: 5,
            default_timeout_secs: 120,
            enable_job_queue: false,
            max_queue_size: 100,
        };
        assert_eq!(standalone.max_concurrent_executions, 5);
        assert_eq!(standalone.default_timeout_secs, 120);
        assert!(!standalone.enable_job_queue);
        assert_eq!(standalone.max_queue_size, 100);
    }

    #[test]
    fn songbird_config_fields() {
        let songbird = SongbirdConfig {
            endpoint: "http://localhost:8080".to_string(),
            auth_token: Some("secret".to_string()),
            health_reporting_interval_secs: 30,
        };
        assert_eq!(songbird.endpoint, "http://localhost:8080");
        assert_eq!(songbird.auth_token.as_deref(), Some("secret"));
        assert_eq!(songbird.health_reporting_interval_secs, 30);
    }

    #[test]
    fn execution_environment_variants() {
        let _container = ExecutionEnvironment::Container {
            runtime: "docker".to_string(),
        };
        let _wasm = ExecutionEnvironment::Wasm {
            runtime: "wasmtime".to_string(),
        };
        let _native = ExecutionEnvironment::Native {
            isolation: IsolationLevel::Standard,
        };
    }

    #[test]
    fn platform_capabilities_fields() {
        let caps = PlatformCapabilities {
            os: "linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: 8,
        };
        assert_eq!(caps.os, "linux");
        assert_eq!(caps.architecture, "x86_64");
        assert_eq!(caps.cpu_cores, 8);
    }

    #[tokio::test]
    async fn toadstool_capabilities_detect_current() {
        let caps = ToadStoolCapabilities::detect_current().await.unwrap();
        assert!(!caps.execution_environments.is_empty());
        assert!(!caps.supported_runtimes.is_empty());
        assert!(!caps.platform_capabilities.os.is_empty());
        assert!(!caps.platform_capabilities.architecture.is_empty());
        assert!(caps.platform_capabilities.cpu_cores > 0);
    }
}
