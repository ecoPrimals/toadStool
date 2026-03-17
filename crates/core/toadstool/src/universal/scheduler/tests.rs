// SPDX-License-Identifier: AGPL-3.0-only
//! Scheduler tests

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::execution::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeConfig,
    RuntimeEngine, RuntimeType,
};
use crate::resources::{ResourceRequirements, RuntimeMetrics};
use crate::workload::WorkloadType;

use super::super::jobs::{JobPriority, UniversalJob, UniversalJobType};
use super::super::registry::UniversalPrimalRegistry;
use super::super::types::{NetworkLocation, PrimalCapability, PrimalContext, SecurityLevel};
use super::UniversalScheduler;

fn make_test_context() -> PrimalContext {
    PrimalContext {
        user_id: "test-user".to_string(),
        device_id: "test-device".to_string(),
        session_id: "test-session".to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: None,
            network_id: None,
            geo_location: None,
        },
        security_level: SecurityLevel::Standard,
        metadata: HashMap::new(),
    }
}

fn make_universal_job(job_type: UniversalJobType) -> UniversalJob {
    UniversalJob {
        id: Uuid::new_v4(),
        job_type,
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(60)),
        created_at: std::time::SystemTime::now(),
        context: make_test_context(),
    }
}

/// Minimal mock RuntimeEngine for scheduler tests
struct SimpleMockRuntimeEngine;

impl RuntimeEngine for SimpleMockRuntimeEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = crate::ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = crate::ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            let runtime_used = request.runtime_hint.unwrap_or(RuntimeType::Native);
            Ok(ExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::Success,
                output: ExecutionOutput::default(),
                metrics: RuntimeMetrics::default(),
                duration: Duration::from_millis(10),
                runtime_used,
                warnings: vec![],
            })
        })
    }

    fn get_capabilities(&self) -> crate::RuntimeCapabilities {
        crate::RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Native, WorkloadType::Wasm],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
            platform_features: HashMap::new(),
            version: "1.0-test".to_string(),
        }
    }

    fn supports_workload(&self, _: &WorkloadType) -> bool {
        true
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = crate::ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async { Ok(RuntimeMetrics::default()) })
    }

    fn shutdown(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = crate::ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test]
async fn test_scheduler_new_basic_construction() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await;
    assert!(scheduler.is_ok());
    let scheduler = scheduler.unwrap();
    assert!(scheduler.available_runtimes().await.is_empty());
}

#[tokio::test]
async fn test_scheduler_with_runtime_engines() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let mut engines = HashMap::new();
    engines.insert(
        RuntimeType::Native,
        Box::new(SimpleMockRuntimeEngine) as Box<dyn RuntimeEngine>,
    );
    let scheduler = UniversalScheduler::with_runtime_engines(registry, engines)
        .await
        .unwrap();
    let runtimes = scheduler.available_runtimes().await;
    assert_eq!(runtimes.len(), 1);
    assert_eq!(runtimes[0], RuntimeType::Native);
}

#[tokio::test]
async fn test_scheduler_register_runtime_engine_and_available_runtimes() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    assert!(scheduler.available_runtimes().await.is_empty());

    scheduler
        .register_runtime_engine(RuntimeType::Native, Box::new(SimpleMockRuntimeEngine))
        .await;

    let runtimes = scheduler.available_runtimes().await;
    assert_eq!(runtimes.len(), 1);
    assert_eq!(runtimes[0], RuntimeType::Native);

    scheduler
        .register_runtime_engine(RuntimeType::Wasm, Box::new(SimpleMockRuntimeEngine))
        .await;

    let runtimes = scheduler.available_runtimes().await;
    assert_eq!(runtimes.len(), 2);
    assert!(runtimes.contains(&RuntimeType::Native));
    assert!(runtimes.contains(&RuntimeType::Wasm));
}

#[tokio::test]
async fn test_scheduler_get_active_job_count_starts_at_zero() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    assert_eq!(scheduler.get_active_job_count().await, 0);
}

#[tokio::test]
async fn test_scheduler_find_primals_by_capability_empty_registry() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let capability = PrimalCapability::NativeExecution {
        architectures: vec!["x86_64".to_string()],
    };
    let providers = scheduler.find_primals_by_capability(&capability).await;
    assert!(providers.is_empty());
}

#[tokio::test]
async fn test_schedule_job_native_echo_hello() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::Native {
        executable: "echo".to_string(),
        args: vec!["hello".to_string()],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok(), "native echo job should succeed");
    let response = result.unwrap();
    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(
        response.output.stdout.as_deref(),
        Some("hello\n"),
        "stdout should contain 'hello'"
    );
}

#[tokio::test]
async fn test_schedule_job_native_nonexistent_binary_returns_failed() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::Native {
        executable: "/nonexistent/binary/that/does/not/exist".to_string(),
        args: vec![],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(
        result.is_ok(),
        "schedule_job returns Ok even when process fails"
    );
    let response = result.unwrap();
    assert!(
        matches!(response.status, ExecutionStatus::Failed { .. }),
        "status should be Failed"
    );
    assert_eq!(response.output.exit_code, Some(127));
}

#[tokio::test]
async fn test_schedule_job_wasm_no_engine_returns_error() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::Wasm {
        module: vec![0x00, 0x61, 0x73, 0x6d],
        args: vec![],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(
        result.is_ok(),
        "schedule_job returns Ok with Failed status in response"
    );
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { ref error } if error.contains("No WASM execution capability")
    ));
    assert_eq!(response.runtime_used, RuntimeType::Wasm);
}

#[tokio::test]
async fn test_schedule_job_primal_no_provider_returns_error() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::Primal {
        primal_type: "nonexistent_primal".to_string(),
        endpoint: "http://localhost:8080/run".to_string(),
        payload: serde_json::json!({}),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { ref error } if error.contains("No primal provider")
    ));
}

#[tokio::test]
async fn test_schedule_job_biome_os_no_provider_returns_error() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::BiomeOS {
        biome_manifest: serde_json::json!({"version": "1"}),
        team_id: "team-42".to_string(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { ref error } if error.contains("BiomeOS integration not available")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_active_job_count_during_execution() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let mut engines = HashMap::new();
    engines.insert(
        RuntimeType::Native,
        Box::new(SimpleMockRuntimeEngine) as Box<dyn RuntimeEngine>,
    );
    let scheduler = UniversalScheduler::with_runtime_engines(registry, engines)
        .await
        .unwrap();

    let job = make_universal_job(UniversalJobType::Native {
        executable: "echo".to_string(),
        args: vec!["hi".to_string()],
        env: HashMap::new(),
    });

    let scheduler_clone = std::sync::Arc::new(scheduler);
    let job_clone = job.clone();

    let handle = tokio::spawn({
        let s = Arc::clone(&scheduler_clone);
        async move { s.schedule_job(job_clone).await }
    });

    // Yield to let the spawned task run, then continue immediately
    tokio::task::yield_now().await;
    let count = scheduler_clone.get_active_job_count().await;
    // Job may have already completed; either 0 or 1 is valid
    assert!(count <= 1, "active job count should be 0 or 1");

    let _ = handle.await.unwrap();
    assert_eq!(scheduler_clone.get_active_job_count().await, 0);
}

#[tokio::test]
async fn test_schedule_job_with_native_engine_prefers_engine_over_direct_exec() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let mut engines = HashMap::new();
    engines.insert(
        RuntimeType::Native,
        Box::new(SimpleMockRuntimeEngine) as Box<dyn RuntimeEngine>,
    );
    let scheduler = UniversalScheduler::with_runtime_engines(registry, engines)
        .await
        .unwrap();

    let job = make_universal_job(UniversalJobType::Native {
        executable: "echo".to_string(),
        args: vec!["via-engine".to_string()],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.runtime_used, RuntimeType::Native);
}

#[tokio::test]
async fn test_schedule_job_wasm_with_engine_succeeds() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let mut engines = HashMap::new();
    engines.insert(
        RuntimeType::Wasm,
        Box::new(SimpleMockRuntimeEngine) as Box<dyn RuntimeEngine>,
    );
    let scheduler = UniversalScheduler::with_runtime_engines(registry, engines)
        .await
        .unwrap();

    let job = make_universal_job(UniversalJobType::Wasm {
        module: vec![0x00, 0x61, 0x73, 0x6d],
        args: vec![],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.runtime_used, RuntimeType::Wasm);
}

#[tokio::test]
async fn test_find_primals_by_capability_with_registered_provider() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(crate::universal::ToadStoolPrimalProvider::new(
        make_test_context(),
    ));
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let capability = PrimalCapability::NativeExecution {
        architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
    };
    let providers = scheduler.find_primals_by_capability(&capability).await;
    assert!(!providers.is_empty());
}

#[tokio::test]
async fn test_schedule_job_native_with_env_vars() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let mut env = HashMap::new();
    env.insert(
        "SCHEDULER_TEST_VAR".to_string(),
        "hello_from_test".to_string(),
    );

    let job = make_universal_job(UniversalJobType::Native {
        executable: "env".to_string(),
        args: vec![],
        env,
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, ExecutionStatus::Success);
    let stdout = response.output.stdout.unwrap_or_default();
    assert!(
        stdout.contains("SCHEDULER_TEST_VAR=hello_from_test"),
        "stdout should contain env var: {stdout}"
    );
}

#[tokio::test]
async fn test_schedule_job_native_stderr_output() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    // sh -c 'echo "stderr message" >&2' writes to stderr
    let job = make_universal_job(UniversalJobType::Native {
        executable: "sh".to_string(),
        args: vec!["-c".to_string(), "echo 'stderr_message' >&2".to_string()],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, ExecutionStatus::Success);
    let stderr = response.output.stderr.unwrap_or_default();
    assert!(
        stderr.contains("stderr_message"),
        "stderr should contain message: {stderr}"
    );
}

#[tokio::test]
async fn test_schedule_job_native_failed_process_nonzero_exit() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    // `false` exits with code 1
    let job = make_universal_job(UniversalJobType::Native {
        executable: "false".to_string(),
        args: vec![],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(
        matches!(response.status, ExecutionStatus::Failed { .. }),
        "status should be Failed for nonzero exit"
    );
    assert_eq!(response.output.exit_code, Some(1));
}

#[tokio::test]
async fn test_schedule_job_native_exit_code_42() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::Native {
        executable: "sh".to_string(),
        args: vec!["-c".to_string(), "exit 42".to_string()],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, ExecutionStatus::Failed { .. }));
    assert_eq!(response.output.exit_code, Some(42));
}

#[tokio::test]
async fn test_schedule_multiple_concurrent_native_jobs() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = Arc::new(UniversalScheduler::new(registry).await.unwrap());

    let mut handles = vec![];
    for i in 0..5 {
        let s = Arc::clone(&scheduler);
        let job = make_universal_job(UniversalJobType::Native {
            executable: "echo".to_string(),
            args: vec![format!("job_{}", i)],
            env: HashMap::new(),
        });
        handles.push(tokio::spawn(async move { s.schedule_job(job).await }));
    }

    let mut results = Vec::with_capacity(5);
    for h in handles {
        results.push(h.await.expect("task panicked"));
    }

    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "job {i} should succeed");
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
        assert_eq!(
            response.output.stdout.as_deref(),
            Some(format!("job_{i}\n").as_str())
        );
    }
}

#[tokio::test]
async fn test_schedule_job_native_stdout_and_stderr() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::Native {
        executable: "sh".to_string(),
        args: vec![
            "-c".to_string(),
            "echo 'stdout_line'; echo 'stderr_line' >&2".to_string(),
        ],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, ExecutionStatus::Success);
    assert!(
        response
            .output
            .stdout
            .unwrap_or_default()
            .contains("stdout_line")
    );
    assert!(
        response
            .output
            .stderr
            .unwrap_or_default()
            .contains("stderr_line")
    );
}

#[tokio::test]
async fn test_schedule_job_native_with_args() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::Native {
        executable: "echo".to_string(),
        args: vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, ExecutionStatus::Success);
    assert!(
        response
            .output
            .stdout
            .unwrap_or_default()
            .contains("arg1 arg2 arg3")
    );
}

#[tokio::test]
async fn test_schedule_job_native_true_succeeds() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::Native {
        executable: "true".to_string(),
        args: vec![],
        env: HashMap::new(),
    });

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, ExecutionStatus::Success);
    assert_eq!(response.output.exit_code, Some(0));
}

#[tokio::test]
async fn test_schedule_job_active_count_after_completion() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    let job = make_universal_job(UniversalJobType::Native {
        executable: "echo".to_string(),
        args: vec!["done".to_string()],
        env: HashMap::new(),
    });

    let _ = scheduler.schedule_job(job).await;
    let count = scheduler.get_active_job_count().await;
    assert_eq!(count, 0, "active jobs should be 0 after completion");
}

#[tokio::test]
async fn test_register_runtime_engine_replaces_existing() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();

    scheduler
        .register_runtime_engine(RuntimeType::Native, Box::new(SimpleMockRuntimeEngine))
        .await;
    scheduler
        .register_runtime_engine(RuntimeType::Native, Box::new(SimpleMockRuntimeEngine))
        .await;

    let runtimes = scheduler.available_runtimes().await;
    assert_eq!(runtimes.len(), 1, "should replace not duplicate");
}

// --- Tests exercising scheduler through crate re-export path (mod.rs coverage) ---

#[tokio::test]
async fn test_universal_scheduler_via_crate_reexport() {
    use crate::UniversalScheduler;
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry)
        .await
        .expect("scheduler creation");
    assert_eq!(scheduler.get_active_job_count().await, 0);
}

#[tokio::test]
async fn test_scheduler_with_runtime_engines_via_crate_reexport() {
    use crate::UniversalScheduler;
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let mut engines = HashMap::new();
    engines.insert(
        RuntimeType::Wasm,
        Box::new(SimpleMockRuntimeEngine) as Box<dyn RuntimeEngine>,
    );
    let scheduler = UniversalScheduler::with_runtime_engines(registry, engines)
        .await
        .expect("scheduler with engines");
    let runtimes = scheduler.available_runtimes().await;
    assert_eq!(runtimes.len(), 1);
    assert_eq!(runtimes[0], RuntimeType::Wasm);
}

#[tokio::test]
async fn test_schedule_job_native_via_crate_reexport() {
    use crate::UniversalScheduler;
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry)
        .await
        .expect("scheduler creation");
    let job = make_universal_job(UniversalJobType::Native {
        executable: "true".to_string(),
        args: vec![],
        env: HashMap::new(),
    });
    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.expect("response");
    assert_eq!(response.status, ExecutionStatus::Success);
}

#[tokio::test]
async fn test_scheduler_find_primals_via_crate_reexport() {
    use crate::UniversalScheduler;
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry)
        .await
        .expect("scheduler creation");
    let capability = PrimalCapability::NativeExecution {
        architectures: vec!["aarch64".to_string()],
    };
    let providers = scheduler.find_primals_by_capability(&capability).await;
    assert!(providers.is_empty());
}
