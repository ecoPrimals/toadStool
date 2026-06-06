// SPDX-License-Identifier: AGPL-3.0-or-later
//! UniBin tests
//!
//! Covers configuration/parsing paths, validation, and logic that does not
//! require actual network binding.

use super::execution::{
    UnibinExecutionConfig, create_executor, is_platform_constraint_str, is_selinux_enforcing,
    write_tcp_discovery_file,
};
use super::format::{ensure_biomeos_directory, get_socket_path, socket_filename_for_family};
use super::*;
use super::{resolve_family_id, resolve_node_id};

#[test]
fn socket_filename_for_family_default() {
    assert_eq!(socket_filename_for_family("default"), "compute.sock");
}

#[test]
fn socket_filename_for_family_empty() {
    assert_eq!(socket_filename_for_family(""), "compute.sock");
}

#[test]
fn socket_filename_for_family_custom() {
    assert_eq!(socket_filename_for_family("nat0"), "compute-nat0.sock");
}

#[test]
fn socket_filename_for_family_alphanumeric() {
    assert_eq!(
        socket_filename_for_family("family123"),
        "compute-family123.sock"
    );
}

#[test]
fn socket_filename_for_family_with_hyphens() {
    assert_eq!(
        socket_filename_for_family("my-family-id"),
        "compute-my-family-id.sock"
    );
}

#[test]
fn socket_filename_for_family_special_chars() {
    assert_eq!(
        socket_filename_for_family("dev_env"),
        "compute-dev_env.sock"
    );
}

#[test]
fn socket_filename_for_family_whitespace() {
    assert_eq!(
        socket_filename_for_family(" default "),
        "compute- default .sock"
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
    temp_env::with_var("XDG_RUNTIME_DIR", Some(temp_path.as_str()), || {
        let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().expect("valid addr");
        let result = write_tcp_discovery_file("test-discovery.txt", &addr);
        assert!(result.is_ok());
        let file_path = temp_dir.path().join("test-discovery.txt");
        let content = std::fs::read_to_string(&file_path).expect("file read");
        assert_eq!(content, "tcp:127.0.0.1:8080");
    });
}

#[test]
fn write_tcp_discovery_file_ipv6() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_string_lossy().to_string();
    temp_env::with_var("XDG_RUNTIME_DIR", Some(temp_path.as_str()), || {
        let addr: std::net::SocketAddr = "[::1]:9000".parse().expect("valid addr");
        let result = write_tcp_discovery_file("ipv6-discovery.txt", &addr);
        assert!(result.is_ok());
        let file_path = temp_dir.path().join("ipv6-discovery.txt");
        let content = std::fs::read_to_string(&file_path).expect("file read");
        assert_eq!(content, "tcp:[::1]:9000");
    });
}

// ── resolve_family_id / resolve_node_id ─────────────────────────────────────

#[test]
fn resolve_family_id_override_takes_precedence() {
    let result = resolve_family_id(Some("cli-override".to_string()));
    assert_eq!(result, "cli-override");
}

#[test]
fn resolve_family_id_default_when_none() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_FAMILY_ID", None::<&str>),
            ("TOADSTOOL_FAMILY", None::<&str>),
            ("BIOMEOS_FAMILY_ID", None::<&str>),
        ],
        || {
            let result = resolve_family_id(None);
            assert_eq!(result, "default");
        },
    );
}

#[test]
fn resolve_family_id_from_toadstool_family_id_env() {
    temp_env::with_var("TOADSTOOL_FAMILY_ID", Some("env-nat0"), || {
        let result = resolve_family_id(None);
        assert_eq!(result, "env-nat0");
    });
}

#[test]
fn resolve_family_id_from_toadstool_family_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_FAMILY_ID", None::<&str>),
            ("TOADSTOOL_FAMILY", Some("toad-family")),
            ("BIOMEOS_FAMILY_ID", None::<&str>),
        ],
        || {
            let result = resolve_family_id(None);
            assert_eq!(result, "toad-family");
        },
    );
}

#[test]
fn resolve_family_id_from_biomeos_family_id_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_FAMILY_ID", None::<&str>),
            ("TOADSTOOL_FAMILY", None::<&str>),
            ("BIOMEOS_FAMILY_ID", Some("biomeos-nat1")),
        ],
        || {
            let result = resolve_family_id(None);
            assert_eq!(result, "biomeos-nat1");
        },
    );
}

#[test]
fn resolve_node_id_default_when_unset() {
    temp_env::with_var("TOADSTOOL_NODE_ID", None::<&str>, || {
        let result = resolve_node_id();
        assert_eq!(result, "default");
    });
}

#[test]
fn resolve_node_id_from_env() {
    temp_env::with_var("TOADSTOOL_NODE_ID", Some("node-42"), || {
        let result = resolve_node_id();
        assert_eq!(result, "node-42");
    });
}

#[test]
fn exit_codes_values() {
    assert_eq!(exit_codes::SUCCESS, 0);
    assert_eq!(exit_codes::GENERAL_ERROR, 1);
    assert_eq!(exit_codes::CONFIG_ERROR, 2);
    assert_eq!(exit_codes::RUNTIME_ERROR, 3);
    assert_eq!(exit_codes::INTERRUPTED, 130);
}

// ── ShutdownSignal ────────────────────────────────────────────────────────

#[test]
fn shutdown_signal_sigint() {
    assert_eq!(ShutdownSignal::Sigint, ShutdownSignal::Sigint);
    assert!(matches!(ShutdownSignal::Sigint, ShutdownSignal::Sigint));
}

#[test]
fn shutdown_signal_sigterm() {
    assert_eq!(ShutdownSignal::Sigterm, ShutdownSignal::Sigterm);
}

#[test]
fn shutdown_signal_error() {
    let err = ShutdownSignal::Error("test error");
    assert_eq!(err, ShutdownSignal::Error("test error"));
}

#[test]
fn shutdown_signal_variants_differ() {
    assert_ne!(ShutdownSignal::Sigint, ShutdownSignal::Sigterm);
    assert_ne!(ShutdownSignal::Sigint, ShutdownSignal::Error("x"));
    assert_ne!(ShutdownSignal::Sigterm, ShutdownSignal::Error("x"));
}

// ── run_server_main early exit on config error ────────────────────────────

#[tokio::test]
async fn run_server_main_fails_when_socket_path_unavailable() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let file_path = temp_dir.path().join("not_a_dir");
    std::fs::File::create(&file_path).expect("create file");
    let path_str = file_path.to_string_lossy().to_string();

    temp_env::async_with_vars(
        [
            ("TOADSTOOL_SOCKET", None::<&str>),
            ("PRIMAL_SOCKET", None::<&str>),
            ("BIOMEOS_SOCKET_PATH", None::<&str>),
            ("XDG_RUNTIME_DIR", Some(path_str.as_str())),
            ("TOADSTOOL_STANDALONE", Some("1")),
        ],
        async {
            let result = super::run_server_main(None, None, None, None, None, false).await;
            assert!(
                result.is_err(),
                "run_server_main should fail when socket path unavailable"
            );
        },
    )
    .await;
}

// ── get_socket_path (format) ──────────────────────────────────────────────

#[test]
fn get_socket_path_from_toadstool_socket() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("custom.sock");
    let path_str = socket_path.to_string_lossy().to_string();
    temp_env::with_var("TOADSTOOL_SOCKET", Some(path_str.as_str()), || {
        let result = get_socket_path("family1", "node1", None, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), socket_path);
    });
}

#[test]
fn get_socket_path_from_primal_socket() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SOCKET", None::<&str>),
            ("PRIMAL_SOCKET", Some("/run/primal")),
            ("BIOMEOS_SOCKET_PATH", None::<&str>),
        ],
        || {
            let result = get_socket_path("nat0", "node1", None, None);
            assert!(result.is_ok());
            assert_eq!(
                result.unwrap(),
                std::path::PathBuf::from("/run/primal-nat0")
            );
        },
    );
}

#[test]
fn get_socket_path_from_biomeos_socket_path() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SOCKET", None::<&str>),
            ("PRIMAL_SOCKET", None::<&str>),
            ("BIOMEOS_SOCKET_PATH", Some("/run/biomeos/toadstool.sock")),
        ],
        || {
            let result = get_socket_path("default", "node1", None, None);
            assert!(result.is_ok());
            assert_eq!(
                result.unwrap(),
                std::path::PathBuf::from("/run/biomeos/toadstool.sock")
            );
        },
    );
}

#[test]
fn get_socket_path_xdg_runtime_dir_fallback() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let xdg_path = temp_dir.path().to_string_lossy().to_string();
    temp_env::with_vars(
        [
            ("TOADSTOOL_SOCKET", None::<&str>),
            ("PRIMAL_SOCKET", None::<&str>),
            ("BIOMEOS_SOCKET_PATH", None::<&str>),
            ("XDG_RUNTIME_DIR", Some(xdg_path.as_str())),
        ],
        || {
            let result = get_socket_path("default", "node1", None, None);
            assert!(result.is_ok());
            let path = result.unwrap();
            assert!(path.ends_with("biomeos/compute.sock"));
            // biomeos dir is created by ensure_biomeos_directory; socket file doesn't exist until server binds
            assert!(path.parent().unwrap().exists());
        },
    );
}

// ── create_executor (standalone mode, no network) ────────────────────────────

#[tokio::test]
async fn create_executor_standalone_mode() {
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("1"))], async {
        let result = create_executor("test-family", &UnibinExecutionConfig::from_env()).await;
        assert!(
            result.is_ok(),
            "create_executor should succeed in standalone mode"
        );
        let executor = result.unwrap();
        assert!(std::sync::Arc::strong_count(&executor) >= 1);
    })
    .await;
}

#[tokio::test]
async fn create_executor_standalone_mode_true_lowercase() {
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("true"))], async {
        let result = create_executor("default", &UnibinExecutionConfig::from_env()).await;
        assert!(
            result.is_ok(),
            "TOADSTOOL_STANDALONE=true should use standalone"
        );
    })
    .await;
}

// ── BTSP insecure guard at server startup ────────────────────────────────

#[tokio::test]
async fn run_server_main_refuses_family_plus_insecure() {
    temp_env::async_with_vars(
        [
            ("BIOMEOS_FAMILY_ID", Some("production-1")),
            ("BIOMEOS_INSECURE", Some("1")),
        ],
        async {
            let result = run_server_main(None, None, None, None, None, false).await;
            assert!(result.is_err(), "must refuse when FAMILY_ID + INSECURE");
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("BTSP security conflict"),
                "error should mention BTSP: {err}"
            );
        },
    )
    .await;
}

#[test]
fn insecure_guard_allows_dev_mode_via_api() {
    temp_env::with_vars(
        [
            ("BIOMEOS_FAMILY_ID", None::<&str>),
            ("BIOMEOS_INSECURE", Some("1")),
            ("TOADSTOOL_FAMILY_ID", None::<&str>),
            ("TOADSTOOL_FAMILY", None::<&str>),
        ],
        || {
            assert!(
                toadstool_common::primal_sockets::check_insecure_guard().is_ok(),
                "guard should not fire without FAMILY_ID"
            );
        },
    );
}

// ─── PG-55: --bind flag + localhost default ───

#[test]
fn unibin_config_defaults_to_localhost_not_all_interfaces() {
    temp_env::with_vars(
        [("TOADSTOOL_BIND_ADDRESS", None::<&str>)],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert_eq!(
                cfg.bind_host, "127.0.0.1",
                "default bind should be loopback, not 0.0.0.0 (PG-55)"
            );
        },
    );
}

#[test]
fn unibin_config_respects_env_override() {
    temp_env::with_vars(
        [("TOADSTOOL_BIND_ADDRESS", Some("0.0.0.0"))],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert_eq!(
                cfg.bind_host, "0.0.0.0",
                "env override should allow all-interfaces when explicitly set"
            );
        },
    );
}
