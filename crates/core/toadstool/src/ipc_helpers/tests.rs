// SPDX-License-Identifier: AGPL-3.0-or-later
//! IPC helpers tests
//!
//! Uses `temp_env` for safe, isolated environment variable testing. No unsafe
//! env var manipulation; temp_env handles serialization and restoration.

use super::connection::{IPC_TIMEOUT, get_default_songbird_socket};
use super::*;
use serde_json::json;

#[test]
fn test_constants() {
    assert_eq!(IPC_TIMEOUT.as_secs(), 5);
}

#[test]
fn test_register_with_songbird_graceful_failure() {
    temp_env::with_var_unset("SONGBIRD_SOCKET", || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let result = register_with_songbird().await;
                assert!(result.is_err());
                let err = result.unwrap_err();
                let err_msg = format!("{err}");
                assert!(err_msg.contains("Songbird") || err_msg.contains("connection"));
            });
        })
        .join()
        .expect("test thread");
    });
}

#[test]
fn test_resolve_primal_graceful_failure() {
    temp_env::with_var_unset("SONGBIRD_SOCKET", || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let result = resolve_primal("beardog").await;
                assert!(result.is_err());
            });
        })
        .join()
        .expect("test thread");
    });
}

#[test]
fn test_connect_to_primal_graceful_failure() {
    temp_env::with_var_unset("SONGBIRD_SOCKET", || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let result = connect_to_primal("beardog").await;
                assert!(result.is_err());
            });
        })
        .join()
        .expect("test thread");
    });
}

#[test]
fn test_find_by_capability_graceful_failure() {
    temp_env::with_var_unset("SONGBIRD_SOCKET", || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let result = find_by_capability("crypto").await;
                assert!(result.is_err());
            });
        })
        .join()
        .expect("test thread");
    });
}

#[test]
fn test_json_rpc_request_format() {
    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "ipc.register",
        "params": {
            "primal_name": "toadstool",
            "capabilities": ["compute"]
        },
        "id": 1
    });
    assert_eq!(request.get("jsonrpc").unwrap(), "2.0");
    assert_eq!(request.get("method").unwrap(), "ipc.register");
    assert!(request.get("params").is_some());
    assert_eq!(request.get("id").unwrap(), 1);
}

#[test]
fn test_resolve_semantic_to_implementation() {
    assert_eq!(resolve_method_name("compute.execute"), "execute_workload");
    assert_eq!(resolve_method_name("resource.health.check"), "check_health");
    assert_eq!(
        resolve_method_name("storage.artifact.store"),
        "store_artifact"
    );
}

#[test]
fn test_resolve_implementation_passthrough() {
    assert_eq!(resolve_method_name("execute_workload"), "execute_workload");
    assert_eq!(resolve_method_name("check_health"), "check_health");
}

#[test]
fn test_resolve_unknown_semantic() {
    assert_eq!(resolve_method_name("unknown.method"), "unknown.method");
    assert_eq!(resolve_method_name("future.api.call"), "future.api.call");
}

#[test]
fn test_is_semantic_method() {
    assert!(is_semantic_method("compute.execute"));
    assert!(is_semantic_method("resource.cpu.get_usage"));
    assert!(!is_semantic_method("execute_workload"));
    assert!(!is_semantic_method("single_word"));
}

#[test]
fn test_get_semantic_name() {
    assert_eq!(
        get_semantic_name("execute_workload"),
        Some("compute.execute".to_string())
    );
    assert_eq!(
        get_semantic_name("check_health"),
        Some("resource.health.check".to_string())
    );
    assert_eq!(get_semantic_name("unknown_method"), None);
}

#[test]
fn test_list_semantic_methods() {
    let methods = list_semantic_methods();
    assert!(methods.len() > 40);
    assert!(methods.contains(&"compute.execute".to_string()));
    assert!(methods.contains(&"resource.health.check".to_string()));
    assert!(methods.contains(&"storage.artifact.store".to_string()));
    assert!(methods.contains(&"network.configure".to_string()));
    assert!(methods.contains(&"security.policy.apply".to_string()));
}

#[test]
fn test_semantic_resolution_bidirectional() {
    let impl_name = resolve_method_name("compute.execute");
    assert_eq!(impl_name, "execute_workload");
    let semantic_name = get_semantic_name(&impl_name);
    assert_eq!(semantic_name, Some("compute.execute".to_string()));
}

#[test]
fn test_runtime_variant_resolution() {
    assert_eq!(
        resolve_method_name("compute.container.run"),
        "run_container"
    );
    assert_eq!(
        resolve_method_name("compute.wasm.execute"),
        "start_wasm_module"
    );
    assert_eq!(
        resolve_method_name("compute.python.execute"),
        "run_python_script"
    );
    assert_eq!(
        resolve_method_name("compute.native.execute"),
        "run_native_binary"
    );
    assert_eq!(
        resolve_method_name("compute.gpu.execute"),
        "run_gpu_compute"
    );
}

#[test]
fn test_all_domains_covered() {
    let methods = list_semantic_methods();
    assert!(methods.iter().any(|m| m.starts_with("compute.")));
    assert!(methods.iter().any(|m| m.starts_with("resource.")));
    assert!(methods.iter().any(|m| m.starts_with("storage.")));
    assert!(methods.iter().any(|m| m.starts_with("network.")));
    assert!(methods.iter().any(|m| m.starts_with("security.")));
    assert!(methods.iter().any(|m| m.starts_with("runtime.")));
}

#[test]
fn test_all_semantic_to_implementation_mappings() {
    assert_eq!(resolve_method_name("compute.execute"), "execute_workload");
    assert_eq!(resolve_method_name("compute.stop"), "stop_workload");
    assert_eq!(
        resolve_method_name("compute.container.run"),
        "run_container"
    );
    assert_eq!(
        resolve_method_name("compute.wasm.execute"),
        "start_wasm_module"
    );
    assert_eq!(
        resolve_method_name("resource.cpu.get_usage"),
        "get_cpu_usage"
    );
    assert_eq!(resolve_method_name("resource.health.check"), "check_health");
    assert_eq!(
        resolve_method_name("storage.artifact.store"),
        "store_artifact"
    );
    assert_eq!(
        resolve_method_name("network.configure"),
        "configure_networking"
    );
    assert_eq!(
        resolve_method_name("security.policy.apply"),
        "apply_security_policies"
    );
    assert_eq!(
        resolve_method_name("runtime.engine.list"),
        "list_runtime_engines"
    );
}

#[test]
fn test_unknown_semantic_names_pass_through() {
    assert_eq!(resolve_method_name("unknown.method"), "unknown.method");
    assert_eq!(resolve_method_name("future.api.call"), "future.api.call");
}

#[test]
fn test_implementation_names_pass_through() {
    assert_eq!(resolve_method_name("execute_workload"), "execute_workload");
    assert_eq!(resolve_method_name("check_health"), "check_health");
    assert_eq!(resolve_method_name("store_artifact"), "store_artifact");
}

#[test]
fn test_is_semantic_method_all_known() {
    let methods = list_semantic_methods();
    for method in &methods {
        assert!(is_semantic_method(method), "{method} should be semantic");
    }
}

#[test]
fn test_is_semantic_method_non_semantic() {
    assert!(!is_semantic_method("execute_workload"));
    assert!(!is_semantic_method("single_word"));
    assert!(!is_semantic_method(""));
}

#[test]
fn test_get_semantic_name_all_implementations() {
    let pairs = [
        ("execute_workload", "compute.execute"),
        ("check_health", "resource.health.check"),
        ("store_artifact", "storage.artifact.store"),
    ];
    for (impl_name, expected) in pairs {
        assert_eq!(
            get_semantic_name(impl_name),
            Some(expected.to_string()),
            "{impl_name}"
        );
    }
}

#[test]
fn test_get_semantic_name_unknown_returns_none() {
    assert_eq!(get_semantic_name("unknown_method"), None);
}

#[test]
fn test_list_semantic_methods_count_and_contents() {
    let methods = list_semantic_methods();
    assert!(
        methods.len() >= 100,
        "registry should have 100+ semantic methods"
    );
    assert!(methods.contains(&"compute.execute".to_string()));
    assert!(methods.contains(&"shader.compile.wgsl".to_string()));
    assert!(methods.contains(&"provenance.query".to_string()));
    assert!(methods.contains(&"ecology.et0_fao56".to_string()));
    assert!(methods.contains(&"discovery.primals".to_string()));
    assert!(methods.contains(&"deploy.capability_call".to_string()));
}

#[test]
fn test_resolution_consistency_roundtrip() {
    let methods = list_semantic_methods();
    for semantic in &methods {
        let impl_name = resolve_method_name(semantic);
        let back = get_semantic_name(&impl_name);
        // Deprecated aliases map to same impl as canonical; back may be canonical form
        assert!(
            back.as_ref()
                .is_some_and(|b| resolve_method_name(b) == impl_name),
            "roundtrip: {semantic} -> {impl_name:?} -> {back:?}"
        );
    }
}

#[test]
fn test_edge_cases_semantic_resolution() {
    assert_eq!(resolve_method_name(""), "");
    assert!(!is_semantic_method(""));
    assert_eq!(get_semantic_name(""), None);
    assert_eq!(resolve_method_name(" "), " ");
    assert_eq!(resolve_method_name("."), ".");
    assert!(is_semantic_method("."));
}

// ── Socket path helpers ───────────────────────────────────────────────────────

#[test]
fn test_get_default_songbird_socket_contains_songbird_sock() {
    let path = get_default_songbird_socket();
    assert!(
        path.ends_with("songbird.sock"),
        "socket path should end with songbird.sock, got: {path}"
    );
}

#[test]
fn test_get_default_songbird_socket_with_xdg_runtime_dir() {
    temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/test-xdg-runtime"), || {
        let path = get_default_songbird_socket();
        assert!(path.starts_with("/tmp/test-xdg-runtime"));
        assert!(path.ends_with("songbird.sock"));
    });
}

// ── Mock Unix socket happy-path tests ────────────────────────────────────────
//
// These tests spin up a temporary Unix socket server, point the connection
// functions at it via SONGBIRD_SOCKET (via temp_env), and exercise the
// JSON-RPC send/receive paths that are otherwise unreachable in CI.
//
// The mock and connection must run on the SAME tokio runtime to avoid
// deadlocks (temp_env blocks the calling thread).

/// Spawn a mock Songbird socket that accepts one connection and replies with
/// the given JSON response (NDJSON framing), then returns.
async fn spawn_mock_songbird(
    socket_path: &str,
    reply: serde_json::Value,
) -> tokio::task::JoinHandle<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    let listener = UnixListener::bind(socket_path).expect("bind mock socket");
    let reply_line = format!("{reply}\n");

    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            let mut line = String::new();
            let _ = reader.read_line(&mut line).await;
            let _ = write_half.write_all(reply_line.as_bytes()).await;
            let _ = write_half.flush().await;
        }
    })
}

#[test]
fn test_register_with_songbird_success_via_mock() {
    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("songbird.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let reply = json!({"jsonrpc": "2.0", "result": {"status": "registered"}, "id": 1});

    let inner_path = path_str.clone();
    temp_env::with_var("SONGBIRD_SOCKET", Some(&path_str), || {
        let p = inner_path.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let handle = spawn_mock_songbird(&p, reply).await;
                let result = register_with_songbird().await;
                handle.abort();
                assert!(result.is_ok(), "registration should succeed: {result:?}");
            });
        })
        .join()
        .expect("test thread");
    });
}

#[test]
fn test_register_with_songbird_error_reply_via_mock() {
    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("songbird_err.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let reply = json!({"jsonrpc": "2.0", "error": {"code": -32000, "message": "already registered"}, "id": 1});

    let inner_path = path_str.clone();
    temp_env::with_var("SONGBIRD_SOCKET", Some(&path_str), || {
        let p = inner_path.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let handle = spawn_mock_songbird(&p, reply).await;
                let result = register_with_songbird().await;
                handle.abort();
                assert!(result.is_err());
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("Songbird registration failed")
                );
            });
        })
        .join()
        .expect("test thread");
    });
}

#[test]
fn test_resolve_primal_success_via_mock() {
    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("resolve.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let reply = json!({
        "jsonrpc": "2.0",
        "result": {"endpoint": "/run/user/1000/biomeos/beardog.sock"},
        "id": 1
    });

    let inner_path = path_str.clone();
    temp_env::with_var("SONGBIRD_SOCKET", Some(&path_str), || {
        let p = inner_path.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let handle = spawn_mock_songbird(&p, reply).await;
                let result = resolve_primal("beardog").await;
                handle.abort();
                assert!(result.is_ok());
                assert!(result.unwrap().contains("beardog.sock"));
            });
        })
        .join()
        .expect("test thread");
    });
}

#[test]
fn test_resolve_primal_missing_endpoint_returns_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("resolve_bad.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let reply = json!({"jsonrpc": "2.0", "result": {}, "id": 1});

    let inner_path = path_str.clone();
    temp_env::with_var("SONGBIRD_SOCKET", Some(&path_str), || {
        let p = inner_path.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let handle = spawn_mock_songbird(&p, reply).await;
                let result = resolve_primal("beardog").await;
                handle.abort();
                assert!(result.is_err());
                assert!(result.unwrap_err().to_string().contains("missing endpoint"));
            });
        })
        .join()
        .expect("test thread");
    });
}

#[test]
fn test_find_by_capability_success_via_mock() {
    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("cap.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let reply = json!({
        "jsonrpc": "2.0",
        "result": {
            "services": [
                {"primal_name": "barracuda"},
                {"primal_name": "hotspring"}
            ]
        },
        "id": 1
    });

    let inner_path = path_str.clone();
    temp_env::with_var("SONGBIRD_SOCKET", Some(&path_str), || {
        let p = inner_path.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let handle = spawn_mock_songbird(&p, reply).await;
                let result = find_by_capability("compute").await;
                handle.abort();
                assert!(result.is_ok());
                let primals = result.unwrap();
                assert_eq!(primals.len(), 2);
                assert!(primals.contains(&"barracuda".to_string()));
            });
        })
        .join()
        .expect("test thread");
    });
}

#[test]
fn test_find_by_capability_error_reply() {
    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("cap_err.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let reply =
        json!({"jsonrpc": "2.0", "error": {"code": -1, "message": "no capabilities"}, "id": 1});

    let inner_path = path_str.clone();
    temp_env::with_var("SONGBIRD_SOCKET", Some(&path_str), || {
        let p = inner_path.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let handle = spawn_mock_songbird(&p, reply).await;
                let result = find_by_capability("gpu").await;
                handle.abort();
                assert!(result.is_err());
            });
        })
        .join()
        .expect("test thread");
    });
}
