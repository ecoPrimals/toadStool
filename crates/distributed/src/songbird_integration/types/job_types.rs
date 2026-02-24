//! Job request/response, complexity, subtask, and coordination types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ResourceRequirements;

// ============================================================================
// Job Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdJobRequest {
    pub job_id: Uuid,
    pub job_payload: Vec<u8>,
    pub target_nodes: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub priority: u8,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAnalysis {
    pub complexity: JobComplexity,
    pub distribution_strategy: JobDistributionStrategy,
    pub estimated_subtasks: usize,
    pub resource_requirements: ResourceRequirements,
    pub preferred_node_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdJobResponse {
    Success {
        job_id: Uuid,
        status: String,
        message: String,
        estimated_completion: Option<DateTime<Utc>>,
    },
    Error {
        job_id: Uuid,
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: Uuid,
    pub status: String,
    pub output: Vec<u8>,
    pub metrics: ExecutionMetrics,
}

// ============================================================================
// Job Complexity and Distribution
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobComplexity {
    Simple,
    Moderate,
    Complex,
    UltraMassive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntensityLevel {
    Low,
    Medium,
    High,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobDistributionStrategy {
    LocalOnly,
    SplitAndDistribute,
    ReplicateAcrossNodes,
    HybridExecution,
    SongbirdEcosystem,
    LoadBalanced,
    MassiveDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MassiveJobResult {
    Local {
        result: JobResult,
    },
    Distributed {
        original_job_id: Uuid,
        subtask_handles: Vec<SubTaskHandle>,
        coordination_job: CoordinationJob,
        distribution_plan: DistributionPlan,
    },
}

// ============================================================================
// Sub-task and Coordination Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: Uuid,
    pub payload: Vec<u8>,
    pub resource_requirements: ResourceRequirements,
    pub priority: u8,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskHandle {
    pub subtask_id: Uuid,
    pub songbird_job_id: Uuid,
    pub target_nodes: Vec<String>,
    pub submitted_at: DateTime<Utc>,
    pub status: SubTaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubTaskStatus {
    Submitted,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationJob {
    pub job_id: Uuid,
    pub original_job_id: Uuid,
    pub subtask_count: usize,
    pub completion_strategy: CompletionStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionStrategy {
    WaitForAll,
    WaitForMajority,
    WaitForAny,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionPlan {
    pub plan_id: Uuid,
    pub job_id: Uuid,
    pub subtasks: Vec<SubTaskPlan>,
    pub coordination_strategy: CoordinationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskPlan {
    pub subtask_id: Uuid,
    pub target_nodes: Vec<String>,
    pub resource_allocation: ResourceRequirements,
    pub dependencies: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    Sequential,
    Parallel,
    Pipeline,
    MapReduce,
}

// ============================================================================
// Execution Metrics (used by JobResult)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub network_io: u64,
    pub disk_io: u64,
}
