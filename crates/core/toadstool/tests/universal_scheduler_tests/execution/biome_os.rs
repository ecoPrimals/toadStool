// SPDX-License-Identifier: AGPL-3.0-or-later
//! BiomeOS job execution and provider routing.

use std::sync::Arc;
use std::time::Duration;

use toadstool::execution::ExecutionStatus;
use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, UniversalJob, UniversalJobType, UniversalPrimalProviderDispatch,
    UniversalPrimalRegistry, UniversalScheduler,
};
use uuid::Uuid;

use super::super::helpers::create_test_context;
use super::fixtures::{BiomeOSErrorProvider, BiomeOSMockProvider, test_ctx};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_biome_os_with_provider_success() {
    let registry = Arc::new(UniversalPrimalRegistry::<BiomeOSMockProvider>::new_typed());
    let provider = Arc::new(BiomeOSMockProvider {
        instance_id: "biomeos-1".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({"version": "1"}),
            team_id: "team-42".to_string(),
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
            .is_some_and(|s| s.contains("BiomeOS"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_biome_os_route_failure() {
    let registry = Arc::new(UniversalPrimalRegistry::<BiomeOSErrorProvider>::new_typed());
    let provider = Arc::new(BiomeOSErrorProvider {
        instance_id: "biomeos-err".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({}),
            team_id: "team-1".to_string(),
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
async fn test_execute_biome_os_no_provider() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({}),
            team_id: "team-1".to_string(),
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
        ExecutionStatus::Failed { error: ref e } if e.contains("BiomeOS integration not available")
    ));
}
