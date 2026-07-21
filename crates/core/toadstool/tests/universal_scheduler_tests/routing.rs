// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal routing and BiomeOS job type tests.
//!
//! Tests that `UniversalScheduler` correctly routes Primal and BiomeOS job types
//! to registered providers discovered at runtime.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, PrimalType, UniversalJob, UniversalJobType, UniversalPrimalProviderDispatch,
    UniversalPrimalRegistry, UniversalScheduler,
};

type SchedDispatchWithSimpleMock = UniversalScheduler<
    UniversalPrimalProviderDispatch,
    simple_mock_engine::SimpleMockRuntimeEngine,
>;
use uuid::Uuid;

use super::helpers::{
    FailingMockProvider, SucceedingMockProvider, create_test_context, make_test_context,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_schedule_primal_job() {
    let registry = Arc::new(UniversalPrimalRegistry::<SucceedingMockProvider>::new_typed());
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
        created_at: std::time::SystemTime::now(),
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
    let registry = Arc::new(UniversalPrimalRegistry::<SucceedingMockProvider>::new_typed());
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
        timeout: Some(Duration::from_mins(1)),
        created_at: std::time::SystemTime::now(),
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
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
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
        created_at: std::time::SystemTime::now(),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_primal_job_no_provider_returns_error_response() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "nonexistent_primal".to_string(),
            endpoint: "unix:///tmp/test.sock".to_string(),
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
        toadstool::execution::ExecutionStatus::Failed { .. }
    ));
    assert!(
        response
            .output
            .stderr
            .as_ref()
            .unwrap()
            .contains("nonexistent_primal")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_biome_os_job_no_provider_returns_error_response() {
    let registry = Arc::new(UniversalPrimalRegistry::<SucceedingMockProvider>::new_typed());
    registry
        .register_primal(Arc::new(SucceedingMockProvider {
            instance_id: "compute-only".to_string(),
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
        toadstool::execution::ExecutionStatus::Failed { .. }
    ));
    assert!(response.output.stderr.as_ref().unwrap().contains("BiomeOS"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_primal_provider_route_fails_returns_error_response() {
    let registry = Arc::new(UniversalPrimalRegistry::<FailingMockProvider>::new_typed());
    registry
        .register_primal(Arc::new(FailingMockProvider {
            instance_id: "failing-compute".to_string(),
            context: make_test_context(),
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
        toadstool::execution::ExecutionStatus::Failed { .. }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_native_nonexistent_executable_returns_error_response() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/nonexistent/binary/that/does/not/exist".to_string(),
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
        toadstool::execution::ExecutionStatus::Failed { .. }
    ));
    assert!(response.output.exit_code == Some(127));
}

/// Minimal mock `RuntimeEngine` for scheduler tests.
/// Uses `toadstool_testing::MockRuntimeEngine::new_successful()` would require
/// mock configuration; this minimal impl avoids that.
mod simple_mock_engine {
    use toadstool::execution::{
        ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities, RuntimeConfig,
        RuntimeEngine, RuntimeType,
    };
    use toadstool::{RuntimeMetrics, ToadStoolResult, WorkloadType};

    pub struct SimpleMockRuntimeEngine;

    impl RuntimeEngine for SimpleMockRuntimeEngine {
        fn initialize(
            &mut self,
            _config: RuntimeConfig,
        ) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + '_ {
            async { Ok(()) }
        }

        fn execute(
            &self,
            request: ExecutionRequest,
        ) -> impl std::future::Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_
        {
            async move {
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Success,
                    output: toadstool::execution::ExecutionOutput::default(),
                    metrics: RuntimeMetrics::default(),
                    duration: std::time::Duration::from_millis(10),
                    runtime_used: RuntimeType::Native,
                    warnings: vec![],
                })
            }
        }

        fn get_capabilities(&self) -> RuntimeCapabilities {
            RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Native, WorkloadType::Wasm],
                max_concurrent_executions: Some(10),
                supported_architectures: vec!["x86_64".to_string()],
                platform_features: std::collections::HashMap::new(),
                version: "1.0-test".to_string(),
            }
        }

        fn supports_workload(&self, _: &WorkloadType) -> bool {
            true
        }

        fn get_metrics(
            &self,
        ) -> impl std::future::Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_
        {
            async { Ok(RuntimeMetrics::default()) }
        }

        fn shutdown(
            &mut self,
        ) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + '_ {
            async { Ok(()) }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_register_runtime_engine_and_available_runtimes() {
    use toadstool::execution::RuntimeType;

    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = SchedDispatchWithSimpleMock::create(registry).await.unwrap();

    assert!(scheduler.available_runtimes().await.is_empty());

    let engine = Arc::new(simple_mock_engine::SimpleMockRuntimeEngine);
    scheduler
        .register_runtime_engine(RuntimeType::Native, engine)
        .await;

    let runtimes = scheduler.available_runtimes().await;
    assert_eq!(runtimes.len(), 1);
    assert_eq!(runtimes[0], RuntimeType::Native);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_with_runtime_engines() {
    use toadstool::execution::RuntimeType;

    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let mut engines = HashMap::new();
    engines.insert(
        RuntimeType::Native,
        Arc::new(simple_mock_engine::SimpleMockRuntimeEngine),
    );

    let scheduler = SchedDispatchWithSimpleMock::create_with_runtime_engines(registry, engines)
        .await
        .unwrap();
    let runtimes = scheduler.available_runtimes().await;
    assert_eq!(runtimes.len(), 1);
    assert_eq!(runtimes[0], RuntimeType::Native);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_primals_by_capability() {
    use toadstool::universal::PrimalCapability;

    let registry = Arc::new(UniversalPrimalRegistry::<SucceedingMockProvider>::new_typed());
    registry
        .register_primal(Arc::new(SucceedingMockProvider {
            instance_id: "cap-1".to_string(),
            context: make_test_context(),
            primal_type: PrimalType::Compute,
        }))
        .await
        .unwrap();

    let scheduler = UniversalScheduler::new(Arc::clone(&registry))
        .await
        .unwrap();
    let cap = PrimalCapability::NativeExecution {
        architectures: vec!["x86_64".to_string()],
    };
    let providers = scheduler.find_primals_by_capability(&cap).await;
    assert!(!providers.is_empty());
}
