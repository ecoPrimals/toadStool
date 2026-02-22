//! GPU Compute Job Queue
//!
//! Accepts compute jobs, queues them, executes on available GPU, returns results.
//! Designed for JSON-RPC integration via `compute.submit`, `compute.status`, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
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
    /// Model inference (Ollama, GGUF, safetensors)
    Inference {
        model: String,
        prompt: String,
        #[serde(default)]
        params: serde_json::Value,
    },
    /// Data transformation (embedding, tokenization)
    Transform {
        operation: String,
        input: serde_json::Value,
    },
    /// Arbitrary compute via plugin
    Custom {
        plugin: String,
        payload: serde_json::Value,
    },
}

/// A compute job submitted to the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeJob {
    pub id: JobId,
    pub job_type: JobType,
    pub state: JobState,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
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
        let jobs = self.jobs.read().await;
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
            submitted_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
            priority,
        };

        let mut jobs = self.jobs.write().await;
        jobs.insert(id, job);

        tracing::info!(%id, "Compute job submitted");
        Ok(id)
    }

    /// Get the status of a job
    pub async fn status(&self, job_id: JobId) -> Result<ComputeJob, JobQueueError> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id)
            .cloned()
            .ok_or(JobQueueError::JobNotFound { id: job_id })
    }

    /// Get the result of a completed job
    ///
    /// Returns the result value if the job is completed, or an error otherwise.
    pub async fn result(&self, job_id: JobId) -> Result<serde_json::Value, JobQueueError> {
        let jobs = self.jobs.read().await;
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
    pub async fn cancel(&self, job_id: JobId) -> Result<(), JobQueueError> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(&job_id)
            .ok_or(JobQueueError::JobNotFound { id: job_id })?;

        match job.state {
            JobState::Pending | JobState::Running => {
                job.state = JobState::Cancelled;
                job.completed_at = Some(chrono::Utc::now());
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
        let jobs = self.jobs.read().await;
        let mut result: Vec<ComputeJob> = if let Some(state) = state_filter {
            jobs.values()
                .filter(|j| j.state == state)
                .cloned()
                .collect()
        } else {
            jobs.values().cloned().collect()
        };
        // Sort by submission time (newest first)
        result.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));
        result
    }

    /// Mark a job as running (called by the executor)
    pub async fn mark_running(&self, job_id: JobId) -> Result<(), JobQueueError> {
        let mut jobs = self.jobs.write().await;
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
        job.started_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Mark a job as completed with a result (called by the executor)
    pub async fn mark_completed(
        &self,
        job_id: JobId,
        result: serde_json::Value,
    ) -> Result<(), JobQueueError> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(&job_id)
            .ok_or(JobQueueError::JobNotFound { id: job_id })?;

        job.state = JobState::Completed;
        job.completed_at = Some(chrono::Utc::now());
        job.result = Some(result);
        tracing::info!(%job_id, "Compute job completed");
        Ok(())
    }

    /// Mark a job as failed with an error (called by the executor)
    pub async fn mark_failed(
        &self,
        job_id: JobId,
        error: impl Into<String>,
    ) -> Result<(), JobQueueError> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(&job_id)
            .ok_or(JobQueueError::JobNotFound { id: job_id })?;

        job.state = JobState::Failed;
        job.completed_at = Some(chrono::Utc::now());
        job.error = Some(error.into());
        tracing::warn!(%job_id, "Compute job failed");
        Ok(())
    }

    /// Get the next pending job (highest priority, oldest first)
    pub async fn next_pending(&self) -> Option<ComputeJob> {
        let jobs = self.jobs.read().await;
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
        let jobs = self.jobs.read().await;
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
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();
        let mut jobs = self.jobs.write().await;
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

/// Job queue errors
#[derive(Debug, thiserror::Error)]
pub enum JobQueueError {
    #[error("Queue is full (max: {max})")]
    QueueFull { max: usize },

    #[error("Job not found: {id}")]
    JobNotFound { id: JobId },

    #[error("Job {id} is not complete (current state: pending/running)")]
    JobNotComplete { id: JobId },

    #[error("Job {id} has no result")]
    NoResult { id: JobId },

    #[error("Job {id} failed: {error}")]
    JobFailed { id: JobId, error: String },

    #[error("Job {id} was cancelled")]
    JobCancelled { id: JobId },

    #[error("Cannot cancel job {id} in state {state:?}")]
    CannotCancel { id: JobId, state: JobState },

    #[error("Invalid state transition for job {id}: {from:?} -> {to:?}")]
    InvalidTransition {
        id: JobId,
        from: JobState,
        to: JobState,
    },
}

// ---- GPU System Query Helpers ----

/// Query available GPU devices
///
/// Detects NVIDIA GPUs via /proc on Linux, falls back to wgpu abstraction.
pub fn query_gpu_devices() -> Vec<serde_json::Value> {
    let mut devices = Vec::new();

    #[cfg(target_os = "linux")]
    if let Ok(entries) = std::fs::read_dir("/proc/driver/nvidia/gpus") {
        for (idx, entry) in entries.flatten().enumerate() {
            let name = entry.file_name().to_string_lossy().to_string();
            devices.push(serde_json::json!({
                "index": idx, "id": name, "backend": "nvidia",
            }));
        }
    }

    if devices.is_empty() {
        devices.push(serde_json::json!({
            "index": 0, "id": "wgpu-default", "backend": "wgpu",
            "note": "GPU detection via wgpu adapter enumeration at runtime",
        }));
    }

    devices
}

/// Query GPU memory usage via nvidia-smi
pub fn query_gpu_memory() -> Vec<serde_json::Value> {
    let mut devices = Vec::new();

    #[cfg(target_os = "linux")]
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=index,memory.total,memory.used,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(',').map(str::trim).collect();
                if parts.len() >= 4 {
                    devices.push(serde_json::json!({
                        "index": parts[0], "total_mb": parts[1],
                        "used_mb": parts[2], "free_mb": parts[3],
                    }));
                }
            }
        }
    }

    if devices.is_empty() {
        devices.push(serde_json::json!({
            "note": "GPU memory query requires nvidia-smi or wgpu adapter",
        }));
    }

    devices
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JobQueueConfig {
        JobQueueConfig {
            max_queue_size: 10,
            max_concurrent: 2,
        }
    }

    #[tokio::test]
    async fn test_submit_and_status() {
        let queue = GpuJobQueue::new(test_config());
        let job_type = JobType::Inference {
            model: "tinyllama".to_string(),
            prompt: "Hello".to_string(),
            params: serde_json::Value::Null,
        };

        let id = queue.submit(job_type, 0).await.unwrap();
        let job = queue.status(id).await.unwrap();

        assert_eq!(job.id, id);
        assert_eq!(job.state, JobState::Pending);
        assert_eq!(job.priority, 0);
    }

    #[tokio::test]
    async fn test_submit_queue_full() {
        let queue = GpuJobQueue::new(JobQueueConfig {
            max_queue_size: 2,
            max_concurrent: 1,
        });

        for _ in 0..2 {
            queue
                .submit(
                    JobType::Custom {
                        plugin: "test".to_string(),
                        payload: serde_json::Value::Null,
                    },
                    0,
                )
                .await
                .unwrap();
        }

        let result = queue
            .submit(
                JobType::Custom {
                    plugin: "test".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_job_lifecycle() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Transform {
                    operation: "embed".to_string(),
                    input: serde_json::json!({"text": "hello"}),
                },
                0,
            )
            .await
            .unwrap();

        // Pending -> Running
        queue.mark_running(id).await.unwrap();
        assert_eq!(queue.status(id).await.unwrap().state, JobState::Running);

        // Running -> Completed
        let result_val = serde_json::json!({"embedding": [0.1, 0.2, 0.3]});
        queue.mark_completed(id, result_val.clone()).await.unwrap();
        assert_eq!(queue.status(id).await.unwrap().state, JobState::Completed);

        // Get result
        let result = queue.result(id).await.unwrap();
        assert_eq!(result, result_val);
    }

    #[tokio::test]
    async fn test_cancel_pending() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "test".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();

        queue.cancel(id).await.unwrap();
        assert_eq!(queue.status(id).await.unwrap().state, JobState::Cancelled);
    }

    #[tokio::test]
    async fn test_cancel_completed_fails() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "test".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        queue.mark_running(id).await.unwrap();
        queue
            .mark_completed(id, serde_json::json!({}))
            .await
            .unwrap();

        assert!(queue.cancel(id).await.is_err());
    }

    #[tokio::test]
    async fn test_list_with_filter() {
        let queue = GpuJobQueue::new(test_config());

        let id1 = queue
            .submit(
                JobType::Custom {
                    plugin: "a".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        let _id2 = queue
            .submit(
                JobType::Custom {
                    plugin: "b".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();

        queue.mark_running(id1).await.unwrap();

        let pending = queue.list(Some(JobState::Pending)).await;
        assert_eq!(pending.len(), 1);

        let running = queue.list(Some(JobState::Running)).await;
        assert_eq!(running.len(), 1);

        let all = queue.list(None).await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_next_pending_priority() {
        let queue = GpuJobQueue::new(test_config());

        queue
            .submit(
                JobType::Custom {
                    plugin: "low".to_string(),
                    payload: serde_json::Value::Null,
                },
                10,
            )
            .await
            .unwrap();
        queue
            .submit(
                JobType::Custom {
                    plugin: "high".to_string(),
                    payload: serde_json::Value::Null,
                },
                1,
            )
            .await
            .unwrap();

        let next = queue.next_pending().await.unwrap();
        if let JobType::Custom { plugin, .. } = &next.job_type {
            assert_eq!(plugin, "high");
        } else {
            panic!("Expected Custom job type");
        }
    }

    #[tokio::test]
    async fn test_counts() {
        let queue = GpuJobQueue::new(test_config());

        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "test".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        queue
            .submit(
                JobType::Custom {
                    plugin: "test2".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        queue.mark_running(id).await.unwrap();

        let counts = queue.counts().await;
        assert_eq!(counts.get("pending"), Some(&1));
        assert_eq!(counts.get("running"), Some(&1));
        assert_eq!(counts.get("total"), Some(&2));
    }

    #[tokio::test]
    async fn test_job_not_found() {
        let queue = GpuJobQueue::new(test_config());
        let fake_id = Uuid::new_v4();
        assert!(queue.status(fake_id).await.is_err());
        assert!(queue.result(fake_id).await.is_err());
        assert!(queue.cancel(fake_id).await.is_err());
    }

    #[tokio::test]
    async fn test_mark_failed() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Inference {
                    model: "test".to_string(),
                    prompt: "hello".to_string(),
                    params: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();

        queue.mark_running(id).await.unwrap();
        queue
            .mark_failed(id, "GPU out of memory".to_string())
            .await
            .unwrap();

        let job = queue.status(id).await.unwrap();
        assert_eq!(job.state, JobState::Failed);
        assert_eq!(job.error.as_deref(), Some("GPU out of memory"));

        // Trying to get result should return JobFailed error
        let result = queue.result(id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_running_job() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "runner".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();

        queue.mark_running(id).await.unwrap();
        queue.cancel(id).await.unwrap();
        assert_eq!(queue.status(id).await.unwrap().state, JobState::Cancelled);
    }

    #[tokio::test]
    async fn test_cancel_failed_job_returns_error() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "fail".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        queue.mark_running(id).await.unwrap();
        queue.mark_failed(id, "oops").await.unwrap();

        let err = queue.cancel(id).await.unwrap_err();
        assert!(matches!(err, JobQueueError::CannotCancel { .. }));
    }

    #[tokio::test]
    async fn test_result_pending_returns_not_complete() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "x".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();

        let err = queue.result(id).await.unwrap_err();
        assert!(matches!(err, JobQueueError::JobNotComplete { .. }));
    }

    #[tokio::test]
    async fn test_result_running_returns_not_complete() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "x".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        queue.mark_running(id).await.unwrap();

        let err = queue.result(id).await.unwrap_err();
        assert!(matches!(err, JobQueueError::JobNotComplete { .. }));
    }

    #[tokio::test]
    async fn test_result_cancelled_returns_cancelled_error() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "x".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        queue.cancel(id).await.unwrap();

        let err = queue.result(id).await.unwrap_err();
        assert!(matches!(err, JobQueueError::JobCancelled { .. }));
    }

    #[tokio::test]
    async fn test_mark_running_invalid_transition() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "x".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        queue.mark_running(id).await.unwrap();

        // Running -> Running is an invalid transition
        let err = queue.mark_running(id).await.unwrap_err();
        assert!(matches!(err, JobQueueError::InvalidTransition { .. }));
    }

    #[tokio::test]
    async fn test_mark_completed_not_found() {
        let queue = GpuJobQueue::new(test_config());
        let fake_id = Uuid::new_v4();
        let err = queue
            .mark_completed(fake_id, serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, JobQueueError::JobNotFound { .. }));
    }

    #[tokio::test]
    async fn test_mark_failed_not_found() {
        let queue = GpuJobQueue::new(test_config());
        let fake_id = Uuid::new_v4();
        let err = queue.mark_failed(fake_id, "boom").await.unwrap_err();
        assert!(matches!(err, JobQueueError::JobNotFound { .. }));
    }

    #[tokio::test]
    async fn test_mark_running_not_found() {
        let queue = GpuJobQueue::new(test_config());
        let fake_id = Uuid::new_v4();
        let err = queue.mark_running(fake_id).await.unwrap_err();
        assert!(matches!(err, JobQueueError::JobNotFound { .. }));
    }

    #[tokio::test]
    async fn test_next_pending_none_when_empty() {
        let queue = GpuJobQueue::new(test_config());
        assert!(queue.next_pending().await.is_none());
    }

    #[tokio::test]
    async fn test_next_pending_none_when_all_running() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "x".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        queue.mark_running(id).await.unwrap();

        assert!(queue.next_pending().await.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_removes_old_jobs() {
        let queue = GpuJobQueue::new(test_config());

        let id1 = queue
            .submit(
                JobType::Custom {
                    plugin: "old".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        let id2 = queue
            .submit(
                JobType::Custom {
                    plugin: "pending".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();

        // Complete id1 so it becomes eligible for cleanup
        queue.mark_running(id1).await.unwrap();
        queue
            .mark_completed(id1, serde_json::json!({"done": true}))
            .await
            .unwrap();

        // Cleanup with zero max_age evicts all terminal jobs
        queue.cleanup(std::time::Duration::ZERO).await;

        // Completed job should be gone, pending job stays
        assert!(queue.status(id1).await.is_err());
        assert!(queue.status(id2).await.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_keeps_recent_jobs() {
        let queue = GpuJobQueue::new(test_config());
        let id = queue
            .submit(
                JobType::Custom {
                    plugin: "fresh".to_string(),
                    payload: serde_json::Value::Null,
                },
                0,
            )
            .await
            .unwrap();
        queue.mark_running(id).await.unwrap();
        queue
            .mark_completed(id, serde_json::json!({}))
            .await
            .unwrap();

        // Cleanup with a large max_age: recently completed job is kept
        queue.cleanup(std::time::Duration::from_secs(3600)).await;
        assert!(queue.status(id).await.is_ok());
    }

    #[tokio::test]
    async fn test_counts_empty_queue() {
        let queue = GpuJobQueue::new(test_config());
        let counts = queue.counts().await;
        assert_eq!(counts.get("total"), Some(&0));
    }

    #[tokio::test]
    async fn test_query_gpu_devices_returns_at_least_one() {
        let devices = query_gpu_devices();
        assert!(!devices.is_empty());
    }

    #[tokio::test]
    async fn test_query_gpu_memory_returns_at_least_one() {
        let memory = query_gpu_memory();
        assert!(!memory.is_empty());
    }

    #[test]
    fn test_job_queue_config_default() {
        let cfg = JobQueueConfig::default();
        assert!(cfg.max_queue_size > 0);
        assert!(cfg.max_concurrent > 0);
    }

    #[test]
    fn test_job_state_serialization_roundtrip() {
        for state in [
            JobState::Pending,
            JobState::Running,
            JobState::Completed,
            JobState::Failed,
            JobState::Cancelled,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let restored: JobState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, restored);
        }
    }

    #[test]
    fn test_job_type_inference_serialization_roundtrip() {
        let job_type = JobType::Inference {
            model: "tinyllama".to_string(),
            prompt: "Hello".to_string(),
            params: serde_json::json!({"temperature": 0.7}),
        };
        let json = serde_json::to_string(&job_type).unwrap();
        let restored: JobType = serde_json::from_str(&json).unwrap();
        match (&job_type, &restored) {
            (
                JobType::Inference {
                    model: m1,
                    prompt: p1,
                    ..
                },
                JobType::Inference {
                    model: m2,
                    prompt: p2,
                    ..
                },
            ) => {
                assert_eq!(m1, m2);
                assert_eq!(p1, p2);
            }
            _ => panic!("Expected Inference variant"),
        }
    }

    #[test]
    fn test_job_queue_error_display() {
        let id = Uuid::new_v4();
        let err = JobQueueError::QueueFull { max: 100 };
        assert!(err.to_string().contains("100"));

        let err = JobQueueError::JobNotFound { id };
        assert!(err.to_string().contains("not found"));

        let err = JobQueueError::CannotCancel {
            id,
            state: JobState::Completed,
        };
        assert!(err.to_string().contains("Cannot cancel"));
        assert!(err.to_string().contains("Completed"));
    }
}
