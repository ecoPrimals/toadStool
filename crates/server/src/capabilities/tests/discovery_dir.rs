// SPDX-License-Identifier: AGPL-3.0-or-later
//! discovery_directory and default_socket_path tests.
use super::super::paths::{default_socket_path, discovery_directory};
use super::*;

#[test]
fn test_discovery_directory_structure() {
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path().to_string_lossy().to_string();
    temp_env::with_var("XDG_RUNTIME_DIR", Some(base.as_str()), || {
        let dir = discovery_directory();
        assert!(
            dir.ends_with("ecoPrimals/discovery") || dir.to_string_lossy().contains("ecoPrimals")
        );
        assert!(dir.to_string_lossy().contains("discovery"));
    });
}

#[test]
fn test_discovery_directory_fallback_when_xdg_unset() {
    temp_env::with_var_unset("XDG_RUNTIME_DIR", || {
        let dir = discovery_directory();
        assert!(dir.to_string_lossy().contains("ecoPrimals"));
        assert!(dir.to_string_lossy().contains("discovery"));
        assert!(dir.starts_with("/tmp") || dir.to_string_lossy().starts_with("/tmp"));
    });
}

#[test]
fn test_default_socket_path_format() {
    temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
        let path = default_socket_path("my-primal-id");
        assert!(path.ends_with("my-primal-id.sock"));
        assert!(path.to_string_lossy().contains("ecoPrimals"));
        assert!(path.to_string_lossy().contains("sockets"));
    });
}

#[test]
fn test_default_socket_path_fallback() {
    temp_env::with_var_unset("XDG_RUNTIME_DIR", || {
        let path = default_socket_path("test-id");
        assert!(path.ends_with("test-id.sock"));
        assert!(path.to_string_lossy().contains("/tmp"));
    });
}
