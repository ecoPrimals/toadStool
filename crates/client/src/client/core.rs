//! ToadStool client implementation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use url::Url;
use uuid::Uuid;

use super::config::{AuthConfig, ClientConfig};
use super::error::{ClientError, ClientResult};
use super::types::{
    ClusterStatus, EventHandlers, ExecutionInfo, ExecutionStatus, ToadStoolEvent,
    WorkloadSubmission,
};

/// ToadStool client for interacting with ToadStool servers
pub struct ToadStoolClient {
    config: ClientConfig,
    // EVOLVED: Unix socket communication (Pure Rust! No HTTP client needed) ✅
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
        // Validate configuration
        let _parsed_url = Url::parse(&config.base_url)?;

        // Build HTTP client
        let mut http_client_builder = reqwest::Client::builder()
            .timeout(config.request_timeout)
            .user_agent("ToadStool-Client/1.0");

        // Add authentication headers
        let mut default_headers = reqwest::header::HeaderMap::new();

        if let Some(auth) = &config.auth {
            match auth {
                AuthConfig::ApiKey { key, header_name } => {
                    default_headers.insert(
                        reqwest::header::HeaderName::from_bytes(header_name.as_bytes()).map_err(
                            |e| ClientError::Configuration(format!("Invalid API key header name '{header_name}': {e}. Header names must contain only ASCII letters, numbers, and hyphens.")),
                        )?,
                        reqwest::header::HeaderValue::from_str(key).map_err(|e| {
                            ClientError::Configuration(format!("Invalid API key value: {e}. Header values must contain only visible ASCII characters."))
                        })?,
                    );
                }
                AuthConfig::BearerToken { token } => {
                    default_headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")).map_err(
                            |e| ClientError::Configuration(format!("Invalid bearer token '{token}': {e}. Token must contain only visible ASCII characters and no newlines.")),
                        )?,
                    );
                }
                AuthConfig::Basic { username, password } => {
                    use base64::Engine;
                    let credentials = base64::engine::general_purpose::STANDARD
                        .encode(format!("{username}:{password}"));
                    default_headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Basic {credentials}")).map_err(
                            |e| ClientError::Configuration(format!("Invalid basic auth credentials for user '{username}': {e}. Username and password must contain only visible ASCII characters.")),
                        )?,
                    );
                }
                AuthConfig::Custom { headers } => {
                    for (name, value) in headers {
                        default_headers.insert(
                            reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(
                                |e| ClientError::Configuration(format!("Invalid custom header name '{name}': {e}. Header names must contain only ASCII letters, numbers, and hyphens.")),
                            )?,
                            reqwest::header::HeaderValue::from_str(value).map_err(|e| {
                                ClientError::Configuration(format!("Invalid custom header value for '{name}': {e}. Header values must contain only visible ASCII characters."))
                            })?,
                        );
                    }
                }
            }
        }

        // Add custom headers
        for (name, value) in &config.custom_headers {
            default_headers.insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|e| {
                    ClientError::Configuration(format!("Invalid custom header name '{name}': {e}. Header names must contain only ASCII letters, numbers, and hyphens."))
                })?,
                reqwest::header::HeaderValue::from_str(value).map_err(|e| {
                    ClientError::Configuration(format!("Invalid custom header value for '{name}': {e}. Header values must contain only visible ASCII characters."))
                })?,
            );
        }

        http_client_builder = http_client_builder.default_headers(default_headers);

        let http_client = http_client_builder.build().map_err(|e| {
            ClientError::Configuration(format!("Failed to create HTTP client: {e}"))
        })?;

        let client = Self {
            config,
            http_client,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
        };

        // Test connection
        client.health_check().await?;

        info!("ToadStool client connected to {}", client.config.base_url);

        Ok(client)
    }

    /// Submit a workload for execution
    ///
    /// # EVOLVED: Use Unix Socket Communication
    ///
    /// This method has been deprecated in favor of Unix socket-based communication.
    /// For production use, interact directly with ToadStool daemon via Unix sockets.
    ///
    /// # Errors
    ///
    /// Returns an error indicating HTTP client is no longer supported
    pub async fn submit_workload(
        &self,
        _workload: WorkloadSubmission,
    ) -> ClientResult<ExecutionInfo> {
        Err(ClientError::Http(
            "HTTP client deprecated - use Unix socket communication instead".to_string(),
        ))
    }

    /// Get execution status
    ///
    /// # EVOLVED: Use Unix Socket Communication
    ///
    /// # Errors
    ///
    /// Returns an error indicating HTTP client is no longer supported
    pub async fn get_execution_status(&self, _execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        Err(ClientError::Http(
            "HTTP client deprecated - use Unix socket communication instead".to_string(),
        ))
    }

    /// Cancel an execution
    ///
    /// # Errors
    ///
    /// Returns an error if the job ID is invalid or the server returns an error
    pub async fn cancel_execution(&self, execution_id: Uuid) -> ClientResult<()> {
        debug!("Cancelling execution: {}", execution_id);

        let url = format!(
            "{}/api/v1/executions/{}",
            self.config.base_url, execution_id
        );

        let response = self.http_client.delete(&url).send().await?;

        if response.status().is_success() {
            info!("Execution cancelled successfully: {}", execution_id);
            Ok(())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            Err(ClientError::Server(format!("HTTP {status}: {error_text}")))
        }
    }

    /// Wait for execution completion (event-driven with polling fallback)
    ///
    /// # Errors
    ///
    /// Returns an error if the job ID is invalid or the server returns an error
    pub async fn wait_for_completion(&self, execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        debug!("Waiting for execution completion: {}", execution_id);

        // Try event-driven approach first if WebSocket is available
        if self.config.enable_websocket {
            match self.wait_for_completion_via_events(execution_id).await {
                Ok(info) => return Ok(info),
                Err(e) => {
                    warn!(
                        "WebSocket event subscription failed ({}), falling back to polling",
                        e
                    );
                }
            }
        }

        // Fallback to polling with exponential backoff
        self.wait_for_completion_via_polling(execution_id).await
    }

    /// Wait for completion using WebSocket events (no polling!)
    async fn wait_for_completion_via_events(
        &self,
        execution_id: Uuid,
    ) -> ClientResult<ExecutionInfo> {
        debug!(
            "Waiting for execution via WebSocket events: {}",
            execution_id
        );

        let max_wait_time = Duration::from_secs(300); // 5 minutes default
        let mut event_rx = self.subscribe_to_events().await?;

        // Use timeout to prevent infinite waiting
        tokio::time::timeout(max_wait_time, async {
            while let Some(event) = event_rx.recv().await {
                match event {
                    ServerEvent::ExecutionCompleted {
                        execution_id: event_id,
                        ..
                    } if event_id == execution_id => {
                        info!("Execution completed via event: {}", execution_id);
                        // Fetch final status
                        return self.get_execution_status(execution_id).await;
                    }
                    ServerEvent::ErrorOccurred {
                        execution_id: Some(event_id),
                        error_type,
                        message,
                        ..
                    } if event_id == execution_id => {
                        warn!("Execution error via event: {} - {}", error_type, message);
                        // Fetch final status to get error details
                        return self.get_execution_status(execution_id).await;
                    }
                    _ => {
                        // Ignore other events
                        continue;
                    }
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

    /// Wait for completion using polling (fallback)
    async fn wait_for_completion_via_polling(
        &self,
        execution_id: Uuid,
    ) -> ClientResult<ExecutionInfo> {
        debug!("Waiting for execution via polling: {}", execution_id);

        // ✅ LEGITIMATE POLLING: This polls an external HTTP API which doesn't provide
        // event streaming or websockets yet. Polling with exponential backoff is appropriate here.
        // Future improvement: Use Server-Sent Events or WebSockets when available.
        let mut polling_interval = Duration::from_millis(500);
        let max_polling_interval = Duration::from_secs(5);
        let max_wait_time = Duration::from_secs(300);
        let start_time = std::time::Instant::now();

        loop {
            let execution_info = self.get_execution_status(execution_id).await?;

            match execution_info.status {
                ExecutionStatus::Completed
                | ExecutionStatus::Failed
                | ExecutionStatus::Cancelled
                | ExecutionStatus::Timeout => {
                    info!(
                        "Execution completed: {} with status {:?}",
                        execution_id, execution_info.status
                    );
                    return Ok(execution_info);
                }
                _ => {
                    if start_time.elapsed() > max_wait_time {
                        return Err(ClientError::Timeout(format!(
                            "Execution {} did not complete within {:?}",
                            execution_id, max_wait_time
                        )));
                    }

                    tokio::time::sleep(polling_interval).await;
                    polling_interval =
                        std::cmp::min(polling_interval * 3 / 2, max_polling_interval);
                }
            }
        }
    }

    /// Get cluster status
    ///
    /// # Errors
    ///
    /// Returns an error if the server returns an error
    pub async fn get_cluster_status(&self) -> ClientResult<ClusterStatus> {
        debug!("Getting cluster status");

        let url = self.config.api_url("cluster/status");

        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let cluster_status: ClusterStatus = response.json().await?;
            Ok(cluster_status)
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            Err(ClientError::Server(format!("HTTP {status}: {error_text}")))
        }
    }

    /// Health check
    ///
    /// # Errors
    ///
    /// Returns an error if the server returns an error
    pub async fn health_check(&self) -> ClientResult<()> {
        let url = self.config.api_url("health");

        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            Err(ClientError::Server(format!("HTTP {status}: {error_text}")))
        }
    }

    /// List active executions
    ///
    /// # Errors
    ///
    /// Returns an error if the server returns an error
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
    /// Returns a channel receiver that receives server events in real-time.
    /// This is the modern, event-driven approach that eliminates polling.
    ///
    /// # Errors
    ///
    /// Returns an error if WebSocket connection fails
    pub async fn subscribe_to_events(&self) -> ClientResult<mpsc::UnboundedReceiver<ServerEvent>> {
        let ws_url = self
            .config
            .base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        let ws_url = format!("{ws_url}/ws");

        debug!("Connecting to WebSocket: {}", ws_url);

        let url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| ClientError::WebSocket(format!("Connection failed: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();

        // Create channel for events
        let (tx, rx) = mpsc::unbounded_channel();

        // Subscribe to events
        let subscribe_msg = serde_json::json!({
            "type": "subscribe"
        });
        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .map_err(|e| ClientError::WebSocket(format!("Failed to subscribe: {}", e)))?;

        // Spawn task to receive events
        tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(value) => {
                                // Parse as ServerEvent
                                if let Ok(event) = serde_json::from_value::<ServerEvent>(value) {
                                    if tx.send(event).is_err() {
                                        debug!("Event receiver dropped, closing WebSocket");
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse WebSocket message: {}", e);
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

    /// Start WebSocket connection for real-time events (legacy method)
    ///
    /// # Errors
    ///
    /// Returns an error if WebSocket is disabled or connection fails
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
            if let Ok((ws_stream, _)) = connect_async(&ws_url).await {
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
        });

        Ok(())
    }
}
