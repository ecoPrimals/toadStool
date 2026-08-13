// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU Compute Job Queue
//!
//! Accepts compute jobs, queues them, executes on available GPU, returns results.
//! Designed for JSON-RPC integration via `compute.submit`, `compute.status`, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use std::sync::RwLock;
use uuid::Uuid;

/// Job identifier
pub type JobId = Uuid;

/// Job state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Queued, waiting for GPU availability
    Pending,
    /// Currently executing on GPU
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
}

/// Type of compute job
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    /// Model inference (GGUF, safetensors, local inference backends)
    Inference {
        /// Model name.
        model: String,
        /// Prompt text.
        prompt: String,
        /// Optional inference parameters.
        #[serde(default)]
        params: serde_json::Value,
    },
    /// Data transformation (embedding, tokenization)
    Transform {
        /// Operation name.
        operation: String,
        /// Input data.
        input: serde_json::Value,
    },
    /// Arbitrary compute via plugin
    Custom {
        /// Plugin identifier.
        plugin: String,
        /// Plugin-specific payload.
        payload: serde_json::Value,
    },
}

/// A compute job submitted to the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeJob {
    /// Unique job identifier.
    pub id: JobId,
    /// Job type and parameters.
    pub job_type: JobType,
    /// Current job state.
    pub state: JobState,
    /// When the job was submitted.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub submitted_at: SystemTime,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "toadstool_common::system_time_serde::opt"
    )]
    /// When execution started (if running or completed).
    pub started_at: Option<SystemTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "toadstool_common::system_time_serde::opt"
    )]
    /// When execution completed (if completed).
    pub completed_at: Option<SystemTime>,
    /// Result payload (if completed successfully).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message (if failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Priority (lower = higher priority)
    pub priority: u32,
}

/// Job queue configuration
#[derive(Debug, Clone)]
pub struct JobQueueConfig {
    /// Maximum number of jobs in the queue
    pub max_queue_size: usize,
    /// Maximum concurrent running jobs
    pub max_concurrent: usize,
}

impl Default for JobQueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_concurrent: 4,
        }
    }
}

/// GPU compute job queue
///
/// Thread-safe, async-first job queue for GPU compute workloads.
/// Designed for integration with the JSON-RPC server.
#[derive(Clone)]
pub struct GpuJobQueue {
    jobs: Arc<RwLock<HashMap<JobId, ComputeJob>>>,
    config: JobQueueConfig,
}

impl GpuJobQueue {
    /// Create a new job queue with the given configuration
    #[must_use]
    pub fn new(config: JobQueueConfig) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Submit a new compute job to the queue
    ///
    /// Returns the job ID for tracking.
    ///
    /// # Errors
    /// Returns an error if the queue is full.
    pub async fn submit(&self, job_type: JobType, priority: u32) -> Result<JobId, JobQueueError> {
        let jobs = self.jobs.read().unwrap_or_else(|e| e.into_inner());
        let pending_count = jobs
            .values()
            .filter(|j| j.state == JobState::Pending)
            .count();
        if pending_count >= self.config.max_queue_size {
            return Err(JobQueueError::QueueFull {
                max: self.config.max_queue_size,
            });
        }
        drop(jobs);

        let id = Uuid::new_v4();
        let job = ComputeJob {
            id,
            job_type,
            state: JobState::Pending,
            submitted_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
            priority,
        };

        let mut jobs = self.jobs.write().unwrap_or_else(|e| e.into_inner());
        jobs.insert(id, job);

        tracing::info!(%id, "Compute job submitted");
        Ok(id)
    }

    /// Get the status of a job
    ///
    /// # Errors
    ///
    /// Returns [`JobQueueError`] if the job is not found.
    pub async fn status(&self, job_id: JobId) -> Result<ComputeJob, JobQueueError> {
        let jobs = self.jobs.read().unwrap_or_else(|e| e.into_inner());
        jobs.get(&job_id)
            .cloned()
            .ok_or(JobQueueError::JobNotFound { id: job_id })
    }

    /// Get the result of a completed job
    ///
    /// Returns the result value if the job is completed, or an error otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`JobQueueError`] if the job is not found, not complete, failed, or was cancelled.
    pub async fn result(&self, job_id: JobId) -> Result<serde_json::Value, JobQueueError> {
        let jobs = self.jobs.read().unwrap_or_else(|e| e.into_inner());
        let job = jobs
            .get(&job_id)
            .ok_or(JobQueueError::JobNotFound { id: job_id })?;

        match job.state {
            JobState::Completed => job
                .result
                .clone()
                .ok_or(JobQueueError::NoResult { id: job_id }),
            JobState::Failed => Err(JobQueueError::JobFailed {
                id: job_id,
                error: job.error.clone().unwrap_or_default(),
            }),
            JobState::Cancelled => Err(JobQueueError::JobCancelled { id: job_id }),
            _ => Err(JobQueueError::JobNotComplete { id: job_id }),
        }
    }

    /// Cancel a pending or running job
    ///
    /// # Errors
    ///
    /// Returns [`JobQueueError`] if the job is not found or cannot be cancelled (e.g., already completed).
    pub async fn cancel(&self, job_id: JobId) -> Result<(), JobQueueError> {
        let mut jobs = self.jobs.write().unwrap_or_else(|e| e.into_inner());
        let job = jobs
            .get_mut(&job_id)
            .ok_or(JobQueueError::JobNotFound { id: job_id })?;

        match job.state {
            JobState::Pending | JobState::Running => {
                job.state = JobState::Cancelled;
                job.completed_at = Some(SystemTime::now());
                tracing::info!(%job_id, "Compute job cancelled");
                Ok(())
            }
            _ => Err(JobQueueError::CannotCancel {
                id: job_id,
                state: job.state,
            }),
        }
    }

    /// List all jobs, optionally filtered by state
    pub async fn list(&self, state_filter: Option<JobState>) -> Vec<ComputeJob> {
        let jobs = self.jobs.read().unwrap_or_else(|e| e.into_inner());
        let mut result: Vec<ComputeJob> = if let Some(state) = state_filter {
            jobs.values()
                .filter(|j| j.state == state)
                .cloned()
                .collect()
        } else {
            jobs.values().cloned().collect()
        };
        // Sort by submission time (newest first)
        result.sort_by_key(|j| std::cmp::Reverse(j.submitted_at));
        result
    }

    /// Mark a job as running (called by the executor)
    ///
    /// # Errors
    ///
    /// Returns [`JobQueueError`] if the job is not found or not in Pending state.
    pub async fn mark_running(&self, job_id: JobId) -> Result<(), JobQueueError> {
        let mut jobs = self.jobs.write().unwrap_or_else(|e| e.into_inner());
        let job = jobs
            .get_mut(&job_id)
            .ok_or(JobQueueError::JobNotFound { id: job_id })?;

        if job.state != JobState::Pending {
            return Err(JobQueueError::InvalidTransition {
                id: job_id,
                from: job.state,
                to: JobState::Running,
            });
        }

        job.state = JobState::Running;
        job.started_at = Some(SystemTime::now());
        Ok(())
    }

    /// Mark a job as completed with a result (called by the executor)
    ///
    /// # Errors
    ///
    /// Returns [`JobQueueError`] if the job is not found.
    pub async fn mark_completed(
        &self,
        job_id: JobId,
        result: serde_json::Value,
    ) -> Result<(), JobQueueError> {
        let mut jobs = self.jobs.write().unwrap_or_else(|e| e.into_inner());
        let job = jobs
            .get_mut(&job_id)
            .ok_or(JobQueueError::JobNotFound { id: job_id })?;

        job.state = JobState::Completed;
        job.completed_at = Some(SystemTime::now());
        job.result = Some(result);
        tracing::info!(%job_id, "Compute job completed");
        Ok(())
    }

    /// Mark a job as failed with an error (called by the executor)
    ///
    /// # Errors
    ///
    /// Returns [`JobQueueError`] if the job is not found.
    pub async fn mark_failed(
        &self,
        job_id: JobId,
        error: impl Into<String>,
    ) -> Result<(), JobQueueError> {
        let mut jobs = self.jobs.write().unwrap_or_else(|e| e.into_inner());
        let job = jobs
            .get_mut(&job_id)
            .ok_or(JobQueueError::JobNotFound { id: job_id })?;

        job.state = JobState::Failed;
        job.completed_at = Some(SystemTime::now());
        job.error = Some(error.into());
        tracing::warn!(%job_id, "Compute job failed");
        Ok(())
    }

    /// Get the next pending job (highest priority, oldest first)
    pub async fn next_pending(&self) -> Option<ComputeJob> {
        let jobs = self.jobs.read().unwrap_or_else(|e| e.into_inner());
        jobs.values()
            .filter(|j| j.state == JobState::Pending)
            .min_by(|a, b| {
                a.priority
                    .cmp(&b.priority)
                    .then(a.submitted_at.cmp(&b.submitted_at))
            })
            .cloned()
    }

    /// Count jobs by state
    pub async fn counts(&self) -> HashMap<String, usize> {
        let jobs = self.jobs.read().unwrap_or_else(|e| e.into_inner());
        let mut counts = HashMap::new();
        for job in jobs.values() {
            let key = format!("{:?}", job.state).to_lowercase();
            *counts.entry(key).or_insert(0) += 1;
        }
        counts.insert("total".to_string(), jobs.len());
        counts
    }

    /// Clean up old completed/failed/cancelled jobs older than the given duration
    pub async fn cleanup(&self, max_age: std::time::Duration) {
        let cutoff = SystemTime::now() - max_age;
        let mut jobs = self.jobs.write().unwrap_or_else(|e| e.into_inner());
        jobs.retain(|_, job| {
            match job.state {
                JobState::Completed | JobState::Failed | JobState::Cancelled => {
                    job.completed_at.is_none_or(|t| t > cutoff)
                }
                _ => true, // Keep pending and running jobs
            }
        });
    }
}

#[cfg(test)]
#[path = "gpu_job_queue_tests.rs"]
mod tests;

/// Job queue errors
#[derive(Debug, thiserror::Error)]
pub enum JobQueueError {
    /// Queue is full.
    #[error("Queue is full (max: {max})")]
    QueueFull {
        /// Maximum queue size.
        max: usize,
    },

    /// Job not found.
    #[error("Job not found: {id}")]
    JobNotFound {
        /// Job ID.
        id: JobId,
    },

    /// Job is not yet complete.
    #[error("Job {id} is not complete (current state: pending/running)")]
    JobNotComplete {
        /// Job ID.
        id: JobId,
    },

    /// Job has no result.
    #[error("Job {id} has no result")]
    NoResult {
        /// Job ID.
        id: JobId,
    },

    /// Job failed with error.
    #[error("Job {id} failed: {error}")]
    JobFailed {
        /// Job ID.
        id: JobId,
        /// Error message.
        error: String,
    },

    /// Job was cancelled.
    #[error("Job {id} was cancelled")]
    JobCancelled {
        /// Job ID.
        id: JobId,
    },

    /// Cannot cancel job in current state.
    #[error("Cannot cancel job {id} in state {state:?}")]
    CannotCancel {
        /// Job ID.
        id: JobId,
        /// Current state.
        state: JobState,
    },

    /// Invalid state transition.
    #[error("Invalid state transition for job {id}: {from:?} -> {to:?}")]
    InvalidTransition {
        /// Job ID.
        id: JobId,
        /// Source state.
        from: JobState,
        /// Target state.
        to: JobState,
    },
}
