// SPDX-License-Identifier: AGPL-3.0-only
//! Job request/response, complexity, subtask, and coordination types

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use crate::ResourceRequirements;

// ============================================================================
// Job Request/Response Types
// ============================================================================

/// Job payload: Bytes = Arc<[u8]> — clone is refcount bump (wateringHole zero-copy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdJobRequest {
    /// Job identifier.
    pub job_id: Uuid,
    /// Job payload bytes (zero-copy clone).
    pub job_payload: Bytes,
    /// Target node IDs for distribution.
    pub target_nodes: Vec<String>,
    /// Resource requirements for placement.
    pub resource_requirements: ResourceRequirements,
    /// Priority (higher = more urgent).
    pub priority: u8,
    /// Placement constraints.
    pub constraints: Vec<String>,
}

/// Analysis of job complexity and distribution strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAnalysis {
    /// Job complexity level.
    pub complexity: JobComplexity,
    /// Recommended distribution strategy.
    pub distribution_strategy: JobDistributionStrategy,
    /// Estimated number of subtasks.
    pub estimated_subtasks: usize,
    /// Resource requirements.
    pub resource_requirements: ResourceRequirements,
    /// Preferred node types for placement.
    pub preferred_node_types: Vec<String>,
}

/// Songbird job submission response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdJobResponse {
    /// Job accepted and queued.
    Success {
        /// Job identifier.
        job_id: Uuid,
        /// Status string.
        status: String,
        /// Human-readable message.
        message: String,
        /// Estimated completion time (optional).
        #[serde(with = "toadstool_common::system_time_serde::opt")]
        estimated_completion: Option<SystemTime>,
    },
    /// Job rejected or failed.
    Error {
        /// Job identifier.
        job_id: Uuid,
        /// Error message.
        error: String,
    },
}

/// Completed job result with output and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Job identifier.
    pub job_id: Uuid,
    /// Final status string.
    pub status: String,
    /// Output bytes.
    pub output: Vec<u8>,
    /// Execution metrics.
    pub metrics: ExecutionMetrics,
}

// ============================================================================
// Job Complexity and Distribution
// ============================================================================

/// Job complexity for scheduling and distribution decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobComplexity {
    /// Single-node, short-running job.
    Simple,
    /// Moderate resource usage, may benefit from distribution.
    Moderate,
    /// High resource or coordination needs.
    Complex,
    /// Ultra-large job requiring massive distribution.
    UltraMassive,
}

/// Complexity level for workload classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    /// Low complexity.
    Low,
    /// Medium complexity.
    Medium,
    /// High complexity.
    High,
    /// Extreme complexity.
    Extreme,
}

/// Intensity level for resource scaling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntensityLevel {
    /// Low intensity.
    Low,
    /// Medium intensity.
    Medium,
    /// High intensity.
    High,
    /// Extreme intensity.
    Extreme,
}

/// Strategy for distributing jobs across nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobDistributionStrategy {
    /// Execute only on local node.
    LocalOnly,
    /// Split job into subtasks and distribute.
    SplitAndDistribute,
    /// Replicate job across multiple nodes (e.g. for redundancy).
    ReplicateAcrossNodes,
    /// Hybrid local + remote execution.
    HybridExecution,
    /// Use Songbird ecosystem for discovery and placement.
    SongbirdEcosystem,
    /// Load-balanced distribution across available nodes.
    LoadBalanced,
    /// Massive distribution for very large jobs.
    MassiveDistribution,
}

/// Result of a massive job (local or distributed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MassiveJobResult {
    /// Job executed locally.
    Local {
        /// Local job result.
        result: JobResult,
    },
    /// Job was distributed across nodes.
    Distributed {
        /// Original job ID.
        original_job_id: Uuid,
        /// Handles for submitted subtasks.
        subtask_handles: Vec<SubTaskHandle>,
        /// Coordination job for aggregation.
        coordination_job: CoordinationJob,
        /// Distribution plan used.
        distribution_plan: DistributionPlan,
    },
}

// ============================================================================
// Sub-task and Coordination Types
// ============================================================================

/// SubTask payload: Bytes = Arc<[u8]> — clone is refcount bump (wateringHole zero-copy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// Subtask identifier.
    pub id: Uuid,
    /// Subtask payload bytes.
    pub payload: Bytes,
    /// Resource requirements.
    pub resource_requirements: ResourceRequirements,
    /// Priority.
    pub priority: u8,
    /// Placement constraints.
    pub constraints: Vec<String>,
}

/// Handle for a submitted subtask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskHandle {
    /// Subtask identifier.
    pub subtask_id: Uuid,
    /// Parent Songbird job ID.
    pub songbird_job_id: Uuid,
    /// Target node IDs.
    pub target_nodes: Vec<String>,
    /// Submission timestamp.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub submitted_at: SystemTime,
    /// Current subtask status.
    pub status: SubTaskStatus,
}

/// Status of a distributed subtask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubTaskStatus {
    /// Submitted, not yet running.
    Submitted,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed.
    Failed,
}

/// Coordination job for aggregating subtask results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationJob {
    /// Coordination job ID.
    pub job_id: Uuid,
    /// Original job ID.
    pub original_job_id: Uuid,
    /// Number of subtasks.
    pub subtask_count: usize,
    /// Strategy for considering job complete.
    pub completion_strategy: CompletionStrategy,
}

/// Strategy for when a distributed job is considered complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionStrategy {
    /// Wait for all subtasks to complete.
    WaitForAll,
    /// Wait for majority of subtasks.
    WaitForMajority,
    /// Wait for any single subtask.
    WaitForAny,
    /// Custom strategy.
    Custom(String),
}

/// Plan for distributing a job across nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionPlan {
    /// Plan identifier.
    pub plan_id: Uuid,
    /// Job identifier.
    pub job_id: Uuid,
    /// Per-subtask plans.
    pub subtasks: Vec<SubTaskPlan>,
    /// Coordination strategy.
    pub coordination_strategy: CoordinationStrategy,
}

/// Plan for a single subtask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskPlan {
    /// Subtask identifier.
    pub subtask_id: Uuid,
    /// Target node IDs.
    pub target_nodes: Vec<String>,
    /// Resource allocation.
    pub resource_allocation: ResourceRequirements,
    /// Subtask dependencies (other subtask IDs).
    pub dependencies: Vec<Uuid>,
}

/// Strategy for coordinating subtask execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    /// Execute subtasks sequentially.
    Sequential,
    /// Execute subtasks in parallel.
    Parallel,
    /// Pipeline execution (stage-by-stage).
    Pipeline,
    /// MapReduce-style execution.
    MapReduce,
}

// ============================================================================
// Execution Metrics (used by JobResult)
// ============================================================================

/// Execution metrics for job completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Execution start time.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub start_time: SystemTime,
    /// Execution end time.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub end_time: SystemTime,
    /// CPU usage fraction (0.0–1.0).
    pub cpu_usage: f64,
    /// Memory usage in bytes.
    pub memory_usage: u64,
    /// Network I/O in bytes.
    pub network_io: u64,
    /// Disk I/O in bytes.
    pub disk_io: u64,
}
