//! Cross-platform sandbox manager implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, info};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_security_policies::{PolicyManager, SecurityPolicy};

use super::helpers;
use super::traits::*;
use super::types::*;

#[cfg(target_os = "linux")]
use super::linux::LinuxSandboxManager;
#[cfg(target_os = "macos")]
use super::macos::MacOSSandboxManager;
#[cfg(target_os = "windows")]
use super::windows::WindowsSandboxManager;

pub struct CrossPlatformSandboxManager {
    config: SandboxConfig,
    sandboxes: Arc<RwLock<HashMap<String, SandboxInfo>>>,
    _policy_manager: Arc<dyn PolicyManager>,

    #[cfg(target_os = "linux")]
    linux_manager: LinuxSandboxManager,

    #[cfg(target_os = "macos")]
    macos_manager: MacOSSandboxManager,

    #[cfg(windows)]
    windows_manager: WindowsSandboxManager,
}

impl CrossPlatformSandboxManager {
    /// Create new cross-platform sandbox manager
    pub async fn new(
        config: SandboxConfig,
        policy_manager: Arc<dyn PolicyManager>,
    ) -> ToadStoolResult<Self> {
        info!("Creating cross-platform sandbox manager");

        // Ensure sandbox directories exist
        tokio::fs::create_dir_all(&config.sandbox_root)
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to create sandbox root directory {}: {}",
                    config.sandbox_root.display(),
                    e
                ))
            })?;

        tokio::fs::create_dir_all(&config.temp_dir)
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to create temp directory {}: {}",
                    config.temp_dir.display(),
                    e
                ))
            })?;

        Ok(Self {
            sandboxes: Arc::new(RwLock::new(HashMap::new())),

            #[cfg(target_os = "linux")]
            linux_manager: LinuxSandboxManager::new(config.clone()),

            #[cfg(target_os = "macos")]
            macos_manager: MacOSSandboxManager::new(config.clone()).await?,

            #[cfg(windows)]
            windows_manager: WindowsSandboxManager::new(config.clone()).await?,

            config,
            _policy_manager: policy_manager,
        })
    }

    /// Setup filesystem mounts for sandbox
    async fn setup_filesystem_mounts(
        &self,
        sandbox_dir: &Path,
        mounts: &[FilesystemMount],
    ) -> ToadStoolResult<()> {
        for mount in mounts {
            let target_path = sandbox_dir.join(&mount.target);

            // Ensure target directory exists
            if let Some(parent) = target_path.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    ToadStoolError::configuration(format!(
                        "Failed to create mount target directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }

            // Platform-specific mount implementation
            #[cfg(target_os = "linux")]
            self.linux_manager.setup_mount(mount, &target_path).await?;

            #[cfg(target_os = "macos")]
            self.macos_manager.setup_mount(mount, &target_path).await?;

            #[cfg(windows)]
            self.windows_manager
                .setup_mount(mount, &target_path)
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl SandboxManager for CrossPlatformSandboxManager {
    async fn create_sandbox(&self, mut spec: SandboxSpec) -> ToadStoolResult<String> {
        info!("Creating sandbox for workload: {:?}", spec.workload);

        // Generate sandbox ID if not provided
        if spec.sandbox_id.is_empty() {
            spec.sandbox_id = helpers::generate_sandbox_id();
        }

        let sandbox_id = spec.sandbox_id.clone();

        // Validate specification
        helpers::validate_sandbox_spec(&spec).await?;

        // Create sandbox directories
        let sandbox_dir =
            helpers::create_sandbox_directories(&self.config.sandbox_root, &sandbox_id).await?;

        // Setup filesystem mounts
        self.setup_filesystem_mounts(&sandbox_dir, &spec.filesystem_mounts)
            .await?;

        // Create sandbox info
        let sandbox_info = SandboxInfo {
            sandbox_id: sandbox_id.clone(),
            status: SandboxStatus::Creating,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            process_id: None,
            resource_usage: ResourceUsage::default(),
            security_violations: Vec::new(),
            metadata: HashMap::new(),
        };

        // Store sandbox info
        {
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.insert(sandbox_id.clone(), sandbox_info);
        }

        // Platform-specific sandbox creation
        #[cfg(target_os = "linux")]
        self.linux_manager
            .create_sandbox(&spec, &sandbox_dir)
            .await?;

        #[cfg(target_os = "macos")]
        self.macos_manager
            .create_sandbox(&spec, &sandbox_dir)
            .await?;

        #[cfg(windows)]
        self.windows_manager
            .create_sandbox(&spec, &sandbox_dir)
            .await?;

        // Update status to ready
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(&sandbox_id) {
                info.status = SandboxStatus::Ready;
                info.updated_at = SystemTime::now();
            }
        }

        info!("Sandbox {} created successfully", sandbox_id);
        Ok(sandbox_id)
    }

    async fn start_execution(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Starting execution in sandbox: {}", sandbox_id);

        // Update status to running
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(sandbox_id) {
                if info.status != SandboxStatus::Ready {
                    return Err(ToadStoolError::runtime(format!(
                        "Sandbox {} is not ready for execution (status: {:?})",
                        sandbox_id, info.status
                    )));
                }
                info.status = SandboxStatus::Running;
                info.updated_at = SystemTime::now();
            } else {
                return Err(ToadStoolError::runtime(format!(
                    "Sandbox {sandbox_id} not found"
                )));
            }
        }

        // Platform-specific execution start
        #[cfg(target_os = "linux")]
        self.linux_manager.start_execution(sandbox_id).await?;

        #[cfg(target_os = "macos")]
        self.macos_manager.start_execution(sandbox_id).await?;

        #[cfg(windows)]
        self.windows_manager.start_execution(sandbox_id).await?;

        info!("Execution started in sandbox: {}", sandbox_id);
        Ok(())
    }

    async fn stop_execution(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Stopping execution in sandbox: {}", sandbox_id);

        // Platform-specific execution stop
        #[cfg(target_os = "linux")]
        self.linux_manager.stop_execution(sandbox_id).await?;

        #[cfg(target_os = "macos")]
        self.macos_manager.stop_execution(sandbox_id).await?;

        #[cfg(windows)]
        self.windows_manager.stop_execution(sandbox_id).await?;

        // Update status
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(sandbox_id) {
                info.status = SandboxStatus::Completed;
                info.updated_at = SystemTime::now();
                info.process_id = None;
            }
        }

        info!("Execution stopped in sandbox: {}", sandbox_id);
        Ok(())
    }

    async fn destroy_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Destroying sandbox: {}", sandbox_id);

        // Update status to destroying
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(sandbox_id) {
                info.status = SandboxStatus::Destroying;
                info.updated_at = SystemTime::now();
            }
        }

        // Stop execution if running
        if let Ok(info) = self.get_sandbox_info(sandbox_id).await {
            if info.status == SandboxStatus::Running {
                let _ = self.stop_execution(sandbox_id).await;
            }
        }

        // Platform-specific cleanup
        #[cfg(target_os = "linux")]
        self.linux_manager.destroy_sandbox(sandbox_id).await?;

        #[cfg(target_os = "macos")]
        self.macos_manager.destroy_sandbox(sandbox_id).await?;

        #[cfg(windows)]
        self.windows_manager.destroy_sandbox(sandbox_id).await?;

        // Remove sandbox directory
        let sandbox_dir = self.config.sandbox_root.join(sandbox_id);
        if sandbox_dir.exists() {
            tokio::fs::remove_dir_all(&sandbox_dir).await.map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to remove sandbox directory {}: {}",
                    sandbox_dir.display(),
                    e
                ))
            })?;
        }

        // Remove from tracking
        {
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.remove(sandbox_id);
        }

        info!("Sandbox {} destroyed successfully", sandbox_id);
        Ok(())
    }

    async fn get_sandbox_info(&self, sandbox_id: &str) -> ToadStoolResult<SandboxInfo> {
        let sandboxes = self.sandboxes.read().await;
        sandboxes
            .get(sandbox_id)
            .cloned()
            .ok_or_else(|| ToadStoolError::runtime(format!("Sandbox {sandbox_id} not found")))
    }

    async fn list_sandboxes(&self) -> ToadStoolResult<Vec<String>> {
        let sandboxes = self.sandboxes.read().await;
        Ok(sandboxes.keys().cloned().collect())
    }

    async fn monitor_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<ResourceUsage> {
        debug!("Monitoring sandbox: {}", sandbox_id);

        // Platform-specific monitoring
        #[cfg(target_os = "linux")]
        let usage = self.linux_manager.monitor_sandbox(sandbox_id).await?;

        #[cfg(target_os = "macos")]
        let usage = self.macos_manager.monitor_sandbox(sandbox_id).await?;

        #[cfg(windows)]
        let usage = self.windows_manager.monitor_sandbox(sandbox_id).await?;

        // Update stored resource usage
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(sandbox_id) {
                info.resource_usage = usage.clone();
                info.updated_at = SystemTime::now();
            }
        }

        Ok(usage)
    }

    async fn apply_security_policy(
        &self,
        sandbox_id: &str,
        policy: &SecurityPolicy,
    ) -> ToadStoolResult<()> {
        debug!("Applying security policy to sandbox: {}", sandbox_id);

        // Platform-specific policy application
        #[cfg(target_os = "linux")]
        self.linux_manager
            .apply_security_policy(sandbox_id, policy)
            .await?;

        #[cfg(target_os = "macos")]
        self.macos_manager
            .apply_security_policy(sandbox_id, policy)
            .await?;

        #[cfg(windows)]
        self.windows_manager
            .apply_security_policy(sandbox_id, policy)
            .await?;

        info!("Security policy applied to sandbox: {}", sandbox_id);
        Ok(())
    }

    async fn get_sandbox_logs(&self, sandbox_id: &str) -> ToadStoolResult<Vec<String>> {
        debug!("Getting logs for sandbox: {}", sandbox_id);

        // Platform-specific log retrieval
        #[cfg(target_os = "linux")]
        let logs = self.linux_manager.get_sandbox_logs(sandbox_id).await?;

        #[cfg(target_os = "macos")]
        let logs = self.macos_manager.get_sandbox_logs(sandbox_id).await?;

        #[cfg(windows)]
        let logs = self.windows_manager.get_sandbox_logs(sandbox_id).await?;

        Ok(logs)
    }
}

// Platform-specific implementations
#[cfg(target_os = "linux")]
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use toadstool::IsolationLevel;
    use toadstool_security_policies::FilePolicyManager;

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
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
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

    #[tokio::test]
    async fn test_sandbox_manager_creation() {
        let config = create_test_config();
        let policy_manager = create_test_policy_manager();

        let manager = CrossPlatformSandboxManager::new(config, policy_manager).await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert!(manager.config.advanced_features_enabled);
        assert_eq!(manager.config.max_concurrent_sandboxes, 10);
        assert_eq!(manager.config.cleanup_timeout_secs, 5);
        assert_eq!(manager.config.monitoring_interval_ms, 100);
    }

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
    async fn test_network_config_defaults() {
        let config = NetworkConfig::default();

        assert!(!config.enabled);
        assert!(matches!(
            config.isolation_mode,
            NetworkIsolationMode::Firewall
        ));
        assert!(config.allowed_hosts.is_empty());
        assert!(config.allowed_ports.is_empty());
        assert_eq!(
            config.dns_servers,
            vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()]
        );
        assert!(config.bandwidth_limits.is_none());
    }

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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
