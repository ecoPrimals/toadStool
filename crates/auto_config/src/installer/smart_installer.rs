// SPDX-License-Identifier: AGPL-3.0-or-later
//! Zero-touch installation orchestrator (requires `runtime` feature).

use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use tracing::{info, warn};

use toadstool_common::constants::primal_identity::PRIMAL_BINARY_NAME;
use toadstool_common::platform_paths::Platform;

use crate::ToadStoolError;
use crate::hardware::SystemCapabilities;
use crate::intelligent::IntelligentAutoConfig;

use super::config_manager::ConfigManager;
use super::{core, integration, paths, platform_components, runtimes};

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
        if docker_available() {
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
            format!("{PRIMAL_BINARY_NAME}.bat")
        } else {
            PRIMAL_BINARY_NAME.to_string()
        });

        if bin_path.exists() {
            info!("✅ ToadStool executable ready at: {}", bin_path.display());
        } else {
            return Err(ToadStoolError::Configuration(
                "ToadStool executable not found".to_string(),
            ));
        }

        if let Ok(output) = ProcessCommand::new(&bin_path).output() {
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
fn docker_available() -> bool {
    std::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .is_ok_and(|o| o.status.success())
}
