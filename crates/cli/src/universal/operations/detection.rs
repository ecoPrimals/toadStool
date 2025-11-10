//! Platform Detection Operations
//!
//! Extension trait for platform detection and capability testing operations.

use anyhow::Result;
use std::future::Future;
use tracing::warn;

use crate::universal::types::DetectedPlatform;
use toadstool_distributed::substrate_detection::PlatformType;

/// Platform detection operations trait
pub trait PlatformDetectionOps {
    /// Test platform capabilities
    fn test_platform_capabilities(
        &self,
        platform_id: &str,
        platform: &DetectedPlatform,
    ) -> impl Future<Output = Result<bool>> + Send;

    /// Test Linux-specific capabilities
    fn test_linux_capabilities(&self) -> impl Future<Output = Result<bool>> + Send;

    /// Test macOS-specific capabilities
    fn test_macos_capabilities(&self) -> impl Future<Output = Result<bool>> + Send;

    /// Test Windows-specific capabilities
    fn test_windows_capabilities(&self) -> impl Future<Output = Result<bool>> + Send;

    /// Test generic/unknown platform capabilities
    fn test_generic_capabilities(&self) -> impl Future<Output = Result<bool>> + Send;
}

/// Implementation of platform detection operations
impl PlatformDetectionOps for crate::universal::UniversalComputeManager {
    async fn test_platform_capabilities(
        &self,
        platform_id: &str,
        platform: &DetectedPlatform,
    ) -> Result<bool> {
        // Run basic capability tests for the detected platform
        // Perform platform-specific capability testing
        let platform_name = match &platform.platform_type {
            PlatformType::Linux { .. } => "linux",
            PlatformType::MacOS { .. } => "macos",
            PlatformType::Windows { .. } => "windows",
            _ => "unknown",
        };

        let _capabilities_result = match platform_name {
            "linux" => self.test_linux_capabilities().await,
            "macos" => self.test_macos_capabilities().await,
            "windows" => self.test_windows_capabilities().await,
            _ => {
                warn!(
                    "Unknown platform type '{}', using generic tests",
                    platform_name
                );
                self.test_generic_capabilities().await
            }
        };

        match &platform.platform_type {
            PlatformType::Linux { .. }
            | PlatformType::Windows { .. }
            | PlatformType::MacOS { .. } => {
                // Test native platform capabilities
                match std::process::Command::new("echo").arg("test").output() {
                    Ok(output) => Ok(output.status.success()),
                    Err(_) => Ok(false),
                }
            }
            PlatformType::Docker | PlatformType::Podman | PlatformType::Containerd => {
                // Test container runtime availability
                match std::process::Command::new("which").arg("docker").output() {
                    Ok(output) => Ok(output.status.success()),
                    Err(_) => Ok(false),
                }
            }
            PlatformType::WebAssembly { runtime: _ } => {
                // Test WASM runtime availability
                // For now, assume WASM is always available if detected
                Ok(true)
            }
            _ => {
                warn!(
                    "⚠️  Platform capability testing not implemented for: {}",
                    platform_id
                );
                Ok(false) // Conservative approach for unknown platforms
            }
        }
    }

    async fn test_linux_capabilities(&self) -> Result<bool> {
        // Test Linux-specific capabilities
        // Check for common Linux features
        let features = vec![
            // Core system commands
            ("uname", vec!["-a"]),
            ("cat", vec!["/proc/version"]),
        ];

        for (cmd, args) in features {
            match std::process::Command::new(cmd).args(&args).output() {
                Ok(output) if output.status.success() => continue,
                _ => return Ok(false),
            }
        }

        Ok(true)
    }

    async fn test_macos_capabilities(&self) -> Result<bool> {
        // Test macOS-specific capabilities
        // Check for macOS-specific features
        let features = vec![
            // macOS system commands
            ("sw_vers", vec![]),
            ("system_profiler", vec!["SPSoftwareDataType"]),
        ];

        for (cmd, args) in features {
            match std::process::Command::new(cmd).args(&args).output() {
                Ok(output) if output.status.success() => continue,
                _ => return Ok(false),
            }
        }

        Ok(true)
    }

    async fn test_windows_capabilities(&self) -> Result<bool> {
        // Test Windows-specific capabilities
        // Note: These commands work in Windows CMD/PowerShell
        let _features: Vec<(&str, Vec<String>)> = vec![("ver", vec![]), ("systeminfo", vec![])];

        // For cross-platform compatibility, we'll do a simple check
        #[cfg(target_os = "windows")]
        {
            match std::process::Command::new("ver").output() {
                Ok(output) => Ok(output.status.success()),
                Err(_) => Ok(false),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Not on Windows, can't test Windows capabilities
            Ok(false)
        }
    }

    async fn test_generic_capabilities(&self) -> Result<bool> {
        // Test generic capabilities that should work on any platform
        // Basic shell command test
        match std::process::Command::new("echo").arg("test").output() {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }
}

