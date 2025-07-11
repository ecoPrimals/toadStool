//! Execution types and runtime engine interface

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ToadStoolResult;

/// Execution request containing all information needed to run a workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Unique identifier for this execution
    pub execution_id: Uuid,
    /// The workload to execute
    pub workload: crate::WorkloadSpec,
    /// Preferred runtime (hint for scheduling)
    pub runtime_hint: Option<RuntimeType>,
    /// Resource requirements
    pub resources: crate::resources::ResourceRequirements,
    /// Security context
    pub security_context: crate::SecurityContext,
    /// Maximum execution time
    pub timeout: Option<Duration>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Input data for the workload
    pub input_data: ExecutionInput,
    /// Callback configuration
    pub callback_config: Option<CallbackConfig>,
}

impl Default for ExecutionRequest {
    fn default() -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            workload: crate::WorkloadSpec::default(),
            runtime_hint: None,
            resources: crate::resources::ResourceRequirements::default(),
            security_context: crate::SecurityContext::default(),
            timeout: Some(Duration::from_secs(300)),
            environment: HashMap::new(),
            input_data: ExecutionInput::default(),
            callback_config: None,
        }
    }
}

/// Response from an execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    /// Execution identifier
    pub execution_id: Uuid,
    /// Execution status
    pub status: ExecutionStatus,
    /// Output from the execution
    pub output: ExecutionOutput,
    /// Runtime metrics
    pub metrics: crate::RuntimeMetrics,
    /// Total execution duration
    pub duration: Duration,
    /// Runtime that was used
    pub runtime_used: RuntimeType,
    /// Warnings generated during execution
    pub warnings: Vec<String>,
}

impl Default for ExecutionResponse {
    fn default() -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Success,
            output: ExecutionOutput::default(),
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::from_secs(0),
            runtime_used: RuntimeType::Native,
            warnings: Vec::new(),
        }
    }
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Execution completed successfully
    Success,
    /// Execution failed
    Failed { error: String },
    /// Execution was cancelled
    Cancelled,
    /// Execution timed out
    TimedOut,
    /// Execution is still running
    Running,
    /// Execution is pending
    Pending,
}

/// Input data for execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionInput {
    /// Binary data
    pub data: Vec<u8>,
    /// Input format
    pub format: Option<String>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Output from execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionOutput {
    /// Binary output data
    pub data: Vec<u8>,
    /// Standard output (text)
    pub stdout: Option<String>,
    /// Standard error (text)
    pub stderr: Option<String>,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Output format
    pub format: Option<String>,
    /// Result metadata
    pub result: HashMap<String, String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Callback configuration for execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackConfig {
    /// Callback URL
    pub url: String,
    /// Authentication token
    pub auth_token: Option<String>,
    /// Events to callback on
    pub events: Vec<CallbackEvent>,
}

/// Events that can trigger callbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallbackEvent {
    /// Execution started
    Started,
    /// Execution completed
    Completed,
    /// Execution failed
    Failed,
    /// Progress update
    Progress,
}

/// Types of runtime engines
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RuntimeType {
    /// Native process execution
    Native,
    /// WebAssembly execution
    Wasm,
    /// Container execution
    Container,
    /// GPU acceleration
    Gpu,
    /// Python runtime
    Python,
    /// Custom runtime
    Custom(String),
}

/// Runtime engine capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    /// Supported workload types
    pub supported_workloads: Vec<crate::WorkloadType>,
    /// Maximum concurrent executions
    pub max_concurrent_executions: Option<u32>,
    /// Supported architectures
    pub supported_architectures: Vec<String>,
    /// Platform-specific features
    pub platform_features: HashMap<String, bool>,
    /// Runtime version
    pub version: String,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    /// Runtime-specific settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Resource limits
    pub resource_limits: Option<crate::resources::ResourceLimits>,
    /// Security settings
    pub security_settings: Option<crate::SecuritySettings>,
    /// Logging configuration
    pub logging: Option<LoggingConfig>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log format
    pub format: String,
    /// Log destination
    pub destination: String,
}

/// Runtime engine trait
#[async_trait]
pub trait RuntimeEngine: Send + Sync {
    /// Initialize the runtime engine
    async fn initialize(&mut self, config: RuntimeConfig) -> ToadStoolResult<()>;
    
    /// Execute a workload
    async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse>;
    
    /// Get runtime capabilities
    fn get_capabilities(&self) -> RuntimeCapabilities;
    
    /// Check if runtime supports a workload type
    fn supports_workload(&self, workload_type: &crate::WorkloadType) -> bool;
    
    /// Get runtime metrics
    async fn get_metrics(&self) -> ToadStoolResult<crate::RuntimeMetrics>;
    
    /// Shutdown the runtime engine
    async fn shutdown(&mut self) -> ToadStoolResult<()>;
}

// RuntimeOrchestrator is now defined in runtime.rs module
// Re-export it here for backward compatibility
pub use crate::runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy};
