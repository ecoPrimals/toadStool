// SPDX-License-Identifier: AGPL-3.0-or-later
//! Data pipeline structures and functionality for `Storage` integration

use std::collections::HashMap;
use std::time::Duration;

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
    FileSystem {
        /// Storage path for the input source.
        path: String,
    },
    /// HTTP/REST endpoint
    Http {
        /// Endpoint URL to fetch data from.
        url: String,
    },
    /// Database query
    Database {
        /// Database connection string.
        connection: String,
        /// SQL query to execute.
        query: String,
    },
    /// Stream processing
    Stream {
        /// Kafka/stream topic name.
        topic: String,
    },
    /// Artifact reference
    Artifact {
        /// Artifact identifier to retrieve.
        artifact_id: String,
    },
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
    FileSystem {
        /// Storage path for the output destination.
        path: String,
    },
    /// HTTP POST endpoint
    Http {
        /// Endpoint URL to POST results to.
        url: String,
    },
    /// Database insert
    Database {
        /// Database connection string.
        connection: String,
        /// Target table name.
        table: String,
    },
    /// Stream publishing
    Stream {
        /// Kafka/stream topic to publish to.
        topic: String,
    },
    /// Store as artifact
    Artifact {
        /// Artifact type for storage classification.
        artifact_type: ArtifactType,
    },
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
    Transform {
        /// Transformation script body.
        script: String,
        /// Script language (e.g. `python`, `sql`).
        language: String,
    },
    /// Data filtering
    Filter {
        /// Filter condition expression.
        condition: String,
    },
    /// Data aggregation
    Aggregate {
        /// Fields to aggregate.
        fields: Vec<String>,
        /// Aggregation operation (e.g. `sum`, `avg`).
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
    Custom {
        /// Processor identifier or path.
        processor: String,
    },
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
    Cron {
        /// Cron expression (e.g. `0 * * * *` for hourly).
        expression: String,
    },
    /// Interval-based schedule
    Interval {
        /// Duration between runs.
        duration: Duration,
    },
    /// Event-triggered
    Event {
        /// Event name or pattern that triggers execution.
        trigger: String,
    },
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
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub started_at: Option<std::time::SystemTime>,

    /// Completed timestamp
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub completed_at: Option<std::time::SystemTime>,

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
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub started_at: Option<std::time::SystemTime>,

    /// Completed timestamp
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub completed_at: Option<std::time::SystemTime>,

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
