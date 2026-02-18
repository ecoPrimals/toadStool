//! UniBin tests

use super::execution::{
    is_platform_constraint_str, is_selinux_enforcing, write_tcp_discovery_file,
};
use super::format::{ensure_biomeos_directory, socket_filename_for_family};
use super::*;

#[test]
fn socket_filename_for_family_default() {
    assert_eq!(socket_filename_for_family("default"), "toadstool.sock");
}

#[test]
fn socket_filename_for_family_empty() {
    assert_eq!(socket_filename_for_family(""), "toadstool.sock");
}

#[test]
fn socket_filename_for_family_custom() {
    assert_eq!(socket_filename_for_family("nat0"), "toadstool-nat0.sock");
}

#[test]
fn socket_filename_for_family_alphanumeric() {
    assert_eq!(
        socket_filename_for_family("family123"),
        "toadstool-family123.sock"
    );
}

#[test]
fn socket_filename_for_family_with_hyphens() {
    assert_eq!(
        socket_filename_for_family("my-family-id"),
        "toadstool-my-family-id.sock"
    );
}

#[test]
fn socket_filename_for_family_special_chars() {
    assert_eq!(
        socket_filename_for_family("dev_env"),
        "toadstool-dev_env.sock"
    );
}

#[test]
fn socket_filename_for_family_whitespace() {
    assert_eq!(
        socket_filename_for_family(" default "),
        "toadstool- default .sock"
    );
}

#[test]
fn is_platform_constraint_str_unsupported() {
    assert!(is_platform_constraint_str("Unsupported"));
    assert!(is_platform_constraint_str("Unix sockets Unsupported"));
}

#[test]
fn is_platform_constraint_str_not_supported() {
    assert!(is_platform_constraint_str("not supported"));
    assert!(is_platform_constraint_str("protocol not supported"));
}

#[test]
fn is_platform_constraint_str_protocol_not_available() {
    assert!(is_platform_constraint_str("protocol not available"));
}

#[test]
fn is_platform_constraint_str_non_matching() {
    assert!(!is_platform_constraint_str("Connection refused"));
    assert!(!is_platform_constraint_str("timeout"));
    assert!(!is_platform_constraint_str(""));
}

#[test]
fn is_platform_constraint_str_permission_denied() {
    let result = is_platform_constraint_str("Permission denied (EACCES)");
    assert_eq!(result, is_selinux_enforcing());
}

#[test]
fn is_platform_constraint_str_operation_not_permitted() {
    let result = is_platform_constraint_str("Operation not permitted");
    assert_eq!(result, is_selinux_enforcing());
}

#[test]
fn is_selinux_enforcing_returns_bool() {
    let _result = is_selinux_enforcing();
}

#[test]
fn ensure_biomeos_directory_creates_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let result = ensure_biomeos_directory(temp_dir.path());
    assert!(result.is_ok());
    let biomeos_dir = result.unwrap();
    assert!(biomeos_dir.exists());
    assert!(biomeos_dir.is_dir());
    assert_eq!(biomeos_dir.file_name().unwrap(), "biomeos");
}

#[test]
fn ensure_biomeos_directory_idempotent() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let result1 = ensure_biomeos_directory(temp_dir.path());
    let result2 = ensure_biomeos_directory(temp_dir.path());
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert_eq!(result1.unwrap(), result2.unwrap());
}

#[cfg(unix)]
#[test]
fn ensure_biomeos_directory_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let biomeos_dir = ensure_biomeos_directory(temp_dir.path()).expect("dir creation succeeded");
    let perms = std::fs::metadata(&biomeos_dir)
        .expect("metadata read")
        .permissions();
    assert_eq!(perms.mode() & 0o777, 0o700);
}

#[test]
fn write_tcp_discovery_file_creates_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_string_lossy().to_string();
    let old_val = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", &temp_path);

    let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().expect("valid addr");
    let result = write_tcp_discovery_file("test-discovery.txt", &addr);

    if let Some(val) = old_val {
        std::env::set_var("XDG_RUNTIME_DIR", val);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    assert!(result.is_ok());
    let file_path = temp_dir.path().join("test-discovery.txt");
    let content = std::fs::read_to_string(&file_path).expect("file read");
    assert_eq!(content, "tcp:127.0.0.1:8080");
}

#[test]
fn write_tcp_discovery_file_ipv6() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_string_lossy().to_string();
    let old_val = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", &temp_path);

    let addr: std::net::SocketAddr = "[::1]:9000".parse().expect("valid addr");
    let result = write_tcp_discovery_file("ipv6-discovery.txt", &addr);

    if let Some(val) = old_val {
        std::env::set_var("XDG_RUNTIME_DIR", val);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    assert!(result.is_ok());
    let file_path = temp_dir.path().join("ipv6-discovery.txt");
    let content = std::fs::read_to_string(&file_path).expect("file read");
    assert_eq!(content, "tcp:[::1]:9000");
}

#[test]
fn exit_codes_values() {
    assert_eq!(exit_codes::SUCCESS, 0);
    assert_eq!(exit_codes::GENERAL_ERROR, 1);
    assert_eq!(exit_codes::CONFIG_ERROR, 2);
    assert_eq!(exit_codes::RUNTIME_ERROR, 3);
    assert_eq!(exit_codes::INTERRUPTED, 130);
}
