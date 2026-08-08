// SPDX-License-Identifier: AGPL-3.0-or-later
//! macOS-specific sandbox implementation
//!
//! This module provides macOS-specific sandboxing functionality using
//! macOS sandbox profiles and system security features.
//!
//! Full implementation requires Core Foundation bindings (sandbox-exec / App Sandbox).

use std::path::Path;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::{
    FilesystemMount, ResourceUsage, SandboxConfig, SandboxError, SandboxResult, SandboxSpec,
};
use toadstool_security_policies::SecurityPolicy;

/// macOS sandbox manager — stub for cross-platform compilation.
///
/// Full implementation requires Core Foundation bindings (sandbox-exec / App Sandbox).
pub struct MacOSSandboxManager {
    _config: SandboxConfig,
}

impl MacOSSandboxManager {
    /// Create a new macOS sandbox manager.
    pub async fn new(config: SandboxConfig) -> ToadStoolResult<Self> {
        Ok(Self { _config: config })
    }

    /// Create a sandbox (stub — returns platform-not-supported error).
    pub async fn create_sandbox(
        &self,
        _spec: &SandboxSpec,
        _sandbox_dir: &Path,
    ) -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime(
            "macOS sandbox not yet implemented".to_string(),
        ))
    }

    /// Start execution in sandbox (stub).
    pub async fn start_execution(&self, _sandbox_id: &str) -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime(
            "macOS sandbox not yet implemented".to_string(),
        ))
    }

    /// Stop execution in sandbox (stub).
    pub async fn stop_execution(&self, _sandbox_id: &str) -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime(
            "macOS sandbox not yet implemented".to_string(),
        ))
    }

    /// Destroy sandbox (stub).
    pub async fn destroy_sandbox(&self, _sandbox_id: &str) -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime(
            "macOS sandbox not yet implemented".to_string(),
        ))
    }

    /// Setup a filesystem mount (stub).
    pub async fn setup_mount(
        &self,
        _mount: &FilesystemMount,
        _target_path: &Path,
    ) -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime(
            "macOS sandbox not yet implemented".to_string(),
        ))
    }

    /// Monitor sandbox resource usage (stub).
    pub async fn monitor_sandbox(&self, _sandbox_id: &str) -> ToadStoolResult<ResourceUsage> {
        Err(ToadStoolError::runtime(
            "macOS sandbox not yet implemented".to_string(),
        ))
    }

    /// Apply security policy to sandbox (stub).
    pub async fn apply_security_policy(
        &self,
        _sandbox_id: &str,
        _policy: &SecurityPolicy,
    ) -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime(
            "macOS sandbox not yet implemented".to_string(),
        ))
    }

    /// Get sandbox logs (stub).
    pub async fn get_sandbox_logs(&self, _sandbox_id: &str) -> ToadStoolResult<Vec<String>> {
        Err(ToadStoolError::runtime(
            "macOS sandbox not yet implemented".to_string(),
        ))
    }
}

/// macOS sandbox implementation (lower-level).
pub struct MacOSSandbox {
    config: SandboxConfig,
}

impl MacOSSandbox {
    /// Create a new macOS sandbox
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// Apply macOS sandbox profile.
    pub async fn apply_sandbox(&self) -> SandboxResult<()> {
        let _ = &self.config;
        Err(SandboxError::PlatformNotSupported(
            "macOS sandbox enforcement not yet implemented (sandbox-exec / App Sandbox)"
                .to_string(),
        ))
    }

    /// Remove macOS sandbox restrictions.
    pub async fn remove_sandbox(&self) -> SandboxResult<()> {
        let _ = &self.config;
        Err(SandboxError::PlatformNotSupported(
            "macOS sandbox enforcement not yet implemented".to_string(),
        ))
    }

    /// Check if macOS sandbox is supported
    pub fn is_supported() -> bool {
        cfg!(target_os = "macos")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_sandbox_creation() {
        let config = SandboxConfig::default();
        let _sandbox = MacOSSandbox::new(config);
        assert!(MacOSSandbox::is_supported() == cfg!(target_os = "macos"));
    }
}
