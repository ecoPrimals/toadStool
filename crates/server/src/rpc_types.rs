//! Pure RPC Types for ToadStool Server
//!
//! Deep debt solution: Extract pure RPC types without HTTP dependencies.
//! Migrated from toadstool_integration_protocols to enable UniBin.
//!
//! ## Evolution Path
//!
//! 1. **Before**: Types in protocols crate with HTTP remnants
//! 2. **Problem**: Cannot import protocols crate (has reqwest)
//! 3. **Solution**: Extract pure RPC types here
//! 4. **Future**: Evolve protocols crate to pure Rust, remove duplication

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// tarpc service definition for ToadStool compute operations
///
/// This trait defines the RPC interface following Songbird's pattern:
/// - Binary protocol for performance
/// - Type-safe at compile time
/// - Async throughout
/// - Self-describing (capabilities query)
#[tarpc::service]
pub trait ToadStoolComputeRpc {
    /// Submit workload for execution
    async fn submit_workload(submission: WorkloadSubmission) -> Result<WorkloadResult, String>;

    /// Query workload execution status
    async fn query_status(workload_id: String) -> Result<WorkloadResult, String>;

    /// Cancel running workload
    async fn cancel_workload(workload_id: String) -> Result<(), String>;

    /// List all workloads for a given filter
    async fn list_workloads(
        filter: Option<HashMap<String, String>>,
    ) -> Result<Vec<WorkloadResult>, String>;

    /// Query compute capabilities (self-knowledge pattern)
    async fn query_capabilities() -> Result<ComputeCapabilities, String>;

    /// Health status check
    async fn health_status() -> Result<HealthStatus, String>;
}

/// Workload submission for compute execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSubmission {
    /// Unique workload identifier
    pub workload_id: String,
    /// Workload type (gpu_compute, cpu_compute, wasm, etc.)
    pub workload_type: String,
    /// Binary workload data
    pub data: Vec<u8>,
    /// Workload metadata
    pub metadata: HashMap<String, String>,
    /// Priority level
    pub priority: WorkloadPriority,
    /// Resource requirements
    pub requirements: ResourceRequirements,
}

/// Workload priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WorkloadPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Resource requirements for workload execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU cores required
    pub cpu_cores: Option<u32>,
    /// Memory required (bytes)
    pub memory_bytes: Option<u64>,
    /// GPU memory required (bytes)
    pub gpu_memory_bytes: Option<u64>,
    /// Execution timeout (seconds)
    pub timeout_secs: Option<u64>,
}

/// Workload execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadResult {
    /// Workload identifier
    pub workload_id: String,
    /// Execution status
    pub status: WorkloadStatus,
    /// Result data (if successful)
    pub data: Option<Vec<u8>>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution metrics
    pub metrics: ExecutionMetrics,
}

/// Workload execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadStatus {
    Pending,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Execution metrics for performance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Time queued (seconds)
    pub queued_duration_secs: f64,
    /// Time executing (seconds)
    pub execution_duration_secs: f64,
    /// CPU cores used
    pub cpu_cores_used: u32,
    /// Memory used (bytes)
    pub memory_used_bytes: u64,
    /// GPU memory used (bytes)
    pub gpu_memory_used_bytes: Option<u64>,
}

/// Compute capabilities query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeCapabilities {
    /// Service identifier
    pub service_id: String,
    /// Available compute units
    pub compute_units: Vec<ComputeUnit>,
    /// Supported workload types
    pub supported_workload_types: Vec<String>,
    /// Total available resources
    pub available_resources: AvailableResources,
    /// Service metadata
    pub metadata: HashMap<String, String>,
}

/// Individual compute unit (CPU, GPU, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeUnit {
    /// Unit identifier
    pub id: String,
    /// Unit type (cpu, gpu, neuromorphic, etc.)
    pub unit_type: String,
    /// Unit name
    pub name: String,
    /// Available cores/processors
    pub cores: u32,
    /// Available memory (bytes)
    pub memory_bytes: u64,
    /// Compute throughput (TFLOPS)
    pub tflops: Option<f64>,
    /// Current utilization (0.0-1.0)
    pub utilization: f32,
}

/// Available resources summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableResources {
    /// Total CPU cores
    pub total_cpu_cores: u32,
    /// Available CPU cores
    pub available_cpu_cores: u32,
    /// Total memory (bytes)
    pub total_memory_bytes: u64,
    /// Available memory (bytes)
    pub available_memory_bytes: u64,
    /// Total GPU memory (bytes)
    pub total_gpu_memory_bytes: Option<u64>,
    /// Available GPU memory (bytes)
    pub available_gpu_memory_bytes: Option<u64>,
    /// Current CPU utilization (0.0-1.0)
    pub cpu_utilization: f32,
    /// Current memory utilization (0.0-1.0)
    pub memory_utilization: f32,
    /// Current GPU utilization (0.0-1.0)
    pub gpu_utilization: Option<f32>,
}

/// Health status for service monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Service is healthy
    pub healthy: bool,
    /// Service version
    pub version: String,
    /// Uptime (seconds)
    pub uptime_secs: u64,
    /// Current resource utilization (0.0-1.0)
    pub resource_utilization: f32,
    /// Active workloads count
    pub active_workloads: usize,
    /// Queued workloads count
    pub queued_workloads: usize,
    /// Error count since startup
    pub error_count: usize,
}

/// Tarpc-specific workload submission (with serialized data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcWorkloadSubmission {
    /// Workload identifier
    pub workload_id: String,
    /// Runtime type
    pub runtime_type: String,
    /// Serialized workload data
    pub payload: Vec<u8>,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Metadata
    pub metadata: HashMap<String, String>,
}
