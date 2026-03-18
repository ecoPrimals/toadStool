// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cross-platform sandbox manager implementation

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

/// Cross-platform sandbox manager delegating to OS-specific implementations.
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
        if let Ok(info) = self.get_sandbox_info(sandbox_id).await
            && info.status == SandboxStatus::Running
        {
            let _ = self.stop_execution(sandbox_id).await;
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
