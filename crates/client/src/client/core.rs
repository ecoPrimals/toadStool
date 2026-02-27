//! ToadStool client implementation
//!
//! Uses JSON-RPC 2.0 over Unix sockets (local) per biomeOS networking policy.
//! NO reqwest/hyper/ring/openssl. Real-time events via JSON-RPC polling (no WebSocket).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock};
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
/// - If base_url is "unix:" or starts with "unix://", extract path
/// - Else use TOADSTOOL_SOCKET env or platform_paths for local
fn resolve_socket_path(base_url: &str) -> ClientResult<PathBuf> {
    // Support unix:///path/to/socket or unix:path
    if base_url.starts_with("unix://") {
        let path = base_url
            .strip_prefix("unix://")
            .unwrap_or(base_url)
            .trim_start_matches('/');
        return Ok(PathBuf::from(path));
    }
    if base_url.starts_with("unix:") {
        let path = base_url
            .strip_prefix("unix:")
            .unwrap_or("")
            .trim_start_matches('/');
        return Ok(PathBuf::from(path));
    }

    // HTTP URL: use JSON-RPC socket (local daemon)
    // Env override for testing
    if let Ok(s) = std::env::var("TOADSTOOL_SOCKET") {
        return Ok(PathBuf::from(s));
    }
    // Default: ToadStool JSON-RPC socket per platform_paths
    Ok(toadstool_common::platform_paths::toadstool_socket_dir().join("toadstool.jsonrpc.sock"))
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

        let socket_path = resolve_socket_path(&config.base_url)?;
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
    /// Use `compute.status` for GPU job status. Execution status (active_executions) not exposed.
    pub async fn get_execution_status(&self, _execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        Err(ClientError::Http(
            "Execution status: use compute.status for GPU jobs; workload executions not yet exposed via JSON-RPC".to_string(),
        ))
    }

    /// Cancel an execution
    ///
    /// Uses `compute.cancel` JSON-RPC for GPU jobs. Maps execution_id → job_id.
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
                            "failed" => ExecutionStatus::Failed,
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
    /// Builds ClusterStatus from toadstool.health + compute.list (partial).
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
            .unwrap_or(serde_json::json!({"jobs": [], "counts": {}}));

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
            healthy_nodes: if healthy { 1 } else { 0 },
            cluster_load: (pending + running) as f64,
            active_executions: (pending + running) as u32,
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
    /// WebSocket deprecated. Use JSON-RPC 2.0 polling: `compute.status` for execution
    /// status, `toadstool.health` for cluster health. biomeOS/songbird coordination.
    pub async fn subscribe_to_events(
        &self,
    ) -> ClientResult<mpsc::UnboundedReceiver<ToadStoolEvent>> {
        Err(ClientError::Http(
            "Real-time events use JSON-RPC 2.0 polling (compute.status, toadstool.health)"
                .to_string(),
        ))
    }

    /// Start event stream. No-op; use JSON-RPC polling instead.
    pub fn start_event_stream(&self) -> ClientResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_socket_path_unix_double_slash() {
        let path = resolve_socket_path("unix:///run/toadstool.sock").unwrap();
        assert!(path.to_string_lossy().contains("run/toadstool.sock"));
    }

    #[test]
    fn test_resolve_socket_path_unix_single() {
        let path = resolve_socket_path("unix:/run/toadstool.sock").unwrap();
        assert!(path.to_string_lossy().contains("run/toadstool.sock"));
    }

    #[test]
    fn test_resolve_socket_path_env_override() {
        std::env::set_var("TOADSTOOL_SOCKET", "/tmp/test_toadstool.sock");
        let path = resolve_socket_path("http://localhost:8080").unwrap();
        assert_eq!(path, std::path::PathBuf::from("/tmp/test_toadstool.sock"));
        std::env::remove_var("TOADSTOOL_SOCKET");
    }

    #[test]
    fn test_resolve_socket_path_default_fallback() {
        std::env::remove_var("TOADSTOOL_SOCKET");
        let path = resolve_socket_path("http://localhost:8080").unwrap();
        // Should return some reasonable socket path
        assert!(
            path.to_string_lossy().ends_with(".sock")
                || path.to_string_lossy().contains("toadstool")
        );
    }
}
