// SPDX-License-Identifier: AGPL-3.0-or-later
//! macOS-specific sandbox implementation
//!
//! This module provides macOS-specific sandboxing functionality using
//! macOS sandbox profiles and system security features.
//!
//! Requires `macos-sandbox` feature to enable Core Foundation bindings.

use crate::{SandboxConfig, SandboxError, SandboxResult};

/// macOS sandbox implementation
pub struct MacOSSandbox {
    config: SandboxConfig,
}

impl MacOSSandbox {
    /// Create a new macOS sandbox
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// Apply macOS sandbox profile.
    ///
    /// Returns `PlatformNotSupported` — sandbox-exec / App Sandbox integration
    /// is not yet implemented. Callers must not assume enforcement.
    pub async fn apply_sandbox(&self) -> SandboxResult<()> {
        let _ = &self.config;
        Err(SandboxError::PlatformNotSupported(
            "macOS sandbox enforcement not yet implemented (sandbox-exec / App Sandbox)"
                .to_string(),
        ))
    }

    /// Remove macOS sandbox restrictions.
    ///
    /// Returns `PlatformNotSupported` — no sandbox is applied, so none can be removed.
    pub async fn remove_sandbox(&self) -> SandboxResult<()> {
        let _ = &self.config;
        Err(SandboxError::PlatformNotSupported(
            "macOS sandbox enforcement not yet implemented".to_string(),
        ))
    }

    /// Check if macOS sandbox is supported
    pub fn is_supported() -> bool {
        // Check if we're on macOS
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
