//! Core execution interfaces and traits
//!
//! This module defines the universal execution interface that abstracts runtime complexity
//! while providing consistent security, monitoring, and resource management.

use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ToadStoolResult;
use crate::resources::{ResourceRequirements, RuntimeMetrics};
use crate::security::SecurityContext;
use crate::workload::WorkloadSpec;

/// Universal runtime engine interface
///
/// All runtime implementations (Container, WASM, Native, GPU) must implement this trait
/// to provide a consistent execution interface across different runtime types.
#[async_trait]
pub trait RuntimeEngine: Send + Sync + Debug {
    /// Initialize the runtime with configuration
    async fn initialize(&mut self, config: RuntimeConfig) -> ToadStoolResult<()>;

    /// Execute a workload with specified context
    async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse>;

    /// Get runtime capabilities and metadata
    fn get_capabilities(&self) -> RuntimeCapabilities;

    /// Check if runtime supports the given workload type
    fn supports_workload(&self, workload_type: &WorkloadType) -> bool;

    /// Get runtime health and performance metrics
    async fn get_metrics(&self) -> ToadStoolResult<RuntimeMetrics>;

    /// Shutdown runtime gracefully
    async fn shutdown(&mut self) -> ToadStoolResult<()>;
}

/// Universal execution request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Unique execution identifier
    pub execution_id: Uuid,
    /// Workload specification
    pub workload: WorkloadSpec,
    /// Runtime preferences (auto-selected if None)
    pub runtime_hint: Option<RuntimeType>,
    /// Resource requirements and limits
    pub resources: ResourceRequirements,
    /// Security and isolation context
    pub security_context: SecurityContext,
    /// Execution timeout
    pub timeout: Option<Duration>,
    /// Environment variables (runtime-agnostic)
    pub environment: HashMap<String, String>,
    /// Input data and parameters
    pub input_data: ExecutionInput,
    /// Callback configuration for async results
    pub callback_config: Option<CallbackConfig>,
}

/// Universal execution response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    /// Execution identifier (matches request)
    pub execution_id: Uuid,
    /// Execution result status
    pub status: ExecutionStatus,
    /// Output data from execution
    pub output: ExecutionOutput,
    /// Runtime metrics and performance data
    pub metrics: RuntimeMetrics,
    /// Execution duration
    pub duration: Duration,
    /// Runtime that executed the workload
    pub runtime_used: RuntimeType,
    /// Any warnings or non-fatal issues
    pub warnings: Vec<String>,
}

/// Execution status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Execution completed successfully
    Success,
    /// Execution failed with error
    Failed { error: String },
    /// Execution timed out
    TimedOut,
    /// Execution was cancelled
    Cancelled,
    /// Resource limits exceeded
    ResourceLimitExceeded {
        resource: String,
        limit: String,
        actual: String,
    },
    /// Security violation occurred
    SecurityViolation { violation: String },
}

/// Runtime type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RuntimeType {
    /// Container runtime (Docker, Containerd, Podman)
    Container,
    /// WebAssembly runtime (Wasmtime, Wasmer)
    Wasm,
    /// Native process runtime
    Native,
    /// GPU compute runtime
    Gpu,
    /// Custom runtime extension
    Custom(String),
}

/// Workload type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkloadType {
    /// Container image workload
    Container,
    /// WebAssembly module
    Wasm,
    /// Native executable
    Native,
    /// GPU compute kernel
    Gpu,
    /// Script or interpreted code
    Script { interpreter: String },
    /// Custom workload type
    Custom(String),
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime-specific configuration
    pub runtime_config: HashMap<String, serde_json::Value>,
    /// Platform-specific optimizations
    pub platform_optimizations: bool,
    /// Debug mode flag
    pub debug_mode: bool,
    /// Telemetry collection settings
    pub telemetry_enabled: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            runtime_config: HashMap::new(),
            platform_optimizations: true,
            debug_mode: false,
            telemetry_enabled: true,
        }
    }
}

/// Runtime capabilities descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    /// Supported workload types
    pub supported_workloads: Vec<WorkloadType>,
    /// Maximum concurrent executions
    pub max_concurrent_executions: Option<u32>,
    /// Supported architectures
    pub supported_architectures: Vec<String>,
    /// Platform-specific features
    pub platform_features: HashMap<String, bool>,
    /// Version information
    pub version: String,
}

/// Execution input data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionInput {
    /// Binary data payload
    pub data: Vec<u8>,
    /// Structured parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Input format hint
    pub format: Option<String>,
}

/// Execution output data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionOutput {
    /// Binary data result
    pub data: Vec<u8>,
    /// Structured result data
    pub result: HashMap<String, serde_json::Value>,
    /// Standard output (if applicable)
    pub stdout: Option<String>,
    /// Standard error (if applicable)
    pub stderr: Option<String>,
    /// Exit code (if applicable)
    pub exit_code: Option<i32>,
    /// Output format
    pub format: Option<String>,
}

/// Callback configuration for async execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackConfig {
    /// Callback URL for completion notification
    pub callback_url: String,
    /// Authentication token for callback
    pub auth_token: Option<String>,
    /// Callback method (POST, PUT, etc.)
    pub method: String,
    /// Additional headers
    pub headers: HashMap<String, String>,
}

impl Default for CallbackConfig {
    fn default() -> Self {
        Self {
            callback_url: String::new(),
            auth_token: None,
            method: "POST".to_string(),
            headers: HashMap::new(),
        }
    }
}
