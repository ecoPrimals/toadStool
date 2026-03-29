// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

use super::{PathEnv, Platform, PlatformPaths};

#[test]
fn test_runtime_dir_with_xdg() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(paths.runtime_dir(), PathBuf::from("/run/user/1000"));
}

#[test]
fn test_runtime_dir_fallback() {
    let env = PathEnv {
        xdg_runtime_dir: None,
        user: Some("testuser".to_string()),
        tmpdir: Some("/tmp".to_string()),
        platform: Platform::Linux,
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    let runtime = paths.runtime_dir();
    assert!(
        runtime
            .to_string_lossy()
            .contains("toadstool-runtime-testuser")
    );
}

#[test]
fn test_toadstool_socket_dir() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.toadstool_socket_dir(),
        PathBuf::from("/run/user/1000/biomeos")
    );
}

#[test]
fn test_toadstool_socket() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.toadstool_socket(),
        PathBuf::from("/run/user/1000/biomeos/toadstool.sock")
    );
}

#[test]
fn test_data_dir_with_xdg() {
    let env = PathEnv {
        xdg_data_home: Some("/home/user/.local/share".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(paths.data_dir(), PathBuf::from("/home/user/.local/share"));
}

#[test]
fn test_primal_socket() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.primal_socket("crypto"),
        PathBuf::from("/run/user/1000/biomeos/crypto.sock")
    );
}

#[test]
fn test_temp_dir_override() {
    let env = PathEnv {
        tmpdir: Some("/custom/tmp".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(paths.temp_dir(), PathBuf::from("/custom/tmp"));
}

#[test]
fn test_platform_detection() {
    let platform = Platform::detect();
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    assert_ne!(platform, Platform::Unknown);
}

#[test]
fn test_runtime_dir_android_platform() {
    let env = PathEnv {
        xdg_runtime_dir: None,
        user: Some("android_user".to_string()),
        tmpdir: Some("/data/local/tmp".to_string()),
        platform: Platform::Android,
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    let runtime = paths.runtime_dir();
    assert!(
        runtime
            .to_string_lossy()
            .contains("toadstool-runtime-android_user")
    );
}

#[test]
fn test_runtime_dir_windows_platform() {
    let env = PathEnv {
        xdg_runtime_dir: None,
        user: Some("winuser".to_string()),
        tmpdir: Some("C:\\Temp".to_string()),
        platform: Platform::Windows,
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    let runtime = paths.runtime_dir();
    assert!(runtime.to_string_lossy().contains("toadstool-winuser"));
}

#[test]
fn test_runtime_dir_wasm_platform() {
    let env = PathEnv {
        xdg_runtime_dir: None,
        platform: Platform::Wasm,
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    let runtime = paths.runtime_dir();
    assert_eq!(runtime, PathBuf::from("/virtual/toadstool"));
}

#[test]
fn test_data_dir_linux_with_home() {
    let env = PathEnv {
        xdg_data_home: None,
        home: Some("/home/user".to_string()),
        platform: Platform::Linux,
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(paths.data_dir(), PathBuf::from("/home/user/.local/share"));
}

#[test]
fn test_cache_dir_with_xdg() {
    let env = PathEnv {
        xdg_cache_home: Some("/home/user/.cache".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(paths.cache_dir(), PathBuf::from("/home/user/.cache"));
}

#[test]
fn test_config_dir_with_xdg() {
    let env = PathEnv {
        xdg_config_home: Some("/home/user/.config".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(paths.config_dir(), PathBuf::from("/home/user/.config"));
}

#[test]
fn test_path_env_from_env() {
    let env = PathEnv::from_env();
    let _ = format!("{env:?}");
}

#[test]
fn test_path_env_test_env() {
    let env = PathEnv::test_env();
    assert!(env.xdg_runtime_dir.is_some());
    assert_eq!(env.user.as_deref(), Some("testuser"));
}

#[test]
fn test_display_socket() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.display_socket(),
        PathBuf::from("/run/user/1000/biomeos/display.sock")
    );
}

#[test]
fn test_ipc_port_file() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    let port_file = paths.ipc_port_file();
    assert!(port_file.to_string_lossy().contains("toadstool-ipc-port"));
}
