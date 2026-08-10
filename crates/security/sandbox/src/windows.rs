// SPDX-License-Identifier: AGPL-3.0-or-later
//! Windows-specific sandbox implementation
//!
//! This module provides Windows-specific sandboxing functionality using
//! Windows security features and process isolation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tracing::{debug, info};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_security_policies::SecurityPolicy;

use crate::types::{FilesystemMount, ResourceUsage, SandboxConfig, SandboxSpec};
use crate::{SandboxError, SandboxResult};

/// Windows sandbox implementation
pub struct WindowsSandbox {
    config: SandboxConfig,
}

impl WindowsSandbox {
    /// Create a new Windows sandbox
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// Apply Windows sandbox restrictions.
    ///
    /// Returns `PlatformNotSupported` — Job Object / AppContainer integration
    /// is not yet implemented. Callers must not assume enforcement.
    pub async fn apply_sandbox(&self) -> SandboxResult<()> {
        let _ = &self.config;
        Err(SandboxError::PlatformNotSupported(
            "Windows sandbox enforcement not yet implemented (Job Objects / AppContainer)"
                .to_string(),
        ))
    }

    /// Remove Windows sandbox restrictions.
    ///
    /// Returns `PlatformNotSupported` — no sandbox is applied, so none can be removed.
    pub async fn remove_sandbox(&self) -> SandboxResult<()> {
        let _ = &self.config;
        Err(SandboxError::PlatformNotSupported(
            "Windows sandbox enforcement not yet implemented".to_string(),
        ))
    }

    /// Check if Windows sandbox is supported
    pub fn is_supported() -> bool {
        cfg!(target_os = "windows")
    }
}

/// Windows-specific sandbox manager.
pub struct WindowsSandboxManager {
    config: SandboxConfig,
    processes: RwLock<HashMap<String, u32>>,
    runtime: RwLock<HashMap<String, WindowsSandboxRuntime>>,
}

#[derive(Debug, Default)]
struct WindowsSandboxRuntime {
    mounts: Vec<PathBuf>,
}

impl WindowsSandboxManager {
    /// Create a new Windows sandbox manager.
    pub async fn new(config: SandboxConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            config,
            processes: RwLock::new(HashMap::new()),
            runtime: RwLock::new(HashMap::new()),
        })
    }

    /// Create sandbox working state for the given spec.
    pub async fn create_sandbox(
        &self,
        spec: &SandboxSpec,
        _sandbox_dir: &Path,
    ) -> ToadStoolResult<()> {
        debug!("Creating Windows sandbox: {}", spec.sandbox_id);
        self.runtime
            .write().unwrap_or_else(|e| e.into_inner())
            .entry(spec.sandbox_id.clone())
            .or_insert_with(WindowsSandboxRuntime::default);
        info!("Windows sandbox {} created successfully", spec.sandbox_id);
        Ok(())
    }

    /// Start execution in Windows sandbox.
    pub async fn start_execution(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        Err(ToadStoolError::not_supported(format!(
            "Windows sandbox execution not yet implemented (sandbox {sandbox_id})"
        )))
    }

    /// Stop execution in Windows sandbox.
    pub async fn stop_execution(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Stopping execution in Windows sandbox: {sandbox_id}");
        self.processes.write().unwrap_or_else(|e| e.into_inner()).remove(sandbox_id);
        info!("Stopped execution in Windows sandbox: {sandbox_id}");
        Ok(())
    }

    /// Destroy sandbox and release tracked state.
    pub async fn destroy_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Destroying Windows sandbox: {sandbox_id}");
        self.stop_execution(sandbox_id).await?;
        self.runtime.write().unwrap_or_else(|e| e.into_inner()).remove(sandbox_id);
        info!("Windows sandbox {sandbox_id} destroyed successfully");
        Ok(())
    }

    /// Set up a filesystem mount inside the sandbox.
    pub async fn setup_mount(
        &self,
        mount_spec: &FilesystemMount,
        target_path: &Path,
    ) -> ToadStoolResult<()> {
        debug!(
            "Setting up Windows filesystem mount: {:?} -> {:?}",
            mount_spec.source, mount_spec.target
        );

        self.runtime
            .write().unwrap_or_else(|e| e.into_inner())
            .values_mut()
            .next()
            .map(|runtime| runtime.mounts.push(target_path.to_path_buf()));

        let _ = &self.config;
        info!(
            "Windows mount bookkeeping recorded: {}",
            target_path.display()
        );
        Ok(())
    }

    /// Monitor sandbox resource usage.
    pub async fn monitor_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<ResourceUsage> {
        Err(ToadStoolError::not_supported(format!(
            "Windows sandbox monitoring not yet implemented (sandbox {sandbox_id})"
        )))
    }

    /// Apply a security policy to the sandbox.
    pub async fn apply_security_policy(
        &self,
        sandbox_id: &str,
        _policy: &SecurityPolicy,
    ) -> ToadStoolResult<()> {
        Err(ToadStoolError::not_supported(format!(
            "Windows sandbox policy enforcement not yet implemented (sandbox {sandbox_id})"
        )))
    }

    /// Retrieve sandbox logs.
    pub async fn get_sandbox_logs(&self, sandbox_id: &str) -> ToadStoolResult<Vec<String>> {
        debug!("Fetching logs for Windows sandbox: {sandbox_id}");
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_sandbox_creation() {
        let config = SandboxConfig::default();
        let _sandbox = WindowsSandbox::new(config);
        assert_eq!(WindowsSandbox::is_supported(), cfg!(target_os = "windows"));
    }
}
