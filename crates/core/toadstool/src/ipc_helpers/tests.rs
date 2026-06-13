// SPDX-License-Identifier: AGPL-3.0-or-later
//! IPC helpers tests
//!
//! Uses `temp_env` for safe, isolated environment variable testing. No unsafe
//! env var manipulation; `temp_env` handles serialization and restoration.

use super::connection::{IPC_TIMEOUT, get_default_coordination_socket};
use super::*;
use serde_json::json;

#[test]
fn test_constants() {
    assert_eq!(IPC_TIMEOUT.as_secs(), 5);
}

#[tokio::test]
async fn test_register_with_discovery_graceful_failure() {
    temp_env::async_with_vars(
        [
            ("DISCOVERY_SOCKET", None::<&str>),
            ("BIOMEOS_COORDINATION_SOCKET", None::<&str>),
        ],
        async {
            let result = register_with_discovery().await;
            assert!(result.is_err());
            let err = result.unwrap_err();
            let err_msg = format!("{err}");
            assert!(err_msg.contains("discovery") || err_msg.contains("connection"));
        },
    )
    .await;
}

#[tokio::test]
async fn test_find_by_capability_graceful_failure() {
    temp_env::async_with_vars(
        [
            ("DISCOVERY_SOCKET", None::<&str>),
            ("BIOMEOS_COORDINATION_SOCKET", None::<&str>),
        ],
        async {
            let result = find_by_capability("crypto").await;
            assert!(result.is_err());
        },
    )
    .await;
}

#[test]
fn test_json_rpc_request_format() {
    use super::connection::DISCOVERY_CAPABILITIES;

    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "ipc.register",
        "params": {
            "primal_id": "toadstool",
            "capabilities": DISCOVERY_CAPABILITIES
        },
        "id": 1
    });
    assert_eq!(request.get("jsonrpc").unwrap(), "2.0");
    assert_eq!(request.get("method").unwrap(), "ipc.register");
    let params = request.get("params").unwrap();
    assert_eq!(params.get("primal_id").unwrap(), "toadstool");
    let caps = params.get("capabilities").unwrap().as_array().unwrap();
    assert!(caps.len() >= 5, "Node Atomic capability set should have 5+ entries");
    assert!(caps.contains(&json!("compute")));
    assert!(caps.contains(&json!("workload")));
    assert!(caps.contains(&json!("orchestration")));
    assert!(caps.contains(&json!("gpu")));
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
    assert!(methods.contains(&"compute.execute"));
    assert!(methods.contains(&"resource.health.check"));
    assert!(methods.contains(&"storage.artifact.store"));
    assert!(methods.contains(&"network.configure"));
    assert!(methods.contains(&"security.policy.apply"));
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
    assert!(methods.contains(&"compute.execute"));
    assert!(methods.contains(&"shader.dispatch"));
    assert!(methods.contains(&"provenance.query"));
    assert!(methods.contains(&"ecology.et0_fao56"));
    assert!(methods.contains(&"discovery.primals"));
    assert!(methods.contains(&"deploy.capability_call"));
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
fn test_get_default_coordination_socket_format() {
    let path = get_default_coordination_socket();
    assert!(
        path.ends_with("coordination.sock"),
        "socket path should end with coordination.sock, got: {path}"
    );
}

#[test]
fn test_get_default_coordination_socket_with_xdg_runtime_dir() {
    temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/test-xdg-runtime"), || {
        let path = get_default_coordination_socket();
        assert!(path.starts_with("/tmp/test-xdg-runtime"));
        assert!(path.ends_with("coordination.sock"));
    });
}

// ── Mock Unix socket happy-path tests ────────────────────────────────────────
//
// These tests spin up a temporary Unix socket server, point the connection
// functions at it via BIOMEOS_COORDINATION_SOCKET (via temp_env), and exercise the
// JSON-RPC send/receive paths that are otherwise unreachable in CI.
//
// The mock and connection run on the same Tokio runtime as the test (via
// `#[tokio::test]` + `async_with_vars`).

/// Spawn a mock Songbird socket that accepts one connection and replies with
/// the given JSON response (NDJSON framing), then returns.
async fn spawn_mock_songbird(
    socket_path: &str,
    reply: serde_json::Value,
) -> tokio::task::JoinHandle<()> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    let listener = UnixListener::bind(socket_path).expect("bind mock socket");
    let reply_line = format!("{reply}\n");

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            // Consume riboCipher clear signal [0xEC, 0x01]
            let mut signal = [0u8; 2];
            let _ = stream.read_exact(&mut signal).await;
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            let mut line = String::new();
            let _ = reader.read_line(&mut line).await;
            let _ = write_half.write_all(reply_line.as_bytes()).await;
            let _ = write_half.flush().await;
        }
    })
}

#[tokio::test]
async fn test_register_with_discovery_success_via_mock() {
    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("discovery.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let reply = json!({"jsonrpc": "2.0", "result": {"status": "registered"}, "id": 1});

    temp_env::async_with_vars([("DISCOVERY_SOCKET", Some(path_str.as_str()))], async {
        let p = path_str.clone();
        let handle = spawn_mock_songbird(&p, reply).await;
        let result = register_with_discovery().await;
        handle.abort();
        assert!(result.is_ok(), "registration should succeed: {result:?}");
    })
    .await;
}

#[tokio::test]
async fn test_register_with_discovery_error_reply_via_mock() {
    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("discovery_err.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let reply = json!({"jsonrpc": "2.0", "error": {"code": -32000, "message": "already registered"}, "id": 1});

    temp_env::async_with_vars([("DISCOVERY_SOCKET", Some(path_str.as_str()))], async {
        let p = path_str.clone();
        let handle = spawn_mock_songbird(&p, reply).await;
        let result = register_with_discovery().await;
        handle.abort();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("registration failed")
        );
    })
    .await;
}

/// Verify the outbound `ipc.register` request contains correct method and fields.
#[tokio::test]
async fn test_register_with_discovery_sends_ipc_register_method() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("discovery_capture.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let listener = UnixListener::bind(&socket_path).expect("bind mock socket");
    let capture_handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        // Consume riboCipher clear signal [0xEC, 0x01] before reading JSON
        let mut signal = [0u8; 2];
        tokio::io::AsyncReadExt::read_exact(&mut stream, &mut signal)
            .await
            .expect("read riboCipher signal");
        assert_eq!(signal, [0xEC, 0x01], "expected riboCipher clear + NDJSON");
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read request");
        let reply = json!({"jsonrpc": "2.0", "result": {"status": "registered"}, "id": 1});
        write_half
            .write_all(format!("{reply}\n").as_bytes())
            .await
            .expect("write reply");
        write_half.flush().await.expect("flush");
        line
    });

    temp_env::async_with_vars([("DISCOVERY_SOCKET", Some(path_str.as_str()))], async {
        let _ = register_with_discovery().await;
    })
    .await;

    let captured = capture_handle.await.expect("capture task");
    let req: serde_json::Value = serde_json::from_str(&captured).expect("parse request");
    assert_eq!(req.get("method").unwrap(), "ipc.register");
    let params = req.get("params").unwrap();
    assert_eq!(params.get("primal_id").unwrap(), "toadstool");
    let caps = params.get("capabilities").unwrap().as_array().unwrap();
    assert!(caps.contains(&json!("compute")));
    assert!(caps.contains(&json!("workload")));
    assert!(caps.contains(&json!("orchestration")));
    let endpoint = params.get("endpoint").unwrap().as_str().unwrap();
    assert!(
        endpoint.starts_with("unix://"),
        "endpoint should start with unix://, got: {endpoint}"
    );
}

#[tokio::test]
async fn test_find_by_capability_success_via_mock() {
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

    temp_env::async_with_vars([("DISCOVERY_SOCKET", Some(path_str.as_str()))], async {
        let p = path_str.clone();
        let handle = spawn_mock_songbird(&p, reply).await;
        let result = find_by_capability("compute").await;
        handle.abort();
        assert!(result.is_ok());
        let primals = result.unwrap();
        assert_eq!(primals.len(), 2);
        assert!(primals.contains(&"barracuda".to_string()));
    })
    .await;
}

#[tokio::test]
async fn test_find_by_capability_error_reply() {
    let dir = tempfile::TempDir::new().unwrap();
    let socket_path = dir.path().join("cap_err.sock");
    let path_str = socket_path.to_str().unwrap().to_string();

    let reply =
        json!({"jsonrpc": "2.0", "error": {"code": -1, "message": "no capabilities"}, "id": 1});

    temp_env::async_with_vars([("DISCOVERY_SOCKET", Some(path_str.as_str()))], async {
        let p = path_str.clone();
        let handle = spawn_mock_songbird(&p, reply).await;
        let result = find_by_capability("gpu").await;
        handle.abort();
        assert!(result.is_err());
    })
    .await;
}
