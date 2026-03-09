// SPDX-License-Identifier: AGPL-3.0-only
//! Installation types and configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
