// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#![allow(clippy::match_same_arms)]
// Platform-specific implementations
#[cfg(target_os = "linux")]
#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use toadstool::IsolationLevel;
    use toadstool_security_policies::FilePolicyManager;
    use toadstool_security_sandbox::{helpers, *};

    fn create_test_config() -> SandboxConfig {
        let temp_dir = TempDir::new().unwrap();
        SandboxConfig {
            sandbox_root: temp_dir.path().join("sandbox"),
            temp_dir: temp_dir.path().join("temp"),
            max_concurrent_sandboxes: 10,
            cleanup_timeout_secs: 5,
            monitoring_interval_ms: 100,
            ..Default::default()
        }
    }

    fn create_test_policy_manager() -> Arc<FilePolicyManager> {
        let temp_dir = TempDir::new().unwrap();
        let policy_config = toadstool_security_policies::PolicyManagerConfig {
            policy_dir: temp_dir.path().join("policies"),
            cache_enabled: false,
            strict_enforcement: false,
            ..Default::default()
        };
        Arc::new(FilePolicyManager::new(policy_config).unwrap())
    }

    fn create_test_sandbox_spec() -> SandboxSpec {
        SandboxSpec {
            sandbox_id: "test-sandbox".to_string(),
            workload: toadstool::WorkloadSpec::Native {
                executable: toadstool::workload::ExecutableSource::File {
                    path: PathBuf::from("/bin/echo"),
                },
                args: Some(vec!["test".to_string()]),
                working_dir: None,
                env_vars: std::collections::HashMap::new(),
                user: None,
            },
            security_context: toadstool::SecurityContext::default(),
            resource_limits: ResourceLimits::default(),
            filesystem_mounts: Vec::new(),
            network_config: NetworkConfig::default(),
            environment: std::collections::HashMap::new(),
            working_directory: None,
            lifetime: SandboxLifetime::Ephemeral,
        }
    }

    fn create_test_policy() -> toadstool_security_policies::SecurityPolicy {
        toadstool_security_policies::SecurityPolicy {
            id: "test-policy".to_string(),
            name: "Test Sandbox Policy".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test policy for sandbox validation".to_string()),
            author: Some("Test Author".to_string()),
            created_at: std::time::SystemTime::now(),
            modified_at: std::time::SystemTime::now(),
            rules: vec![toadstool_security_policies::PolicyRule {
                id: "rule-1".to_string(),
                name: "Allow Native Workloads".to_string(),
                condition: toadstool_security_policies::PolicyCondition::Always,
                action: toadstool_security_policies::PolicyAction::Allow,
                priority: 100,
                enabled: true,
                description: Some("Allow all workloads for testing".to_string()),
            }],
            inherits: Vec::new(),
            metadata: std::collections::HashMap::new(),
            signature: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_manager_creation() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager).await;
        assert!(manager.is_ok());

        let _manager = manager.unwrap();
        // Config validation happens internally during creation
        // Successful creation implies config is properly initialized
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_spec_validation() {
        // Test valid spec
        let spec = create_test_sandbox_spec();
        let result = helpers::validate_sandbox_spec(&spec).await;
        assert!(result.is_ok());

        // Test invalid spec - zero memory limit
        let mut invalid_spec = create_test_sandbox_spec();
        invalid_spec.resource_limits.max_memory_bytes = Some(0);
        let result = helpers::validate_sandbox_spec(&invalid_spec).await;
        assert!(result.is_err());

        // Test invalid spec - invalid CPU limit
        let mut invalid_spec = create_test_sandbox_spec();
        invalid_spec.resource_limits.max_cpu_percent = Some(150.0);
        let result = helpers::validate_sandbox_spec(&invalid_spec).await;
        assert!(result.is_err());

        // Test invalid spec - negative CPU limit
        let mut invalid_spec = create_test_sandbox_spec();
        invalid_spec.resource_limits.max_cpu_percent = Some(-10.0);
        let result = helpers::validate_sandbox_spec(&invalid_spec).await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_directory_creation() {
        let config = create_test_config();
        let sandbox_id = "test-sandbox";

        let result = helpers::create_sandbox_directories(&config.sandbox_root, sandbox_id).await;
        assert!(result.is_ok());

        let sandbox_dir = result.unwrap();
        assert!(sandbox_dir.exists());
        assert!(sandbox_dir.join("bin").exists());
        assert!(sandbox_dir.join("etc").exists());
        assert!(sandbox_dir.join("tmp").exists());
        assert!(sandbox_dir.join("proc").exists());
        assert!(sandbox_dir.join("sys").exists());
        assert!(sandbox_dir.join("dev").exists());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_creation_and_destruction() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager)
            .await
            .unwrap();
        let spec = create_test_sandbox_spec();

        // Create sandbox
        let sandbox_id = manager.create_sandbox(spec).await.unwrap();
        assert!(!sandbox_id.is_empty());

        // Verify sandbox exists
        let sandbox_info = manager.get_sandbox_info(&sandbox_id).await.unwrap();
        assert_eq!(sandbox_info.sandbox_id, sandbox_id);
        assert_eq!(sandbox_info.status, SandboxStatus::Ready);

        // Destroy sandbox
        let result = manager.destroy_sandbox(&sandbox_id).await;
        assert!(result.is_ok());

        // Verify sandbox is removed
        let result = manager.get_sandbox_info(&sandbox_id).await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_execution_lifecycle() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager)
            .await
            .unwrap();
        let spec = create_test_sandbox_spec();

        // Create sandbox
        let sandbox_id = manager.create_sandbox(spec).await.unwrap();

        // Start execution
        let start_result = manager.start_execution(&sandbox_id).await;
        assert!(start_result.is_ok());

        // Check running status
        let sandbox_info = manager.get_sandbox_info(&sandbox_id).await.unwrap();
        assert_eq!(sandbox_info.status, SandboxStatus::Running);

        // Stop execution
        let stop_result = manager.stop_execution(&sandbox_id).await;
        assert!(stop_result.is_ok());

        // Check completed status
        let sandbox_info = manager.get_sandbox_info(&sandbox_id).await.unwrap();
        assert_eq!(sandbox_info.status, SandboxStatus::Completed);

        // Cleanup
        let _ = manager.destroy_sandbox(&sandbox_id).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_resource_limits() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager)
            .await
            .unwrap();

        // Test custom resource limits
        let mut spec = create_test_sandbox_spec();
        spec.resource_limits = ResourceLimits {
            max_memory_bytes: Some(1024 * 1024 * 1024), // 1GB
            max_cpu_percent: Some(50.0),
            max_file_descriptors: Some(512),
            max_processes: Some(50),
            max_disk_bytes: Some(500 * 1024 * 1024), // 500MB
            max_network_bps: Some(5 * 1024 * 1024),  // 5MB/s
            max_execution_time: Some(Duration::from_secs(60)),
        };

        let sandbox_id = manager.create_sandbox(spec).await.unwrap();
        let sandbox_info = manager.get_sandbox_info(&sandbox_id).await.unwrap();

        assert_eq!(sandbox_info.sandbox_id, sandbox_id);
        assert_eq!(sandbox_info.status, SandboxStatus::Ready);

        // Test resource monitoring
        let resource_usage = manager.monitor_sandbox(&sandbox_id).await.unwrap();
        assert_eq!(resource_usage.memory_bytes, 0); // Not running yet
        assert_eq!(resource_usage.cpu_percent, 0.0);

        // Cleanup
        let _ = manager.destroy_sandbox(&sandbox_id).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_filesystem_mounts() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager)
            .await
            .unwrap();

        // Create temporary directories for testing
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();

        let mut spec = create_test_sandbox_spec();
        spec.filesystem_mounts = vec![
            FilesystemMount {
                source: source_dir.clone(),
                target: temp_dir.path().join("mnt_test"),
                mount_type: MountType::ReadOnlyBind,
                options: vec!["ro".to_string()],
            },
            FilesystemMount {
                source: temp_dir.path().join("tmp_source"),
                target: temp_dir.path().join("tmp_target"),
                mount_type: MountType::TmpFs,
                options: vec!["size=100M".to_string()],
            },
        ];

        // Create the source directories so they exist for the test
        std::fs::create_dir_all(temp_dir.path().join("tmp_source")).unwrap();

        let sandbox_id = manager.create_sandbox(spec).await.unwrap();
        let sandbox_info = manager.get_sandbox_info(&sandbox_id).await.unwrap();

        assert_eq!(sandbox_info.sandbox_id, sandbox_id);
        assert_eq!(sandbox_info.status, SandboxStatus::Ready);

        // Cleanup
        let _ = manager.destroy_sandbox(&sandbox_id).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_network_configuration() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager)
            .await
            .unwrap();

        let mut spec = create_test_sandbox_spec();
        spec.network_config = NetworkConfig {
            enabled: true,
            isolation_mode: NetworkIsolationMode::Firewall,
            allowed_hosts: vec!["example.com".to_string(), "api.github.com".to_string()],
            allowed_ports: vec![80, 443, 8080],
            dns_servers: vec!["8.8.8.8".to_string(), "1.1.1.1".to_string()],
            bandwidth_limits: Some(BandwidthLimits {
                upload_bps: 1024 * 1024,       // 1MB/s
                download_bps: 5 * 1024 * 1024, // 5MB/s
            }),
        };

        let sandbox_id = manager.create_sandbox(spec).await.unwrap();
        let sandbox_info = manager.get_sandbox_info(&sandbox_id).await.unwrap();

        assert_eq!(sandbox_info.sandbox_id, sandbox_id);
        assert_eq!(sandbox_info.status, SandboxStatus::Ready);

        // Cleanup
        let _ = manager.destroy_sandbox(&sandbox_id).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_security_policy_application() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager)
            .await
            .unwrap();
        let spec = create_test_sandbox_spec();

        // Create sandbox
        let sandbox_id = manager.create_sandbox(spec).await.unwrap();

        // Create and apply security policy
        let policy = create_test_policy();
        let result = manager.apply_security_policy(&sandbox_id, &policy).await;
        assert!(result.is_ok());

        // Verify sandbox still exists and is ready
        let sandbox_info = manager.get_sandbox_info(&sandbox_id).await.unwrap();
        assert_eq!(sandbox_info.status, SandboxStatus::Ready);

        // Cleanup
        let _ = manager.destroy_sandbox(&sandbox_id).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_monitoring() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager)
            .await
            .unwrap();
        let spec = create_test_sandbox_spec();

        // Create sandbox
        let sandbox_id = manager.create_sandbox(spec).await.unwrap();

        // Start execution
        let _ = manager.start_execution(&sandbox_id).await;

        // Monitor sandbox
        let resource_usage = manager.monitor_sandbox(&sandbox_id).await.unwrap();
        assert_eq!(resource_usage.memory_bytes, 0); // Mock implementation returns 0
        assert_eq!(resource_usage.cpu_percent, 0.0);
        assert_eq!(resource_usage.file_descriptors, 0);
        assert_eq!(resource_usage.processes, 0);

        // Stop execution and cleanup
        let _ = manager.stop_execution(&sandbox_id).await;
        let _ = manager.destroy_sandbox(&sandbox_id).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_logs() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager)
            .await
            .unwrap();
        let spec = create_test_sandbox_spec();

        // Create sandbox
        let sandbox_id = manager.create_sandbox(spec).await.unwrap();

        // Get logs
        let logs = manager.get_sandbox_logs(&sandbox_id).await.unwrap();
        assert!(!logs.is_empty());

        // Cleanup
        let _ = manager.destroy_sandbox(&sandbox_id).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_listing() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager)
            .await
            .unwrap();

        // Initially no sandboxes
        let sandboxes = manager.list_sandboxes().await.unwrap();
        assert_eq!(sandboxes.len(), 0);

        // Create multiple sandboxes
        let mut sandbox_ids = Vec::new();
        for i in 0..3 {
            let mut spec = create_test_sandbox_spec();
            spec.sandbox_id = format!("test-sandbox-{i}");
            let sandbox_id = manager.create_sandbox(spec).await.unwrap();
            sandbox_ids.push(sandbox_id);
        }

        // List sandboxes
        let sandboxes = manager.list_sandboxes().await.unwrap();
        assert_eq!(sandboxes.len(), 3);

        // Cleanup
        for sandbox_id in sandbox_ids {
            let _ = manager.destroy_sandbox(&sandbox_id).await;
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_config_defaults() {
        let config = SandboxConfig::default();

        assert!(config.advanced_features_enabled);
        assert_eq!(config.default_isolation_level, IsolationLevel::Standard);
        assert_eq!(config.max_concurrent_sandboxes, 100);
        assert_eq!(config.cleanup_timeout_secs, 30);
        assert!(config.enable_monitoring);
        assert_eq!(config.monitoring_interval_ms, 1000);

        // Platform-specific defaults
        #[cfg(target_os = "linux")]
        {
            assert!(config.enable_seccomp);
            assert!(config.enable_namespace_isolation);
        }

        #[cfg(not(target_os = "linux"))]
        {
            assert!(!config.enable_seccomp);
            assert!(!config.enable_namespace_isolation);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_resource_limits_defaults() {
        let limits = ResourceLimits::default();

        assert_eq!(limits.max_memory_bytes, Some(512 * 1024 * 1024)); // 512MB
        assert_eq!(limits.max_cpu_percent, Some(80.0));
        assert_eq!(limits.max_file_descriptors, Some(1024));
        assert_eq!(limits.max_processes, Some(100));
        assert_eq!(limits.max_disk_bytes, Some(1024 * 1024 * 1024)); // 1GB
        assert_eq!(limits.max_network_bps, Some(10 * 1024 * 1024)); // 10MB/s
        assert_eq!(limits.max_execution_time, Some(Duration::from_secs(300))); // 5 minutes
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_network_config_defaults() {
        let config = NetworkConfig::default();

        assert!(!config.enabled);
        assert!(matches!(
            config.isolation_mode,
            NetworkIsolationMode::Firewall
        ));
        assert!(config.allowed_hosts.is_empty());
        assert!(config.allowed_ports.is_empty());
        // Capability-based DNS: defaults to empty (host-inherited at runtime)
        assert!(config.dns_servers.is_empty());
        assert!(config.bandwidth_limits.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_mount_type_variants() {
        let mount_types = vec![
            MountType::ReadOnlyBind,
            MountType::ReadWriteBind,
            MountType::TmpFs,
            MountType::Device,
            MountType::Proc,
            MountType::Sys,
        ];

        for mount_type in mount_types {
            // Test serialization
            let json = serde_json::to_string(&mount_type).unwrap();
            assert!(!json.is_empty());

            // Test deserialization
            let deserialized: MountType = serde_json::from_str(&json).unwrap();
            match (&mount_type, &deserialized) {
                (MountType::ReadOnlyBind, MountType::ReadOnlyBind) => {}
                (MountType::ReadWriteBind, MountType::ReadWriteBind) => {}
                (MountType::TmpFs, MountType::TmpFs) => {}
                (MountType::Device, MountType::Device) => {}
                (MountType::Proc, MountType::Proc) => {}
                (MountType::Sys, MountType::Sys) => {}
                _ => panic!(
                    "Serialization/deserialization mismatch for MountType: {mount_type:?} != {deserialized:?}"
                ),
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_status_transitions() {
        let statuses = vec![
            SandboxStatus::Creating,
            SandboxStatus::Ready,
            SandboxStatus::Running,
            SandboxStatus::Completed,
            SandboxStatus::Failed,
            SandboxStatus::Destroying,
            SandboxStatus::Destroyed,
        ];

        for status in statuses {
            // Test serialization
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());

            // Test deserialization
            let deserialized: SandboxStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sandbox_lifetime_variants() {
        let lifetimes = vec![
            SandboxLifetime::Ephemeral,
            SandboxLifetime::Persistent {
                ttl: Duration::from_secs(3600),
            },
            SandboxLifetime::Manual,
        ];

        for lifetime in lifetimes {
            // Test serialization
            let json = serde_json::to_string(&lifetime).unwrap();
            assert!(!json.is_empty());

            // Test deserialization
            let deserialized: SandboxLifetime = serde_json::from_str(&json).unwrap();
            match (&lifetime, &deserialized) {
                (SandboxLifetime::Ephemeral, SandboxLifetime::Ephemeral) => {}
                (
                    SandboxLifetime::Persistent { ttl: ttl1 },
                    SandboxLifetime::Persistent { ttl: ttl2 },
                ) => {
                    assert_eq!(ttl1, ttl2);
                }
                (SandboxLifetime::Manual, SandboxLifetime::Manual) => {}
                _ => panic!(
                    "Serialization/deserialization mismatch for SandboxLifetime: {lifetime:?} != {deserialized:?}"
                ),
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_violation_severity_levels() {
        let severities = vec![
            ViolationSeverity::Low,
            ViolationSeverity::Medium,
            ViolationSeverity::High,
            ViolationSeverity::Critical,
        ];

        for severity in severities {
            // Test serialization
            let json = serde_json::to_string(&severity).unwrap();
            assert!(!json.is_empty());

            // Test deserialization
            let deserialized: ViolationSeverity = serde_json::from_str(&json).unwrap();
            match (&severity, &deserialized) {
                (ViolationSeverity::Low, ViolationSeverity::Low) => {}
                (ViolationSeverity::Medium, ViolationSeverity::Medium) => {}
                (ViolationSeverity::High, ViolationSeverity::High) => {}
                (ViolationSeverity::Critical, ViolationSeverity::Critical) => {}
                _ => panic!(
                    "Serialization/deserialization mismatch for ViolationSeverity: {severity:?} != {deserialized:?}"
                ),
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_security_violation_tracking() {
        let violation = SecurityViolation {
            violation_type: "capability_violation".to_string(),
            description: "Attempt to access restricted capability".to_string(),
            timestamp: std::time::SystemTime::now(),
            severity: ViolationSeverity::High,
            action_taken: "Process terminated".to_string(),
        };

        // Test structure
        assert_eq!(violation.violation_type, "capability_violation");
        assert_eq!(
            violation.description,
            "Attempt to access restricted capability"
        );
        assert!(matches!(violation.severity, ViolationSeverity::High));
        assert_eq!(violation.action_taken, "Process terminated");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_resource_usage_tracking() {
        let mut usage = ResourceUsage::default();

        // Test default values
        assert_eq!(usage.memory_bytes, 0);
        assert_eq!(usage.cpu_percent, 0.0);
        assert_eq!(usage.file_descriptors, 0);
        assert_eq!(usage.processes, 0);
        assert_eq!(usage.disk_bytes, 0);
        assert_eq!(usage.network_bytes_sent, 0);
        assert_eq!(usage.network_bytes_received, 0);
        assert_eq!(usage.execution_time, Duration::from_secs(0));

        // Test value updates
        usage.memory_bytes = 1024 * 1024; // 1MB
        usage.cpu_percent = 25.5;
        usage.file_descriptors = 50;
        usage.processes = 5;
        usage.disk_bytes = 2 * 1024 * 1024; // 2MB
        usage.network_bytes_sent = 1024;
        usage.network_bytes_received = 2048;
        usage.execution_time = Duration::from_secs(30);

        assert_eq!(usage.memory_bytes, 1024 * 1024);
        assert_eq!(usage.cpu_percent, 25.5);
        assert_eq!(usage.file_descriptors, 50);
        assert_eq!(usage.processes, 5);
        assert_eq!(usage.disk_bytes, 2 * 1024 * 1024);
        assert_eq!(usage.network_bytes_sent, 1024);
        assert_eq!(usage.network_bytes_received, 2048);
        assert_eq!(usage.execution_time, Duration::from_secs(30));
    }
}
