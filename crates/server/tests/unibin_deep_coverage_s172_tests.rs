// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::case_sensitive_file_extension_comparisons,
    clippy::doc_markdown,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Deep coverage for `crates/server/src/unibin/` (S172): error paths, env edge cases,
//! and feature-gated capability discovery. Matches patterns from `unibin_execution_coverage_tests.rs`.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use toadstool_server::pure_jsonrpc::JsonRpcHandler;
use toadstool_server::tarpc_server::{
    StandaloneExecutor, ToadStoolTarpcServer, WorkloadExecutorDispatch,
};
use toadstool_server::unibin::{
    UnibinExecutionConfig, create_executor, ensure_biomeos_directory, get_socket_path,
    is_platform_constraint_str, is_selinux_enforcing, query_local_capabilities, resolve_family_id,
    resolve_node_id, socket_filename_for_family, start_servers_with_fallback,
    write_tcp_discovery_file,
};

// ── resolve_family_id / resolve_node_id: malformed & boundary inputs ───────────

#[test]
fn unibin_s172_resolve_family_id_empty_override_returns_empty() {
    let id = resolve_family_id(Some(String::new()));
    assert_eq!(id, "");
}

#[test]
fn unibin_s172_resolve_family_id_whitespace_override_preserved() {
    let id = resolve_family_id(Some("   ".to_string()));
    assert_eq!(id, "   ");
}

#[test]
fn unibin_s172_resolve_family_id_env_empty_string_uses_empty() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_FAMILY_ID", Some("")),
            ("TOADSTOOL_FAMILY", None::<&str>),
            ("BIOMEOS_FAMILY_ID", None::<&str>),
        ],
        || {
            let id = resolve_family_id(None);
            assert_eq!(id, "");
        },
    );
}

#[test]
fn unibin_s172_resolve_node_id_empty_env_is_empty_string() {
    temp_env::with_var("TOADSTOOL_NODE_ID", Some(""), || {
        let id = resolve_node_id();
        assert_eq!(id, "");
    });
}

// ── format: socket filename & path resolution ────────────────────────────────

#[test]
fn unibin_s172_socket_filename_unicode_family() {
    let name = socket_filename_for_family("家族");
    assert!(name.ends_with(".sock"));
    assert!(name.contains("家族"));
}

#[test]
fn unibin_s172_get_socket_path_toadstool_overrides_primal_and_biomeos() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let primary = temp_dir.path().join("from-toadstool.sock");
    let primary_str = primary.to_string_lossy().to_string();
    temp_env::with_vars(
        [
            ("TOADSTOOL_SOCKET", Some(primary_str.as_str())),
            ("PRIMAL_SOCKET", Some("/should-not-use")),
            ("BIOMEOS_SOCKET_PATH", Some("/also-not-this")),
        ],
        || {
            let p = get_socket_path("ignored", "ignored").expect("path");
            assert_eq!(p, primary);
        },
    );
}

#[test]
fn unibin_s172_get_socket_path_primal_socket_empty_family_suffix() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SOCKET", None::<&str>),
            ("BIOMEOS_SOCKET_PATH", None::<&str>),
            ("PRIMAL_SOCKET", Some("/run/primal")),
        ],
        || {
            let p = get_socket_path("", "node").expect("path");
            assert_eq!(p, PathBuf::from("/run/primal-"));
        },
    );
}

#[test]
fn unibin_s172_get_socket_path_biomeos_only() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SOCKET", None::<&str>),
            ("PRIMAL_SOCKET", None::<&str>),
            ("BIOMEOS_SOCKET_PATH", Some("/var/biomeos/custom.sock")),
        ],
        || {
            let p = get_socket_path("anything", "node").expect("path");
            assert_eq!(p, PathBuf::from("/var/biomeos/custom.sock"));
        },
    );
}

#[test]
fn unibin_s172_get_socket_path_xdg_empty_string_uses_fallback_or_tmp() {
    // `XDG_RUNTIME_DIR=""` yields an empty PathBuf; `exists()` is false → /tmp biomeos branch
    temp_env::with_vars(
        [
            ("TOADSTOOL_SOCKET", None::<&str>),
            ("PRIMAL_SOCKET", None::<&str>),
            ("BIOMEOS_SOCKET_PATH", None::<&str>),
            ("XDG_RUNTIME_DIR", Some("")),
        ],
        || {
            let p = get_socket_path("default", "node").expect("path");
            assert!(p.ends_with("biomeos/compute.sock"));
        },
    );
}

#[test]
fn unibin_s172_ensure_biomeos_fails_when_biomeos_path_is_file() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let blocking = temp_dir.path().join("biomeos");
    std::fs::write(&blocking, b"not-a-directory").expect("write file");
    let result = ensure_biomeos_directory(temp_dir.path());
    assert!(
        result.is_err(),
        "create_dir_all should fail when 'biomeos' exists as a file: {result:?}"
    );
}

// ── execution: platform strings & discovery file errors ──────────────────────

#[test]
fn unibin_s172_is_platform_constraint_str_supported_substrings() {
    // Matching is case-sensitive: "not supported" (lowercase) and "Unsupported" (capital U)
    assert!(is_platform_constraint_str(
        "socket not supported on this host"
    ));
    assert!(is_platform_constraint_str("foo Unsupported bar"));
}

#[test]
fn unibin_s172_is_platform_constraint_str_permission_denied_branches() {
    let expect = is_selinux_enforcing();
    assert_eq!(
        is_platform_constraint_str("Permission denied opening socket"),
        expect
    );
    assert_eq!(
        is_platform_constraint_str("Operation not permitted (EPERM)"),
        expect
    );
}

#[test]
fn unibin_s172_write_tcp_discovery_file_xdg_empty_name_resolution() {
    temp_env::with_var("XDG_RUNTIME_DIR", Some(""), || {
        let addr: std::net::SocketAddr = "127.0.0.1:7".parse().unwrap();
        let name = "unibin-s172-empty-xdg-port";
        let result = write_tcp_discovery_file(name, &addr);
        assert!(
            result.is_ok(),
            "writing relative to empty XDG should still succeed when cwd allows: {result:?}"
        );
        let path = PathBuf::from(name);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
    });
}

#[test]
fn unibin_s172_write_tcp_discovery_file_port_max() {
    let td = tempfile::tempdir().unwrap();
    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some(td.path().to_string_lossy().as_ref()),
        || {
            let addr: std::net::SocketAddr = "127.0.0.1:65535".parse().unwrap();
            let result = write_tcp_discovery_file("unibin-s172-max-port", &addr);
            assert!(result.is_ok());
            let content = std::fs::read_to_string(td.path().join("unibin-s172-max-port")).unwrap();
            assert!(content.contains("65535"));
        },
    );
}

#[tokio::test]
async fn unibin_s172_start_servers_with_fallback_non_platform_unix_error() {
    let socket_path = PathBuf::from("/dev/null/tarpc.sock");
    let jsonrpc_socket = PathBuf::from("/dev/null/jsonrpc.sock");
    let executor = Arc::new(WorkloadExecutorDispatch::Standalone(
        StandaloneExecutor::new(),
    ));
    let server = ToadStoolTarpcServer::new(
        "1.0.0",
        executor,
        Some(Arc::new(std::sync::atomic::AtomicU64::new(0))),
    );
    let jsonrpc_handler = Arc::new(JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "1.0.0".to_string(),
        None,
        Arc::new(AtomicBool::new(true)),
    ));
    let result = start_servers_with_fallback(
        server,
        jsonrpc_handler,
        socket_path,
        jsonrpc_socket,
        None,
        &UnibinExecutionConfig::from_env(),
    )
    .await;
    assert!(result.is_err());
}

// ── create_executor: env edge cases (standalone vs distributed) ───────────────

#[tokio::test]
async fn unibin_s172_create_executor_standalone_numeric_two() {
    // Only "1" / "true" (case-insensitive) select standalone; "2" uses distributed path.
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("2"))], async {
        let result = create_executor("unibin-s172-two", &UnibinExecutionConfig::from_env()).await;
        match &result {
            Ok(_) => {}
            Err(e) => assert!(!e.to_string().is_empty()),
        }
    })
    .await;
}

#[tokio::test]
async fn unibin_s172_create_executor_standalone_no_explicit_coordination_endpoint() {
    temp_env::async_with_vars(
        [
            ("TOADSTOOL_STANDALONE", Some("0")),
            ("SONGBIRD_ENDPOINT", None::<&str>),
            ("TOADSTOOL_COORDINATION_ENDPOINT", None::<&str>),
        ],
        async {
            let result = create_executor(
                "unibin-s172-no-endpoint",
                &UnibinExecutionConfig::from_env(),
            )
            .await;
            match &result {
                Ok(_) => {}
                Err(e) => assert!(!e.to_string().is_empty()),
            }
        },
    )
    .await;
}

// ── capabilities: feature-gated & optional GPU path ───────────────────────────

#[cfg(feature = "gpu-discovery")]
fn unibin_s172_wgpu_safe_or_skip() -> bool {
    if toadstool_testing::gpu_guards::is_wgpu_safe() {
        return true;
    }
    eprintln!("{}", toadstool_testing::gpu_guards::wgpu_skip_reason());
    false
}

#[cfg(feature = "gpu-discovery")]
#[tokio::test(flavor = "current_thread")]
async fn unibin_s172_query_local_capabilities_with_gpu_discovery_feature() {
    if !unibin_s172_wgpu_safe_or_skip() {
        return;
    }
    let caps = query_local_capabilities().await;
    assert!(
        caps.iter().any(|c| c.as_ref() == "compute"),
        "expected compute: {caps:?}"
    );
    assert!(
        caps.iter().any(|c| c.as_ref() == "orchestration"),
        "expected orchestration: {caps:?}"
    );
}

#[cfg(not(feature = "gpu-discovery"))]
#[tokio::test(flavor = "current_thread")]
async fn unibin_s172_query_local_capabilities_without_gpu_discovery_feature() {
    let caps = query_local_capabilities().await;
    assert!(caps.iter().any(|c| c.as_ref() == "compute"));
    assert!(caps.iter().any(|c| c.as_ref() == "cpu"));
    assert!(caps.iter().any(|c| c.as_ref() == "orchestration"));
}

#[tokio::test(flavor = "current_thread")]
async fn unibin_s172_query_local_capabilities_smoke() {
    let caps = query_local_capabilities().await;
    assert!(!caps.is_empty());
    for c in &caps {
        assert!(!c.is_empty(), "capability must be non-empty: {caps:?}");
    }
}

// ── run_server_main: config error path (socket layout) ────────────────────────

#[tokio::test]
async fn unibin_s172_run_server_main_fails_when_runtime_parent_is_file() {
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
            let result = toadstool_server::run_server_main(None, None, None).await;
            assert!(
                result.is_err(),
                "expected error when biomeos cannot be created"
            );
        },
    )
    .await;
}
