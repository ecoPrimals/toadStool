// SPDX-License-Identifier: AGPL-3.0-or-later
//! Smart installer for zero-touch ToadStool installation
//!
//! Domain modules:
//! - **paths**: Platform-specific path resolution
//! - **core**: Directory structure and toadstool executable
//! - **runtimes**: Container (Docker) and GPU runtime setup
//! - **`platform_components`**: Linux/macOS/Windows service configs
//! - **integration**: PATH, desktop shortcuts, shell completion
//! - **`config_manager`**: Apply ToadStool configuration
//! - **types**: `InstallationResult`, `InstallationConfig`

mod config_manager;
mod core;
mod integration;
mod paths;
mod platform_components;
mod runtimes;
mod types;

use std::path::PathBuf;

use tokio::process::Command as AsyncCommand;
use tracing::{info, warn};

use toadstool_common::platform_paths::Platform;

use crate::ToadStoolError;
use crate::hardware::SystemCapabilities;
use crate::intelligent::IntelligentAutoConfig;

pub use config_manager::ConfigManager;
pub use types::{InstallationConfig, InstallationResult};

/// Zero-touch installation and setup
#[derive(Debug, Clone)]
pub struct SmartInstaller {
    platform: Platform,
    installation_path: PathBuf,
    config_manager: ConfigManager,
}

impl SmartInstaller {
    /// Creates a new smart installer with auto-detected platform.
    #[must_use]
    pub fn new() -> Self {
        let platform = Platform::detect();
        let installation_path = paths::default_installation_path(platform);

        Self {
            platform,
            installation_path,
            config_manager: ConfigManager::new(),
        }
    }

    /// Complete zero-touch installation
    pub async fn install_zero_touch() -> Result<(), ToadStoolError> {
        info!("🚀 Starting zero-touch ToadStool installation");

        let installer = Self::new();

        // 1. Auto-detect everything
        let mut hardware_detector = crate::HardwareDetector::new();
        let capabilities = hardware_detector.scan_system().await?;

        // 2. Install only what's needed
        installer.install_optimal_components(&capabilities).await?;

        // 3. Configure automatically
        let config = IntelligentAutoConfig::auto_configure().await?;

        // 4. Apply configuration
        installer
            .config_manager
            .apply_configuration(&config)
            .await?;

        // 5. Set up system integration
        installer.setup_system_integration().await?;

        // 6. Start services
        installer.start_services().await?;

        info!("✅ ToadStool ready! Try: toadstool --help");
        Ok(())
    }

    /// Install only what's needed for this system
    async fn install_optimal_components(
        &self,
        capabilities: &SystemCapabilities,
    ) -> Result<(), ToadStoolError> {
        info!(
            "📦 Installing platform-specific components for {}...",
            paths::platform_as_str(self.platform)
        );

        core::ensure_installation_directory(&self.installation_path).await?;
        core::install_core_components(&self.installation_path).await?;

        // Install runtime-specific dependencies based on hardware
        if docker_available().await {
            runtimes::setup_container_runtime(&self.installation_path).await?;
        }

        if capabilities.gpu_count > 0 {
            runtimes::setup_gpu_runtime(&self.installation_path).await?;
        }

        platform_components::install_platform_components(self.platform, &self.installation_path)
            .await?;

        Ok(())
    }

    /// Smart system integration
    async fn setup_system_integration(&self) -> Result<(), ToadStoolError> {
        info!("🔗 Setting up system integration...");

        integration::add_to_path(self.platform, &self.installation_path).await?;

        if integration::has_gui(self.platform) {
            integration::create_desktop_shortcuts(self.platform, &self.installation_path).await?;
        }

        integration::setup_shell_completion(&self.installation_path).await?;

        Ok(())
    }

    /// Start ToadStool services
    async fn start_services(&self) -> Result<(), ToadStoolError> {
        info!("🚀 Starting ToadStool services...");

        let bin_path = self.installation_path.join("bin").join(if cfg!(windows) {
            "toadstool.bat"
        } else {
            "toadstool"
        });

        if bin_path.exists() {
            info!("✅ ToadStool executable ready at: {}", bin_path.display());
        } else {
            return Err(ToadStoolError::Configuration(
                "ToadStool executable not found".to_string(),
            ));
        }

        if let Ok(output) = AsyncCommand::new(&bin_path).output().await {
            if output.status.success() {
                info!("✅ ToadStool services started successfully");
            } else {
                warn!("⚠️ ToadStool services may not be fully functional");
            }
        }

        Ok(())
    }
}

impl Default for SmartInstaller {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if Docker is available
async fn docker_available() -> bool {
    tokio::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .await
        .is_ok_and(|o| o.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_common::platform_paths::Platform;

    #[test]
    fn test_smart_installer_creation() {
        let installer = SmartInstaller::new();
        assert!(!installer.installation_path.as_os_str().is_empty());
    }

    #[test]
    fn test_default_installation_path() {
        let linux_path = paths::default_installation_path(Platform::Linux);
        assert!(linux_path.to_string_lossy().contains("toadstool"));

        let mac_path = paths::default_installation_path(Platform::MacOS);
        assert!(mac_path.to_string_lossy().contains("ToadStool"));

        let win_path = paths::default_installation_path(Platform::Windows);
        assert!(win_path.to_string_lossy().contains("ToadStool"));
    }

    #[test]
    fn test_platform_as_str() {
        assert_eq!(paths::platform_as_str(Platform::Linux), "Linux");
        assert_eq!(paths::platform_as_str(Platform::MacOS), "macOS");
        assert_eq!(paths::platform_as_str(Platform::Windows), "Windows");
    }

    #[test]
    fn test_config_manager_creation() {
        let _manager = ConfigManager::new();
        // ConfigManager constructs successfully with platform-specific path
    }

    #[test]
    fn test_config_path_for_platform() {
        let linux_path = paths::config_path_for_platform(Platform::Linux);
        assert!(linux_path.to_string_lossy().contains("toadstool"));

        let mac_path = paths::config_path_for_platform(Platform::MacOS);
        assert!(mac_path.to_string_lossy().contains("ToadStool"));

        let win_path = paths::config_path_for_platform(Platform::Windows);
        assert!(win_path.to_string_lossy().contains("ToadStool"));
    }

    #[test]
    fn test_installation_config_default() {
        let config = InstallationConfig::default();
        assert!(config.add_to_path);
        assert!(config.enable_shell_completion);
    }

    #[test]
    fn test_installation_result_default() {
        let result = InstallationResult::default();
        assert!(!result.success);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_ensure_installation_directory_creates_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let install_path = dir.path().join("toadstool");
        assert!(!install_path.exists());

        let result = core::ensure_installation_directory(&install_path).await;
        assert!(result.is_ok());
        assert!(install_path.exists());
        assert!(install_path.is_dir());
    }

    #[tokio::test]
    async fn test_install_core_components_creates_structure() {
        let dir = tempfile::tempdir().expect("tempdir");
        let install_path = dir.path().to_path_buf();

        let result = core::install_core_components(&install_path).await;
        assert!(result.is_ok());

        assert!(install_path.join("bin").exists());
        assert!(install_path.join("config").exists());
        assert!(install_path.join("data").exists());
        assert!(install_path.join("logs").exists());

        let script_name = if cfg!(windows) {
            "toadstool.bat"
        } else {
            "toadstool"
        };
        assert!(install_path.join("bin").join(script_name).exists());
    }

    #[tokio::test]
    async fn test_paths_default_installation_with_home() {
        temp_env::with_var("HOME", Some("/tmp/test-home"), || {
            let path = paths::default_installation_path(Platform::Linux);
            assert!(path.to_string_lossy().contains("toadstool"));
            assert!(path.to_string_lossy().contains("test-home"));
        });
    }

    #[tokio::test]
    async fn test_paths_config_path_with_home() {
        temp_env::with_var("HOME", Some("/tmp/test-home"), || {
            let path = paths::config_path_for_platform(Platform::Linux);
            assert!(path.to_string_lossy().contains("toadstool"));
            assert!(path.to_string_lossy().contains("config"));
        });
    }
}
