//! ToadStool client implementation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::StreamExt;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info};
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
    http_client: reqwest::Client,
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
    /// # Errors
    ///
    /// Returns an error if the job submission fails or the server returns an error
    pub async fn submit_workload(
        &self,
        workload: WorkloadSubmission,
    ) -> ClientResult<ExecutionInfo> {
        debug!("Submitting workload: {:?}", workload.workload_type);

        let request_body = serde_json::json!({
            "workload_type": workload.workload_type,
            "runtime_hint": workload.runtime_hint,
            "priority": workload.priority,
            "timeout_seconds": workload.timeout.map(|d| d.as_secs()),
            "environment": workload.environment,
            "resources": workload.resources,
            "metadata": workload.metadata,
        });

        let url = self.config.api_url("executions");

        let response = self
            .http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if response.status().is_success() {
            let execution_info: ExecutionInfo = response.json().await?;

            // Store execution info
            {
                let mut executions = self.active_executions.write().await;
                executions.insert(execution_info.execution_id, execution_info.clone());
            }

            info!(
                "Workload submitted successfully: {}",
                execution_info.execution_id
            );
            Ok(execution_info)
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            Err(ClientError::Server(format!("HTTP {status}: {error_text}")))
        }
    }

    /// Get execution status
    ///
    /// # Errors
    ///
    /// Returns an error if the job ID is invalid or the server returns an error
    pub async fn get_execution_status(&self, execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        debug!("Getting execution status for: {}", execution_id);

        let url = format!(
            "{}/api/v1/executions/{}",
            self.config.base_url, execution_id
        );

        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let execution_info: ExecutionInfo = response.json().await?;

            // Update stored execution info
            {
                let mut executions = self.active_executions.write().await;
                executions.insert(execution_id, execution_info.clone());
            }

            Ok(execution_info)
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            Err(ClientError::Server(format!("HTTP {status}: {error_text}")))
        }
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

    /// Wait for execution completion
    ///
    /// # Errors
    ///
    /// Returns an error if the job ID is invalid or the server returns an error
    pub async fn wait_for_completion(&self, execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        debug!("Waiting for execution completion: {}", execution_id);

        let mut polling_interval = Duration::from_millis(500); // Start with 500ms
        let max_polling_interval = Duration::from_secs(5); // Cap at 5 seconds
        let max_wait_time = Duration::from_secs(300); // 5 minutes default
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
                    // Check timeout
                    if start_time.elapsed() > max_wait_time {
                        return Err(ClientError::Timeout(format!(
                            "Execution {execution_id} did not complete within {max_wait_time:?}. Current status: {:?}. Try increasing the timeout or check if the workload is stuck.",
                            execution_info.status
                        )));
                    }

                    // Wait before next poll with exponential backoff
                    tokio::time::sleep(polling_interval).await;

                    // Exponential backoff: increase interval by 50% each time, capped at max
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

    /// Start WebSocket connection for real-time events
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

        let event_handlers = self.event_handlers.clone();

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
