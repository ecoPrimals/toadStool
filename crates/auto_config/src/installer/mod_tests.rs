// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use toadstool_common::platform_paths::Platform;

#[test]
fn test_smart_installer_creation() {
    let installer = SmartInstaller::new();
    let default_path = paths::default_installation_path(Platform::detect());
    assert!(!default_path.as_os_str().is_empty());
    let _ = installer;
}

#[test]
fn test_default_installation_path() {
    let linux_path = paths::default_installation_path(Platform::Linux);
    assert!(linux_path.to_string_lossy().contains("toadstool"));

    let mac_path = paths::default_installation_path(Platform::MacOS);
    assert!(mac_path.to_string_lossy().contains("ToadStool"));

    let win_path = paths::default_installation_path(Platform::Windows);
    assert!(win_path.to_string_lossy().contains("ToadStool"));
}

#[test]
fn test_platform_as_str() {
    assert_eq!(paths::platform_as_str(Platform::Linux), "Linux");
    assert_eq!(paths::platform_as_str(Platform::MacOS), "macOS");
    assert_eq!(paths::platform_as_str(Platform::Windows), "Windows");
}

#[test]
fn test_config_manager_creation() {
    let _manager = ConfigManager::new();
}

#[test]
fn test_config_path_for_platform() {
    let linux_path = paths::config_path_for_platform(Platform::Linux);
    assert!(linux_path.to_string_lossy().contains("toadstool"));

    let mac_path = paths::config_path_for_platform(Platform::MacOS);
    assert!(mac_path.to_string_lossy().contains("ToadStool"));

    let win_path = paths::config_path_for_platform(Platform::Windows);
    assert!(win_path.to_string_lossy().contains("ToadStool"));
}

#[test]
fn test_installation_config_default() {
    let config = InstallationConfig::default();
    assert!(config.add_to_path);
    assert!(config.enable_shell_completion);
}

#[test]
fn test_installation_result_default() {
    let result = InstallationResult::default();
    assert!(!result.success);
    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_ensure_installation_directory_creates_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let install_path = dir.path().join("toadstool");
    assert!(!install_path.exists());

    let result = core::ensure_installation_directory(&install_path).await;
    assert!(result.is_ok());
    assert!(install_path.exists());
    assert!(install_path.is_dir());
}

#[tokio::test]
async fn test_install_core_components_creates_structure() {
    use toadstool_common::constants::primal_identity::PRIMAL_BINARY_NAME;

    let dir = tempfile::tempdir().expect("tempdir");
    let install_path = dir.path().to_path_buf();

    let result = core::install_core_components(&install_path).await;
    assert!(result.is_ok());

    assert!(install_path.join("bin").exists());
    assert!(install_path.join("config").exists());
    assert!(install_path.join("data").exists());
    assert!(install_path.join("logs").exists());

    let script_name = if cfg!(windows) {
        format!("{PRIMAL_BINARY_NAME}.bat")
    } else {
        PRIMAL_BINARY_NAME.to_string()
    };
    assert!(install_path.join("bin").join(script_name).exists());
}

#[tokio::test]
async fn test_paths_default_installation_with_home() {
    temp_env::with_var("HOME", Some("/tmp/test-home"), || {
        let path = paths::default_installation_path(Platform::Linux);
        assert!(path.to_string_lossy().contains("toadstool"));
        assert!(path.to_string_lossy().contains("test-home"));
    });
}

#[tokio::test]
async fn test_paths_config_path_with_home() {
    temp_env::with_var("HOME", Some("/tmp/test-home"), || {
        let path = paths::config_path_for_platform(Platform::Linux);
        assert!(path.to_string_lossy().contains("toadstool"));
        assert!(path.to_string_lossy().contains("config"));
    });
}
