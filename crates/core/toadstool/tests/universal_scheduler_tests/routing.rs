//! Primal routing and BiomeOS job type tests.
//!
//! Tests that UniversalScheduler correctly routes Primal and BiomeOS job types
//! to registered providers discovered at runtime.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, PrimalType, UniversalJob, UniversalJobType, UniversalPrimalRegistry,
    UniversalScheduler,
};
use uuid::Uuid;

use super::helpers::{create_test_context, make_test_context, SucceedingMockProvider};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_schedule_primal_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    registry
        .register_primal(Arc::new(SucceedingMockProvider {
            instance_id: "compute-mock-1".to_string(),
            context: make_test_context(),
            primal_type: PrimalType::Compute,
        }))
        .await
        .unwrap();

    let scheduler = UniversalScheduler::new(Arc::clone(&registry))
        .await
        .unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "compute".to_string(),
            endpoint: "unix:///tmp/toadstool.sock".to_string(),
            payload: serde_json::json!({"task": "test"}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok(), "Primal job scheduling must succeed");
    let response = result.unwrap();
    assert!(
        response
            .output
            .stdout
            .as_ref()
            .is_some_and(|s| s.contains("executed successfully")),
        "stdout should confirm execution"
    );
    assert_eq!(
        response.runtime_used,
        toadstool::execution::RuntimeType::Native
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_schedule_biome_os_job() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    registry
        .register_primal(Arc::new(SucceedingMockProvider {
            instance_id: "biome-os-mock-1".to_string(),
            context: make_test_context(),
            primal_type: PrimalType::OS,
        }))
        .await
        .unwrap();

    let scheduler = UniversalScheduler::new(Arc::clone(&registry))
        .await
        .unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({"name": "test-biome", "version": "1.0"}),
            team_id: "team-001".to_string(),
        },
        priority: JobPriority::High,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(60)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok(), "BiomeOS job scheduling must succeed");
    assert!(matches!(
        result.unwrap().status,
        toadstool::execution::ExecutionStatus::Success
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_wasm_job_response_structure() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: chrono::Utc::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(
        result.is_ok(),
        "WASM job returns Ok even when no engine is registered"
    );
    let response = result.unwrap();
    assert_eq!(
        response.runtime_used,
        toadstool::execution::RuntimeType::Wasm
    );
    assert!(response.output.stderr.is_some() || response.output.stdout.is_some());
}
