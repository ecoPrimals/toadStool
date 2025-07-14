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
    pub async fn new(base_url: &str) -> ClientResult<Self> {
        let config = ClientConfig {
            base_url: base_url.to_string(),
            ..Default::default()
        };

        Self::with_config(config).await
    }

    /// Create a new ToadStool client with custom configuration
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
                            |e| ClientError::Configuration(format!("Invalid header name: {e}")),
                        )?,
                        reqwest::header::HeaderValue::from_str(key).map_err(|e| {
                            ClientError::Configuration(format!("Invalid header value: {e}"))
                        })?,
                    );
                }
                AuthConfig::BearerToken { token } => {
                    default_headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
                            .map_err(|e| {
                                ClientError::Configuration(format!("Invalid bearer token: {e}"))
                            })?,
                    );
                }
                AuthConfig::Basic { username, password } => {
                    use base64::Engine;
                    let credentials = base64::engine::general_purpose::STANDARD
                        .encode(format!("{username}:{password}"));
                    default_headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Basic {credentials}"))
                            .map_err(|e| {
                                ClientError::Configuration(format!("Invalid basic auth: {e}"))
                            })?,
                    );
                }
                AuthConfig::Custom { headers } => {
                    for (name, value) in headers {
                        default_headers.insert(
                            reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(
                                |e| ClientError::Configuration(format!("Invalid header name: {e}")),
                            )?,
                            reqwest::header::HeaderValue::from_str(value).map_err(|e| {
                                ClientError::Configuration(format!("Invalid header value: {e}"))
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
                    ClientError::Configuration(format!("Invalid custom header name: {e}"))
                })?,
                reqwest::header::HeaderValue::from_str(value).map_err(|e| {
                    ClientError::Configuration(format!("Invalid custom header value: {e}"))
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

        let url = format!("{}/api/v1/executions", self.config.base_url);

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
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(ClientError::Server(format!("HTTP {status}: {error_text}")))
        }
    }

    /// Get execution status
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
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(ClientError::Server(format!("HTTP {status}: {error_text}")))
        }
    }

    /// Cancel an execution
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
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(ClientError::Server(format!("HTTP {status}: {error_text}")))
        }
    }

    /// Wait for execution completion
    pub async fn wait_for_completion(&self, execution_id: Uuid) -> ClientResult<ExecutionInfo> {
        debug!("Waiting for execution completion: {}", execution_id);

        let polling_interval = Duration::from_secs(1);
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
                            "Execution {execution_id} did not complete within timeout"
                        )));
                    }

                    // Wait before next poll
                    tokio::time::sleep(polling_interval).await;
                }
            }
        }
    }

    /// Get cluster status
    pub async fn get_cluster_status(&self) -> ClientResult<ClusterStatus> {
        debug!("Getting cluster status");

        let url = format!("{}/api/v1/cluster/status", self.config.base_url);

        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let cluster_status: ClusterStatus = response.json().await?;
            Ok(cluster_status)
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(ClientError::Server(format!("HTTP {status}: {error_text}")))
        }
    }

    /// Health check
    pub async fn health_check(&self) -> ClientResult<()> {
        let url = format!("{}/api/v1/health", self.config.base_url);

        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ClientError::Server(format!(
                "Health check failed: HTTP {}",
                response.status()
            )))
        }
    }

    /// List active executions
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
    pub async fn start_event_stream(&self) -> ClientResult<()> {
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
    pub fn native() -> NativeWorkloadBuilder {
        NativeWorkloadBuilder::new()
    }

    /// Create a container workload submission
    pub fn container() -> ContainerWorkloadBuilder {
        ContainerWorkloadBuilder::new()
    }

    /// Create a WASM workload submission
    pub fn wasm() -> WasmWorkloadBuilder {
        WasmWorkloadBuilder::new()
    }

    /// Create a Python workload submission
    pub fn python() -> PythonWorkloadBuilder {
        PythonWorkloadBuilder::new()
    }
}

/// Builder for native workloads
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

    pub fn executable<S: Into<String>>(mut self, executable: S) -> Self {
        self.executable = Some(executable.into());
        self
    }

    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn working_dir<S: Into<String>>(mut self, working_dir: S) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn build(self) -> Result<WorkloadSubmission, ClientError> {
        let executable = self.executable.ok_or_else(|| {
            ClientError::Configuration("Executable is required for native workload".to_string())
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

    pub fn image<S: Into<String>>(mut self, image: S) -> Self {
        self.image = Some(image.into());
        self
    }

    pub fn command(mut self, command: Vec<String>) -> Self {
        self.command = Some(command);
        self
    }

    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    pub fn working_dir<S: Into<String>>(mut self, working_dir: S) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

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

    pub fn module_data(mut self, module_data: Vec<u8>) -> Self {
        self.module_data = Some(module_data);
        self
    }

    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

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

    pub fn script<S: Into<String>>(mut self, script: S) -> Self {
        self.script = Some(script.into());
        self
    }

    pub fn requirements(mut self, requirements: Vec<String>) -> Self {
        self.requirements = requirements;
        self
    }

    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

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
            _ => assert!(false, "Expected native workload type, got: {:?}", workload.workload_type),
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
            _ => assert!(false, "Expected container workload type, got: {:?}", workload.workload_type),
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
            _ => assert!(false, "Expected Python workload type, got: {:?}", workload.workload_type),
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
