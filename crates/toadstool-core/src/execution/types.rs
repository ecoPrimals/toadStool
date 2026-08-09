// SPDX-License-Identifier: AGPL-3.0-or-later
//! Execution types — pure data structures, no async runtime required.

use bytes::Bytes;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use toadstool_common::constants::timeouts;
use uuid::Uuid;

use crate::encryption::EncryptionConfig;
use crate::resources::{ResourceLimits, ResourceRequirements, RuntimeMetrics};
use crate::security::{SecurityContext, SecuritySettings};
use crate::workload::{WorkloadSpec, WorkloadType};

/// Execution request containing all information needed to run a workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Unique identifier for this execution
    pub execution_id: Uuid,
    /// The workload to execute
    pub workload: WorkloadSpec,
    /// Preferred runtime (hint for scheduling)
    pub runtime_hint: Option<RuntimeType>,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Security context
    pub security_context: SecurityContext,
    /// Maximum execution time
    pub timeout: Option<Duration>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Input data for the workload
    pub input_data: ExecutionInput,
    /// Callback configuration
    pub callback_config: Option<CallbackConfig>,
    /// Encryption configuration (for secure distributed workloads)
    pub encryption_config: Option<EncryptionConfig>,
}

impl Default for ExecutionRequest {
    fn default() -> Self {
        Self {
            #[cfg(feature = "runtime")]
            execution_id: Uuid::new_v4(),
            #[cfg(not(feature = "runtime"))]
            execution_id: Uuid::nil(),
            workload: WorkloadSpec::default(),
            runtime_hint: None,
            resources: ResourceRequirements::default(),
            security_context: SecurityContext::default(),
            timeout: Some(timeouts::WORKLOAD_EXECUTION_TIMEOUT),
            environment: HashMap::new(),
            input_data: ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
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
    pub metrics: RuntimeMetrics,
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
            #[cfg(feature = "runtime")]
            execution_id: Uuid::new_v4(),
            #[cfg(not(feature = "runtime"))]
            execution_id: Uuid::nil(),
            status: ExecutionStatus::Success,
            output: ExecutionOutput::default(),
            metrics: RuntimeMetrics::default(),
            duration: Duration::from_secs(0),
            runtime_used: RuntimeType::Native,
            warnings: Vec::new(),
        }
    }
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Completed successfully
    Success,
    /// Failed with an error message
    Failed {
        /// Error description
        error: Cow<'static, str>,
    },
    /// Cancelled by user or system
    Cancelled,
    /// Exceeded timeout
    TimedOut,
    /// Currently executing
    Running,
    /// Queued, not yet started
    Pending,
}

/// Input data for execution.
///
/// `data` is [`bytes::Bytes`] (an `Arc<[u8]>`): cloning the struct across
/// handlers or threads is a refcount bump, not a memcpy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionInput {
    /// Binary input data (zero-copy: clone bumps refcount, not memcpy)
    pub data: Bytes,
    /// Input format identifier
    pub format: Option<String>,
    /// Metadata key-value pairs
    pub metadata: HashMap<String, String>,
}

/// Output from execution.
///
/// `data` is [`bytes::Bytes`] so result payloads can be shared with a cache
/// layer and the original caller simultaneously without copying.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionOutput {
    /// Binary output data (zero-copy sharing via Arc)
    pub data: Bytes,
    /// Captured stdout
    pub stdout: Option<String>,
    /// Captured stderr
    pub stderr: Option<String>,
    /// Process exit code (if applicable)
    pub exit_code: Option<i32>,
    /// Output format identifier
    pub format: Option<String>,
    /// Structured result key-value pairs
    pub result: HashMap<String, String>,
    /// Output metadata
    pub metadata: HashMap<String, String>,
}

/// Callback configuration for execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackConfig {
    /// Callback URL
    pub url: String,
    /// Authentication token for callback
    pub auth_token: Option<String>,
    /// Events to trigger callbacks on
    pub events: Vec<CallbackEvent>,
}

/// Events that can trigger callbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallbackEvent {
    /// Execution started
    Started,
    /// Execution completed successfully
    Completed,
    /// Execution failed
    Failed,
    /// Progress update available
    Progress,
}

fn serialize_arc_str<S>(s: &Arc<str>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(s)
}

fn deserialize_arc_str<'de, D>(de: D) -> Result<Arc<str>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(de)?;
    Ok(Arc::from(s))
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
    /// Custom runtime (zero-copy `Arc<str>` for sharing across threads)
    Custom(
        #[serde(
            serialize_with = "serialize_arc_str",
            deserialize_with = "deserialize_arc_str"
        )]
        Arc<str>,
    ),
}

impl From<String> for RuntimeType {
    fn from(s: String) -> Self {
        Self::Custom(Arc::from(s))
    }
}

impl From<&str> for RuntimeType {
    fn from(s: &str) -> Self {
        Self::Custom(Arc::from(s))
    }
}

/// Runtime engine capabilities
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
    /// Runtime version
    pub version: String,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    /// Runtime-specific settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Resource limits for the runtime
    pub resource_limits: Option<ResourceLimits>,
    /// Security settings
    pub security_settings: Option<SecuritySettings>,
    /// Logging configuration
    pub logging: Option<LoggingConfig>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (e.g. "info", "debug", "trace")
    pub level: String,
    /// Log format (e.g. "json", "pretty")
    pub format: String,
    /// Log destination (e.g. "stdout", "file")
    pub destination: String,
}
