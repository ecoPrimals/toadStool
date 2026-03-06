// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform-specific installation components (systemd, launchd, Windows service)

use std::path::Path;

use tokio::fs;
use tracing::info;

use toadstool_common::platform_paths::Platform;

use crate::ToadStoolError;

/// Install Linux-specific components (systemd service)
pub async fn install_linux_components(installation_path: &Path) -> Result<(), ToadStoolError> {
    info!("🐧 Installing Linux-specific components...");

    let service_content = format!(
        r"[Unit]
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
",
        installation_path.display()
    );

    let systemd_dir = installation_path.join("systemd");
    if !systemd_dir.exists() {
        fs::create_dir_all(&systemd_dir).await?;
    }

    fs::write(systemd_dir.join("toadstool.service"), service_content).await?;

    info!("🐧 Linux components installed");
    Ok(())
}

/// Install macOS-specific components (launchd plist)
pub async fn install_macos_components(installation_path: &Path) -> Result<(), ToadStoolError> {
    info!("🍎 Installing macOS-specific components...");

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
        installation_path.display(),
        installation_path.display(),
        installation_path.display()
    );

    let launchd_dir = installation_path.join("launchd");
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

/// Install Windows-specific components (service configuration)
pub async fn install_windows_components(installation_path: &Path) -> Result<(), ToadStoolError> {
    info!("🪟 Installing Windows-specific components...");

    let service_config = serde_json::json!({
        "service_name": "ToadStool",
        "display_name": "ToadStool Universal Compute Platform",
        "description": "Universal compute platform for workload execution",
        "executable": format!("{}/bin/toadstool.exe", installation_path.display()),
        "arguments": ["daemon"],
        "start_type": "automatic"
    });

    let service_dir = installation_path.join("service");
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

/// Install platform-specific components based on detected platform
pub async fn install_platform_components(
    platform: Platform,
    installation_path: &Path,
) -> Result<(), ToadStoolError> {
    match platform {
        Platform::Linux => install_linux_components(installation_path).await,
        Platform::MacOS => install_macos_components(installation_path).await,
        Platform::Windows => install_windows_components(installation_path).await,
        Platform::Android | Platform::Wasm | Platform::Unknown => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_android_is_noop() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let result = install_platform_components(Platform::Android, dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wasm_is_noop() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let result = install_platform_components(Platform::Wasm, dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unknown_is_noop() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let result = install_platform_components(Platform::Unknown, dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_linux_creates_systemd_service() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let result = install_linux_components(dir.path()).await;
        assert!(result.is_ok());
        let service_file = dir.path().join("systemd/toadstool.service");
        assert!(service_file.exists());
        let content = std::fs::read_to_string(service_file).expect("read");
        assert!(content.contains("[Unit]"));
        assert!(content.contains("ExecStart="));
        assert!(content.contains("toadstool daemon"));
    }

    #[tokio::test]
    async fn test_macos_creates_plist() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let result = install_macos_components(dir.path()).await;
        assert!(result.is_ok());
        let plist_file = dir.path().join("launchd/dev.toadstool.daemon.plist");
        assert!(plist_file.exists());
        let content = std::fs::read_to_string(plist_file).expect("read");
        assert!(content.contains("<plist"));
        assert!(content.contains("dev.toadstool.daemon"));
    }

    #[tokio::test]
    async fn test_windows_creates_service_json() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let result = install_windows_components(dir.path()).await;
        assert!(result.is_ok());
        let json_file = dir.path().join("service/service.json");
        assert!(json_file.exists());
        let content = std::fs::read_to_string(json_file).expect("read");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("json");
        assert_eq!(parsed["service_name"], "ToadStool");
        assert_eq!(parsed["start_type"], "automatic");
    }
}
