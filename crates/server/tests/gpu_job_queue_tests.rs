// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Integration-style tests for GPU job queue

use toadstool_server::gpu_job_queue::*;
use uuid::Uuid;

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
    assert!(
        matches!(&next.job_type, JobType::Custom { plugin, .. } if plugin == "high"),
        "Expected Custom job type with plugin 'high'"
    );
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
    assert!(matches!(
        (&job_type, &restored),
        (
            JobType::Inference { model: m1, prompt: p1, .. },
            JobType::Inference { model: m2, prompt: p2, .. }
        ) if m1 == m2 && p1 == p2
    ));
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

#[test]
fn test_job_queue_error_display_all_variants() {
    let id = Uuid::new_v4();

    let err = JobQueueError::QueueFull { max: 42 };
    assert!(err.to_string().contains("42"));
    assert!(err.to_string().contains("full"));

    let err = JobQueueError::JobNotComplete { id };
    assert!(err.to_string().contains("not complete"));

    let err = JobQueueError::NoResult { id };
    assert!(err.to_string().contains("no result"));

    let err = JobQueueError::JobFailed {
        id,
        error: "OOM".to_string(),
    };
    assert!(err.to_string().contains("OOM"));
    assert!(err.to_string().contains("failed"));

    let err = JobQueueError::JobCancelled { id };
    assert!(err.to_string().contains("cancelled"));

    let err = JobQueueError::InvalidTransition {
        id,
        from: JobState::Running,
        to: JobState::Pending,
    };
    let msg = err.to_string();
    assert!(msg.to_lowercase().contains("invalid"));
    assert!(msg.to_lowercase().contains("transition"));
}

#[tokio::test]
async fn test_queue_full_only_counts_pending() {
    let queue = GpuJobQueue::new(JobQueueConfig {
        max_queue_size: 2,
        max_concurrent: 1,
    });

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
    let id2 = queue
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
    queue.mark_running(id2).await.unwrap();

    let err = queue
        .submit(
            JobType::Custom {
                plugin: "c".to_string(),
                payload: serde_json::Value::Null,
            },
            0,
        )
        .await;
    assert!(
        err.is_ok(),
        "Queue full should only count Pending, not Running"
    );
}

#[test]
fn test_job_type_transform_serialization_roundtrip() {
    let job_type = JobType::Transform {
        operation: "embed".to_string(),
        input: serde_json::json!({"text": "hello world"}),
    };
    let json = serde_json::to_string(&job_type).unwrap();
    let restored: JobType = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        (&job_type, &restored),
        (
            JobType::Transform { operation: o1, input: i1 },
            JobType::Transform { operation: o2, input: i2 }
        ) if o1 == o2 && i1 == i2
    ));
}

#[test]
fn test_job_type_custom_serialization_roundtrip() {
    let job_type = JobType::Custom {
        plugin: "my_plugin".to_string(),
        payload: serde_json::json!({"key": "value"}),
    };
    let json = serde_json::to_string(&job_type).unwrap();
    let restored: JobType = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        (&job_type, &restored),
        (
            JobType::Custom { plugin: p1, payload: pl1 },
            JobType::Custom { plugin: p2, payload: pl2 }
        ) if p1 == p2 && pl1 == pl2
    ));
}
