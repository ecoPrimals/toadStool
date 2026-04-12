// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

fn make_inference_job(model: &str, prompt: &str) -> JobType {
    JobType::Inference {
        model: model.to_string(),
        prompt: prompt.to_string(),
        params: serde_json::json!({}),
    }
}

#[tokio::test]
async fn test_queue_creation() {
    let config = JobQueueConfig {
        max_queue_size: 100,
        max_concurrent: 4,
    };
    let queue = GpuJobQueue::new(config);
    let counts = queue.counts().await;
    assert_eq!(counts.get("total"), Some(&0));
}

#[tokio::test]
async fn test_queue_creation_default_config() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let total = queue.counts().await.get("total").copied().unwrap_or(0);
    assert_eq!(total, 0);
}

#[tokio::test]
async fn test_job_submission_and_retrieval() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let id = queue
        .submit(make_inference_job("llama", "Hello"), 0)
        .await
        .unwrap();
    let job = queue.status(id).await.unwrap();
    assert_eq!(job.id, id);
    assert_eq!(job.state, JobState::Pending);
    assert_eq!(job.priority, 0);
}

#[tokio::test]
async fn test_job_submission_returns_unique_ids() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let id1 = queue
        .submit(make_inference_job("m1", "p1"), 0)
        .await
        .unwrap();
    let id2 = queue
        .submit(make_inference_job("m2", "p2"), 0)
        .await
        .unwrap();
    assert_ne!(id1, id2);
}

#[tokio::test]
async fn test_priority_ordering_next_pending() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let _id_low = queue
        .submit(make_inference_job("m1", "p1"), 10)
        .await
        .unwrap();
    let _id_high = queue
        .submit(make_inference_job("m2", "p2"), 0)
        .await
        .unwrap();
    let next = queue.next_pending().await.expect("has pending job");
    assert_eq!(next.priority, 0);
}

#[tokio::test]
async fn test_queue_capacity_limits() {
    let config = JobQueueConfig {
        max_queue_size: 2,
        max_concurrent: 4,
    };
    let queue = GpuJobQueue::new(config);
    let _ = queue
        .submit(make_inference_job("m1", "p1"), 0)
        .await
        .unwrap();
    let _ = queue
        .submit(make_inference_job("m2", "p2"), 0)
        .await
        .unwrap();
    let res = queue.submit(make_inference_job("m3", "p3"), 0).await;
    assert!(matches!(res, Err(JobQueueError::QueueFull { .. })));
}

#[tokio::test]
async fn test_status_job_not_found() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let res = queue.status(Uuid::nil()).await;
    assert!(matches!(res, Err(JobQueueError::JobNotFound { .. })));
}

#[tokio::test]
async fn test_mark_completed_and_result() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let id = queue.submit(make_inference_job("m", "p"), 0).await.unwrap();
    queue
        .mark_completed(id, serde_json::json!({"output": "done"}))
        .await
        .unwrap();
    let result = queue.result(id).await.unwrap();
    assert_eq!(result.get("output").and_then(|v| v.as_str()), Some("done"));
}

#[tokio::test]
async fn test_mark_failed_and_result_returns_error() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let id = queue.submit(make_inference_job("m", "p"), 0).await.unwrap();
    queue.mark_failed(id, "oops").await.unwrap();
    let res = queue.result(id).await;
    assert!(matches!(res, Err(JobQueueError::JobFailed { .. })));
}

#[tokio::test]
async fn test_cancel_pending_job() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let id = queue.submit(make_inference_job("m", "p"), 0).await.unwrap();
    queue.cancel(id).await.unwrap();
    let job = queue.status(id).await.unwrap();
    assert_eq!(job.state, JobState::Cancelled);
}

#[tokio::test]
async fn test_list_jobs_filtered_by_state() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let id = queue.submit(make_inference_job("m", "p"), 0).await.unwrap();
    let pending = queue.list(Some(JobState::Pending)).await;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, id);
}

#[tokio::test]
async fn test_counts_by_state() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    queue
        .submit(make_inference_job("m1", "p1"), 0)
        .await
        .unwrap();
    queue
        .submit(make_inference_job("m2", "p2"), 0)
        .await
        .unwrap();
    let counts = queue.counts().await;
    assert_eq!(counts.get("total"), Some(&2));
}
