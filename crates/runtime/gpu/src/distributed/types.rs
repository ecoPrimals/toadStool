//! Types for distributed GPU scheduling
//!
//! Shared types used across the distributed scheduling system

use crate::universal::{ComputeCapabilities, UniversalWorkload, WorkloadResult};
use std::time::Instant;

/// Remote tower endpoint discovered via Songbird
#[derive(Debug, Clone)]
pub struct RemoteTowerEndpoint {
    pub tower_id: String,
    pub address: String,
    pub gpu_capabilities: Option<ComputeCapabilities>,
    pub last_seen: Instant,
    pub latency_ms: u64,
}

/// Distributed job state tracking
#[derive(Debug, Clone)]
pub struct DistributedJobState {
    pub job_id: String,
    pub workload: UniversalWorkload,
    pub status: JobStatus,
    pub assigned_tower: Option<String>,
    pub result: Option<WorkloadResult>,
    pub created_at: Instant,
    pub completed_at: Option<Instant>,
}

/// Job execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
}

/// Workload partitioning strategy for distributed execution
#[derive(Debug, Clone)]
pub enum PartitionStrategy {
    /// Execute on single best tower (no partitioning)
    Single,

    /// Split by data chunks for parallel processing
    DataParallel { chunk_size: usize },

    /// Replicate and race - use fastest response
    Redundant { replicas: usize },

    /// Pipeline stages across different towers
    Pipeline { stages: Vec<String> },
}

/// Distributed scheduler statistics
#[derive(Debug, Clone)]
pub struct DistributedStats {
    pub total_towers: usize,
    pub active_towers: usize,
    pub total_jobs: usize,
    pub pending_jobs: usize,
    pub running_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
}

impl DistributedStats {
    /// Create empty statistics
    pub fn empty() -> Self {
        Self {
            total_towers: 0,
            active_towers: 0,
            total_jobs: 0,
            pending_jobs: 0,
            running_jobs: 0,
            completed_jobs: 0,
            failed_jobs: 0,
        }
    }
}
