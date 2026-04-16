// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native execution and primal-backed native response handling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use toadstool::execution::{ExecutionStatus, RuntimeType};
use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, UniversalJob, UniversalJobType, UniversalPrimalProviderDispatch,
    UniversalPrimalRegistry, UniversalScheduler,
};
use uuid::Uuid;

use super::super::helpers::create_test_context;
use super::fixtures::{
    ErrorResponseMockProvider, ServiceUnavailableMockProvider, SuccessWithOutputMockProvider,
    TimeoutResponseMockProvider, test_ctx,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_via_primal_provider() {
    let registry = Arc::new(UniversalPrimalRegistry::<SuccessWithOutputMockProvider>::new_typed());
    let provider = Arc::new(SuccessWithOutputMockProvider {
        instance_id: "native-provider-1".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            env: HashMap::new(),
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
    assert_eq!(response.runtime_used, RuntimeType::Native);
    assert!(matches!(response.status, ExecutionStatus::Success));
    assert_eq!(response.output.stdout.as_deref(), Some("primal output"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_via_primal_provider_error_response() {
    let registry = Arc::new(UniversalPrimalRegistry::<ErrorResponseMockProvider>::new_typed());
    let provider = Arc::new(ErrorResponseMockProvider {
        instance_id: "error-provider".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["x".to_string()],
            env: HashMap::new(),
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
        ExecutionStatus::Failed { error: _ }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_via_primal_provider_timeout_response() {
    let registry = Arc::new(UniversalPrimalRegistry::<TimeoutResponseMockProvider>::new_typed());
    let provider = Arc::new(TimeoutResponseMockProvider {
        instance_id: "timeout-provider".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["x".to_string()],
            env: HashMap::new(),
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
    assert_eq!(response.status, ExecutionStatus::TimedOut);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_via_primal_provider_service_unavailable() {
    let registry = Arc::new(UniversalPrimalRegistry::<ServiceUnavailableMockProvider>::new_typed());
    let provider = Arc::new(ServiceUnavailableMockProvider {
        instance_id: "unavail-provider".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["x".to_string()],
            env: HashMap::new(),
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
        ExecutionStatus::Failed { error: ref e } if e.contains("Service unavailable")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_spawn_failure() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/nonexistent/executable/that/does/not/exist".to_string(),
            args: vec![],
            env: HashMap::new(),
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
        ExecutionStatus::Failed { error: ref e } if e.contains("Failed to spawn")
    ));
    assert_eq!(response.output.exit_code, Some(127));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_process_failure_exit_code() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), "exit 42".to_string()],
            env: HashMap::new(),
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
        ExecutionStatus::Failed { error: ref e } if e.contains("42")
    ));
}

#[tokio::test]
async fn test_discover_self_ip_via_env_toadstool_bind_address() {
    temp_env::async_with_vars(
        [("TOADSTOOL_BIND_ADDRESS", Some("192.168.1.1:8080"))],
        async {
            let registry =
                Arc::new(UniversalPrimalRegistry::<SuccessWithOutputMockProvider>::new_typed());
            let provider = Arc::new(SuccessWithOutputMockProvider {
                instance_id: "p1".to_string(),
                context: test_ctx(),
            });
            registry.register_primal(provider).await.unwrap();
            let scheduler = UniversalScheduler::new(registry).await.unwrap();
            let job = UniversalJob {
                id: Uuid::new_v4(),
                job_type: UniversalJobType::Native {
                    executable: "/bin/echo".to_string(),
                    args: vec!["x".to_string()],
                    env: HashMap::new(),
                },
                priority: JobPriority::Normal,
                resources: ResourceRequirements::default(),
                timeout: Some(Duration::from_secs(30)),
                created_at: std::time::SystemTime::now(),
                context: create_test_context(),
            };
            let result = scheduler.schedule_job(job).await;
            assert!(result.is_ok());
        },
    )
    .await;
}
