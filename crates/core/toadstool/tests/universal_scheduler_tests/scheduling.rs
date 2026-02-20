//! Basic scheduling tests — creation, queue management, job output, resource constraints.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, UniversalJob, UniversalJobType, UniversalPrimalRegistry, UniversalScheduler,
};
use uuid::Uuid;

use super::helpers::{create_resource_spec, create_test_context, create_test_native_job};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_creation() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let result = UniversalScheduler::new(registry).await;
    assert!(result.is_ok(), "Scheduler creation should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_creation_with_empty_registry() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    assert_eq!(scheduler.get_active_job_count().await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_creation_result_is_ok() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let result = UniversalScheduler::new(registry).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_active_job_count().await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_get_active_job_count() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    assert_eq!(
        scheduler.get_active_job_count().await,
        0,
        "New scheduler should have 0 active jobs"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_schedule_native_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = create_test_native_job(JobPriority::Normal);
    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok(), "Scheduling a native job should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_schedule_wasm_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::High,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };
    // Ok or Err both acceptable — no WASM engine required
    let _ = scheduler.schedule_job(job).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_active_job_count_after_completion() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = create_test_native_job(JobPriority::Normal);
    let _ = scheduler.schedule_job(job).await.unwrap();
    assert_eq!(
        scheduler.get_active_job_count().await,
        0,
        "Active jobs should be cleared after job completes"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_sequential_job_submission() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    for i in 0..5 {
        let job = UniversalJob {
            id: Uuid::new_v4(),
            job_type: UniversalJobType::Native {
                executable: "/bin/echo".to_string(),
                args: vec![format!("job-{i}")],
                env: HashMap::new(),
            },
            priority: JobPriority::Normal,
            resources: ResourceRequirements::default(),
            timeout: Some(Duration::from_secs(30)),
            created_at: chrono::Utc::now(),
            context: create_test_context(),
        };
        assert!(
            scheduler.schedule_job(job).await.is_ok(),
            "Job {} should succeed",
            i
        );
    }
    assert_eq!(scheduler.get_active_job_count().await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_job_result_contains_execution_id() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let response = scheduler
        .schedule_job(create_test_native_job(JobPriority::Normal))
        .await
        .unwrap();
    assert_ne!(response.execution_id, uuid::Uuid::nil());
    assert!(matches!(
        response.status,
        toadstool::execution::ExecutionStatus::Success
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_native_job_output_has_runtime_type() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let response = scheduler
        .schedule_job(create_test_native_job(JobPriority::High))
        .await
        .unwrap();
    assert_eq!(
        response.runtime_used,
        toadstool::execution::RuntimeType::Native
    );
    assert!(response.output.stdout.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_job_with_custom_resources() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/true".to_string(),
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: create_resource_spec(2.0, Some(4.0), 1, Some(2)),
        timeout: Some(Duration::from_secs(60)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };
    assert!(scheduler.schedule_job(job).await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_job_with_minimal_resources() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["minimal".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Background,
        resources: ResourceRequirements::default(),
        timeout: None,
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };
    assert!(scheduler.schedule_job(job).await.is_ok());
}
