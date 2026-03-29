// SPDX-License-Identifier: AGPL-3.0-only
//! ToadStool client implementation
//!
//! Uses JSON-RPC 2.0 over Unix sockets (local) per biomeOS networking policy.
//! NO reqwest/hyper/ring/openssl. Real-time events via JSON-RPC polling (no `WebSocket`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info};
use url::Url;
use uuid::Uuid;

use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

use super::config::ClientConfig;
use super::error::{ClientError, ClientResult};
use super::types::{
    ClusterStatus, EventHandlers, ExecutionInfo, ExecutionStatus, ToadStoolEvent,
    WorkloadSubmission,
};

/// Resolve socket path from config.
/// - If `base_url` is "unix:" or starts with "unix://", extract path
/// - Else use `TOADSTOOL_SOCKET` env or `platform_paths` for local
fn resolve_socket_path(base_url: &str) -> PathBuf {
    // Support unix:///path/to/socket or unix:path
    if base_url.starts_with("unix://") {
        let path = base_url
            .strip_prefix("unix://")
            .unwrap_or(base_url)
            .trim_start_matches('/');
        return PathBuf::from(path);
    }
    if base_url.starts_with("unix:") {
        let path = base_url
            .strip_prefix("unix:")
            .unwrap_or("")
            .trim_start_matches('/');
        return PathBuf::from(path);
    }

    // HTTP URL: use JSON-RPC socket (local daemon)
    // Env override for testing
    if let Ok(s) = std::env::var("TOADSTOOL_SOCKET") {
        return PathBuf::from(s);
    }
    // Default: ToadStool JSON-RPC socket per platform_paths
    toadstool_common::platform_paths::toadstool_socket_dir().join("toadstool.jsonrpc.sock")
}

/// ToadStool client for interacting with ToadStool servers
///
/// Uses Unix JSON-RPC (local) per biomeOS networking policy.
pub struct ToadStoolClient {
    config: ClientConfig,
    rpc_client: UnixJsonRpcClient,
    active_executions: Arc<RwLock<HashMap<Uuid, ExecutionInfo>>>,
    event_handlers: Arc<RwLock<EventHandlers>>,
}

impl ToadStoolClient {
    /// Create a new ToadStool client
    ///
    /// # Errors
    ///
    /// Returns an error if the base URL is invalid or client initialization fails
    pub async fn new(base_url: &str) -> ClientResult<Self> {
        let config = ClientConfig {
            base_url: base_url.to_string(),
            ..Default::default()
        };

        Self::with_config(config).await
    }

    /// Create a new ToadStool client with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid or client initialization fails
    pub async fn with_config(config: ClientConfig) -> ClientResult<Self> {
        // Validate base_url (URL parse for http, or path for unix)
        if !config.base_url.starts_with("unix:") {
            let _ = Url::parse(&config.base_url)?;
        }

        let socket_path = resolve_socket_path(&config.base_url);
        let rpc_client = UnixJsonRpcClient::new(socket_path);

        let client = Self {
            config,
            rpc_client,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
        };

        // Auth/custom headers: stored but not used for Unix JSON-RPC
        // (auth can be added to JSON-RPC params when server supports it)
        if client.config.auth.is_some() || !client.config.custom_headers.is_empty() {
            debug!("Auth/custom headers configured (JSON-RPC server may not use them yet)");
        }

        // Test connection
        client.health_check().await?;

        info!("ToadStool client connected via Unix JSON-RPC");

        Ok(client)
    }

    /// Submit a workload for execution
    ///
    /// Use `compute.submit` JSON-RPC method for GPU/compute jobs.
    /// Workload execution (native/container/wasm/python) is not yet mapped to JSON-RPC.
    ///
    /// # Errors
    ///
    /// Returns an error - workload submission via JSON-RPC not fully implemented
    #[expect(
        clippy::unused_async,
        reason = "API surface; may perform async I/O in future"
    )]
    pub async fn submit_workload(
        &self,
        _workload: WorkloadSubmission,
    ) -> ClientResult<ExecutionInfo> {
        Err(ClientError::Http(
            "Workload submission: use compute.submit for GPU jobs; native/container/wasm/python not yet exposed via JSON-RPC".to_string(),
        ))
    }

    /// Get execution status
    ///
    /// Use `compute.status` for GPU job status. Execution status (`active_executions`) not exposed.
    #[expect(
        clippy::unused_async,
        reason = "API surface; may perform async I/O in future"
    )]
    pub async fn get_execution_status(&self, _execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        Err(ClientError::Http(
            "Execution status: use compute.status for GPU jobs; workload executions not yet exposed via JSON-RPC".to_string(),
        ))
    }

    /// Cancel an execution
    ///
    /// Uses `compute.cancel` JSON-RPC for GPU jobs. Maps `execution_id` → `job_id`.
    pub async fn cancel_execution(&self, execution_id: Uuid) -> ClientResult<()> {
        debug!("Cancelling execution: {}", execution_id);

        let params = serde_json::json!({ "job_id": execution_id.to_string() });
        let _ = self
            .rpc_client
            .call("compute.cancel", params)
            .await
            .map_err(|e| ClientError::Server(format!("compute.cancel failed: {e}")))?;

        info!("Execution cancelled successfully: {}", execution_id);
        Ok(())
    }

    /// Wait for execution completion via JSON-RPC polling (`compute.status` method).
    pub async fn wait_for_completion(&self, execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        debug!(
            "Waiting for execution completion via polling: {}",
            execution_id
        );

        let max_wait = Duration::from_secs(300);
        let poll_interval = Duration::from_millis(500);
        let mut interval = tokio::time::interval(poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let start = std::time::Instant::now();

        while start.elapsed() < max_wait {
            interval.tick().await;

            let status_result = self
                .rpc_client
                .call(
                    "compute.status",
                    serde_json::json!({ "job_id": execution_id.to_string() }),
                )
                .await;

            match status_result {
                Ok(status) => {
                    let status_str = status
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if matches!(status_str.as_str(), "completed" | "failed" | "cancelled") {
                        info!(
                            "Execution {} completed with status: {}",
                            execution_id, status_str
                        );
                        let exec_status = match status_str.as_str() {
                            "completed" => ExecutionStatus::Completed,
                            "cancelled" => ExecutionStatus::Cancelled,
                            _ => ExecutionStatus::Failed,
                        };
                        return Ok(ExecutionInfo {
                            execution_id,
                            status: exec_status,
                            submitted_at: std::time::SystemTime::now(),
                            started_at: None,
                            completed_at: Some(std::time::SystemTime::now()),
                            runtime_type: None,
                            error_message: status
                                .get("error")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            output: None,
                            metrics: None,
                        });
                    }
                }
                Err(_) => {
                    // compute.status may not exist or job not found - workload executions use different path
                    break;
                }
            }
        }

        Err(ClientError::Http(
            "wait_for_completion: use compute.status polling for GPU jobs; workload executions not exposed via JSON-RPC".to_string(),
        ))
    }

    /// Get cluster status
    ///
    /// Builds `ClusterStatus` from toadstool.health + compute.list (partial).
    #[expect(
        clippy::cast_precision_loss,
        reason = "cluster load for display; u64 count fits f64 for typical values"
    )]
    pub async fn get_cluster_status(&self) -> ClientResult<ClusterStatus> {
        debug!("Getting cluster status");

        let health: serde_json::Value = self
            .rpc_client
            .call("toadstool.health", serde_json::json!({}))
            .await
            .map_err(|e| ClientError::Server(format!("toadstool.health failed: {e}")))?;

        let jobs: serde_json::Value = self
            .rpc_client
            .call("compute.list", serde_json::json!({}))
            .await
            .unwrap_or_else(|_| serde_json::json!({"jobs": [], "counts": {}}));

        let counts = jobs.get("counts").and_then(|c| c.as_object());
        let pending = counts
            .and_then(|m| m.get("pending"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let running = counts
            .and_then(|m| m.get("running"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let _completed = counts
            .and_then(|m| m.get("completed"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);

        let healthy = health
            .get("healthy")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        Ok(ClusterStatus {
            total_nodes: 1,
            healthy_nodes: u32::from(healthy),
            cluster_load: (pending + running) as f64,
            active_executions: u32::try_from(pending + running).unwrap_or(u32::MAX),
            available_runtimes: vec!["native".to_string(), "wasm".to_string()],
        })
    }

    /// Health check
    pub async fn health_check(&self) -> ClientResult<()> {
        let result = self
            .rpc_client
            .call("toadstool.health", serde_json::json!({}))
            .await
            .map_err(|e| ClientError::Server(format!("Health check failed: {e}")))?;

        let healthy = result
            .get("healthy")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if healthy {
            Ok(())
        } else {
            Err(ClientError::Server("Server reported unhealthy".to_string()))
        }
    }

    /// List active executions (local cache; server-side list via compute.list)
    pub async fn list_executions(&self) -> ClientResult<Vec<ExecutionInfo>> {
        let executions = self.active_executions.read().await;
        Ok(executions.values().cloned().collect())
    }

    /// Add event handler for real-time events
    pub async fn add_event_handler<F>(&self, handler: F)
    where
        F: Fn(ToadStoolEvent) + Send + Sync + 'static,
    {
        let mut handlers = self.event_handlers.write().await;
        handlers.push(Box::new(handler));
    }

    /// Subscribe to server events.
    ///
    /// `WebSocket` deprecated. Use JSON-RPC 2.0 polling: `compute.status` for execution
    /// status, `toadstool.health` for cluster health. biomeOS/songbird coordination.
    #[expect(
        clippy::unused_async,
        reason = "API surface; may perform async I/O in future"
    )]
    pub async fn subscribe_to_events(
        &self,
    ) -> ClientResult<mpsc::UnboundedReceiver<ToadStoolEvent>> {
        Err(ClientError::Http(
            "Real-time events use JSON-RPC 2.0 polling (compute.status, toadstool.health)"
                .to_string(),
        ))
    }

    /// Start event stream. No-op; use JSON-RPC polling instead.
    pub const fn start_event_stream(&self) -> ClientResult<()> {
        Ok(())
    }

    /// Test-only constructor that skips health check (no real network).
    #[doc(hidden)]
    pub fn new_for_testing(config: ClientConfig) -> ClientResult<Self> {
        if !config.base_url.starts_with("unix:") {
            let _ = Url::parse(&config.base_url)?;
        }
        let socket_path = resolve_socket_path(&config.base_url);
        let rpc_client = UnixJsonRpcClient::new(socket_path);
        Ok(Self {
            config,
            rpc_client,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!(
                err.to_string().contains("compute.submit") || err.to_string().contains("JSON-RPC")
            );
        }

        #[tokio::test]
        async fn test_get_execution_status_returns_error() {
            let client = test_client();
            let result = client.get_execution_status(uuid::Uuid::new_v4()).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("compute.status"));
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

    /// JSON-RPC mock + [`TOADSTOOL_SOCKET`] so the client resolves the same absolute path as
    /// [`UnixListener::bind`].
    #[allow(clippy::await_holding_lock, unsafe_code)]
    mod jsonrpc_unix_mock {

        use super::super::{ClientConfig, ExecutionStatus, ToadStoolClient};
        use serde_json::{Value, json};
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::{UnixListener, UnixStream};
        use tokio::task::JoinHandle;
        use uuid::Uuid;

        /// Serialize tests that mutate `TOADSTOOL_SOCKET` (parallel runs would race otherwise).
        static SOCKET_ENV_LOCK: Mutex<()> = Mutex::new(());

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

        fn http_config() -> ClientConfig {
            ClientConfig {
                base_url: "http://127.0.0.1:1".to_string(),
                ..Default::default()
            }
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
                "compute.cancel" => json!({}),
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

        pub(super) fn spawn_mock(path: PathBuf, state: MockState) -> JoinHandle<()> {
            let _ = std::fs::remove_file(&path);
            let listener = UnixListener::bind(&path).expect("bind mock unix socket");
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
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let result = client.health_check().await;
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn health_check_err_when_healthy_false() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            *state.health.lock().expect("lock") = json!({"healthy": false});
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let result = client.health_check().await;
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
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
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            *state.health.lock().expect("lock") = json!({"version": "1"});
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let result = client.health_check().await;
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn with_config_connects_and_lists_cluster() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            let handle = spawn_mock(path.clone(), state.clone());
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::with_config(http_config())
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
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }

        #[tokio::test]
        async fn new_connects_via_health() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new("http://127.0.0.1:1")
                .await
                .expect("new");
            let h = client.health_check().await;
            assert!(h.is_ok());
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }

        #[tokio::test]
        async fn get_cluster_status_defaults_when_counts_missing() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            *state.compute_list.lock().expect("lock") = json!({"jobs": []});
            let handle = spawn_mock(path.clone(), state.clone());
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let cluster = client.get_cluster_status().await.expect("cluster");
            assert!((cluster.cluster_load).abs() < f64::EPSILON);
            assert_eq!(cluster.active_executions, 0);
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }

        #[tokio::test]
        async fn get_cluster_status_healthy_nodes_zero_when_unhealthy() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            *state.health.lock().expect("lock") = json!({"healthy": false});
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let cluster = client.get_cluster_status().await.expect("cluster");
            assert_eq!(cluster.healthy_nodes, 0);
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }

        #[tokio::test]
        async fn cancel_execution_ok() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let id = Uuid::new_v4();
            let result = client.cancel_execution(id).await;
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn cancel_execution_jsonrpc_error() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            state.cancel_fail.store(true, Ordering::SeqCst);
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let result = client.cancel_execution(Uuid::new_v4()).await;
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
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
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            *state.compute_status.lock().expect("lock") =
                vec![json!({"status": "completed", "error": null})];
            state.compute_status_idx.store(0, Ordering::SeqCst);
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let id = Uuid::new_v4();
            let info = client.wait_for_completion(id).await.expect("done");
            assert_eq!(info.execution_id, id);
            assert!(matches!(info.status, ExecutionStatus::Completed));
            assert!(info.error_message.is_none());
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }

        #[tokio::test]
        async fn wait_for_completion_failed_with_error_message() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            *state.compute_status.lock().expect("lock") = vec![json!({
                "status": "failed",
                "error": "oom"
            })];
            state.compute_status_idx.store(0, Ordering::SeqCst);
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let id = Uuid::new_v4();
            let info = client.wait_for_completion(id).await.expect("done");
            assert!(matches!(info.status, ExecutionStatus::Failed));
            assert_eq!(info.error_message.as_deref(), Some("oom"));
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }

        #[tokio::test]
        async fn wait_for_completion_failed_non_string_error_field() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            *state.compute_status.lock().expect("lock") =
                vec![json!({"status": "failed", "error": 42})];
            state.compute_status_idx.store(0, Ordering::SeqCst);
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let info = client
                .wait_for_completion(Uuid::new_v4())
                .await
                .expect("done");
            assert!(matches!(info.status, ExecutionStatus::Failed));
            assert!(info.error_message.is_none());
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }

        #[tokio::test]
        async fn wait_for_completion_cancelled() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            *state.compute_status.lock().expect("lock") = vec![json!({"status": "cancelled"})];
            state.compute_status_idx.store(0, Ordering::SeqCst);
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let info = client
                .wait_for_completion(Uuid::new_v4())
                .await
                .expect("done");
            assert!(matches!(info.status, ExecutionStatus::Cancelled));
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }

        #[tokio::test]
        async fn wait_for_completion_status_uppercase() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            *state.compute_status.lock().expect("lock") = vec![json!({"status": "COMPLETED"})];
            state.compute_status_idx.store(0, Ordering::SeqCst);
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let info = client
                .wait_for_completion(Uuid::new_v4())
                .await
                .expect("done");
            assert!(matches!(info.status, ExecutionStatus::Completed));
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }

        #[tokio::test]
        async fn wait_for_completion_breaks_on_compute_status_rpc_error() {
            let path = abs_socket_path();
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            state.compute_status_fail.store(true, Ordering::SeqCst);
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let result = client.wait_for_completion(Uuid::new_v4()).await;
            handle.abort();
            let _ = std::fs::remove_file(&path);
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
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
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var(
                    "TOADSTOOL_SOCKET",
                    path.with_extension("definitely_missing")
                        .to_str()
                        .expect("utf8"),
                );
            }
            let client = ToadStoolClient::new_for_testing(http_config()).expect("client");
            let result = client.get_cluster_status().await;
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
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
            let _guard = SOCKET_ENV_LOCK.lock().expect("socket env lock");
            unsafe {
                std::env::set_var("TOADSTOOL_SOCKET", path.to_str().expect("utf8 path"));
            }
            let state = MockState::new();
            let handle = spawn_mock(path.clone(), state);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            let mut headers = std::collections::HashMap::new();
            headers.insert("X-Test".to_string(), "1".to_string());
            let config = ClientConfig {
                base_url: "http://127.0.0.1:1".to_string(),
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
            unsafe {
                std::env::remove_var("TOADSTOOL_SOCKET");
            }
            drop(_guard);
        }
    }
}
