// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform-specific path resolution for installation

use std::path::PathBuf;

use toadstool_common::constants::platform_paths::{etc_paths, install_paths};
use toadstool_common::constants::primal_identity::{PRIMAL_DISPLAY_NAME, PRIMAL_NAME};
use toadstool_common::interned_strings::socket_env;
use toadstool_common::platform_paths::Platform;

/// Get default installation path for platform
pub fn default_installation_path(platform: Platform) -> PathBuf {
    match platform {
        Platform::Linux | Platform::Android | Platform::Wasm | Platform::Unknown => {
            std::env::var(socket_env::HOME).map_or_else(
                |_| PathBuf::from(install_paths::OPT_TOADSTOOL),
                |home| {
                    PathBuf::from(home)
                        .join(".local")
                        .join("share")
                        .join(PRIMAL_NAME)
                },
            )
        }
        Platform::MacOS => std::env::var(socket_env::HOME).map_or_else(
            |_| PathBuf::from(format!("/Applications/{PRIMAL_DISPLAY_NAME}")),
            |home| {
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join(PRIMAL_DISPLAY_NAME)
            },
        ),
        Platform::Windows => std::env::var(socket_env::APPDATA).map_or_else(
            |_| PathBuf::from(format!("C:\\Program Files\\{PRIMAL_DISPLAY_NAME}")),
            |appdata| PathBuf::from(appdata).join(PRIMAL_DISPLAY_NAME),
        ),
    }
}

/// Get config path for platform
pub fn config_path_for_platform(platform: Platform) -> PathBuf {
    match platform {
        Platform::Linux | Platform::Android | Platform::Wasm | Platform::Unknown => {
            std::env::var(socket_env::HOME).map_or_else(
                |_| PathBuf::from(etc_paths::TOADSTOOL_DIR),
                |home| PathBuf::from(home).join(".config").join(PRIMAL_NAME),
            )
        }
        Platform::MacOS => std::env::var(socket_env::HOME).map_or_else(
            |_| PathBuf::from(format!("/Library/Preferences/{PRIMAL_DISPLAY_NAME}")),
            |home| {
                PathBuf::from(home)
                    .join("Library")
                    .join("Preferences")
                    .join(PRIMAL_DISPLAY_NAME)
            },
        ),
        Platform::Windows => std::env::var(socket_env::APPDATA).map_or_else(
            |_| PathBuf::from(format!("C:\\ProgramData\\{PRIMAL_DISPLAY_NAME}\\config")),
            |appdata| {
                PathBuf::from(appdata)
                    .join(PRIMAL_DISPLAY_NAME)
                    .join("config")
            },
        ),
    }
}

/// Platform display name
pub const fn platform_as_str(platform: Platform) -> &'static str {
    match platform {
        Platform::Linux => "Linux",
        Platform::MacOS => "macOS",
        Platform::Windows => "Windows",
        Platform::Android => "Android",
        Platform::Wasm => "WASM",
        Platform::Unknown => "Unknown",
    }
}
