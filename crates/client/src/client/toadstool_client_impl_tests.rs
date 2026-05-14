// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for [`ToadStoolClient`] Unix JSON-RPC implementation (moved from `core.rs`).

use crate::client::config::ClientConfig;
use crate::client::core::{ToadStoolClient, resolve_socket_path};
use crate::client::types::ExecutionStatus;

#[test]
fn test_resolve_socket_path_unix_double_slash() {
    let path = resolve_socket_path("unix:///run/toadstool.sock");
    assert!(path.to_string_lossy().contains("run/toadstool.sock"));
}

#[test]
fn test_resolve_socket_path_unix_single() {
    let path = resolve_socket_path("unix:/run/toadstool.sock");
    assert!(path.to_string_lossy().contains("run/toadstool.sock"));
}

#[test]
fn test_resolve_socket_path_env_override() {
    temp_env::with_var("TOADSTOOL_SOCKET", Some("/tmp/test_toadstool.sock"), || {
        let path = resolve_socket_path("http://localhost:8080");
        assert_eq!(path, std::path::PathBuf::from("/tmp/test_toadstool.sock"));
    });
}

#[test]
fn test_resolve_socket_path_default_fallback() {
    temp_env::with_var_unset("TOADSTOOL_SOCKET", || {
        let path = resolve_socket_path("http://localhost:8080");
        assert!(
            path.to_string_lossy().ends_with(".sock")
                || path.to_string_lossy().contains("toadstool")
        );
    });
}

#[test]
fn test_resolve_socket_path_unix_triple_slash() {
    let path = resolve_socket_path("unix:///tmp/toadstool.sock");
    assert!(path.to_string_lossy().contains("tmp"));
    assert!(path.to_string_lossy().contains("toadstool"));
}

#[test]
fn test_resolve_socket_path_unix_no_leading_slash() {
    let path = resolve_socket_path("unix:relative/path.sock");
    assert!(!path.to_string_lossy().starts_with("//"));
}

#[test]
fn test_resolve_socket_path_unix_empty_after_prefix() {
    let path = resolve_socket_path("unix:");
    assert!(path.as_os_str().is_empty() || path.to_string_lossy().is_empty());
}

#[test]
fn test_resolve_socket_path_unix_strip_leading_slashes() {
    let path = resolve_socket_path("unix://///tmp/sock");
    assert!(path.to_string_lossy().contains("tmp"));
}

mod client_method_tests {
    use super::*;
    use crate::client::config::ClientConfig;
    use crate::client::types::{ToadStoolEvent, WorkloadSubmission, WorkloadType};

    fn test_client() -> ToadStoolClient {
        let config = ClientConfig {
            base_url: "unix:///tmp/test-toadstool.sock".to_string(),
            ..Default::default()
        };
        ToadStoolClient::new_for_testing(config).expect("test client")
    }

    fn default_workload() -> WorkloadSubmission {
        WorkloadSubmission {
            workload_type: WorkloadType::Native {
                executable: "/bin/echo".to_string(),
                args: vec![],
                working_dir: None,
            },
            runtime_hint: None,
            priority: None,
            timeout: None,
            environment: std::collections::HashMap::new(),
            resources: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_submit_workload_returns_error() {
        let client = test_client();
        let workload = default_workload();
        let result = client.submit_workload(workload).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("execution.submit_native")
                || msg.contains("JSON-RPC")
                || msg.contains("connect")
                || msg.contains("Failed"),
        );
    }

    #[tokio::test]
    async fn test_get_execution_status_returns_error() {
        let client = test_client();
        let result = client.get_execution_status(uuid::Uuid::new_v4()).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("execution.status") || msg.contains("connect") || msg.contains("Failed")
        );
    }

    #[tokio::test]
    async fn test_subscribe_to_events_returns_error() {
        let client = test_client();
        let result = client.subscribe_to_events().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("polling"));
    }

    #[tokio::test]
    async fn test_start_event_stream_ok() {
        let client = test_client();
        let result = client.start_event_stream();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_executions_empty() {
        let client = test_client();
        let result = client.list_executions().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_add_event_handler_no_panic() {
        let client = test_client();
        client.add_event_handler(|_: ToadStoolEvent| {}).await;
    }

    #[test]
    fn test_with_config_invalid_url_fails() {
        let config = ClientConfig {
            base_url: "not-a-valid-url!!!".to_string(),
            ..Default::default()
        };
        let result = ToadStoolClient::new_for_testing(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_with_config_unix_url_succeeds() {
        let config = ClientConfig {
            base_url: "unix:///tmp/test.sock".to_string(),
            ..Default::default()
        };
        let result = ToadStoolClient::new_for_testing(config);
        assert!(result.is_ok());
    }
}

/// JSON-RPC mock server on a temp Unix socket; clients use [`config_for_socket`] (no env mutation).
#[allow(clippy::await_holding_lock, reason = "test mock server holds lock across await intentionally for deterministic sequencing")]
mod jsonrpc_unix_mock {

    use super::{ClientConfig, ExecutionStatus, ToadStoolClient};
    use serde_json::{Value, json};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{UnixListener, UnixStream};
    use tokio::task::JoinHandle;
    use uuid::Uuid;

    /// Build a [`ClientConfig`] with its `base_url` pointing at the given Unix socket
    /// (via the `unix://` scheme that `resolve_socket_path` understands). No env mutation required.
    fn config_for_socket(path: &std::path::Path) -> ClientConfig {
        ClientConfig {
            base_url: format!("unix://{}", path.display()),
            ..Default::default()
        }
    }

    #[derive(Clone)]
    pub(super) struct MockState {
        pub(super) health: Arc<Mutex<Value>>,
        pub(super) compute_list: Arc<Mutex<Value>>,
        pub(super) compute_status: Arc<Mutex<Vec<Value>>>,
        pub(super) compute_status_idx: Arc<AtomicUsize>,
        pub(super) cancel_fail: Arc<AtomicBool>,
        pub(super) compute_status_fail: Arc<AtomicBool>,
    }

    impl MockState {
        pub(super) fn new() -> Self {
            Self {
                health: Arc::new(Mutex::new(json!({"healthy": true}))),
                compute_list: Arc::new(Mutex::new(json!({
                    "jobs": [],
                    "counts": {"pending": 2, "running": 3, "completed": 10}
                }))),
                compute_status: Arc::new(Mutex::new(vec![json!({"status": "completed"})])),
                compute_status_idx: Arc::new(AtomicUsize::new(0)),
                cancel_fail: Arc::new(AtomicBool::new(false)),
                compute_status_fail: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    fn abs_socket_path() -> PathBuf {
        std::env::temp_dir().join(format!("toadstool_core_{}.sock", Uuid::new_v4()))
    }

    async fn handle_one_connection(stream: UnixStream, state: MockState) {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        if reader.read_line(&mut line).await.is_err() {
            return;
        }
        let req: Value = match serde_json::from_str(line.trim()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let id = req
            .get("id")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        let method = req
            .get("method")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        if method == "compute.status" && state.compute_status_fail.load(Ordering::SeqCst) {
            let err_line = serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "compute.status failed"}
            }))
            .expect("serialize json-rpc error");
            let mut err_line = err_line;
            err_line.push('\n');
            let _ = writer.write_all(err_line.as_bytes()).await;
            let _ = writer.flush().await;
            return;
        }

        if method == "compute.cancel" && state.cancel_fail.load(Ordering::SeqCst) {
            let err_line = serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "compute.cancel failed"}
            }))
            .expect("serialize json-rpc error");
            let mut err_line = err_line;
            err_line.push('\n');
            let _ = writer.write_all(err_line.as_bytes()).await;
            let _ = writer.flush().await;
            return;
        }

        let result = match method {
            "toadstool.health" => state.health.lock().expect("lock health").clone(),
            "compute.list" => state.compute_list.lock().expect("lock list").clone(),
            "compute.status" => {
                let idx = state.compute_status_idx.fetch_add(1, Ordering::SeqCst);
                let g = state.compute_status.lock().expect("lock status");
                g.get(idx)
                    .cloned()
                    .or_else(|| g.last().cloned())
                    .unwrap_or_else(|| json!({"status": "running"}))
            }
            _ => json!({}),
        };

        let body = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
        .expect("serialize json-rpc ok");
        let mut body = body;
        body.push('\n');
        let _ = writer.write_all(body.as_bytes()).await;
        let _ = writer.flush().await;
    }

    pub(super) fn spawn_mock(path: &Path, state: MockState) -> JoinHandle<()> {
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path).expect("bind mock unix socket");
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };
                let st = state.clone();
                tokio::spawn(async move {
                    handle_one_connection(stream, st).await;
                });
            }
        })
    }

    #[tokio::test]
    async fn health_check_ok_when_healthy_true() {
        let path = abs_socket_path();
        let state = MockState::new();
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let result = client.health_check().await;
        handle.abort();
        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn health_check_err_when_healthy_false() {
        let path = abs_socket_path();
        let state = MockState::new();
        *state.health.lock().expect("lock") = json!({"healthy": false});
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let result = client.health_check().await;
        handle.abort();
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
        let msg = result.expect_err("unhealthy").to_string();
        assert!(
            msg.contains("unhealthy") || msg.contains("Server error"),
            "msg={msg}"
        );
    }

    #[tokio::test]
    async fn health_check_err_when_healthy_missing() {
        let path = abs_socket_path();
        let state = MockState::new();
        *state.health.lock().expect("lock") = json!({"version": "1"});
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let result = client.health_check().await;
        handle.abort();
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn with_config_connects_and_lists_cluster() {
        let path = abs_socket_path();
        let state = MockState::new();
        let handle = spawn_mock(&path, state.clone());
        tokio::task::yield_now().await;
        let client = ToadStoolClient::with_config(config_for_socket(&path))
            .await
            .expect("with_config");
        let cluster = client.get_cluster_status().await.expect("cluster");
        assert_eq!(cluster.total_nodes, 1);
        assert_eq!(cluster.healthy_nodes, 1);
        assert!((cluster.cluster_load - 5.0).abs() < f64::EPSILON);
        assert_eq!(cluster.active_executions, 5);
        assert!(cluster.available_runtimes.contains(&"native".to_string()));
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn new_connects_via_health() {
        let path = abs_socket_path();
        let state = MockState::new();
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new(&format!("unix://{}", path.display()))
            .await
            .expect("new");
        let h = client.health_check().await;
        assert!(h.is_ok());
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn get_cluster_status_defaults_when_counts_missing() {
        let path = abs_socket_path();
        let state = MockState::new();
        *state.compute_list.lock().expect("lock") = json!({"jobs": []});
        let handle = spawn_mock(&path, state.clone());
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let cluster = client.get_cluster_status().await.expect("cluster");
        assert!((cluster.cluster_load).abs() < f64::EPSILON);
        assert_eq!(cluster.active_executions, 0);
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn get_cluster_status_healthy_nodes_zero_when_unhealthy() {
        let path = abs_socket_path();
        let state = MockState::new();
        *state.health.lock().expect("lock") = json!({"healthy": false});
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let cluster = client.get_cluster_status().await.expect("cluster");
        assert_eq!(cluster.healthy_nodes, 0);
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn cancel_execution_ok() {
        let path = abs_socket_path();
        let state = MockState::new();
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let id = Uuid::new_v4();
        let result = client.cancel_execution(id).await;
        handle.abort();
        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cancel_execution_jsonrpc_error() {
        let path = abs_socket_path();
        let state = MockState::new();
        state.cancel_fail.store(true, Ordering::SeqCst);
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let result = client.cancel_execution(Uuid::new_v4()).await;
        handle.abort();
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
        assert!(
            result
                .expect_err("cancel")
                .to_string()
                .contains("compute.cancel")
        );
    }

    #[tokio::test]
    async fn wait_for_completion_completed() {
        let path = abs_socket_path();
        let state = MockState::new();
        *state.compute_status.lock().expect("lock") =
            vec![json!({"status": "completed", "error": null})];
        state.compute_status_idx.store(0, Ordering::SeqCst);
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let id = Uuid::new_v4();
        let info = client.wait_for_completion(id).await.expect("done");
        assert_eq!(info.execution_id, id);
        assert!(matches!(info.status, ExecutionStatus::Completed));
        assert!(info.error_message.is_none());
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn wait_for_completion_failed_with_error_message() {
        let path = abs_socket_path();
        let state = MockState::new();
        *state.compute_status.lock().expect("lock") = vec![json!({
            "status": "failed",
            "error": "oom"
        })];
        state.compute_status_idx.store(0, Ordering::SeqCst);
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let id = Uuid::new_v4();
        let info = client.wait_for_completion(id).await.expect("done");
        assert!(matches!(info.status, ExecutionStatus::Failed));
        assert_eq!(info.error_message.as_deref(), Some("oom"));
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn wait_for_completion_failed_non_string_error_field() {
        let path = abs_socket_path();
        let state = MockState::new();
        *state.compute_status.lock().expect("lock") =
            vec![json!({"status": "failed", "error": 42})];
        state.compute_status_idx.store(0, Ordering::SeqCst);
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let info = client
            .wait_for_completion(Uuid::new_v4())
            .await
            .expect("done");
        assert!(matches!(info.status, ExecutionStatus::Failed));
        assert!(info.error_message.is_none());
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn wait_for_completion_cancelled() {
        let path = abs_socket_path();
        let state = MockState::new();
        *state.compute_status.lock().expect("lock") = vec![json!({"status": "cancelled"})];
        state.compute_status_idx.store(0, Ordering::SeqCst);
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let info = client
            .wait_for_completion(Uuid::new_v4())
            .await
            .expect("done");
        assert!(matches!(info.status, ExecutionStatus::Cancelled));
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn wait_for_completion_status_uppercase() {
        let path = abs_socket_path();
        let state = MockState::new();
        *state.compute_status.lock().expect("lock") = vec![json!({"status": "COMPLETED"})];
        state.compute_status_idx.store(0, Ordering::SeqCst);
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let info = client
            .wait_for_completion(Uuid::new_v4())
            .await
            .expect("done");
        assert!(matches!(info.status, ExecutionStatus::Completed));
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn wait_for_completion_breaks_on_compute_status_rpc_error() {
        let path = abs_socket_path();
        let state = MockState::new();
        state.compute_status_fail.store(true, Ordering::SeqCst);
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let client = ToadStoolClient::new_for_testing(config_for_socket(&path)).expect("client");
        let result = client.wait_for_completion(Uuid::new_v4()).await;
        handle.abort();
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
        assert!(
            result
                .expect_err("http")
                .to_string()
                .contains("wait_for_completion")
        );
    }

    #[tokio::test]
    async fn get_cluster_status_fails_when_health_unreachable() {
        let path = abs_socket_path();
        let client = ToadStoolClient::new_for_testing(config_for_socket(
            &path.with_extension("definitely_missing"),
        ))
        .expect("client");
        let result = client.get_cluster_status().await;
        assert!(result.is_err());
    }

    #[test]
    fn new_for_testing_accepts_https_url() {
        let config = ClientConfig {
            base_url: "https://example.com/path".to_string(),
            ..Default::default()
        };
        temp_env::with_var(
            "TOADSTOOL_SOCKET",
            Some("/tmp/unused-for-this-test.sock"),
            || {
                assert!(ToadStoolClient::new_for_testing(config).is_ok());
            },
        );
    }

    #[tokio::test]
    async fn with_config_auth_and_custom_headers() {
        let path = abs_socket_path();
        let state = MockState::new();
        let handle = spawn_mock(&path, state);
        tokio::task::yield_now().await;
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Test".to_string(), "1".to_string());
        let config = ClientConfig {
            base_url: format!("unix://{}", path.display()),
            auth: Some(crate::client::config::AuthConfig::BearerToken {
                token: "t".to_string(),
            }),
            custom_headers: headers,
            ..Default::default()
        };
        let _ = ToadStoolClient::with_config(config)
            .await
            .expect("with_config");
        handle.abort();
        let _ = std::fs::remove_file(&path);
    }
}
