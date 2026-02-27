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

// JobPriority is now imported from toadstool core (canonical definition in universal.rs)

/// Resource requirements for workload execution (simplified client version)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: Option<u32>,
    pub memory_mb: Option<u64>,
    pub disk_mb: Option<u64>,
    pub gpu_required: Option<bool>,
}

// Conversion to/from core ResourceRequirements
impl From<ResourceRequirements> for toadstool::resources::ResourceRequirements {
    fn from(client: ResourceRequirements) -> Self {
        toadstool::resources::ResourceRequirements {
            cpu: toadstool::resources::CpuRequirements {
                min_cores: client.cpu_cores.unwrap_or(1) as f64,
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
    fn from(core: toadstool::resources::ResourceRequirements) -> Self {
        ResourceRequirements {
            cpu_cores: Some(core.cpu.min_cores as u32),
            memory_mb: Some(core.memory.min_bytes / (1024 * 1024)),
            disk_mb: Some(core.storage.min_bytes / (1024 * 1024)),
            gpu_required: Some(core.gpu.is_some()),
        }
    }
}

/// Execution status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInfo {
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub submitted_at: SystemTime,
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub started_at: Option<SystemTime>,
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub completed_at: Option<SystemTime>,
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

/// Real-time execution events.
///
/// These events are NOT delivered via WebSocket (deprecated, used C-FFI ring).
/// Use JSON-RPC 2.0 polling (`compute.status` method) or biomeOS/songbird
/// coordination for event streaming over Unix sockets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToadStoolEvent {
    /// Execution status changed — poll via `compute.status` JSON-RPC call
    ExecutionStatusChanged {
        execution_id: String,
        status: String,
    },
    /// Cluster health changed — poll via `toadstool.health` JSON-RPC call
    ClusterHealthChanged { healthy: bool },
}

/// ToadStool cluster status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    pub total_nodes: u32,
    pub healthy_nodes: u32,
    pub cluster_load: f64,
    pub active_executions: u32,
    pub available_runtimes: Vec<String>,
}
