//! Modern API types with `OpenAPI` support and validation

use std::collections::HashMap;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use toadstool::RuntimeType;
use toadstool_common::{ToadStoolError, ToadStoolErrorWithCode};

/// Modern execution status enum
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Execution has been submitted but not started
    Submitted,
    /// Execution is queued for processing
    Queued,
    /// Execution is currently running
    Running,
    /// Execution completed successfully
    Completed,
    /// Execution failed with error
    Failed,
    /// Execution was cancelled by user
    Cancelled,
    /// Execution timed out
    TimedOut,
    /// Execution is paused
    Paused,
}

/// Modern execution request with validation
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ExecutionRequest {
    /// Workload specification
    #[validate(custom = "validate_workload")]
    pub workload: WorkloadSpec,
    /// Runtime type for execution
    pub runtime_type: RuntimeType,
    /// Execution priority (1-10, higher is more important)
    #[validate(range(min = 1, max = 10))]
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Maximum execution time in seconds
    #[validate(range(min = 1, max = 86400))] // Max 24 hours
    pub timeout_secs: Option<u64>,
    /// Resource requirements
    #[validate]
    pub resources: Option<ResourceRequirements>,
    /// Environment variables
    #[validate(length(max = 100))]
    #[serde(default)]
    pub environment: HashMap<String, String>,
    /// Execution metadata
    #[validate(length(max = 50))]
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Callback URL for notifications
    #[validate(url)]
    pub callback_url: Option<String>,
}

const fn default_priority() -> u8 {
    5
}

fn validate_workload(workload: &WorkloadSpec) -> Result<(), validator::ValidationError> {
    match workload {
        WorkloadSpec::Native { executable, .. } => {
            if executable.is_empty() {
                return Err(validator::ValidationError::new("empty_executable"));
            }
        }
        WorkloadSpec::Container { image, .. } => {
            if image.is_empty() {
                return Err(validator::ValidationError::new("empty_image"));
            }
        }
        WorkloadSpec::Wasm {
            module, function, ..
        } => {
            if module.is_empty() || function.is_empty() {
                return Err(validator::ValidationError::new("empty_wasm_spec"));
            }
        }
        WorkloadSpec::Python { script, .. } => {
            if script.is_empty() {
                return Err(validator::ValidationError::new("empty_script"));
            }
        }
        WorkloadSpec::Gpu {
            kernel, platform, ..
        } => {
            if kernel.is_empty() || platform.is_empty() {
                return Err(validator::ValidationError::new("empty_gpu_spec"));
            }
        }
    }
    Ok(())
}

/// Workload specification
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "spec")]
pub enum WorkloadSpec {
    /// Native binary execution
    Native {
        executable: String,
        args: Vec<String>,
    },
    /// Container execution
    Container {
        image: String,
        command: Option<Vec<String>>,
        args: Option<Vec<String>>,
    },
    /// WebAssembly execution
    Wasm {
        module: String,
        function: String,
        args: Vec<String>,
    },
    /// Python script execution
    Python {
        script: String,
        requirements: Option<Vec<String>>,
    },
    /// GPU computation
    Gpu {
        kernel: String,
        platform: String,
        args: Vec<String>,
    },
}

/// Resource requirements specification
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct ResourceRequirements {
    /// CPU cores (can be fractional)
    #[validate(range(min = 0.1, max = 1000.0))]
    pub cpu_cores: Option<f64>,
    /// Memory in MB
    #[validate(range(min = 1, max = 1048576))] // Max 1TB
    pub memory_mb: Option<u64>,
    /// Storage in MB
    #[validate(range(min = 1, max = 10485760))] // Max 10TB
    pub storage_mb: Option<u64>,
    /// GPU count
    #[validate(range(min = 1, max = 100))]
    pub gpu_count: Option<u32>,
    /// Network bandwidth in Mbps
    #[validate(range(min = 1, max = 100000))]
    pub network_mbps: Option<u64>,
}

/// Modern execution response with comprehensive information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecutionResponse {
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub submitted_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub queue_position: Option<u32>,
    pub resource_allocation: Option<ResourceAllocation>,
    pub monitoring_endpoints: MonitoringEndpoints,
}

/// Resource allocation information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResourceAllocation {
    pub node_id: String,
    pub cpu_cores: f64,
    pub memory_mb: u64,
    pub storage_mb: u64,
    pub gpu_count: u32,
}

/// Monitoring endpoints for execution
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MonitoringEndpoints {
    pub status_url: String,
    pub logs_url: String,
    pub metrics_url: String,
    pub websocket_url: String,
}

/// Modern execution information with comprehensive data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionInfo {
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub runtime_type: RuntimeType,
    pub submitted_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub progress: Option<f64>,
    pub error_message: Option<String>,
    pub resource_usage: Option<ResourceUsage>,
    pub metadata: HashMap<String, String>,
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub network_bytes_in: u64,
    pub network_bytes_out: u64,
    pub gpu_percent: Option<f64>,
}

/// Modern cluster status with detailed information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterStatusResponse {
    pub cluster_id: String,
    pub total_nodes: u32,
    pub healthy_nodes: u32,
    pub cluster_load: f64,
    pub active_executions: u32,
    pub queued_executions: u32,
    pub total_capacity: ClusterCapacity,
    pub used_capacity: ClusterCapacity,
    pub node_details: Vec<ClusterNodeInfo>,
    pub last_updated: DateTime<Utc>,
}

/// Cluster capacity information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterCapacity {
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub gpu_count: u32,
}

/// Cluster node information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterNodeInfo {
    pub id: String,
    pub address: String,
    pub status: NodeStatus,
    pub capabilities: Vec<String>,
    pub resources: NodeResources,
}

/// Node status enum
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Offline,
}

/// Node resource information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeResources {
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub gpu_count: u32,
}

/// Modern API events with structured data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "data")]
pub enum ApiEvent {
    /// Execution lifecycle events
    ExecutionStarted {
        execution_id: Uuid,
        runtime_type: RuntimeType,
        timestamp: DateTime<Utc>,
    },
    ExecutionCompleted {
        execution_id: Uuid,
        status: ExecutionStatus,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    ExecutionFailed {
        execution_id: Uuid,
        error: String,
        timestamp: DateTime<Utc>,
    },

    /// Cluster events
    ClusterNodeAdded {
        node_id: String,
        node_info: ClusterNodeInfo,
        timestamp: DateTime<Utc>,
    },
    ClusterNodeRemoved {
        node_id: String,
        timestamp: DateTime<Utc>,
    },
    ClusterLoadChanged {
        current_load: f64,
        timestamp: DateTime<Utc>,
    },

    /// System events
    AlertTriggered {
        alert_id: Uuid,
        severity: AlertSeverity,
        message: String,
        timestamp: DateTime<Utc>,
    },
    MetricsUpdated {
        metrics: ApiMetrics,
        timestamp: DateTime<Utc>,
    },
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// API metrics tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ApiMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub active_connections: u32,
    pub uptime_seconds: u64,
}

/// Execution logs response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecutionLogs {
    pub execution_id: Uuid,
    pub logs: Vec<LogEntry>,
    pub total_lines: u64,
    pub has_more: bool,
    pub next_token: Option<String>,
}

/// Log entry structure
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub source: String,
}

/// Log level enum
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Execution metrics response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecutionMetrics {
    pub execution_id: Uuid,
    pub metrics: Vec<MetricPoint>,
    pub time_range: TimeRange,
}

/// Metric point
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
}

/// Time range for metrics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Paginated response wrapper
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

/// Pagination information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub total_items: u64,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Execution filter parameters
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate, IntoParams)]
#[serde(default)]
pub struct ExecutionFilter {
    pub status: Option<ExecutionStatus>,
    pub runtime_type: Option<RuntimeType>,
    pub submitted_after: Option<DateTime<Utc>>,
    pub submitted_before: Option<DateTime<Utc>>,
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<u32>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<u32>,
}

impl Default for ExecutionFilter {
    fn default() -> Self {
        Self {
            status: None,
            runtime_type: None,
            submitted_after: None,
            submitted_before: None,
            page: Some(1),
            per_page: Some(20),
        }
    }
}

/// Modern API error with structured information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub error_code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub request_id: Option<String>,
    pub documentation_url: Option<String>,
}

impl ApiError {
    #[must_use]
    pub fn new(error_code: &str, message: &str) -> Self {
        Self {
            error_code: error_code.to_string(),
            message: message.to_string(),
            details: None,
            timestamp: Utc::now(),
            request_id: None,
            documentation_url: Some("https://docs.toadstool.dev/api/errors".to_string()),
        }
    }

    #[must_use]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    #[must_use]
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    #[must_use]
    pub fn validation_error(errors: &validator::ValidationErrors) -> Self {
        let details = serde_json::to_value(errors).unwrap_or_default();
        Self::new("VALIDATION_ERROR", "Request validation failed").with_details(details)
    }

    /// Create from ToadStoolError (legacy support)
    #[must_use]
    pub fn from_toadstool_error(err: ToadStoolError) -> Self {
        // Use generic code for legacy errors
        let error_code = match err {
            ToadStoolError::Execution(_) => "EXECUTION_ERROR",
            ToadStoolError::Configuration(_) => "CONFIG_ERROR",
            ToadStoolError::Resource(_) => "RESOURCE_ERROR",
            ToadStoolError::Integration(_) => "INTEGRATION_ERROR",
            ToadStoolError::Security(_) => "SECURITY_ERROR",
            ToadStoolError::Network(_) => "NETWORK_ERROR",
            ToadStoolError::System(_) => "SYSTEM_ERROR",
        };
        Self::new(error_code, &err.to_string())
    }
}

/// Conversion from ToadStoolErrorWithCode to ApiError (with structured codes)
impl From<ToadStoolErrorWithCode> for ApiError {
    fn from(err: ToadStoolErrorWithCode) -> Self {
        // If error has a structured code, use it
        if let Some(code) = err.error_code() {
            let mut api_err = Self::new(code.code, &err.error.to_string());

            // Add remediation as details if available
            if let Some(remediation) = code.remediation {
                let details = serde_json::json!({
                    "category": code.category_str(),
                    "remediation": remediation
                });
                api_err = api_err.with_details(details);
            }

            api_err
        } else {
            // Fall back to legacy conversion
            Self::from_toadstool_error(err.error)
        }
    }
}

/// Conversion from ToadStoolError to ApiError (for compatibility)
impl From<ToadStoolError> for ApiError {
    fn from(err: ToadStoolError) -> Self {
        Self::from_toadstool_error(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = match self.error_code.as_str() {
            "VALIDATION_ERROR" => StatusCode::BAD_REQUEST,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "MISSING_TOKEN" => StatusCode::UNAUTHORIZED,
            "INVALID_TOKEN" => StatusCode::UNAUTHORIZED,
            "EXPIRED_TOKEN" => StatusCode::UNAUTHORIZED,
            "FORBIDDEN" => StatusCode::FORBIDDEN,
            "RATE_LIMITED" => StatusCode::TOO_MANY_REQUESTS,
            "TIMEOUT" => StatusCode::REQUEST_TIMEOUT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(serde_json::json!({
            "error_code": self.error_code,
            "message": self.message,
            "details": self.details,
            "timestamp": self.timestamp,
            "request_id": self.request_id,
            "documentation_url": self.documentation_url
        }));

        (status_code, body).into_response()
    }
}

/// Modern API server configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiConfig {
    /// Server bind address
    pub bind_address: String,
    /// Enable REST API endpoints
    pub enable_rest: bool,
    /// Enable WebSocket real-time updates
    pub enable_websocket: bool,
    /// Enable CORS support
    pub cors_enabled: bool,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Enable `OpenAPI` documentation
    pub enable_openapi: bool,
    /// API version
    pub api_version: String,
    /// Enable authentication
    pub enable_auth: bool,
    /// JWT secret for authentication
    pub jwt_secret: Option<String>,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limit requests per minute
    pub rate_limit_rpm: u32,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Enable request tracing
    pub enable_tracing: bool,
}

impl Default for ApiConfig {
    #[allow(deprecated)] // Using deprecated field during migration to capability-based discovery
    fn default() -> Self {
        let config = toadstool_config::env_config::EnvironmentConfig::from_env();
        Self {
            bind_address: format!(
                "{}:{}",
                config.network.bind_address, config.network.songbird_port
            ),
            enable_rest: true,
            enable_websocket: true,
            cors_enabled: true,
            request_timeout_secs: 30,
            enable_openapi: true,
            api_version: "2.0.0".to_string(),
            enable_auth: false,
            jwt_secret: None,
            enable_rate_limiting: false,
            rate_limit_rpm: 1000,
            enable_metrics: true,
            enable_tracing: true,
        }
    }
}

/// Authentication claims for JWT
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthClaims {
    pub sub: String, // Subject (user ID)
    pub exp: u64,    // Expiration time
    pub iat: u64,    // Issued at
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// Authentication request
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct AuthRequest {
    #[validate(length(min = 1, max = 255))]
    pub username: String,
    #[validate(length(min = 1, max = 255))]
    pub password: String,
}

/// Authentication response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: Vec<HealthCheck>,
}

/// Individual health check
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: u64,
}
