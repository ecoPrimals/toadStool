// SPDX-License-Identifier: AGPL-3.0-only
//! Platform detection and discovery for IPC
//!
//! Handles platform constraints (`SELinux`, Unix socket support) and
//! capability-based discovery (socket paths, TCP fallback discovery files).

use crate::DisplayError;
use std::net::SocketAddr;
use std::path::PathBuf;

/// Discover socket path from environment
///
/// **Capability-based**: Uses `XDG_RUNTIME_DIR`, no hardcoding!
#[must_use]
pub fn discover_socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());

    #[allow(deprecated)]
    let primal_name = toadstool_common::interned_strings::primals::TOADSTOOL;
    let mut path = PathBuf::from(runtime_dir);
    path.push(primal_name);
    path.push("display.sock");

    path
}

/// Detect platform constraints (not real errors!)
///
/// Platform constraints should trigger TCP fallback, not failure.
#[must_use]
pub fn is_platform_constraint(error: &DisplayError) -> bool {
    let error_str = error.to_string();

    // Check for permission denied + SELinux
    if error_str.contains("Permission denied") && is_selinux_enforcing() {
        tracing::debug!("   Platform constraint: SELinux enforcing (Android?)");
        return true;
    }

    // Check for unsupported operation (platform lacks Unix sockets)
    if error_str.contains("Unsupported") || error_str.contains("not supported") {
        tracing::debug!("   Platform constraint: Unix sockets not supported");
        return true;
    }

    false
}

/// Check if `SELinux` is enforcing (common on Android)
#[must_use]
pub fn is_selinux_enforcing() -> bool {
    std::fs::read_to_string("/sys/fs/selinux/enforce")
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .is_some_and(|v| v == 1)
}

/// Write TCP discovery file for clients
///
/// **XDG-compliant**: Tries `XDG_RUNTIME_DIR`, `HOME`, `/tmp`
pub fn write_tcp_discovery_file(addr: &SocketAddr) {
    let discovery_dirs: Vec<Option<String>> = vec![
        std::env::var("XDG_RUNTIME_DIR").ok(),
        std::env::var("HOME")
            .ok()
            .map(|h| format!("{h}/.local/share")),
        Some("/tmp".to_string()),
    ];

    for dir in discovery_dirs.iter().filter_map(|d| d.as_ref()) {
        if matches!(std::fs::create_dir_all(dir), Ok(())) {
            let discovery_file = format!("{dir}/toadstool-ipc-port");

            if let Ok(mut f) = std::fs::File::create(&discovery_file) {
                use std::io::Write;
                writeln!(f, "tcp:{addr}").ok();
                tracing::info!("📁 TCP discovery file: {}", discovery_file);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_discovery() {
        let path = discover_socket_path();
        assert!(path.to_string_lossy().contains("toadstool"));
        assert!(path.to_string_lossy().ends_with("display.sock"));
    }

    #[test]
    fn test_socket_path_has_toadstool_and_display_sock() {
        let path = discover_socket_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("toadstool"),
            "path should contain toadstool: {path_str}"
        );
        assert!(
            path_str.ends_with("display.sock"),
            "path should end with display.sock: {path_str}"
        );
    }

    #[test]
    fn test_discover_socket_path_format() {
        let path = discover_socket_path();
        let components: Vec<_> = path.components().collect();
        assert!(!components.is_empty());
        assert_eq!(path.file_name().unwrap(), "display.sock");
    }

    #[test]
    fn test_is_platform_constraint_permission_denied() {
        let err = DisplayError::IpcError("Permission denied".to_string());
        let result = is_platform_constraint(&err);
        assert!(!result);
    }

    #[test]
    fn test_is_platform_constraint_unsupported() {
        let err = DisplayError::IpcError("Unsupported operation".to_string());
        let result = is_platform_constraint(&err);
        assert!(result);
    }

    #[test]
    fn test_is_platform_constraint_not_supported() {
        let err = DisplayError::IpcError("not supported".to_string());
        let result = is_platform_constraint(&err);
        assert!(result);
    }

    #[test]
    fn test_is_platform_constraint_other_error() {
        let err = DisplayError::IpcError("Connection refused".to_string());
        let result = is_platform_constraint(&err);
        assert!(!result);
    }

    #[test]
    fn test_write_tcp_discovery_file_via_display_server() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg_path = tmp.path().to_string_lossy().to_string();
        temp_env::with_var("XDG_RUNTIME_DIR", Some(xdg_path.as_str()), || {
            let addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
            write_tcp_discovery_file(&addr);
            let discovery_file = tmp.path().join("toadstool-ipc-port");
            assert!(discovery_file.exists(), "discovery file should be created");
            let content = std::fs::read_to_string(&discovery_file).unwrap();
            assert!(content.starts_with("tcp:"));
            assert!(content.contains("127.0.0.1"));
            assert!(content.contains("9999"));
        });
    }

    #[test]
    fn test_write_tcp_discovery_file_fallback_to_home() {
        let tmp = tempfile::tempdir().unwrap();
        let home_local = tmp.path().join(".local/share");
        std::fs::create_dir_all(&home_local).unwrap();
        let home_path = tmp.path().to_string_lossy().to_string();
        temp_env::with_vars(
            [
                ("XDG_RUNTIME_DIR", None::<&str>),
                ("HOME", Some(home_path.as_str())),
            ],
            || {
                let addr: SocketAddr = "127.0.0.1:8888".parse().unwrap();
                write_tcp_discovery_file(&addr);
                let discovery_file = home_local.join("toadstool-ipc-port");
                assert!(discovery_file.exists());
                let content = std::fs::read_to_string(&discovery_file).unwrap();
                assert!(content.contains("tcp:"));
            },
        );
    }
}
