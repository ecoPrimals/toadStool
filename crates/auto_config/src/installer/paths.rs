// SPDX-License-Identifier: AGPL-3.0-only
//! Platform-specific path resolution for installation

use std::path::PathBuf;

use toadstool_common::platform_paths::Platform;

/// Get default installation path for platform
pub fn default_installation_path(platform: Platform) -> PathBuf {
    match platform {
        Platform::Linux | Platform::Android | Platform::Wasm | Platform::Unknown => {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("toadstool")
            } else {
                PathBuf::from("/opt/toadstool")
            }
        }
        Platform::MacOS => {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("ToadStool")
            } else {
                PathBuf::from("/Applications/ToadStool")
            }
        }
        Platform::Windows => {
            if let Ok(appdata) = std::env::var("APPDATA") {
                PathBuf::from(appdata).join("ToadStool")
            } else {
                PathBuf::from("C:\\Program Files\\ToadStool")
            }
        }
    }
}

/// Get config path for platform
pub fn config_path_for_platform(platform: Platform) -> PathBuf {
    match platform {
        Platform::Linux | Platform::Android | Platform::Wasm | Platform::Unknown => {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".config").join("toadstool")
            } else {
                PathBuf::from("/etc/toadstool")
            }
        }
        Platform::MacOS => {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home)
                    .join("Library")
                    .join("Preferences")
                    .join("ToadStool")
            } else {
                PathBuf::from("/Library/Preferences/ToadStool")
            }
        }
        Platform::Windows => {
            if let Ok(appdata) = std::env::var("APPDATA") {
                PathBuf::from(appdata).join("ToadStool").join("config")
            } else {
                PathBuf::from("C:\\ProgramData\\ToadStool\\config")
            }
        }
    }
}

/// Platform display name
pub fn platform_as_str(platform: Platform) -> &'static str {
    match platform {
        Platform::Linux => "Linux",
        Platform::MacOS => "macOS",
        Platform::Windows => "Windows",
        Platform::Android => "Android",
        Platform::Wasm => "WASM",
        Platform::Unknown => "Unknown",
    }
}
