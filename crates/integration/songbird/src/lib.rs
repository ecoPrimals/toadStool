//! # ToadStool Songbird Integration
//!
//! Comprehensive Songbird ecosystem integration providing service discovery,
//! capability registration, health monitoring, and intelligent request routing
//! for seamless ecosystem communication.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeType,
};
use toadstool::resources::RuntimeMetrics;
use toadstool::security::SecurityContext;

/// Songbird integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdConfig {
    /// Songbird service discovery endpoint
    pub discovery_endpoint: String,
    /// Service registration endpoint
    pub registration_endpoint: String,
    /// Health reporting endpoint
    pub health_endpoint: String,
    /// Metrics reporting endpoint
    pub metrics_endpoint: String,
    /// Authentication token for Songbird API
    pub auth_token: Option<String>,
    /// Service registration interval in seconds
    pub registration_interval_secs: u64,
    /// Health reporting interval in seconds
    pub health_reporting_interval_secs: u64,
    /// Capability reporting interval in seconds
    pub capability_reporting_interval_secs: u64,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Retry configuration
    pub retry_config: RetryConfig,
}

impl Default for SongbirdConfig {
    fn default() -> Self {
        Self {
            discovery_endpoint: "http://localhost:8080/api/v1/discovery".to_string(),
            registration_endpoint: "http://localhost:8080/api/v1/register".to_string(),
            health_endpoint: "http://localhost:8080/api/v1/health".to_string(),
            metrics_endpoint: "http://localhost:8080/api/v1/metrics".to_string(),
            auth_token: None,
            registration_interval_secs: 30,
            health_reporting_interval_secs: 15,
            capability_reporting_interval_secs: 60,
            request_timeout_secs: 10,
            retry_config: RetryConfig::default(),
        }
    }
}

/// Retry configuration for Songbird communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

/// ToadStool service registration with Songbird
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistration {
    /// Unique service identifier
    pub service_id: String,
    /// Service type identifier
    pub service_type: String,
    /// Service version
    pub version: String,
    /// Instance identifier
    pub instance_id: String,
    /// Service capabilities
    pub capabilities: ToadStoolCapabilities,
    /// Service endpoints
    pub endpoints: Vec<ServiceEndpoint>,
    /// Health check configuration
    pub health_check: HealthCheckConfig,
    /// Service metadata
    pub metadata: HashMap<String, String>,
    /// Service tags for discovery
    pub tags: Vec<String>,
}

/// Service endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Endpoint type (http, grpc, websocket, etc.)
    pub endpoint_type: String,
    /// Endpoint URL
    pub url: String,
    /// Endpoint capabilities
    pub capabilities: Vec<String>,
    /// Protocol version
    pub protocol_version: String,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check endpoint URL
    pub endpoint: String,
    /// Check interval in seconds
    pub interval_secs: u64,
    /// Timeout for health checks
    pub timeout_secs: u64,
    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,
}

/// ToadStool capabilities for Songbird registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToadStoolCapabilities {
    /// Available execution environments
    pub execution_environments: Vec<ExecutionEnvironment>,
    /// Current resource capacity
    pub resource_capacity: ResourceCapacity,
    /// Supported runtime technologies
    pub supported_runtimes: Vec<RuntimeType>,
    /// Security and sandboxing features
    pub security_features: Vec<SecurityFeature>,
    /// Performance characteristics
    pub performance_metrics: PerformanceMetrics,
    /// Platform-specific capabilities
    pub platform_capabilities: PlatformCapabilities,
}

/// Execution environment types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEnvironment {
    Container { runtime: String },
    Wasm { runtime: String },
    Native { isolation: String },
    Gpu { compute_type: String },
}

/// Resource capacity specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapacity {
    /// Total CPU cores available
    pub cpu_cores: u32,
    /// Total memory in GB
    pub memory_gb: f64,
    /// Available GPU memory in GB
    pub gpu_memory_gb: Option<f64>,
    /// Available disk space in GB
    pub disk_space_gb: f64,
    /// Current utilization percentage
    pub current_utilization: f64,
}

/// Security features available
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityFeature {
    Sandboxing,
    ResourceLimiting,
    NetworkIsolation,
    FileSystemIsolation,
    CapabilityDropping,
    Seccomp,
    AppArmor,
    SELinux,
}

/// Performance metrics for capability reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average execution startup time in ms
    pub avg_startup_time_ms: f64,
    /// Average request processing time in ms
    pub avg_processing_time_ms: f64,
    /// Current requests per second
    pub current_rps: f64,
    /// Maximum supported concurrent executions
    pub max_concurrent_executions: u32,
    /// Resource efficiency score (0-100)
    pub resource_efficiency_score: f64,
}

/// Platform-specific capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    /// Operating system
    pub os: String,
    /// CPU architecture
    pub architecture: String,
    /// Available CPU features
    pub cpu_features: Vec<String>,
    /// Available GPU models
    pub gpu_models: Vec<String>,
    /// Network capabilities
    pub network_capabilities: Vec<String>,
}

/// ToadStool health status for Songbird reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToadStoolHealthStatus {
    /// Overall health status
    pub status: HealthStatus,
    /// Current resource utilization
    pub resource_utilization: ResourceUtilization,
    /// Active executions count
    pub active_executions: u32,
    /// Performance snapshot
    pub performance: PerformanceSnapshot,
    /// Error rates
    pub error_rates: ErrorRates,
    /// System information
    pub system_info: SystemInfo,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded {
        reason: String,
        severity: u8,
    },
    Unhealthy {
        reason: String,
        estimated_recovery: Option<Duration>,
    },
    Maintenance {
        message: String,
        duration: Option<Duration>,
    },
}

/// Resource utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// CPU usage percentage
    pub cpu_percent: f64,
    /// Memory usage percentage
    pub memory_percent: f64,
    /// Storage usage percentage
    pub storage_percent: f64,
    /// Network usage in Mbps
    pub network_mbps: f64,
    /// GPU usage percentage
    pub gpu_percent: Option<f64>,
}

/// Performance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    /// Current throughput (jobs/second)
    pub throughput: f64,
    /// Average response time (ms)
    pub avg_response_time_ms: f64,
    /// Queue depth
    pub queue_depth: u32,
    /// Success rate (0-1)
    pub success_rate: f64,
}

/// Error rates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRates {
    /// Execution error rate (0-1)
    pub execution_error_rate: f64,
    /// Resource allocation error rate (0-1)
    pub resource_error_rate: f64,
    /// Network error rate (0-1)
    pub network_error_rate: f64,
    /// Timeout rate (0-1)
    pub timeout_rate: f64,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// System uptime in seconds
    pub uptime_secs: u64,
    /// Load averages
    pub load_averages: [f64; 3],
    /// Available disk space in GB
    pub available_disk_gb: f64,
    /// Network interface status
    pub network_interfaces: Vec<String>,
}

/// Songbird request from ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdRequest {
    /// Request identifier
    pub request_id: String,
    /// Source service identifier
    pub source_service: String,
    /// Request type
    pub request_type: RequestType,
    /// Request payload
    pub payload: serde_json::Value,
    /// Security context
    pub security_context: SecurityContext,
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Request timestamp
    pub timestamp: DateTime<Utc>,
}

/// Request types from Songbird
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestType {
    Execute,
    HealthCheck,
    CapabilityQuery,
    ResourceStatus,
    MetricsQuery,
}

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestType::Execute => write!(f, "Execute"),
            RequestType::HealthCheck => write!(f, "HealthCheck"),
            RequestType::CapabilityQuery => write!(f, "CapabilityQuery"),
            RequestType::ResourceStatus => write!(f, "ResourceStatus"),
            RequestType::MetricsQuery => write!(f, "MetricsQuery"),
        }
    }
}

/// Songbird response to ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdResponse {
    /// Request identifier
    pub request_id: String,
    /// Response status
    pub status: ResponseStatus,
    /// Response payload
    pub payload: serde_json::Value,
    /// Response metadata
    pub metadata: HashMap<String, String>,
    /// Response timestamp
    pub timestamp: DateTime<Utc>,
}

/// Response status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStatus {
    Success,
    Error { code: String, message: String },
    Timeout,
    ServiceUnavailable,
}

/// Main Songbird integration client
pub struct SongbirdIntegration {
    /// Configuration
    config: SongbirdConfig,
    /// HTTP client
    client: Client,
    /// Service registration
    registration: Arc<RwLock<Option<ServiceRegistration>>>,
    /// Current capabilities
    capabilities: Arc<RwLock<ToadStoolCapabilities>>,
    /// Health status
    health_status: Arc<RwLock<ToadStoolHealthStatus>>,
    /// Registration token
    registration_token: Arc<Mutex<Option<String>>>,
    /// Last successful communication
    last_communication: Arc<Mutex<Instant>>,
}

impl SongbirdIntegration {
    /// Create a new Songbird integration client
    pub fn new(config: SongbirdConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            registration: Arc::new(RwLock::new(None)),
            capabilities: Arc::new(RwLock::new(Self::default_capabilities())),
            health_status: Arc::new(RwLock::new(Self::default_health_status())),
            registration_token: Arc::new(Mutex::new(None)),
            last_communication: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Register ToadStool service with Songbird
    #[instrument(skip(self))]
    pub async fn register_service(&self) -> ToadStoolResult<String> {
        info!("Registering ToadStool service with Songbird");

        let capabilities = self.capabilities.read().await.clone();
        let instance_id = format!("toadstool-{}", Uuid::new_v4().simple());

        let registration = ServiceRegistration {
            service_id: "toadstool-compute".to_string(),
            service_type: "compute-platform".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            instance_id: instance_id.clone(),
            capabilities,
            endpoints: vec![
                ServiceEndpoint {
                    endpoint_type: "http".to_string(),
                    url: "http://localhost:8082".to_string(),
                    capabilities: vec!["execute".to_string(), "health".to_string()],
                    protocol_version: "1.0".to_string(),
                },
                ServiceEndpoint {
                    endpoint_type: "grpc".to_string(),
                    url: "grpc://localhost:8083".to_string(),
                    capabilities: vec!["execute".to_string(), "stream".to_string()],
                    protocol_version: "1.0".to_string(),
                },
            ],
            health_check: HealthCheckConfig {
                endpoint: "http://localhost:8082/health".to_string(),
                interval_secs: 30,
                timeout_secs: 5,
                failure_threshold: 3,
            },
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("platform".to_string(), std::env::consts::OS.to_string());
                metadata.insert(
                    "architecture".to_string(),
                    std::env::consts::ARCH.to_string(),
                );
                metadata.insert("startup_time".to_string(), Utc::now().to_rfc3339());
                metadata
            },
            tags: vec![
                "compute".to_string(),
                "execution".to_string(),
                "sandboxing".to_string(),
                format!("platform-{}", std::env::consts::OS),
            ],
        };

        let response = self
            .make_request(&self.config.registration_endpoint, &registration, "POST")
            .await?;

        if response.status().is_success() {
            let registration_response: HashMap<String, serde_json::Value> = response
                .json()
                .await
                .map_err(|e| {
                ToadStoolError::integration(format!("Failed to parse registration response: {}", e))
            })?;

            let token = registration_response
                .get("token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToadStoolError::integration("Missing registration token"))?;

            *self.registration_token.lock().await = Some(token.to_string());
            *self.registration.write().await = Some(registration);

            info!("Successfully registered with Songbird, token: {}", token);
            Ok(instance_id)
        } else {
            Err(ToadStoolError::integration(format!(
                "Registration failed: {}",
                response.status()
            )))
        }
    }

    /// Update service capabilities
    #[instrument(skip(self))]
    pub async fn update_capabilities(
        &self,
        capabilities: ToadStoolCapabilities,
    ) -> ToadStoolResult<()> {
        info!("Updating service capabilities");

        *self.capabilities.write().await = capabilities.clone();

        if let Some(token) = &*self.registration_token.lock().await {
            let response = self
                .make_authenticated_request(
                    &format!("{}/capabilities", self.config.registration_endpoint),
                    &capabilities,
                    "PUT",
                    token,
                )
                .await?;

            if response.status().is_success() {
                info!("Successfully updated capabilities");
                Ok(())
            } else {
                warn!("Failed to update capabilities: {}", response.status());
                Err(ToadStoolError::integration("Failed to update capabilities"))
            }
        } else {
            warn!("No registration token available");
            Err(ToadStoolError::integration("Service not registered"))
        }
    }

    /// Report health status
    #[instrument(skip(self))]
    pub async fn report_health(&self, health_status: ToadStoolHealthStatus) -> ToadStoolResult<()> {
        debug!("Reporting health status to Songbird");

        *self.health_status.write().await = health_status.clone();

        if let Some(token) = &*self.registration_token.lock().await {
            let response = self
                .make_authenticated_request(
                    &self.config.health_endpoint,
                    &health_status,
                    "POST",
                    token,
                )
                .await?;

            if response.status().is_success() {
                *self.last_communication.lock().await = Instant::now();
                debug!("Successfully reported health status");
                Ok(())
            } else {
                warn!("Failed to report health status: {}", response.status());
                Err(ToadStoolError::integration("Failed to report health"))
            }
        } else {
            warn!("No registration token available for health reporting");
            Err(ToadStoolError::integration("Service not registered"))
        }
    }

    /// Handle incoming request from Songbird
    #[instrument(skip(self, request))]
    pub async fn handle_request(
        &self,
        request: SongbirdRequest,
    ) -> ToadStoolResult<SongbirdResponse> {
        info!(
            "Handling Songbird request: {} ({})",
            request.request_id, request.request_type
        );

        let response_payload = match request.request_type {
            RequestType::Execute => {
                // Convert Songbird request to ToadStool execution request
                let execution_request: ExecutionRequest = serde_json::from_value(request.payload)
                    .map_err(|e| {
                    ToadStoolError::integration(format!("Invalid execution request: {}", e))
                })?;

                // Execute the request (this would integrate with the actual execution engine)
                let result = self.execute_request(execution_request).await?;
                serde_json::to_value(result).unwrap()
            }
            RequestType::HealthCheck => {
                let health = self.health_status.read().await.clone();
                serde_json::to_value(health).unwrap()
            }
            RequestType::CapabilityQuery => {
                let capabilities = self.capabilities.read().await.clone();
                serde_json::to_value(capabilities).unwrap()
            }
            RequestType::ResourceStatus => {
                let health = self.health_status.read().await;
                serde_json::to_value(&health.resource_utilization).unwrap()
            }
            RequestType::MetricsQuery => {
                let health = self.health_status.read().await;
                serde_json::to_value(&health.performance).unwrap()
            }
        };

        Ok(SongbirdResponse {
            request_id: request.request_id,
            status: ResponseStatus::Success,
            payload: response_payload,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        })
    }

    /// Start background tasks for Songbird integration
    pub async fn start_background_tasks(self: Arc<Self>) -> ToadStoolResult<()> {
        info!("Starting Songbird integration background tasks");

        // Capability reporting task
        let capability_integration = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                capability_integration
                    .config
                    .capability_reporting_interval_secs,
            ));

            loop {
                interval.tick().await;
                let capabilities = capability_integration.collect_current_capabilities().await;
                if let Err(e) = capability_integration
                    .update_capabilities(capabilities)
                    .await
                {
                    warn!("Failed to update capabilities: {}", e);
                }
            }
        });

        // Health reporting task
        let health_integration = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                health_integration.config.health_reporting_interval_secs,
            ));

            loop {
                interval.tick().await;
                let health_status = health_integration.collect_current_health().await;
                if let Err(e) = health_integration.report_health(health_status).await {
                    warn!("Failed to report health: {}", e);
                }
            }
        });

        // Registration maintenance task
        let registration_integration = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                registration_integration.config.registration_interval_secs,
            ));

            loop {
                interval.tick().await;

                // Check if registration needs renewal
                let last_comm = *registration_integration.last_communication.lock().await;
                if last_comm.elapsed() > Duration::from_secs(120) {
                    info!("Re-registering with Songbird due to communication timeout");
                    if let Err(e) = registration_integration.register_service().await {
                        error!("Failed to re-register with Songbird: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Deregister from Songbird
    #[instrument(skip(self))]
    pub async fn deregister(&self) -> ToadStoolResult<()> {
        info!("Deregistering from Songbird");

        if let Some(token) = &*self.registration_token.lock().await {
            let response = self
                .make_authenticated_request(
                    &format!("{}/deregister", self.config.registration_endpoint),
                    &serde_json::json!({}),
                    "DELETE",
                    token,
                )
                .await?;

            if response.status().is_success() {
                *self.registration_token.lock().await = None;
                *self.registration.write().await = None;
                info!("Successfully deregistered from Songbird");
                Ok(())
            } else {
                warn!("Failed to deregister: {}", response.status());
                Err(ToadStoolError::integration("Failed to deregister"))
            }
        } else {
            warn!("No registration token available for deregistration");
            Ok(()) // Already deregistered
        }
    }

    // Helper methods

    async fn make_request<T: Serialize>(
        &self,
        url: &str,
        payload: &T,
        method: &str,
    ) -> ToadStoolResult<reqwest::Response> {
        let mut request = match method {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url).json(payload),
            "PUT" => self.client.put(url).json(payload),
            "DELETE" => self.client.delete(url).json(payload),
            _ => return Err(ToadStoolError::integration("Unsupported HTTP method")),
        };

        if let Some(auth_token) = &self.config.auth_token {
            request = request.bearer_auth(auth_token);
        }

        request
            .send()
            .await
            .map_err(|e| ToadStoolError::integration(format!("HTTP request failed: {}", e)))
    }

    async fn make_authenticated_request<T: Serialize>(
        &self,
        url: &str,
        payload: &T,
        method: &str,
        token: &str,
    ) -> ToadStoolResult<reqwest::Response> {
        let request = match method {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url).json(payload),
            "PUT" => self.client.put(url).json(payload),
            "DELETE" => self.client.delete(url).json(payload),
            _ => return Err(ToadStoolError::integration("Unsupported HTTP method")),
        };

        request
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ToadStoolError::integration(format!("HTTP request failed: {}", e)))
    }

    async fn execute_request(
        &self,
        _request: ExecutionRequest,
    ) -> ToadStoolResult<ExecutionResponse> {
        // This would integrate with the actual ToadStool execution engine
        // For now, return a mock response
        Ok(ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Success,
            output: ExecutionOutput {
                data: Vec::new(),
                result: HashMap::new(),
                stdout: Some("Mock execution completed".to_string()),
                stderr: None,
                exit_code: Some(0),
                format: None,
            },
            metrics: RuntimeMetrics::default(),
            duration: Duration::from_secs(1),
            runtime_used: RuntimeType::Native,
            warnings: vec![],
        })
    }

    async fn collect_current_capabilities(&self) -> ToadStoolCapabilities {
        // This would collect real capabilities from the system
        // For now, return default capabilities
        Self::default_capabilities()
    }

    async fn collect_current_health(&self) -> ToadStoolHealthStatus {
        // This would collect real health status from the system
        // For now, return default health status
        Self::default_health_status()
    }

    fn default_capabilities() -> ToadStoolCapabilities {
        ToadStoolCapabilities {
            execution_environments: vec![
                ExecutionEnvironment::Native {
                    isolation: "sandbox".to_string(),
                },
                ExecutionEnvironment::Container {
                    runtime: "docker".to_string(),
                },
                ExecutionEnvironment::Wasm {
                    runtime: "wasmtime".to_string(),
                },
            ],
            resource_capacity: ResourceCapacity {
                cpu_cores: 8,
                memory_gb: 32.0,
                gpu_memory_gb: None,
                disk_space_gb: 1000.0,
                current_utilization: 25.0,
            },
            supported_runtimes: vec![
                RuntimeType::Native,
                RuntimeType::Container,
                RuntimeType::Wasm,
            ],
            security_features: vec![
                SecurityFeature::Sandboxing,
                SecurityFeature::ResourceLimiting,
                SecurityFeature::NetworkIsolation,
            ],
            performance_metrics: PerformanceMetrics {
                avg_startup_time_ms: 50.0,
                avg_processing_time_ms: 100.0,
                current_rps: 10.0,
                max_concurrent_executions: 20,
                resource_efficiency_score: 85.0,
            },
            platform_capabilities: PlatformCapabilities {
                os: std::env::consts::OS.to_string(),
                architecture: std::env::consts::ARCH.to_string(),
                cpu_features: vec!["sse4".to_string(), "avx2".to_string()],
                gpu_models: vec![],
                network_capabilities: vec!["gigabit".to_string()],
            },
        }
    }

    fn default_health_status() -> ToadStoolHealthStatus {
        ToadStoolHealthStatus {
            status: HealthStatus::Healthy,
            resource_utilization: ResourceUtilization {
                cpu_percent: 25.0,
                memory_percent: 40.0,
                storage_percent: 60.0,
                network_mbps: 50.0,
                gpu_percent: None,
            },
            active_executions: 3,
            performance: PerformanceSnapshot {
                throughput: 15.0,
                avg_response_time_ms: 120.0,
                queue_depth: 2,
                success_rate: 0.98,
            },
            error_rates: ErrorRates {
                execution_error_rate: 0.02,
                resource_error_rate: 0.01,
                network_error_rate: 0.005,
                timeout_rate: 0.01,
            },
            system_info: SystemInfo {
                uptime_secs: 86400,
                load_averages: [0.5, 0.7, 0.8],
                available_disk_gb: 400.0,
                network_interfaces: vec!["eth0".to_string()],
            },
            last_updated: Utc::now(),
        }
    }
}

/// Songbird integration trait for dependency injection
#[async_trait]
pub trait SongbirdIntegrationTrait: Send + Sync {
    async fn register_service(&self) -> ToadStoolResult<String>;
    async fn update_capabilities(&self, capabilities: ToadStoolCapabilities)
        -> ToadStoolResult<()>;
    async fn report_health(&self, health_status: ToadStoolHealthStatus) -> ToadStoolResult<()>;
    async fn handle_request(&self, request: SongbirdRequest) -> ToadStoolResult<SongbirdResponse>;
    async fn deregister(&self) -> ToadStoolResult<()>;
}

#[async_trait]
impl SongbirdIntegrationTrait for SongbirdIntegration {
    async fn register_service(&self) -> ToadStoolResult<String> {
        self.register_service().await
    }

    async fn update_capabilities(
        &self,
        capabilities: ToadStoolCapabilities,
    ) -> ToadStoolResult<()> {
        self.update_capabilities(capabilities).await
    }

    async fn report_health(&self, health_status: ToadStoolHealthStatus) -> ToadStoolResult<()> {
        self.report_health(health_status).await
    }

    async fn handle_request(&self, request: SongbirdRequest) -> ToadStoolResult<SongbirdResponse> {
        self.handle_request(request).await
    }

    async fn deregister(&self) -> ToadStoolResult<()> {
        self.deregister().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_songbird_integration_creation() {
        let config = SongbirdConfig::default();
        let integration = SongbirdIntegration::new(config);

        // Verify integration was created successfully
        assert!(integration.registration.read().await.is_none());
        assert!(integration.registration_token.lock().await.is_none());
    }

    #[tokio::test]
    async fn test_default_capabilities() {
        let capabilities = SongbirdIntegration::default_capabilities();

        assert!(!capabilities.execution_environments.is_empty());
        assert!(!capabilities.supported_runtimes.is_empty());
        assert!(!capabilities.security_features.is_empty());
        assert!(capabilities.performance_metrics.max_concurrent_executions > 0);
    }

    #[tokio::test]
    async fn test_default_health_status() {
        let health = SongbirdIntegration::default_health_status();

        assert!(matches!(health.status, HealthStatus::Healthy));
        assert!(health.resource_utilization.cpu_percent > 0.0);
        assert!(health.performance.success_rate > 0.0);
    }
}
