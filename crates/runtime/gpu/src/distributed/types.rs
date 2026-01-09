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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_eq() {
        assert_eq!(JobStatus::Pending, JobStatus::Pending);
        assert_eq!(JobStatus::Running, JobStatus::Running);
        assert_eq!(JobStatus::Completed, JobStatus::Completed);
        assert_eq!(JobStatus::Failed, JobStatus::Failed);
        assert_ne!(JobStatus::Pending, JobStatus::Running);
    }

    #[test]
    fn test_distributed_stats_empty() {
        let stats = DistributedStats::empty();
        assert_eq!(stats.total_towers, 0);
        assert_eq!(stats.active_towers, 0);
        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.pending_jobs, 0);
        assert_eq!(stats.running_jobs, 0);
        assert_eq!(stats.completed_jobs, 0);
        assert_eq!(stats.failed_jobs, 0);
    }

    #[test]
    fn test_remote_tower_endpoint_creation() {
        let endpoint = RemoteTowerEndpoint {
            tower_id: "tower-1".to_string(),
            address: "http://tower1.local:8080".to_string(),
            gpu_capabilities: None,
            last_seen: Instant::now(),
            latency_ms: 10,
        };
        
        assert_eq!(endpoint.tower_id, "tower-1");
        assert_eq!(endpoint.address, "http://tower1.local:8080");
        assert_eq!(endpoint.latency_ms, 10);
        assert!(endpoint.gpu_capabilities.is_none());
    }

    #[test]
    fn test_distributed_job_state_creation() {
        use crate::universal::{UniversalWorkload, ComputeRequirements, UniversalKernel, OptimizationHints, KernelLanguage};
        
        let workload = UniversalWorkload {
            id: "workload-123".to_string(),
            requirements: ComputeRequirements::default(),
            kernel: UniversalKernel::Source {
                language: KernelLanguage::Wgsl,
                code: "// test kernel".to_string(),
                entry_point: "main".to_string(),
            },
            inputs: vec![],
            output_size: 1024,
            hints: OptimizationHints::default(),
        };
        
        let job = DistributedJobState {
            job_id: "job-123".to_string(),
            workload: workload.clone(),
            status: JobStatus::Pending,
            assigned_tower: None,
            result: None,
            created_at: Instant::now(),
            completed_at: None,
        };
        
        assert_eq!(job.job_id, "job-123");
        assert_eq!(job.status, JobStatus::Pending);
        assert!(job.assigned_tower.is_none());
        assert!(job.result.is_none());
        assert!(job.completed_at.is_none());
    }

    #[test]
    fn test_partition_strategy_single() {
        let strategy = PartitionStrategy::Single;
        match strategy {
            PartitionStrategy::Single => {}, // Expected
            _ => panic!("Expected Single strategy"),
        }
    }

    #[test]
    fn test_partition_strategy_data_parallel() {
        let strategy = PartitionStrategy::DataParallel { chunk_size: 1024 };
        match strategy {
            PartitionStrategy::DataParallel { chunk_size } => {
                assert_eq!(chunk_size, 1024);
            }
            _ => panic!("Expected DataParallel strategy"),
        }
    }

    #[test]
    fn test_partition_strategy_redundant() {
        let strategy = PartitionStrategy::Redundant { replicas: 3 };
        match strategy {
            PartitionStrategy::Redundant { replicas } => {
                assert_eq!(replicas, 3);
            }
            _ => panic!("Expected Redundant strategy"),
        }
    }

    #[test]
    fn test_partition_strategy_pipeline() {
        let stages = vec!["preprocess".to_string(), "compute".to_string(), "postprocess".to_string()];
        let strategy = PartitionStrategy::Pipeline { stages: stages.clone() };
        match strategy {
            PartitionStrategy::Pipeline { stages: s } => {
                assert_eq!(s.len(), 3);
                assert_eq!(s[0], "preprocess");
                assert_eq!(s[1], "compute");
                assert_eq!(s[2], "postprocess");
            }
            _ => panic!("Expected Pipeline strategy"),
        }
    }

    #[test]
    fn test_job_status_clone() {
        let status1 = JobStatus::Running;
        let status2 = status1.clone();
        assert_eq!(status1, status2);
    }

    #[test]
    fn test_distributed_stats_clone() {
        let stats1 = DistributedStats::empty();
        let stats2 = stats1.clone();
        assert_eq!(stats1.total_towers, stats2.total_towers);
        assert_eq!(stats1.total_jobs, stats2.total_jobs);
    }
}
