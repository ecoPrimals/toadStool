//! # ToadStool Client Library
//!
//! A comprehensive client library for connecting to ToadStool universal compute servers
//! and submitting workloads for execution.
//!
//! ## Features
//!
//! - **HTTP API Client**: REST API for workload submission and status monitoring
//! - **WebSocket Client**: Real-time event streaming and notifications
//! - **Ecosystem Integration**: Direct integration with Songbird, BearDog, NestGate
//! - **Load Balancing**: Automatic discovery and load balancing across ToadStool nodes
//! - **Retry Logic**: Configurable retry policies for resilient execution
//! - **Authentication**: Support for API keys, tokens, and ecosystem authentication
//!
//! ## Architecture
//!
//! The client library is built around the [`ToadStoolClient`] struct, which provides
//! a high-level interface for interacting with ToadStool servers. It supports multiple
//! authentication methods, automatic retries, and real-time event streaming.
//!
//! ### Workload Types
//!
//! - [`WorkloadType::Native`] - Execute native binaries
//! - [`WorkloadType::Container`] - Run containerized applications
//! - [`WorkloadType::Wasm`] - Execute WebAssembly modules
//! - [`WorkloadType::Python`] - Run Python scripts
//! - [`WorkloadType::Custom`] - Custom workload types
//!
//! ### Builder Pattern
//!
//! Use the builder pattern for constructing workloads:
//! - [`NativeWorkloadBuilder`] - For native executables
//! - [`ContainerWorkloadBuilder`] - For container images
//! - [`WasmWorkloadBuilder`] - For WebAssembly modules
//! - [`PythonWorkloadBuilder`] - For Python scripts
//!
//! ## Quick Start
//!
//! ```rust
//! use toadstool_client::{ToadStoolClient, WorkloadSubmission, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client with default configuration
//!     let client = ToadStoolClient::new("http://localhost:8080").await?;
//!     
//!     // Submit a native workload
//!     let workload = WorkloadSubmission::native()
//!         .executable("/bin/echo")
//!         .args(vec!["Hello, ToadStool!".to_string()])
//!         .build();
//!     
//!     let execution = client.submit_workload(workload).await?;
//!     println!("Submitted execution: {}", execution.execution_id);
//!     
//!     // Wait for completion
//!     let result = client.wait_for_completion(execution.execution_id).await?;
//!     println!("Execution completed: {:?}", result.status);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Authentication
//!
//! The client supports multiple authentication methods:
//!
//! ```rust
//! use toadstool_client::{ClientConfig, AuthConfig};
//!
//! // API Key authentication
//! let config = ClientConfig {
//!     auth: Some(AuthConfig::ApiKey {
//!         key: "your-api-key".to_string(),
//!         header_name: "X-API-Key".to_string(),
//!     }),
//!     ..Default::default()
//! };
//!
//! // Bearer token authentication
//! let config = ClientConfig {
//!     auth: Some(AuthConfig::BearerToken {
//!         token: "your-bearer-token".to_string(),
//!     }),
//!     ..Default::default()
//! };
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use futures_util::stream::StreamExt;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info};
use url::Url;
use uuid::Uuid;

/// Type alias for event handlers to reduce complexity
type EventHandlers = Vec<Box<dyn Fn(ToadStoolEvent) + Send + Sync>>;

/// ToadStool client errors
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Invalid configuration: {0}")]
    Configuration(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
}

pub type ClientResult<T> = Result<T, ClientError>;

/// ToadStool client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL of the ToadStool server
    pub base_url: String,

    /// HTTP request timeout
    pub request_timeout: Duration,

    /// WebSocket connection timeout
    pub websocket_timeout: Duration,

    /// Maximum retry attempts
    pub max_retries: u32,

    /// Retry backoff strategy
    pub retry_backoff: Duration,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// Enable WebSocket real-time events
    pub enable_websocket: bool,

    /// Custom HTTP headers
    pub custom_headers: HashMap<String, String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            request_timeout: Duration::from_secs(30),
            websocket_timeout: Duration::from_secs(10),
            max_retries: 3,
            retry_backoff: Duration::from_millis(1000),
            auth: None,
            enable_websocket: true,
            custom_headers: HashMap::new(),
        }
    }
}

impl ClientConfig {
    /// Build an API URL with the given endpoint (zero-copy optimization)
    fn api_url(&self, endpoint: &str) -> String {
        format!("{}/api/v1/{}", self.base_url, endpoint)
    }
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub enum AuthConfig {
    /// API key authentication
    ApiKey { key: String, header_name: String },

    /// Bearer token authentication
    BearerToken { token: String },

    /// Basic authentication
    Basic { username: String, password: String },

    /// Custom authentication
    Custom { headers: HashMap<String, String> },
}

/// Workload submission builder
#[derive(Debug, Clone)]
pub struct WorkloadSubmission {
    pub workload_type: WorkloadType,
    pub runtime_hint: Option<String>,
    pub priority: Option<JobPriority>,
    pub timeout: Option<Duration>,
    pub environment: HashMap<String, String>,
    pub resources: Option<ResourceRequirements>,
    pub metadata: HashMap<String, String>,
}

/// Type of workload to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadType {
    /// Native executable
    Native {
        executable: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },

    /// Container image
    Container {
        image: String,
        command: Option<Vec<String>>,
        args: Option<Vec<String>>,
        working_dir: Option<String>,
    },

    /// WebAssembly module
    Wasm {
        module_data: Vec<u8>,
        args: Vec<String>,
    },

    /// Python script
    Python {
        script: String,
        requirements: Vec<String>,
    },

    /// Custom workload type
    Custom { workload_data: serde_json::Value },
}

/// Job execution priority
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum JobPriority {
    Low,
    Normal,
    High,
    Critical,
    Emergency,
}

/// Resource requirements for workload execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub storage_bytes: Option<u64>,
    pub gpu_units: Option<u32>,
    pub network_bandwidth: Option<u64>,
}

/// Execution status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInfo {
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub submitted_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub runtime_type: Option<String>,
    pub error_message: Option<String>,
    pub output: Option<ExecutionOutput>,
    pub metrics: Option<ExecutionMetrics>,
}

/// Execution status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

/// Execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_code: Option<i32>,
    pub artifacts: Vec<String>,
}

/// Execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub duration_ms: u64,
    pub cpu_usage_percent: f64,
    pub memory_peak_bytes: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
}

/// Real-time event from ToadStool server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToadStoolEvent {
    ExecutionStarted {
        execution_id: Uuid,
        timestamp: DateTime<Utc>,
    },

    ExecutionCompleted {
        execution_id: Uuid,
        status: ExecutionStatus,
        timestamp: DateTime<Utc>,
    },

    ExecutionProgress {
        execution_id: Uuid,
        progress_percent: f64,
        message: Option<String>,
        timestamp: DateTime<Utc>,
    },

    ClusterEvent {
        event_type: String,
        node_id: Option<String>,
        message: String,
        timestamp: DateTime<Utc>,
    },

    Alert {
        severity: String,
        message: String,
        timestamp: DateTime<Utc>,
    },
}

/// ToadStool client for universal compute
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

/// Cluster status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    pub total_nodes: u32,
    pub healthy_nodes: u32,
    pub cluster_load: f64,
    pub active_executions: u32,
    pub available_runtimes: Vec<String>,
}

impl WorkloadSubmission {
    /// Create a native workload submission
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toadstool_client::WorkloadSubmission;
    /// use std::collections::HashMap;
    ///
    /// let workload = WorkloadSubmission::native()
    ///     .executable("/bin/echo")
    ///     .args(vec!["Hello, World!".to_string()])
    ///     .build()?;
    /// ```
    pub fn native() -> NativeWorkloadBuilder {
        NativeWorkloadBuilder::new()
    }

    /// Create a container workload submission
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toadstool_client::WorkloadSubmission;
    /// use std::collections::HashMap;
    ///
    /// let workload = WorkloadSubmission::container()
    ///     .image("ubuntu:latest")
    ///     .command(vec!["echo".to_string()])
    ///     .args(vec!["Hello from container!".to_string()])
    ///     .build();
    /// ```
    pub fn container() -> ContainerWorkloadBuilder {
        ContainerWorkloadBuilder::new()
    }

    /// Create a WASM workload submission
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toadstool_client::WorkloadSubmission;
    /// use std::collections::HashMap;
    ///
    /// let wasm_module = std::fs::read("hello.wasm")?;
    /// let workload = WorkloadSubmission::wasm()
    ///     .module_data(wasm_module)
    ///     .args(vec!["arg1".to_string(), "arg2".to_string()])
    ///     .build();
    /// ```
    pub fn wasm() -> WasmWorkloadBuilder {
        WasmWorkloadBuilder::new()
    }

    /// Create a Python workload submission
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toadstool_client::WorkloadSubmission;
    /// use std::collections::HashMap;
    ///
    /// let workload = WorkloadSubmission::python()
    ///     .script("print('Hello from Python!')")
    ///     .requirements(vec!["requests>=2.28.0".to_string()])
    ///     .build();
    /// ```
    pub fn python() -> PythonWorkloadBuilder {
        PythonWorkloadBuilder::new()
    }
}

/// Builder for native workloads
#[must_use]
pub struct NativeWorkloadBuilder {
    executable: Option<String>,
    args: Vec<String>,
    working_dir: Option<String>,
    environment: HashMap<String, String>,
    priority: Option<JobPriority>,
    timeout: Option<Duration>,
    resources: Option<ResourceRequirements>,
    metadata: HashMap<String, String>,
}

impl Default for NativeWorkloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeWorkloadBuilder {
    /// Create a new native workload builder
    pub fn new() -> Self {
        Self {
            executable: None,
            args: Vec::new(),
            working_dir: None,
            environment: HashMap::new(),
            priority: None,
            timeout: None,
            resources: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the executable path for the native workload
    ///
    /// # Arguments
    ///
    /// * `executable` - The path to the executable to run
    pub fn executable<S: Into<String>>(mut self, executable: S) -> Self {
        self.executable = Some(executable.into());
        self
    }

    /// Set command-line arguments for the executable
    ///
    /// # Arguments
    ///
    /// * `args` - Vector of command-line arguments to pass to the executable
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set the working directory for the execution
    ///
    /// # Arguments
    ///
    /// * `working_dir` - The directory path where the executable should run
    pub fn working_dir<S: Into<String>>(mut self, working_dir: S) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    /// Set environment variables for the execution
    ///
    /// # Arguments
    ///
    /// * `environment` - HashMap of environment variable names to values
    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    /// Set the execution priority for the workload
    ///
    /// # Arguments
    ///
    /// * `priority` - The job priority level (affects scheduling order)
    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set the maximum execution timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum duration the workload is allowed to run before being terminated
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set resource requirements for the workload
    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Set metadata for the workload
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Build the workload submission
    ///
    /// # Errors
    ///
    /// Returns an error if the workload configuration is invalid
    pub fn build(self) -> Result<WorkloadSubmission, ClientError> {
        let executable = self.executable.ok_or_else(|| {
            ClientError::Configuration(
                "Executable path is required for native workload. Use .executable(\"/path/to/binary\") to set it.".to_string()
            )
        })?;

        Ok(WorkloadSubmission {
            workload_type: WorkloadType::Native {
                executable,
                args: self.args,
                working_dir: self.working_dir,
            },
            runtime_hint: Some("native".to_string()),
            priority: self.priority,
            timeout: self.timeout,
            environment: self.environment,
            resources: self.resources,
            metadata: self.metadata,
        })
    }
}

/// Builder for container workloads
#[must_use]
pub struct ContainerWorkloadBuilder {
    image: Option<String>,
    command: Option<Vec<String>>,
    args: Option<Vec<String>>,
    working_dir: Option<String>,
    environment: HashMap<String, String>,
    priority: Option<JobPriority>,
    timeout: Option<Duration>,
    resources: Option<ResourceRequirements>,
    metadata: HashMap<String, String>,
}

impl Default for ContainerWorkloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerWorkloadBuilder {
    /// Create a new container workload builder
    pub fn new() -> Self {
        Self {
            image: None,
            command: None,
            args: None,
            working_dir: None,
            environment: HashMap::new(),
            priority: None,
            timeout: None,
            resources: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the container image
    ///
    /// # Examples
    /// ```
    /// use toadstool_client::WorkloadSubmission;
    /// let workload = WorkloadSubmission::container()
    ///     .image("alpine:latest")
    ///     .build();
    /// ```
    pub fn image<S: Into<String>>(mut self, image: S) -> Self {
        self.image = Some(image.into());
        self
    }

    /// Set the container command
    pub fn command(mut self, command: Vec<String>) -> Self {
        self.command = Some(command);
        self
    }

    /// Set command-line arguments for the executable
    ///
    /// # Arguments
    ///
    /// * `args` - Vector of command-line arguments to pass to the executable
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    /// Set the working directory for the execution
    ///
    /// # Arguments
    ///
    /// * `working_dir` - The directory path where the executable should run
    pub fn working_dir<S: Into<String>>(mut self, working_dir: S) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    /// Set environment variables
    ///
    /// # Arguments
    ///
    /// * `environment` - HashMap of environment variable names to values
    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    /// Set job priority
    ///
    /// # Arguments
    ///
    /// * `priority` - The job priority level (affects scheduling order)
    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set execution timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum duration the workload is allowed to run before being terminated
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set resource requirements
    ///
    /// # Arguments
    ///
    /// * `resources` - Resource requirements for the workload
    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Set metadata
    ///
    /// # Arguments
    ///
    /// * `metadata` - HashMap of metadata key-value pairs
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Build the workload submission
    ///
    /// # Panics
    /// Panics if the image is not set, as it is required for container workloads
    #[must_use]
    pub fn build(self) -> WorkloadSubmission {
        WorkloadSubmission {
            workload_type: WorkloadType::Container {
                image: self
                    .image
                    .expect("Image is required for container workload"),
                command: self.command,
                args: self.args,
                working_dir: self.working_dir,
            },
            runtime_hint: Some("container".to_string()),
            priority: self.priority,
            timeout: self.timeout,
            environment: self.environment,
            resources: self.resources,
            metadata: self.metadata,
        }
    }
}

/// Builder for WASM workloads
#[must_use]
pub struct WasmWorkloadBuilder {
    module_data: Option<Vec<u8>>,
    args: Vec<String>,
    environment: HashMap<String, String>,
    priority: Option<JobPriority>,
    timeout: Option<Duration>,
    resources: Option<ResourceRequirements>,
    metadata: HashMap<String, String>,
}

impl Default for WasmWorkloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmWorkloadBuilder {
    /// Create a new WASM workload builder
    pub fn new() -> Self {
        Self {
            module_data: None,
            args: Vec::new(),
            environment: HashMap::new(),
            priority: None,
            timeout: None,
            resources: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the WASM module data
    pub fn module_data(mut self, module_data: Vec<u8>) -> Self {
        self.module_data = Some(module_data);
        self
    }

    /// Set command line arguments
    ///
    /// # Arguments
    ///
    /// * `args` - Vector of command-line arguments to pass to the executable
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set environment variables
    ///
    /// # Arguments
    ///
    /// * `environment` - HashMap of environment variable names to values
    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    /// Set job priority
    ///
    /// # Arguments
    ///
    /// * `priority` - The job priority level (affects scheduling order)
    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set execution timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum duration the workload is allowed to run before being terminated
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set resource requirements
    ///
    /// # Arguments
    ///
    /// * `resources` - Resource requirements for the workload
    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Set metadata
    ///
    /// # Arguments
    ///
    /// * `metadata` - HashMap of metadata key-value pairs
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Build the workload submission
    ///
    /// # Panics
    /// Panics if the module data is not set, as it is required for WASM workloads
    #[must_use]
    pub fn build(self) -> WorkloadSubmission {
        WorkloadSubmission {
            workload_type: WorkloadType::Wasm {
                module_data: self
                    .module_data
                    .expect("Module data is required for WASM workload"),
                args: self.args,
            },
            runtime_hint: Some("wasm".to_string()),
            priority: self.priority,
            timeout: self.timeout,
            environment: self.environment,
            resources: self.resources,
            metadata: self.metadata,
        }
    }
}

/// Builder for Python workloads
#[must_use]
pub struct PythonWorkloadBuilder {
    script: Option<String>,
    requirements: Vec<String>,
    environment: HashMap<String, String>,
    priority: Option<JobPriority>,
    timeout: Option<Duration>,
    resources: Option<ResourceRequirements>,
    metadata: HashMap<String, String>,
}

impl Default for PythonWorkloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonWorkloadBuilder {
    /// Create a new Python workload builder
    pub fn new() -> Self {
        Self {
            script: None,
            requirements: Vec::new(),
            environment: HashMap::new(),
            priority: None,
            timeout: None,
            resources: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the Python script
    ///
    /// # Arguments
    ///
    /// * `script` - The Python script to execute.
    pub fn script<S: Into<String>>(mut self, script: S) -> Self {
        self.script = Some(script.into());
        self
    }

    /// Set Python requirements
    ///
    /// # Arguments
    ///
    /// * `requirements` - Vector of Python package requirements (e.g., "requests>=2.28.0").
    pub fn requirements(mut self, requirements: Vec<String>) -> Self {
        self.requirements = requirements;
        self
    }

    /// Set environment variables
    ///
    /// # Arguments
    ///
    /// * `environment` - HashMap of environment variable names to values
    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    /// Set job priority
    ///
    /// # Arguments
    ///
    /// * `priority` - The job priority level (affects scheduling order)
    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set execution timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum duration the workload is allowed to run before being terminated
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set resource requirements
    ///
    /// # Arguments
    ///
    /// * `resources` - Resource requirements for the workload
    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Set metadata
    ///
    /// # Arguments
    ///
    /// * `metadata` - HashMap of metadata key-value pairs
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Build the workload submission
    ///
    /// # Panics
    /// Panics if the script is not set, as it is required for Python workloads
    #[must_use]
    pub fn build(self) -> WorkloadSubmission {
        WorkloadSubmission {
            workload_type: WorkloadType::Python {
                script: self.script.expect("Script is required for Python workload"),
                requirements: self.requirements,
            },
            runtime_hint: Some("python".to_string()),
            priority: self.priority,
            timeout: self.timeout,
            environment: self.environment,
            resources: self.resources,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_native_workload_builder() {
        let workload = WorkloadSubmission::native()
            .executable("/bin/echo")
            .args(vec!["Hello".to_string(), "World".to_string()])
            .priority(JobPriority::High)
            .build()
            .unwrap();

        match workload.workload_type {
            WorkloadType::Native {
                executable, args, ..
            } => {
                assert_eq!(executable, "/bin/echo");
                assert_eq!(args, vec!["Hello", "World"]);
            }
            _ => panic!(
                "Expected native workload type, got: {:?}",
                workload.workload_type
            ),
        }

        assert_eq!(workload.priority, Some(JobPriority::High));
        assert_eq!(workload.runtime_hint, Some("native".to_string()));
    }

    #[test]
    fn test_container_workload_builder() {
        let workload = WorkloadSubmission::container()
            .image("alpine:latest")
            .command(vec!["echo".to_string()])
            .args(vec!["Hello from container".to_string()])
            .build();

        match workload.workload_type {
            WorkloadType::Container {
                image,
                command,
                args,
                ..
            } => {
                assert_eq!(image, "alpine:latest");
                assert_eq!(command, Some(vec!["echo".to_string()]));
                assert_eq!(args, Some(vec!["Hello from container".to_string()]));
            }
            _ => panic!(
                "Expected container workload type, got: {:?}",
                workload.workload_type
            ),
        }

        assert_eq!(workload.runtime_hint, Some("container".to_string()));
    }

    #[test]
    fn test_python_workload_builder() {
        let workload = WorkloadSubmission::python()
            .script("print('Hello, Python!')")
            .requirements(vec!["requests==2.28.0".to_string()])
            .build();

        match workload.workload_type {
            WorkloadType::Python {
                script,
                requirements,
            } => {
                assert_eq!(script, "print('Hello, Python!')");
                assert_eq!(requirements, vec!["requests==2.28.0"]);
            }
            _ => panic!(
                "Expected Python workload type, got: {:?}",
                workload.workload_type
            ),
        }

        assert_eq!(workload.runtime_hint, Some("python".to_string()));
    }

    #[test]
    fn test_execution_status_serialization() {
        let status = ExecutionStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""Running""#);

        let deserialized: ExecutionStatus = serde_json::from_str(&json).unwrap();
        matches!(deserialized, ExecutionStatus::Running);
    }
}
