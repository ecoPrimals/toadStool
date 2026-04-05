// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal job type routing and provider interaction.

use std::sync::Arc;
use std::time::Duration;

use toadstool::execution::ExecutionStatus;
use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, UniversalJob, UniversalJobType, UniversalPrimalRegistry, UniversalScheduler,
};
use uuid::Uuid;

use super::super::helpers::create_test_context;
use super::fixtures::{PrimalRouteErrorProvider, SuccessWithOutputMockProvider, test_ctx};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_primal_with_provider_success() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(SuccessWithOutputMockProvider {
        instance_id: "compute-1".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "compute".to_string(),
            endpoint: "execute".to_string(),
            payload: serde_json::json!({"task": "test"}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, ExecutionStatus::Success));
    assert!(
        response
            .output
            .stdout
            .as_ref()
            .is_some_and(|s| s.to_lowercase().contains("primal")),
        "stdout: {:?}",
        response.output.stdout
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_primal_route_failure() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(PrimalRouteErrorProvider {
        instance_id: "compute-err".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "compute".to_string(),
            endpoint: "execute".to_string(),
            payload: serde_json::json!({}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("execution failed")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_primal_no_provider() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "nonexistent".to_string(),
            endpoint: "run".to_string(),
            payload: serde_json::json!({}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("No primal provider")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_primal_no_provider_with_available_list() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(SuccessWithOutputMockProvider {
        instance_id: "other-1".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "different-type".to_string(),
            endpoint: "run".to_string(),
            payload: serde_json::json!({}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("No primal provider")
    ));
}
