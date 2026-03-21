// SPDX-License-Identifier: AGPL-3.0-only
//! Platform-specific path resolution for installation

use std::path::PathBuf;

use toadstool_common::platform_paths::Platform;

/// Get default installation path for platform
pub fn default_installation_path(platform: Platform) -> PathBuf {
    match platform {
        Platform::Linux | Platform::Android | Platform::Wasm | Platform::Unknown => {
            std::env::var("HOME").map_or_else(
                |_| PathBuf::from("/opt/toadstool"),
                |home| {
                    PathBuf::from(home)
                        .join(".local")
                        .join("share")
                        .join("toadstool")
                },
            )
        }
        Platform::MacOS => std::env::var("HOME").map_or_else(
            |_| PathBuf::from("/Applications/ToadStool"),
            |home| {
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("ToadStool")
            },
        ),
        Platform::Windows => std::env::var("APPDATA").map_or_else(
            |_| PathBuf::from("C:\\Program Files\\ToadStool"),
            |appdata| PathBuf::from(appdata).join("ToadStool"),
        ),
    }
}

/// Get config path for platform
pub fn config_path_for_platform(platform: Platform) -> PathBuf {
    match platform {
        Platform::Linux | Platform::Android | Platform::Wasm | Platform::Unknown => {
            std::env::var("HOME").map_or_else(
                |_| PathBuf::from("/etc/toadstool"),
                |home| PathBuf::from(home).join(".config").join("toadstool"),
            )
        }
        Platform::MacOS => std::env::var("HOME").map_or_else(
            |_| PathBuf::from("/Library/Preferences/ToadStool"),
            |home| {
                PathBuf::from(home)
                    .join("Library")
                    .join("Preferences")
                    .join("ToadStool")
            },
        ),
        Platform::Windows => std::env::var("APPDATA").map_or_else(
            |_| PathBuf::from("C:\\ProgramData\\ToadStool\\config"),
            |appdata| PathBuf::from(appdata).join("ToadStool").join("config"),
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
