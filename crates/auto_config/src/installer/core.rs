// SPDX-License-Identifier: AGPL-3.0-only
//! Core installation: directory structure and toadstool executable

use std::path::Path;

use tokio::fs;
use tracing::info;

use crate::ToadStoolError;

/// Ensure installation directory exists
pub async fn ensure_installation_directory(installation_path: &Path) -> Result<(), ToadStoolError> {
    if !installation_path.exists() {
        info!(
            "📁 Creating installation directory: {}",
            installation_path.display()
        );
        fs::create_dir_all(installation_path).await?;
    }
    Ok(())
}

/// Install core ToadStool components (directories and executable script)
pub async fn install_core_components(installation_path: &Path) -> Result<(), ToadStoolError> {
    info!("🔧 Installing core ToadStool components...");

    let bin_dir = installation_path.join("bin");
    let config_dir = installation_path.join("config");
    let data_dir = installation_path.join("data");
    let logs_dir = installation_path.join("logs");

    for dir in [&bin_dir, &config_dir, &data_dir, &logs_dir] {
        if !dir.exists() {
            fs::create_dir_all(dir).await?;
        }
    }

    // Create a simple toadstool executable script
    let toadstool_script = if cfg!(windows) {
        format!(
            r"@echo off
echo ToadStool Universal Compute Platform
echo Installation: {}
echo.
echo Available commands:
echo   status  - Show system status
echo   config  - Configure ToadStool
echo   run     - Execute workloads
echo   help    - Show help
",
            installation_path.display()
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
            installation_path.display()
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ensure_installation_directory_creates_dir() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("install");
        assert!(!path.exists());
        let result = ensure_installation_directory(&path).await;
        assert!(result.is_ok());
        assert!(path.exists());
        assert!(path.is_dir());
    }

    #[tokio::test]
    async fn test_ensure_installation_directory_existing_dir() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        assert!(path.exists());
        let result = ensure_installation_directory(path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ensure_installation_directory_nested() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("a").join("b").join("c");
        assert!(!path.exists());
        let result = ensure_installation_directory(&path).await;
        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_install_core_components() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let result = install_core_components(path).await;
        assert!(result.is_ok());

        let bin_dir = path.join("bin");
        let config_dir = path.join("config");
        let data_dir = path.join("data");
        let logs_dir = path.join("logs");

        assert!(bin_dir.exists());
        assert!(config_dir.exists());
        assert!(data_dir.exists());
        assert!(logs_dir.exists());

        let script_name = if cfg!(windows) {
            "toadstool.bat"
        } else {
            "toadstool"
        };
        let script_path = bin_dir.join(script_name);
        assert!(script_path.exists());
        let content = tokio::fs::read_to_string(&script_path).await.unwrap();
        assert!(content.contains("ToadStool"));
        assert!(content.contains("status"));
    }

    #[tokio::test]
    async fn test_install_core_components_idempotent() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let r1 = install_core_components(path).await;
        let r2 = install_core_components(path).await;
        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }
}
