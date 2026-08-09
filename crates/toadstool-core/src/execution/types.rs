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
    pub execution_id: Uuid,
    pub workload: WorkloadSpec,
    pub runtime_hint: Option<RuntimeType>,
    pub resources: ResourceRequirements,
    pub security_context: SecurityContext,
    pub timeout: Option<Duration>,
    pub environment: HashMap<String, String>,
    pub input_data: ExecutionInput,
    pub callback_config: Option<CallbackConfig>,
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
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub output: ExecutionOutput,
    pub metrics: RuntimeMetrics,
    pub duration: Duration,
    pub runtime_used: RuntimeType,
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
    Success,
    Failed {
        error: Cow<'static, str>,
    },
    Cancelled,
    TimedOut,
    Running,
    Pending,
}

/// Input data for execution.
///
/// `data` is [`bytes::Bytes`] (an `Arc<[u8]>`): cloning the struct across
/// handlers or threads is a refcount bump, not a memcpy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionInput {
    pub data: Bytes,
    pub format: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Output from execution.
///
/// `data` is [`bytes::Bytes`] so result payloads can be shared with a cache
/// layer and the original caller simultaneously without copying.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionOutput {
    pub data: Bytes,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_code: Option<i32>,
    pub format: Option<String>,
    pub result: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
}

/// Callback configuration for execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackConfig {
    pub url: String,
    pub auth_token: Option<String>,
    pub events: Vec<CallbackEvent>,
}

/// Events that can trigger callbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallbackEvent {
    Started,
    Completed,
    Failed,
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
    Native,
    Wasm,
    Container,
    Gpu,
    Python,
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
    pub supported_workloads: Vec<WorkloadType>,
    pub max_concurrent_executions: Option<u32>,
    pub supported_architectures: Vec<String>,
    pub platform_features: HashMap<String, bool>,
    pub version: String,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    pub settings: HashMap<String, serde_json::Value>,
    pub resource_limits: Option<ResourceLimits>,
    pub security_settings: Option<SecuritySettings>,
    pub logging: Option<LoggingConfig>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub destination: String,
}
