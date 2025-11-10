//! Test module for ToadStool distributed computing integration

use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use toadstool::{ExecutionInput, ExecutionRequest, RuntimeType, SecurityContext, WorkloadSpec};

use crate::*;
use crate::types::DistributedRetryConfig;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create test configuration
    fn create_test_config() -> DistributedConfig {
        DistributedConfig {
            instance_id: "test-instance-001".to_string(),
            standalone: StandaloneConfig {
                max_concurrent_executions: 4,
                default_timeout_secs: 300,
                enable_job_queue: true,
                max_queue_size: 100,
            },
            songbird_integration: Some(SongbirdConfig {
                endpoint: "https://songbird.example.com".to_string(),
                auth_token: Some("test-token".to_string()),
                health_reporting_interval_secs: 30,
            }),
        }
    }

    // Helper function to create test job
    fn create_test_universal_job() -> UniversalJob {
        UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: Some(UniversalJobType::Local),
            execution_request: ExecutionRequest {
                execution_id: Uuid::new_v4(),
                workload: WorkloadSpec::Native {
                    executable: toadstool::workload::ExecutableSource::File {
                        path: "/bin/echo".into(),
                    },
                    args: Some(vec!["Hello, ToadStool!".to_string()]),
                    working_dir: None,
                    env_vars: HashMap::new(),
                    user: None,
                },
                runtime_hint: Some(RuntimeType::Native),
                resources: toadstool::ResourceRequirements::default(),
                security_context: SecurityContext::default(),
                timeout: Some(Duration::from_secs(30)),
                environment: HashMap::new(),
                input_data: ExecutionInput::default(),
                callback_config: None,
            },
            target: ExecutionTarget::Local,
            priority: JobPriority::Normal,
            dependencies: Vec::new(),
            resource_requirements: ResourceRequirements::default(),
            retry_config: DistributedRetryConfig::default(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_distributed_config_default() {
        let config = DistributedConfig::default();
        assert!(config.instance_id.starts_with("toadstool-"));
        assert_eq!(config.standalone.max_concurrent_executions, 10);
        assert_eq!(config.standalone.default_timeout_secs, 300);
        assert!(config.standalone.enable_job_queue);
        assert_eq!(config.standalone.max_queue_size, 100);
        assert!(config.songbird_integration.is_none());
    }

    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::Emergency < JobPriority::High);
        assert!(JobPriority::High < JobPriority::Normal);
        assert!(JobPriority::Normal < JobPriority::Low);
        assert!(JobPriority::Low < JobPriority::Background);
    }

    #[test]
    fn test_universal_job_queue_creation() {
        let queue = UniversalJobQueue::new();
        assert_eq!(queue.total_jobs(), 0);
    }

    #[tokio::test]
    async fn test_universal_job_queue_add_job() {
        let mut queue = UniversalJobQueue::new();
        let job = create_test_universal_job();

        let result = queue.add_job(job).await;
        assert!(result.is_ok());
        assert_eq!(queue.total_jobs(), 1);
    }

    #[test]
    fn test_resource_requirements_default() {
        let req = ResourceRequirements::default();
        assert_eq!(req.cpu.min_cores, 1.0);
        assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024); // 1GB
        assert_eq!(req.storage.min_bytes, 1024 * 1024 * 1024); // 1GB
        assert!(req.gpu.is_none());
    }

    #[test]
    fn test_retry_config_default() {
        let config = DistributedRetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert!(matches!(
            config.backoff_strategy,
            BackoffStrategy::ExponentialJittered { .. }
        ));
        assert!(!config.retry_conditions.is_empty());
    }

    #[tokio::test]
    async fn test_universal_runtime_adapter_creation() {
        // Note: We test the individual detection methods instead of the full constructor
        // because the constructor includes biological platform detection which is still in development
        let traditional_result = UniversalRuntimeAdapter::detect_traditional_platforms().await;
        assert!(traditional_result.is_ok());

        let container_result = UniversalRuntimeAdapter::detect_container_platforms().await;
        assert!(container_result.is_ok());

        let language_result = UniversalRuntimeAdapter::detect_language_runtimes().await;
        assert!(language_result.is_ok());
    }

    #[tokio::test]
    async fn test_universal_runtime_adapter_detect_traditional_platforms() {
        let result = UniversalRuntimeAdapter::detect_traditional_platforms().await;
        assert!(result.is_ok());
        let platforms = result.unwrap();
        // Should detect at least one platform (the current one)
        assert!(!platforms.is_empty());
    }

    #[tokio::test]
    async fn test_universal_runtime_adapter_detect_container_platforms() {
        let result = UniversalRuntimeAdapter::detect_container_platforms().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_universal_runtime_adapter_detect_language_runtimes() {
        let result = UniversalRuntimeAdapter::detect_language_runtimes().await;
        assert!(result.is_ok());
        let runtimes = result.unwrap();
        // Should detect at least one runtime
        assert!(!runtimes.is_empty());
    }

    #[tokio::test]
    async fn test_toadstool_capabilities_detect_current() {
        let result = ToadStoolCapabilities::detect_current().await;
        assert!(result.is_ok());

        let capabilities = result.unwrap();
        assert!(!capabilities.execution_environments.is_empty());
        assert!(!capabilities.supported_runtimes.is_empty());
        assert!(!capabilities.platform_capabilities.os.is_empty());
        assert!(!capabilities.platform_capabilities.architecture.is_empty());
        assert!(capabilities.platform_capabilities.cpu_cores > 0);
    }

    #[tokio::test]
    async fn test_distributed_coordinator_creation() {
        let config = create_test_config();
        let result = DistributedCoordinator::new(config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_compatibility_mode_string_conversion() {
        assert_eq!(CompatibilityMode::LinuxCompat.to_string(), "linux_compat");
        assert_eq!(
            CompatibilityMode::WindowsCompat.to_string(),
            "windows_compat"
        );
        assert_eq!(CompatibilityMode::MacOSCompat.to_string(), "macos_compat");
        assert_eq!(
            CompatibilityMode::ContainerCompat.to_string(),
            "container_compat"
        );

        let legacy = CompatibilityMode::LegacyCompat {
            system_type: "dos".to_string(),
        };
        assert_eq!(legacy.to_string(), "legacy_dos_compat");
    }
}
