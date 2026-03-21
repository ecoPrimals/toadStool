// SPDX-License-Identifier: AGPL-3.0-only
//! Coverage tests for installer: creation, platform detection, installation steps, error handling

#![allow(clippy::pedantic)]

use std::path::PathBuf;

use toadstool_auto_config::ToadStoolError;
use toadstool_auto_config::installer::{
    ConfigManager, InstallationConfig, InstallationResult, SmartInstaller,
};
use toadstool_common::platform_paths::Platform;

#[test]
fn test_installer_smart_installer_new() {
    let installer = SmartInstaller::new();
    let _ = installer; // Construction succeeds
}

#[test]
fn test_installer_smart_installer_default() {
    let installer = SmartInstaller::default();
    let _ = installer; // Construction succeeds
}

#[test]
fn test_installer_config_manager_new() {
    let manager = ConfigManager::new();
    let _ = manager; // Construction succeeds
}

#[test]
fn test_installer_config_manager_with_path() {
    let path = PathBuf::from("/tmp/test-toadstool-config-coverage");
    let manager = ConfigManager::with_path(path);
    let _ = manager; // with_path creates manager with custom path
}

#[test]
fn test_installer_config_manager_default() {
    let _manager = ConfigManager::default();
}

#[test]
fn test_installer_installation_config_default() {
    let config = InstallationConfig::default();
    assert!(config.add_to_path);
    assert!(config.enable_shell_completion);
    assert!(config.install_systemd_service);
    assert!(config.create_desktop_shortcuts);
    assert!(config.start_services);
}

#[test]
fn test_installer_installation_result_default() {
    let result = InstallationResult::default();
    assert!(!result.success);
    assert!(result.errors.is_empty());
    assert!(result.installed_components.is_empty());
}

#[test]
fn test_installer_installation_result_with_values() {
    let result = InstallationResult {
        success: true,
        installation_path: PathBuf::from("/opt/toadstool"),
        installed_components: vec!["bin".to_string(), "config".to_string()],
        configuration_applied: true,
        services_started: true,
        errors: vec![],
    };
    assert!(result.success);
    assert_eq!(result.installed_components.len(), 2);
}

#[test]
fn test_installer_platform_detection() {
    let platform = Platform::detect();
    let name = match platform {
        Platform::Linux => "linux",
        Platform::MacOS => "macos",
        Platform::Windows => "windows",
        Platform::Android => "android",
        Platform::Wasm => "wasm",
        Platform::Unknown => "unknown",
    };
    assert!(!name.is_empty());
}

#[test]
fn test_installer_toadstool_error_display() {
    let err = ToadStoolError::Configuration("test error".to_string());
    assert!(err.to_string().contains("Configuration"));
    assert!(err.to_string().contains("test error"));
}

#[tokio::test]
async fn test_installer_config_manager_apply_creates_files() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config_path = temp_dir.path().to_path_buf();
    let manager = ConfigManager::with_path(config_path.clone());

    let config = toadstool_config::ToadStoolConfig::default();
    let result = manager.apply_configuration(&config).await;
    assert!(result.is_ok());

    assert!(config_path.join("toadstool.json").exists());
    assert!(config_path.join("runtimes").exists());
    assert!(config_path.join("security.json").exists());
}

#[tokio::test]
async fn test_installer_config_manager_apply_with_gpu() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config_path = temp_dir.path().to_path_buf();
    let manager = ConfigManager::with_path(config_path.clone());

    let mut config = toadstool_config::ToadStoolConfig::default();
    config.runtime.gpu = Some(toadstool_config::GpuConfig::default());

    let result = manager.apply_configuration(&config).await;
    assert!(result.is_ok());
    assert!(config_path.join("runtimes").join("gpu.json").exists());
}

#[test]
fn test_installer_smart_installer_construction() {
    let installer = SmartInstaller::new();
    let _ = installer; // Construction succeeds with platform detection
}
