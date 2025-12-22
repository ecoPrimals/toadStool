//! Distributed Job State Tracking
//!
//! Manages the lifecycle and state of distributed GPU jobs across multiple towers.
//! Provides fault recovery and result aggregation.

use super::types::{DistributedJobState, DistributedStats, JobStatus};
use crate::universal::WorkloadResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tracks distributed job state and lifecycle
pub struct JobTracker {
    /// Active and completed jobs
    jobs: Arc<RwLock<HashMap<String, DistributedJobState>>>,
}

impl JobTracker {
    /// Create new job tracker
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new job
    pub async fn register_job(&self, job: DistributedJobState) {
        let mut jobs = self.jobs.write().await;
        let job_id = job.job_id.clone();

        tracing::debug!("Registering job: {}", job_id);
        jobs.insert(job_id, job);
    }

    /// Update job status
    pub async fn update_status(&self, job_id: &str, status: JobStatus) {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            job.status = status.clone();

            if matches!(status, JobStatus::Completed | JobStatus::Failed) {
                job.completed_at = Some(std::time::Instant::now());
            }

            tracing::debug!("Job {} status updated to: {:?}", job_id, status);
        }
    }

    /// Assign job to tower
    pub async fn assign_to_tower(&self, job_id: &str, tower_id: String) {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            job.assigned_tower = Some(tower_id.clone());
            job.status = JobStatus::Scheduled;

            tracing::debug!("Job {} assigned to tower {}", job_id, tower_id);
        }
    }

    /// Mark job as running
    pub async fn mark_running(&self, job_id: &str) {
        self.update_status(job_id, JobStatus::Running).await;
    }

    /// Complete job with result
    pub async fn complete_job(&self, job_id: &str, result: WorkloadResult) {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Completed;
            job.result = Some(result);
            job.completed_at = Some(std::time::Instant::now());

            tracing::info!("Job {} completed successfully", job_id);
        }
    }

    /// Mark job as failed
    pub async fn fail_job(&self, job_id: &str) {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Failed;
            job.completed_at = Some(std::time::Instant::now());

            tracing::warn!("Job {} marked as failed", job_id);
        }
    }

    /// Get job state
    pub async fn get_job(&self, job_id: &str) -> Option<DistributedJobState> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// Get all jobs
    pub async fn all_jobs(&self) -> Vec<DistributedJobState> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Get jobs by status
    pub async fn jobs_by_status(&self, status: JobStatus) -> Vec<DistributedJobState> {
        let jobs = self.jobs.read().await;
        jobs.values()
            .filter(|job| job.status == status)
            .cloned()
            .collect()
    }

    /// Get statistics
    pub async fn statistics(&self) -> DistributedStats {
        let jobs = self.jobs.read().await;

        let total_jobs = jobs.len();
        let pending_jobs = jobs
            .values()
            .filter(|j| j.status == JobStatus::Pending)
            .count();
        let running_jobs = jobs
            .values()
            .filter(|j| j.status == JobStatus::Running)
            .count();
        let completed_jobs = jobs
            .values()
            .filter(|j| j.status == JobStatus::Completed)
            .count();
        let failed_jobs = jobs
            .values()
            .filter(|j| j.status == JobStatus::Failed)
            .count();

        DistributedStats {
            total_towers: 0,  // Filled in by coordinator
            active_towers: 0, // Filled in by coordinator
            total_jobs,
            pending_jobs,
            running_jobs,
            completed_jobs,
            failed_jobs,
        }
    }

    /// Prune completed jobs older than specified duration
    pub async fn prune_old_jobs(&self, max_age_secs: u64) {
        let mut jobs = self.jobs.write().await;
        let now = std::time::Instant::now();

        let before_count = jobs.len();
        jobs.retain(|_, job| {
            if let Some(completed_at) = job.completed_at {
                now.duration_since(completed_at).as_secs() < max_age_secs
            } else {
                true // Keep pending/running jobs
            }
        });
        let after_count = jobs.len();

        if before_count != after_count {
            tracing::info!("Pruned {} old completed jobs", before_count - after_count);
        }
    }

    /// Clear all job history
    #[cfg(test)]
    pub async fn clear(&self) {
        let mut jobs = self.jobs.write().await;
        jobs.clear();
    }
}

impl Default for JobTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_tracker_creation() {
        let tracker = JobTracker::new();
        let jobs = tracker.all_jobs().await;
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn test_statistics_empty() {
        let tracker = JobTracker::new();
        let stats = tracker.statistics().await;
        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.pending_jobs, 0);
    }
}
