// SPDX-License-Identifier: AGPL-3.0-or-later
//! ToadStool client implementation
//!
//! Uses JSON-RPC 2.0 over Unix sockets (local) per biomeOS networking policy.
//! NO reqwest/hyper/ring/openssl. Real-time events via JSON-RPC polling (no `WebSocket`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info};
use url::Url;
use uuid::Uuid;

use toadstool_common::interned_strings::socket_env;
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

use super::config::ClientConfig;
use super::error::{ClientError, ClientResult};
use super::types::{
    ClusterStatus, EventHandlers, ExecutionInfo, ExecutionStatus, ToadStoolEvent,
    WorkloadSubmission, WorkloadType,
};

/// Resolve socket path from config.
/// - If `base_url` is "unix:" or starts with "unix://", extract path
/// - Else use `TOADSTOOL_SOCKET` env or `platform_paths` for local
pub(crate) fn resolve_socket_path(base_url: &str) -> PathBuf {
    // Support unix:///path/to/socket or unix:path
    if base_url.starts_with("unix://") {
        let path = base_url.strip_prefix("unix://").unwrap_or(base_url);
        // unix:///tmp/sock → remainder is /tmp/sock (absolute); do not strip leading slash.
        return if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            PathBuf::from(path.trim_start_matches('/'))
        };
    }
    if base_url.starts_with("unix:") {
        let path = base_url.strip_prefix("unix:").unwrap_or("");
        return if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            PathBuf::from(path.trim_start_matches('/'))
        };
    }

    // HTTP URL: use JSON-RPC socket (local daemon)
    // Env override for testing
    if let Ok(s) = std::env::var(socket_env::TOADSTOOL_SOCKET) {
        return PathBuf::from(s);
    }
    // Default: domain-based socket per PRIMAL_SELF_KNOWLEDGE_STANDARD v1.1
    toadstool_common::platform_paths::toadstool_socket_dir().join("compute.sock")
}

/// JSON-RPC method name for workload submission (see `execution.submit_*` family).
#[must_use]
pub fn execution_submit_method(workload_type: &WorkloadType) -> &'static str {
    match workload_type {
        WorkloadType::Native { .. } => "execution.submit_native",
        WorkloadType::Container { .. } => "execution.submit_container",
        WorkloadType::Wasm { .. } => "execution.submit_wasm",
        WorkloadType::Python { .. } => "execution.submit_python",
        WorkloadType::Custom { .. } => "execution.submit_custom",
    }
}

fn workload_submission_params(workload: &WorkloadSubmission) -> Value {
    serde_json::json!({
        "workload_type": workload.workload_type,
        "runtime_hint": workload.runtime_hint,
        "priority": workload.priority,
        "timeout_secs": workload.timeout.map(|d| d.as_secs()),
        "environment": workload.environment,
        "resources": workload.resources,
        "metadata": workload.metadata,
    })
}

fn parse_execution_id(value: &Value) -> Option<Uuid> {
    let id = value
        .get("execution_id")
        .or_else(|| value.get("workload_id"))
        .or_else(|| value.get("job_id"))
        .or_else(|| value.get("id"))?;
    if let Some(s) = id.as_str() {
        return Uuid::parse_str(s).ok();
    }
    None
}

fn execution_info_from_submit_response(
    value: &Value,
    submitted_at: std::time::SystemTime,
) -> ClientResult<ExecutionInfo> {
    let execution_id = parse_execution_id(value).ok_or_else(|| {
        ClientError::Server(
            "execution submit: response missing execution_id/workload_id/job_id".to_string(),
        )
    })?;
    let status = value
        .get("status")
        .and_then(|v| v.as_str())
        .map_or(ExecutionStatus::Queued, map_execution_status_str);
    Ok(ExecutionInfo {
        execution_id,
        status,
        submitted_at,
        started_at: None,
        completed_at: None,
        runtime_type: value
            .get("runtime_type")
            .and_then(|v| v.as_str())
            .map(String::from),
        error_message: value
            .get("error")
            .and_then(|v| v.as_str())
            .map(String::from),
        output: None,
        metrics: None,
    })
}

fn execution_info_from_status_response(value: &Value, execution_id: Uuid) -> ExecutionInfo {
    let status_str = value
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let status = map_execution_status_str(status_str);
    let terminal = matches!(
        status,
        ExecutionStatus::Completed
            | ExecutionStatus::Failed
            | ExecutionStatus::Cancelled
            | ExecutionStatus::Timeout
    );
    ExecutionInfo {
        execution_id,
        status,
        submitted_at: std::time::SystemTime::now(),
        started_at: None,
        completed_at: terminal.then(std::time::SystemTime::now),
        runtime_type: value
            .get("runtime_type")
            .and_then(|v| v.as_str())
            .map(String::from),
        error_message: value
            .get("error")
            .or_else(|| value.get("error_message"))
            .and_then(|v| v.as_str())
            .map(String::from),
        output: None,
        metrics: None,
    }
}

fn map_execution_status_str(s: &str) -> ExecutionStatus {
    match s.to_lowercase().as_str() {
        "completed" | "success" | "succeeded" | "done" => ExecutionStatus::Completed,
        "failed" | "error" | "failure" => ExecutionStatus::Failed,
        "cancelled" | "canceled" => ExecutionStatus::Cancelled,
        "running" | "active" => ExecutionStatus::Running,
        "queued" => ExecutionStatus::Queued,
        "pending" => ExecutionStatus::Pending,
        "timeout" | "timed_out" | "timed out" => ExecutionStatus::Timeout,
        _ => ExecutionStatus::Running,
    }
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
    /// Uses `execution.submit_native` / `execution.submit_container` / `execution.submit_wasm` /
    /// `execution.submit_python` / `execution.submit_custom` over Unix JSON-RPC. GPU jobs use
    /// `compute.submit` (see server docs).
    pub async fn submit_workload(
        &self,
        workload: WorkloadSubmission,
    ) -> ClientResult<ExecutionInfo> {
        let method = execution_submit_method(&workload.workload_type);
        let params = workload_submission_params(&workload);
        let submitted_at = std::time::SystemTime::now();
        let result = self
            .rpc_client
            .call(method, params)
            .await
            .map_err(|e| ClientError::Server(format!("{method}: {e}")))?;
        let info = execution_info_from_submit_response(&result, submitted_at)?;
        self.active_executions
            .write()
            .await
            .insert(info.execution_id, info.clone());
        Ok(info)
    }

    /// Get execution status for a workload execution
    ///
    /// Calls `execution.status` with the workload/execution id. GPU jobs continue to use
    /// `compute.status` via [`Self::wait_for_completion`].
    pub async fn get_execution_status(&self, execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        let params = serde_json::json!({
            "workload_id": execution_id.to_string(),
            "execution_id": execution_id.to_string(),
        });
        let result = self
            .rpc_client
            .call("execution.status", params)
            .await
            .map_err(|e| ClientError::Server(format!("execution.status: {e}")))?;
        let info = execution_info_from_status_response(&result, execution_id);
        self.active_executions
            .write()
            .await
            .insert(execution_id, info.clone());
        Ok(info)
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
        const MAX_WAIT_SECS: u64 = 300;
        const POLL_INTERVAL_MS: u64 = 500;

        debug!(
            "Waiting for execution completion via polling: {}",
            execution_id
        );
        let max_wait = Duration::from_secs(MAX_WAIT_SECS);
        let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);
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
    /// status, `toadstool.health` for cluster health via coordination service.
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
