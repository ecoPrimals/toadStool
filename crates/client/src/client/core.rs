//! ToadStool client implementation
//!
//! Uses JSON-RPC 2.0 over Unix sockets (local) per biomeOS networking policy.
//! NO reqwest/hyper/ring/openssl.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(feature = "websocket")]
use std::time::Duration;

#[cfg(feature = "websocket")]
use futures_util::stream::StreamExt;
#[cfg(feature = "websocket")]
use futures_util::SinkExt;
use tokio::sync::{mpsc, RwLock};
#[cfg(feature = "websocket")]
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info};
use url::Url;
use uuid::Uuid;

use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

use super::config::ClientConfig;
use super::error::{ClientError, ClientResult};
use super::types::{
    ClusterStatus, EventHandlers, ExecutionInfo, ToadStoolEvent, WorkloadSubmission,
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

/// Server events received via WebSocket
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    ExecutionStarted {
        execution_id: Uuid,
        runtime_type: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ExecutionCompleted {
        execution_id: Uuid,
        status: String,
        duration_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ResourceUsageUpdate {
        cpu_usage_percent: f64,
        memory_usage_percent: f64,
        active_executions: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ErrorOccurred {
        error_type: String,
        message: String,
        execution_id: Option<Uuid>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
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
            .map_err(|e| ClientError::Server(format!("compute.cancel failed: {}", e)))?;

        info!("Execution cancelled successfully: {}", execution_id);
        Ok(())
    }

    /// Wait for execution completion (event-driven with polling fallback)
    pub async fn wait_for_completion(&self, execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        debug!("Waiting for execution completion: {}", execution_id);

        #[cfg(feature = "websocket")]
        {
            if self.config.enable_websocket {
                if let Ok(info) = self.wait_for_completion_via_events(execution_id).await {
                    return Ok(info);
                }
                warn!("WebSocket event subscription failed, falling back to NotImplemented");
            }
        }

        // get_execution_status returns NotImplemented for workload executions
        Err(ClientError::Http(
            "wait_for_completion requires get_execution_status; use compute.status for GPU jobs"
                .to_string(),
        ))
    }

    /// Wait for completion using WebSocket events (no polling!)
    #[cfg(feature = "websocket")]
    async fn wait_for_completion_via_events(
        &self,
        execution_id: Uuid,
    ) -> ClientResult<ExecutionInfo> {
        debug!(
            "Waiting for execution via WebSocket events: {}",
            execution_id
        );

        let max_wait_time = Duration::from_secs(300);
        let mut event_rx = self.subscribe_to_events().await?;

        tokio::time::timeout(max_wait_time, async {
            while let Some(event) = event_rx.recv().await {
                match event {
                    ServerEvent::ExecutionCompleted {
                        execution_id: event_id,
                        ..
                    } if event_id == execution_id => {
                        info!("Execution completed via event: {}", execution_id);
                        return self.get_execution_status(execution_id).await;
                    }
                    ServerEvent::ErrorOccurred {
                        execution_id: Some(event_id),
                        error_type,
                        message,
                        ..
                    } if event_id == execution_id => {
                        warn!("Execution error via event: {} - {}", error_type, message);
                        return self.get_execution_status(execution_id).await;
                    }
                    _ => continue,
                }
            }
            Err(ClientError::WebSocket(
                "Event stream closed before completion".to_string(),
            ))
        })
        .await
        .map_err(|_| {
            ClientError::Timeout(format!(
                "Execution {} did not complete within {:?}",
                execution_id, max_wait_time
            ))
        })?
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
            .map_err(|e| ClientError::Server(format!("toadstool.health failed: {}", e)))?;

        let jobs: serde_json::Value = self
            .rpc_client
            .call("compute.list", serde_json::json!({}))
            .await
            .unwrap_or(serde_json::json!({"jobs": [], "counts": {}}));

        let counts = jobs.get("counts").and_then(|c| c.as_object());
        let pending = counts
            .and_then(|m| m.get("pending"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let running = counts
            .and_then(|m| m.get("running"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let _completed = counts
            .and_then(|m| m.get("completed"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let healthy = health
            .get("healthy")
            .and_then(|v| v.as_bool())
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
            .map_err(|e| ClientError::Server(format!("Health check failed: {}", e)))?;

        let healthy = result
            .get("healthy")
            .and_then(|v| v.as_bool())
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

    /// Subscribe to server events via WebSocket
    ///
    /// Note: Requires "websocket" feature. WebSocket needs HTTP server.
    #[cfg(feature = "websocket")]
    pub async fn subscribe_to_events(&self) -> ClientResult<mpsc::UnboundedReceiver<ServerEvent>> {
        let ws_url = self
            .config
            .base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        let ws_url = format!("{ws_url}/ws");

        debug!("Connecting to WebSocket: {}", ws_url);

        let url = Url::parse(&ws_url)
            .map_err(|e| ClientError::WebSocket(format!("Invalid WebSocket URL: {}", e)))?;
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| ClientError::WebSocket(format!("Connection failed: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();

        let (tx, rx) = mpsc::unbounded_channel();

        let subscribe_msg = serde_json::json!({ "type": "subscribe" });
        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .map_err(|e| ClientError::WebSocket(format!("Failed to subscribe: {}", e)))?;

        tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Ok(event) = serde_json::from_value::<ServerEvent>(value) {
                                if tx.send(event).is_err() {
                                    debug!("Event receiver dropped, closing WebSocket");
                                    break;
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        debug!("WebSocket closed by server");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        info!("WebSocket event subscription established");
        Ok(rx)
    }

    /// Subscribe to events (stub when websocket feature disabled)
    #[cfg(not(feature = "websocket"))]
    pub async fn subscribe_to_events(&self) -> ClientResult<mpsc::UnboundedReceiver<ServerEvent>> {
        Err(ClientError::WebSocket(
            "WebSocket support requires 'websocket' feature".to_string(),
        ))
    }

    /// Start event stream (stub when websocket feature disabled)
    #[cfg(not(feature = "websocket"))]
    pub fn start_event_stream(&self) -> ClientResult<()> {
        Ok(())
    }

    /// Start WebSocket connection for real-time events (legacy method)
    #[cfg(feature = "websocket")]
    pub fn start_event_stream(&self) -> ClientResult<()> {
        if !self.config.enable_websocket {
            return Ok(());
        }

        let ws_url = self
            .config
            .base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        let ws_url = format!("{ws_url}/ws");

        debug!("Connecting to WebSocket: {}", ws_url);

        let event_handlers = Arc::clone(&self.event_handlers);

        tokio::spawn(async move {
            if let Ok(url) = Url::parse(&ws_url) {
                if let Ok((ws_stream, _)) = connect_async(url).await {
                    info!("WebSocket connected: {}", ws_url);

                    let (_, mut read) = ws_stream.split();

                    while let Ok(Some(message)) = read.next().await.transpose() {
                        if let Message::Text(text) = message {
                            if let Ok(event) = serde_json::from_str::<ToadStoolEvent>(&text) {
                                let handlers = event_handlers.read().await;
                                for handler in handlers.iter() {
                                    handler(event.clone());
                                }
                            }
                        }
                    }
                } else {
                    error!("Failed to connect to WebSocket: {}", ws_url);
                }
            }
        });

        Ok(())
    }
}
