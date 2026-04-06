// SPDX-License-Identifier: AGPL-3.0-or-later
//! Client type definitions
//!
//! This module defines all the data types used in workload submission,
//! execution tracking, and event handling.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

// Re-export canonical JobPriority from toadstool core
pub use toadstool::JobPriority;

/// Type alias for event handlers to reduce complexity
pub type EventHandlers = Vec<Box<dyn Fn(ToadStoolEvent) + Send + Sync>>;

/// Workload submission specification.
#[derive(Debug, Clone)]
pub struct WorkloadSubmission {
    /// Type of workload to execute.
    pub workload_type: WorkloadType,
    /// Optional runtime hint (e.g. wasm, native).
    pub runtime_hint: Option<String>,
    /// Job priority.
    pub priority: Option<JobPriority>,
    /// Execution timeout.
    pub timeout: Option<Duration>,
    /// Environment variables.
    pub environment: HashMap<String, String>,
    /// Resource requirements.
    pub resources: Option<ResourceRequirements>,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

/// Type of workload to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadType {
    /// Native executable workload.
    Native {
        /// Path to executable.
        executable: String,
        /// Command-line arguments.
        args: Vec<String>,
        /// Working directory.
        working_dir: Option<String>,
    },

    /// Container image workload.
    Container {
        /// Container image reference.
        image: String,
        /// Override command.
        command: Option<Vec<String>>,
        /// Override arguments.
        args: Option<Vec<String>>,
        /// Working directory.
        working_dir: Option<String>,
    },

    /// WebAssembly module workload.
    Wasm {
        /// WASM module bytes.
        module_data: Vec<u8>,
        /// Arguments passed to the module.
        args: Vec<String>,
    },

    /// Python script workload.
    Python {
        /// Script content.
        script: String,
        /// pip requirements.
        requirements: Vec<String>,
    },

    /// Custom workload with arbitrary data.
    Custom {
        /// Workload payload (JSON).
        workload_data: serde_json::Value,
    },
}

// JobPriority is now imported from toadstool core (canonical definition in universal.rs)

/// Resource requirements for workload execution (simplified client version).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU cores required.
    pub cpu_cores: Option<u32>,
    /// Memory in MB.
    pub memory_mb: Option<u64>,
    /// Disk in MB.
    pub disk_mb: Option<u64>,
    /// Whether GPU is required.
    pub gpu_required: Option<bool>,
}

// Conversion to/from core ResourceRequirements
impl From<ResourceRequirements> for toadstool::resources::ResourceRequirements {
    fn from(client: ResourceRequirements) -> Self {
        Self {
            cpu: toadstool::resources::CpuRequirements {
                min_cores: f64::from(client.cpu_cores.unwrap_or(1)),
                max_cores: None,
                architecture: None,
            },
            memory: toadstool::resources::MemoryRequirements {
                min_bytes: client.memory_mb.unwrap_or(1024) * 1024 * 1024, // MB to bytes
                max_bytes: None,
            },
            storage: toadstool::resources::StorageRequirements {
                min_bytes: client.disk_mb.unwrap_or(1024) * 1024 * 1024, // MB to bytes
                max_bytes: None,
                storage_type: None,
            },
            gpu: if client.gpu_required.unwrap_or(false) {
                Some(toadstool::resources::GpuRequirements {
                    min_units: 1,
                    max_units: None,
                    gpu_type: None,
                    min_memory_bytes: None,
                })
            } else {
                None
            },
            network: toadstool::resources::NetworkRequirements::default(),
        }
    }
}

impl From<toadstool::resources::ResourceRequirements> for ResourceRequirements {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "min_cores from f64 to u32 for display; truncation acceptable"
    )]
    fn from(core: toadstool::resources::ResourceRequirements) -> Self {
        Self {
            cpu_cores: Some(core.cpu.min_cores as u32),
            memory_mb: Some(core.memory.min_bytes / (1024 * 1024)),
            disk_mb: Some(core.storage.min_bytes / (1024 * 1024)),
            gpu_required: Some(core.gpu.is_some()),
        }
    }
}

/// Execution status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInfo {
    /// Unique execution ID.
    pub execution_id: Uuid,
    /// Current status.
    pub status: ExecutionStatus,
    /// When the job was submitted.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub submitted_at: SystemTime,
    /// When execution started (if running/completed).
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub started_at: Option<SystemTime>,
    /// When execution completed (if completed).
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub completed_at: Option<SystemTime>,
    /// Runtime type used.
    pub runtime_type: Option<String>,
    /// Error message (if failed).
    pub error_message: Option<String>,
    /// Execution output (if completed).
    pub output: Option<ExecutionOutput>,
    /// Execution metrics.
    pub metrics: Option<ExecutionMetrics>,
}

/// Execution status enumeration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Awaiting scheduling.
    Pending,
    /// Queued for execution.
    Queued,
    /// Currently running.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed with error.
    Failed,
    /// Cancelled by user.
    Cancelled,
    /// Timed out.
    Timeout,
}

/// Execution output (stdout, stderr, exit code).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    /// Standard output.
    pub stdout: Option<String>,
    /// Standard error.
    pub stderr: Option<String>,
    /// Process exit code.
    pub exit_code: Option<i32>,
    /// Output artifact paths.
    pub artifacts: Vec<String>,
}

/// Execution resource metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Total duration in milliseconds.
    pub duration_ms: u64,
    /// CPU usage percentage.
    pub cpu_usage_percent: f64,
    /// Peak memory in bytes.
    pub memory_peak_bytes: u64,
    /// Network bytes sent.
    pub network_bytes_sent: u64,
    /// Network bytes received.
    pub network_bytes_received: u64,
}

/// Real-time execution events.
///
/// These events are NOT delivered via `WebSocket` (deprecated, used C-FFI ring).
/// Use JSON-RPC 2.0 polling (`compute.status` method) or coordination service
/// for event streaming over Unix sockets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToadStoolEvent {
    /// Execution status changed — poll via `compute.status` JSON-RPC call.
    ExecutionStatusChanged {
        /// Execution ID.
        execution_id: String,
        /// New status string.
        status: String,
    },
    /// Cluster health changed — poll via `toadstool.health` JSON-RPC call.
    ClusterHealthChanged {
        /// Whether cluster is healthy.
        healthy: bool,
    },
}

/// ToadStool cluster status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    /// Total nodes in cluster.
    pub total_nodes: u32,
    /// Healthy nodes.
    pub healthy_nodes: u32,
    /// Cluster load (0–1).
    pub cluster_load: f64,
    /// Active executions.
    pub active_executions: u32,
    /// Available runtime types.
    pub available_runtimes: Vec<String>,
}
