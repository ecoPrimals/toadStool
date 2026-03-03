// SPDX-License-Identifier: AGPL-3.0-or-later
//! discovery_directory and default_socket_path tests.
use super::*;

#[test]
fn test_discovery_directory_structure() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path();
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", base);

    let dir = super::super::discovery_directory();
    assert!(dir.ends_with("ecoPrimals/discovery") || dir.to_string_lossy().contains("ecoPrimals"));
    assert!(dir.to_string_lossy().contains("discovery"));

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
}

#[test]
fn test_discovery_directory_fallback_when_xdg_unset() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::remove_var("XDG_RUNTIME_DIR");

    let dir = super::super::discovery_directory();
    assert!(dir.to_string_lossy().contains("ecoPrimals"));
    assert!(dir.to_string_lossy().contains("discovery"));
    // Fallback is /tmp when XDG_RUNTIME_DIR not set
    assert!(dir.starts_with("/tmp") || dir.to_string_lossy().starts_with("/tmp"));

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    }
}

#[test]
fn test_default_socket_path_format() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");

    let path = super::super::default_socket_path("my-primal-id");
    assert!(path.ends_with("my-primal-id.sock"));
    assert!(path.to_string_lossy().contains("ecoPrimals"));
    assert!(path.to_string_lossy().contains("sockets"));

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
}

#[test]
fn test_default_socket_path_fallback() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::remove_var("XDG_RUNTIME_DIR");

    let path = super::super::default_socket_path("test-id");
    assert!(path.ends_with("test-id.sock"));
    assert!(path.to_string_lossy().contains("/tmp"));

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    }
}
