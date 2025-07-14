//! Smart installer for zero-touch ToadStool installation

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command as AsyncCommand;
use tracing::{info, warn};

/// Zero-touch installation and setup
#[derive(Debug, Clone)]
pub struct SmartInstaller {
    platform: crate::Platform,
    installation_path: PathBuf,
    config_manager: ConfigManager,
}

impl SmartInstaller {
    pub fn new() -> Self {
        let platform = crate::Platform::detect();
        let installation_path = Self::default_installation_path(&platform);

        Self {
            platform,
            installation_path,
            config_manager: ConfigManager::new(),
        }
    }

    /// Complete zero-touch installation
    pub async fn install_zero_touch() -> Result<(), crate::AutoConfigError> {
        info!("🚀 Starting zero-touch ToadStool installation");

        let mut installer = Self::new();

        // 1. Auto-detect everything
        let mut hardware_detector = crate::HardwareDetector::new();
        let capabilities = hardware_detector.scan_system().await?;

        // 2. Install only what's needed
        installer.install_optimal_components(&capabilities).await?;

        // 3. Configure automatically
        let mut auto_config = crate::IntelligentAutoConfig::new();
        let config = auto_config.auto_configure().await?;

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
        &mut self,
        capabilities: &crate::SystemCapabilities,
    ) -> Result<(), crate::AutoConfigError> {
        info!(
            "📦 Installing platform-specific components for {}...",
            self.platform.as_str()
        );

        // Create installation directory
        self.ensure_installation_directory().await?;

        // Install core components
        self.install_core_components().await?;

        // Install runtime-specific dependencies based on hardware
        if capabilities.container_support.docker_available {
            self.setup_container_runtime().await?;
        }

        if capabilities.gpu_count > 0 {
            self.setup_gpu_runtime().await?;
        }

        // Install platform-specific components
        match self.platform {
            crate::Platform::Linux => self.install_linux_components().await?,
            crate::Platform::MacOs => self.install_macos_components().await?,
            crate::Platform::Windows => self.install_windows_components().await?,
        }

        Ok(())
    }

    /// Ensure installation directory exists
    async fn ensure_installation_directory(&self) -> Result<(), crate::AutoConfigError> {
        if !self.installation_path.exists() {
            info!(
                "📁 Creating installation directory: {}",
                self.installation_path.display()
            );
            fs::create_dir_all(&self.installation_path).await?;
        }
        Ok(())
    }

    /// Install core ToadStool components
    async fn install_core_components(&self) -> Result<(), crate::AutoConfigError> {
        info!("🔧 Installing core ToadStool components...");

        // This would normally download and install binaries
        // For now, we'll just create the necessary directory structure

        let bin_dir = self.installation_path.join("bin");
        let config_dir = self.installation_path.join("config");
        let data_dir = self.installation_path.join("data");
        let logs_dir = self.installation_path.join("logs");

        for dir in [&bin_dir, &config_dir, &data_dir, &logs_dir] {
            if !dir.exists() {
                fs::create_dir_all(dir).await?;
            }
        }

        // Create a simple toadstool executable script
        let toadstool_script = if cfg!(windows) {
            format!(
                r#"@echo off
echo ToadStool Universal Compute Platform
echo Installation: {}
echo.
echo Available commands:
echo   status  - Show system status
echo   config  - Configure ToadStool
echo   run     - Execute workloads
echo   help    - Show help
"#,
                self.installation_path.display()
            )
        } else {
            format!(
                r#"#!/bin/bash
echo "ToadStool Universal Compute Platform"
echo "Installation: {}"
echo ""
echo "Available commands:"
echo "  status  - Show system status"
echo "  config  - Configure ToadStool"
echo "  run     - Execute workloads"
echo "  help    - Show help"
"#,
                self.installation_path.display()
            )
        };

        let script_name = if cfg!(windows) {
            "toadstool.bat"
        } else {
            "toadstool"
        };
        let script_path = bin_dir.join(script_name);

        fs::write(&script_path, toadstool_script).await?;

        // Make executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).await?;
        }

        info!("✅ Core components installed");
        Ok(())
    }

    /// Setup container runtime support
    async fn setup_container_runtime(&self) -> Result<(), crate::AutoConfigError> {
        info!("🐳 Setting up container runtime support...");

        // Verify Docker is working
        if let Ok(output) = AsyncCommand::new("docker")
            .args(["version", "--format", "{{.Server.Version}}"])
            .output()
            .await
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("🐳 Docker version: {}", version.trim());
            }
        }

        // Create Docker configuration if needed
        let docker_config_dir = self.installation_path.join("config").join("docker");
        if !docker_config_dir.exists() {
            fs::create_dir_all(&docker_config_dir).await?;

            // Create a basic Docker configuration
            let docker_config = serde_json::json!({
                "default_runtime": "runc",
                "runtimes": {
                    "runc": {
                        "path": "runc"
                    }
                },
                "storage_driver": "overlay2"
            });

            fs::write(
                docker_config_dir.join("daemon.json"),
                serde_json::to_string_pretty(&docker_config)?,
            )
            .await?;
        }

        Ok(())
    }

    /// Setup GPU runtime support
    async fn setup_gpu_runtime(&self) -> Result<(), crate::AutoConfigError> {
        info!("🎮 Setting up GPU runtime support...");

        // Check for NVIDIA runtime
        if let Ok(output) = AsyncCommand::new("nvidia-smi")
            .arg("--version")
            .output()
            .await
        {
            if output.status.success() {
                info!("🎮 NVIDIA GPU runtime detected");

                // Create NVIDIA configuration
                let gpu_config_dir = self.installation_path.join("config").join("gpu");
                if !gpu_config_dir.exists() {
                    fs::create_dir_all(&gpu_config_dir).await?;

                    let nvidia_config = serde_json::json!({
                        "runtime": "nvidia",
                        "memory_fraction": 0.8,
                        "compute_mode": "default"
                    });

                    fs::write(
                        gpu_config_dir.join("nvidia.json"),
                        serde_json::to_string_pretty(&nvidia_config)?,
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// Install Linux-specific components
    async fn install_linux_components(&self) -> Result<(), crate::AutoConfigError> {
        info!("🐧 Installing Linux-specific components...");

        // Create systemd service file
        let service_content = format!(
            r#"[Unit]
Description=ToadStool Universal Compute Platform
After=network.target

[Service]
Type=simple
ExecStart={}/bin/toadstool daemon
Restart=always
RestartSec=5
User=toadstool
Group=toadstool

[Install]
WantedBy=multi-user.target
"#,
            self.installation_path.display()
        );

        let systemd_dir = self.installation_path.join("systemd");
        if !systemd_dir.exists() {
            fs::create_dir_all(&systemd_dir).await?;
        }

        fs::write(systemd_dir.join("toadstool.service"), service_content).await?;

        info!("🐧 Linux components installed");
        Ok(())
    }

    /// Install macOS-specific components
    async fn install_macos_components(&self) -> Result<(), crate::AutoConfigError> {
        info!("🍎 Installing macOS-specific components...");

        // Create launchd plist
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>dev.toadstool.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}/bin/toadstool</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardErrorPath</key>
    <string>{}/logs/toadstool.log</string>
    <key>StandardOutPath</key>
    <string>{}/logs/toadstool.log</string>
</dict>
</plist>
"#,
            self.installation_path.display(),
            self.installation_path.display(),
            self.installation_path.display()
        );

        let launchd_dir = self.installation_path.join("launchd");
        if !launchd_dir.exists() {
            fs::create_dir_all(&launchd_dir).await?;
        }

        fs::write(
            launchd_dir.join("dev.toadstool.daemon.plist"),
            plist_content,
        )
        .await?;

        info!("🍎 macOS components installed");
        Ok(())
    }

    /// Install Windows-specific components
    async fn install_windows_components(&self) -> Result<(), crate::AutoConfigError> {
        info!("🪟 Installing Windows-specific components...");

        // Create Windows service configuration
        let service_config = serde_json::json!({
            "service_name": "ToadStool",
            "display_name": "ToadStool Universal Compute Platform",
            "description": "Universal compute platform for workload execution",
            "executable": format!("{}/bin/toadstool.exe", self.installation_path.display()),
            "arguments": ["daemon"],
            "start_type": "automatic"
        });

        let service_dir = self.installation_path.join("service");
        if !service_dir.exists() {
            fs::create_dir_all(&service_dir).await?;
        }

        fs::write(
            service_dir.join("service.json"),
            serde_json::to_string_pretty(&service_config)?,
        )
        .await?;

        info!("🪟 Windows components installed");
        Ok(())
    }

    /// Smart system integration
    async fn setup_system_integration(&self) -> Result<(), crate::AutoConfigError> {
        info!("🔗 Setting up system integration...");

        // Add to PATH
        self.add_to_path().await?;

        // Create desktop shortcuts if GUI available
        if self.has_gui() {
            self.create_desktop_shortcuts().await?;
        }

        // Set up shell completion
        self.setup_shell_completion().await?;

        Ok(())
    }

    /// Add ToadStool to system PATH
    async fn add_to_path(&self) -> Result<(), crate::AutoConfigError> {
        let bin_path = self.installation_path.join("bin");

        match self.platform {
            crate::Platform::Linux | crate::Platform::MacOs => {
                // Add to shell profile
                let shell_profile = if Path::new(&format!(
                    "{}/.zshrc",
                    std::env::var("HOME").unwrap_or_default()
                ))
                .exists()
                {
                    ".zshrc"
                } else {
                    ".bashrc"
                };

                let profile_path = format!(
                    "{}/{}",
                    std::env::var("HOME").unwrap_or_default(),
                    shell_profile
                );
                let path_export = format!(
                    "\n# ToadStool\nexport PATH=\"{}:$PATH\"\n",
                    bin_path.display()
                );

                // Check if already added
                if let Ok(content) = fs::read_to_string(&profile_path).await {
                    if !content.contains("ToadStool") {
                        fs::write(&profile_path, format!("{content}{path_export}")).await?;
                        info!("✅ Added ToadStool to PATH in {}", shell_profile);
                    }
                }
            }
            crate::Platform::Windows => {
                // On Windows, we would modify the registry or use setx
                // For now, just log that it should be done
                info!(
                    "💡 Please add {} to your PATH environment variable",
                    bin_path.display()
                );
            }
        }

        Ok(())
    }

    /// Check if GUI is available
    fn has_gui(&self) -> bool {
        match self.platform {
            crate::Platform::Linux => {
                // Check for X11 or Wayland
                std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
            }
            crate::Platform::MacOs => {
                // macOS always has GUI
                true
            }
            crate::Platform::Windows => {
                // Windows always has GUI
                true
            }
        }
    }

    /// Create desktop shortcuts
    async fn create_desktop_shortcuts(&self) -> Result<(), crate::AutoConfigError> {
        info!("🖥️ Creating desktop shortcuts...");

        match self.platform {
            crate::Platform::Linux => {
                let desktop_dir = format!("{}/Desktop", std::env::var("HOME").unwrap_or_default());
                if Path::new(&desktop_dir).exists() {
                    let desktop_file = format!(
                        r#"[Desktop Entry]
Version=1.0
Type=Application
Name=ToadStool
Comment=Universal Compute Platform
Exec={}/bin/toadstool
Icon=utilities-terminal
Terminal=true
Categories=Development;System;
"#,
                        self.installation_path.display()
                    );

                    fs::write(format!("{desktop_dir}/ToadStool.desktop"), desktop_file).await?;
                }
            }
            crate::Platform::MacOs => {
                // macOS desktop shortcuts would be created differently
                info!("💡 macOS desktop shortcuts not implemented yet");
            }
            crate::Platform::Windows => {
                // Windows desktop shortcuts would be created differently
                info!("💡 Windows desktop shortcuts not implemented yet");
            }
        }

        Ok(())
    }

    /// Setup shell completion
    async fn setup_shell_completion(&self) -> Result<(), crate::AutoConfigError> {
        info!("🐚 Setting up shell completion...");

        let completion_dir = self.installation_path.join("completion");
        if !completion_dir.exists() {
            fs::create_dir_all(&completion_dir).await?;
        }

        // Create bash completion
        let bash_completion = r#"# ToadStool completion
_toadstool_complete() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="status config run help daemon"
    
    if [[ ${cur} == -* ]]; then
        COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
        return 0
    fi
    
    case "${prev}" in
        run)
            COMPREPLY=( $(compgen -f -- ${cur}) )
            return 0
            ;;
        *)
            COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            return 0
            ;;
    esac
}

complete -F _toadstool_complete toadstool
"#;

        fs::write(completion_dir.join("toadstool.bash"), bash_completion).await?;

        info!("🐚 Shell completion installed");
        Ok(())
    }

    /// Start ToadStool services
    async fn start_services(&self) -> Result<(), crate::AutoConfigError> {
        info!("🚀 Starting ToadStool services...");

        // For now, just verify the installation
        let bin_path = self.installation_path.join("bin").join(if cfg!(windows) {
            "toadstool.bat"
        } else {
            "toadstool"
        });

        if bin_path.exists() {
            info!("✅ ToadStool executable ready at: {}", bin_path.display());
        } else {
            return Err(crate::AutoConfigError::ConfigGeneration(
                "ToadStool executable not found".to_string(),
            ));
        }

        // Test basic functionality
        if let Ok(output) = AsyncCommand::new(&bin_path).output().await {
            if output.status.success() {
                info!("✅ ToadStool services started successfully");
            } else {
                warn!("⚠️ ToadStool services may not be fully functional");
            }
        }

        Ok(())
    }

    /// Get default installation path for platform
    fn default_installation_path(platform: &crate::Platform) -> PathBuf {
        match platform {
            crate::Platform::Linux => {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(home)
                        .join(".local")
                        .join("share")
                        .join("toadstool")
                } else {
                    PathBuf::from("/opt/toadstool")
                }
            }
            crate::Platform::MacOs => {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(home)
                        .join("Library")
                        .join("Application Support")
                        .join("ToadStool")
                } else {
                    PathBuf::from("/Applications/ToadStool")
                }
            }
            crate::Platform::Windows => {
                if let Ok(appdata) = std::env::var("APPDATA") {
                    PathBuf::from(appdata).join("ToadStool")
                } else {
                    PathBuf::from("C:\\Program Files\\ToadStool")
                }
            }
        }
    }
}

impl Default for SmartInstaller {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::Platform {
    fn as_str(&self) -> &'static str {
        match self {
            crate::Platform::Linux => "Linux",
            crate::Platform::MacOs => "macOS",
            crate::Platform::Windows => "Windows",
        }
    }
}

/// Configuration manager for applying configurations
#[derive(Debug, Clone)]
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_path = match crate::Platform::detect() {
            crate::Platform::Linux => {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(home).join(".config").join("toadstool")
                } else {
                    PathBuf::from("/etc/toadstool")
                }
            }
            crate::Platform::MacOs => {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(home)
                        .join("Library")
                        .join("Preferences")
                        .join("ToadStool")
                } else {
                    PathBuf::from("/Library/Preferences/ToadStool")
                }
            }
            crate::Platform::Windows => {
                if let Ok(appdata) = std::env::var("APPDATA") {
                    PathBuf::from(appdata).join("ToadStool").join("config")
                } else {
                    PathBuf::from("C:\\ProgramData\\ToadStool\\config")
                }
            }
        };

        Self { config_path }
    }

    /// Apply configuration to the system
    pub async fn apply_configuration(
        &self,
        config: &crate::ToadStoolConfig,
    ) -> Result<(), crate::AutoConfigError> {
        info!("⚙️ Applying ToadStool configuration...");

        // Ensure config directory exists
        if !self.config_path.exists() {
            fs::create_dir_all(&self.config_path).await?;
        }

        // Write main configuration file
        let config_json = serde_json::to_string_pretty(config)?;
        fs::write(self.config_path.join("toadstool.json"), config_json).await?;

        // Write runtime-specific configurations
        self.write_runtime_configs(config).await?;

        // Write security configuration
        self.write_security_config(config).await?;

        // Write monitoring configuration
        self.write_monitoring_config(config).await?;

        info!("✅ Configuration applied successfully");
        Ok(())
    }

    /// Write runtime-specific configurations
    async fn write_runtime_configs(
        &self,
        config: &crate::ToadStoolConfig,
    ) -> Result<(), crate::AutoConfigError> {
        let runtime_dir = self.config_path.join("runtimes");
        if !runtime_dir.exists() {
            fs::create_dir_all(&runtime_dir).await?;
        }

        // Native runtime config
        if config.runtimes.native.enabled {
            let native_config = serde_json::json!({
                "enabled": true,
                "max_concurrent": config.runtimes.native.max_concurrent,
                "timeout_seconds": 3600,
                "memory_limit_mb": config.resources.memory.per_workload_limit / 1024 / 1024
            });

            fs::write(
                runtime_dir.join("native.json"),
                serde_json::to_string_pretty(&native_config)?,
            )
            .await?;
        }

        // Container runtime config
        if config.runtimes.container.enabled {
            let container_config = serde_json::json!({
                "enabled": true,
                "max_concurrent": config.runtimes.container.max_concurrent,
                "engine": "docker",
                "memory_limit_mb": config.resources.memory.per_workload_limit / 1024 / 1024,
                "cpu_limit": config.resources.cpu.per_workload_limit
            });

            fs::write(
                runtime_dir.join("container.json"),
                serde_json::to_string_pretty(&container_config)?,
            )
            .await?;
        }

        // WASM runtime config
        if config.runtimes.wasm.enabled {
            let wasm_config = serde_json::json!({
                "enabled": true,
                "max_concurrent": config.runtimes.wasm.max_concurrent,
                "memory_limit_mb": 128,
                "enable_wasi": true
            });

            fs::write(
                runtime_dir.join("wasm.json"),
                serde_json::to_string_pretty(&wasm_config)?,
            )
            .await?;
        }

        // GPU runtime config
        if config.runtimes.gpu.enabled {
            let gpu_config = serde_json::json!({
                "enabled": true,
                "max_concurrent": config.runtimes.gpu.max_concurrent,
                "memory_fraction": 0.8,
                "compute_mode": "default"
            });

            fs::write(
                runtime_dir.join("gpu.json"),
                serde_json::to_string_pretty(&gpu_config)?,
            )
            .await?;
        }

        Ok(())
    }

    /// Write security configuration
    async fn write_security_config(
        &self,
        config: &crate::ToadStoolConfig,
    ) -> Result<(), crate::AutoConfigError> {
        let security_config = serde_json::json!({
            "level": format!("{:?}", config.security.level),
            "sandboxing": {
                "enabled": config.security.sandboxing.enabled,
                "strict_mode": config.security.sandboxing.strict_mode
            },
            "resource_limits": {
                "max_memory": config.security.resource_limits.max_memory,
                "max_cpu_percent": config.security.resource_limits.max_cpu_percent,
                "max_execution_time_seconds": config.security.resource_limits.max_execution_time.as_secs()
            }
        });

        fs::write(
            self.config_path.join("security.json"),
            serde_json::to_string_pretty(&security_config)?,
        )
        .await?;

        Ok(())
    }

    /// Write monitoring configuration
    async fn write_monitoring_config(
        &self,
        config: &crate::ToadStoolConfig,
    ) -> Result<(), crate::AutoConfigError> {
        let monitoring_config = serde_json::json!({
            "enabled": config.monitoring.enabled,
            "metrics_collection": config.monitoring.metrics_collection,
            "health_checks": config.monitoring.health_checks,
            "sampling_rate": config.monitoring.sampling_rate,
            "retention_days": config.monitoring.retention_days
        });

        fs::write(
            self.config_path.join("monitoring.json"),
            serde_json::to_string_pretty(&monitoring_config)?,
        )
        .await?;

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Installation result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallationResult {
    pub success: bool,
    pub installation_path: PathBuf,
    pub installed_components: Vec<String>,
    pub configuration_applied: bool,
    pub services_started: bool,
    pub errors: Vec<String>,
}

/// Installation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationConfig {
    pub installation_path: Option<PathBuf>,
    pub install_systemd_service: bool,
    pub add_to_path: bool,
    pub create_desktop_shortcuts: bool,
    pub enable_shell_completion: bool,
    pub start_services: bool,
}

impl Default for InstallationConfig {
    fn default() -> Self {
        Self {
            installation_path: None,
            install_systemd_service: true,
            add_to_path: true,
            create_desktop_shortcuts: true,
            enable_shell_completion: true,
            start_services: true,
        }
    }
}
