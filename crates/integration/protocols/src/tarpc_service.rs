//! # ToadStool Compute RPC Service (tarpc)
//!
//! High-performance, type-safe binary RPC protocol for primal-to-primal communication.
//! Following Songbird's proven pattern for ecosystem integration.
//!
//! ## Design Principles (from Songbird)
//!
//! - **Pure Rust**: No C++ dependencies (no gRPC/protobuf)
//! - **Type-Safe**: Full Rust type checking at compile time
//! - **Async Native**: Built on tokio
//! - **Low Latency**: Binary encoding, direct communication
//! - **High Throughput**: Optimized for high-volume operations
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │      ToadStool Compute Service          │
//! ├─────────────────────────────────────────┤
//! │                                          │
//! │  tarpc (PRIMARY)    JSON-RPC (PRIMARY)  │
//! │  Binary RPC         Universal RPC        │
//! │  ↓                  ↓                    │
//! │  Primal ←→ Primal   External Clients    │
//! │  • BearDog          • Python             │
//! │  • Songbird         • JavaScript         │
//! │  • NestGate         • Any language       │
//! │                                          │
//! │  HTTPS (FALLBACK - Optional)            │
//! │  ↓                                       │
//! │  Legacy/Debugging only                  │
//! └─────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Available resources across all compute units
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
}

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
    ///
    /// # Arguments
    /// * `submission` - Workload submission details
    ///
    /// # Returns
    /// * `WorkloadResult` - Initial submission result with workload ID
    async fn submit_workload(
        submission: WorkloadSubmission,
    ) -> Result<WorkloadResult, String>;

    /// Query workload execution status
    ///
    /// # Arguments
    /// * `workload_id` - Workload identifier
    ///
    /// # Returns
    /// * `WorkloadResult` - Current execution status and results
    async fn query_status(
        workload_id: String,
    ) -> Result<WorkloadResult, String>;

    /// Cancel running workload
    ///
    /// # Arguments
    /// * `workload_id` - Workload identifier
    ///
    /// # Returns
    /// * Success or error message
    async fn cancel_workload(
        workload_id: String,
    ) -> Result<(), String>;

    /// List all workloads for a given filter
    ///
    /// # Arguments
    /// * `filter` - Optional filter (status, type, etc.)
    ///
    /// # Returns
    /// * List of workload results
    async fn list_workloads(
        filter: Option<HashMap<String, String>>,
    ) -> Result<Vec<WorkloadResult>, String>;

    /// Query compute capabilities (self-knowledge pattern)
    ///
    /// This follows the "primal only knows itself" principle:
    /// - Returns only this primal's capabilities
    /// - No knowledge of other primals
    /// - Discovery happens at runtime
    ///
    /// # Returns
    /// * `ComputeCapabilities` - This primal's compute resources
    async fn query_capabilities() -> Result<ComputeCapabilities, String>;

    /// Health check endpoint
    ///
    /// # Returns
    /// * Service health status
    async fn health_check() -> Result<HealthStatus, String>;
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Service is healthy
    pub healthy: bool,
    /// Service version
    pub version: String,
    /// Uptime (seconds)
    pub uptime_secs: u64,
    /// Active workloads count
    pub active_workloads: u32,
    /// Resource utilization
    pub resource_utilization: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_submission_serialization() {
        let submission = WorkloadSubmission {
            workload_id: "work-123".to_string(),
            workload_type: "gpu_compute".to_string(),
            data: vec![1, 2, 3, 4],
            metadata: HashMap::new(),
            priority: WorkloadPriority::Normal,
            requirements: ResourceRequirements {
                cpu_cores: Some(4),
                memory_bytes: Some(1024 * 1024 * 1024),
                gpu_memory_bytes: Some(512 * 1024 * 1024),
                timeout_secs: Some(300),
            },
        };

        // Verify serialization works
        let json = serde_json::to_string(&submission).expect("Serialization failed");
        let deserialized: WorkloadSubmission = serde_json::from_str(&json)
            .expect("Deserialization failed");
        
        assert_eq!(submission.workload_id, deserialized.workload_id);
    }

    #[test]
    fn test_compute_capabilities_structure() {
        let capabilities = ComputeCapabilities {
            service_id: "toadstool-1".to_string(),
            compute_units: vec![
                ComputeUnit {
                    id: "cpu-0".to_string(),
                    unit_type: "cpu".to_string(),
                    name: "AMD Ryzen".to_string(),
                    cores: 128,
                    memory_bytes: 270 * 1024 * 1024 * 1024,
                    tflops: Some(12.8),
                    utilization: 0.25,
                },
            ],
            supported_workload_types: vec![
                "cpu_compute".to_string(),
                "gpu_compute".to_string(),
                "wasm".to_string(),
            ],
            available_resources: AvailableResources {
                total_cpu_cores: 128,
                available_cpu_cores: 96,
                total_memory_bytes: 270 * 1024 * 1024 * 1024,
                available_memory_bytes: 200 * 1024 * 1024 * 1024,
                total_gpu_memory_bytes: Some(40 * 1024 * 1024 * 1024),
                available_gpu_memory_bytes: Some(30 * 1024 * 1024 * 1024),
            },
            metadata: HashMap::new(),
        };

        // Verify structure is valid
        assert_eq!(capabilities.compute_units.len(), 1);
        assert_eq!(capabilities.compute_units[0].cores, 128);
    }
}

