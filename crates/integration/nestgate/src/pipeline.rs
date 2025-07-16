//! Data pipeline structures and functionality for `NestGate` integration

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::ArtifactType;

/// Pipeline configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Pipeline identifier
    pub pipeline_id: String,

    /// Pipeline name
    pub name: String,

    /// Input sources
    pub inputs: Vec<PipelineInput>,

    /// Output destinations
    pub outputs: Vec<PipelineOutput>,

    /// Processing steps
    pub steps: Vec<PipelineStep>,

    /// Schedule configuration
    pub schedule: Option<PipelineSchedule>,

    /// Resource requirements
    pub resources: Option<PipelineResources>,
}

/// Pipeline input configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInput {
    /// Input identifier
    pub id: String,

    /// Input type
    pub input_type: InputType,

    /// Configuration for this input
    pub config: HashMap<String, serde_json::Value>,
}

/// Pipeline input types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputType {
    /// File system input
    FileSystem { path: String },
    /// HTTP/REST endpoint
    Http { url: String },
    /// Database query
    Database { connection: String, query: String },
    /// Stream processing
    Stream { topic: String },
    /// Artifact reference
    Artifact { artifact_id: String },
}

/// Pipeline output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
    /// Output identifier
    pub id: String,

    /// Output type
    pub output_type: OutputType,

    /// Configuration for this output
    pub config: HashMap<String, serde_json::Value>,
}

/// Pipeline output types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputType {
    /// File system output
    FileSystem { path: String },
    /// HTTP POST endpoint
    Http { url: String },
    /// Database insert
    Database { connection: String, table: String },
    /// Stream publishing
    Stream { topic: String },
    /// Store as artifact
    Artifact { artifact_type: ArtifactType },
}

/// Pipeline processing step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    /// Step identifier
    pub id: String,

    /// Step name
    pub name: String,

    /// Step type
    pub step_type: StepType,

    /// Dependencies on other steps
    pub depends_on: Vec<String>,

    /// Step configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Pipeline step types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// Data transformation
    Transform { script: String, language: String },
    /// Data filtering
    Filter { condition: String },
    /// Data aggregation
    Aggregate {
        fields: Vec<String>,
        operation: String,
    },
    /// `ToadStool` execution
    ToadStool {
        /// Workload specification
        workload: String,
        /// Runtime configuration
        runtime: Option<String>,
    },
    /// Custom processing
    Custom { processor: String },
}

/// Pipeline scheduling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSchedule {
    /// Schedule type
    pub schedule_type: ScheduleType,

    /// Timezone for scheduling
    pub timezone: Option<String>,

    /// Maximum concurrent executions
    pub max_concurrent: Option<u32>,
}

/// Pipeline schedule types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleType {
    /// Run once immediately
    Once,
    /// Cron-style schedule
    Cron { expression: String },
    /// Interval-based schedule
    Interval { duration: Duration },
    /// Event-triggered
    Event { trigger: String },
}

/// Pipeline resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResources {
    /// CPU requirements
    pub cpu_cores: Option<f64>,

    /// Memory requirements in bytes
    pub memory_bytes: Option<u64>,

    /// Storage requirements in bytes
    pub storage_bytes: Option<u64>,

    /// Network bandwidth requirements
    pub network_bandwidth: Option<u64>,
}

/// Pipeline execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStatus {
    /// Pipeline ID
    pub pipeline_id: String,

    /// Current execution status
    pub status: PipelineExecutionStatus,

    /// Started timestamp
    pub started_at: Option<DateTime<Utc>>,

    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,

    /// Progress information
    pub progress: PipelineProgress,

    /// Error message if failed
    pub error: Option<String>,

    /// Step statuses
    pub steps: Vec<StepStatus>,
}

/// Pipeline execution status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineExecutionStatus {
    /// Waiting to start
    Pending,
    /// Currently running
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
    /// Paused
    Paused,
}

/// Pipeline progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineProgress {
    /// Total number of steps
    pub total_steps: u32,

    /// Number of completed steps
    pub completed_steps: u32,

    /// Number of failed steps
    pub failed_steps: u32,

    /// Overall progress percentage
    pub progress_percent: f64,
}

/// Individual step status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStatus {
    /// Step ID
    pub step_id: String,

    /// Step name
    pub name: String,

    /// Current status
    pub status: StepExecutionStatus,

    /// Started timestamp
    pub started_at: Option<DateTime<Utc>>,

    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,

    /// Error message if failed
    pub error: Option<String>,
}

/// Step execution status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepExecutionStatus {
    /// Waiting to start
    Pending,
    /// Currently running
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Skipped
    Skipped,
}
