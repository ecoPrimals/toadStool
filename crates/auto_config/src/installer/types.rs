// SPDX-License-Identifier: AGPL-3.0-only
//! Installation types and configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Result of a ToadStool installation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallationResult {
    /// Whether installation succeeded.
    pub success: bool,
    /// Path where ToadStool was installed.
    pub installation_path: PathBuf,
    /// List of installed components.
    pub installed_components: Vec<String>,
    /// Whether configuration was applied.
    pub configuration_applied: bool,
    /// Whether services were started.
    pub services_started: bool,
    /// Any errors encountered.
    pub errors: Vec<String>,
}

/// Installation configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::struct_excessive_bools, reason = "configuration type")]
pub struct InstallationConfig {
    /// Custom installation path (None = default).
    pub installation_path: Option<PathBuf>,
    /// Install systemd service.
    pub install_systemd_service: bool,
    /// Add ToadStool to PATH.
    pub add_to_path: bool,
    /// Create desktop shortcuts.
    pub create_desktop_shortcuts: bool,
    /// Enable shell completion.
    pub enable_shell_completion: bool,
    /// Start services after install.
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
