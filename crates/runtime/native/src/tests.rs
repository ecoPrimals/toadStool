// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use toadstool::{
    RuntimeEngine, WorkloadType,
    execution::{ExecutionInput, ExecutionRequest, ExecutionStatus, RuntimeConfig, RuntimeType},
    resources::ResourceRequirements,
    security::{Capability, IsolationLevel, SecurityContext},
    workload::{ExecutableSource, WorkloadSpec},
};

use crate::engine::NativeRuntimeEngine;

async fn create_test_engine() -> NativeRuntimeEngine {
    let mut engine = NativeRuntimeEngine::new();
    engine
        .initialize(RuntimeConfig::default())
        .await
        .expect("Test engine initialization should succeed");
    engine
}

fn create_test_request(executable_path: &str, args: Vec<String>) -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from(executable_path),
            },
            args: Some(args),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        runtime_hint: Some(RuntimeType::Native),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic)
            .with_capability(Capability::Execute)
            .with_capability(Capability::Read),
        timeout: Some(Duration::from_secs(10)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_engine_initialization() {
    let engine = create_test_engine().await;
    assert!(engine.supports_workload(&WorkloadType::Native));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_capabilities() {
    let engine = create_test_engine().await;
    let capabilities = engine.get_capabilities();

    assert!(
        capabilities
            .supported_workloads
            .contains(&WorkloadType::Native)
    );
    assert!(capabilities.max_concurrent_executions.is_some());
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_simple_execution() {
    let engine = create_test_engine().await;
    let request = create_test_request("/bin/echo", vec!["hello".to_string()]);

    let response = engine
        .execute(request)
        .await
        .expect("Echo execution should succeed");

    assert_eq!(response.status, ExecutionStatus::Success);
    assert!(
        response
            .output
            .stdout
            .expect("Echo should produce stdout")
            .contains("hello")
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_with_args() {
    let engine = create_test_engine().await;
    let request = create_test_request("/bin/ls", vec!["-la".to_string(), "/tmp".to_string()]);

    let response = engine.execute(request).await.unwrap();

    assert_eq!(response.status, ExecutionStatus::Success);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_timeout_handling() {
    let engine = create_test_engine().await;
    let mut request = create_test_request("/bin/sleep", vec!["5".to_string()]);
    request.timeout = Some(Duration::from_millis(100));

    let response = engine.execute(request).await.unwrap();

    assert_eq!(response.status, ExecutionStatus::TimedOut);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_invalid_executable() {
    let engine = create_test_engine().await;
    let request = create_test_request("/nonexistent/executable", vec![]);

    let result = engine.execute(request).await;

    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_metrics() {
    let engine = create_test_engine().await;
    let metrics = engine.get_metrics().await.unwrap();

    assert!(metrics.cpu.usage_percent >= 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_shutdown() {
    let mut engine = create_test_engine().await;
    let shutdown_result = engine.shutdown().await;

    assert!(shutdown_result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_supports_workload() {
    let engine = NativeRuntimeEngine::new();

    assert!(engine.supports_workload(&WorkloadType::Native));
    assert!(!engine.supports_workload(&WorkloadType::Wasm));
    assert!(!engine.supports_workload(&WorkloadType::Container));
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_with_env_vars() {
    let engine = create_test_engine().await;
    let mut env_vars = HashMap::new();
    env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());

    let mut request = create_test_request(
        "/bin/sh",
        vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
    );
    if let WorkloadSpec::Native {
        env_vars: ref mut env,
        ..
    } = request.workload
    {
        *env = env_vars;
    }

    let response = engine.execute(request).await.unwrap();

    assert!(matches!(
        response.status,
        ExecutionStatus::Success | ExecutionStatus::Failed { .. }
    ));
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_with_working_dir() {
    let engine = create_test_engine().await;
    let mut request = create_test_request("/bin/pwd", vec![]);

    if let WorkloadSpec::Native {
        working_dir: ref mut dir,
        ..
    } = request.workload
    {
        *dir = Some(PathBuf::from("/tmp"));
    }

    let response = engine.execute(request).await.unwrap();

    assert_eq!(response.status, ExecutionStatus::Success);
    if let Some(stdout) = response.output.stdout {
        assert!(stdout.contains("/tmp"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_concurrent_executions() {
    let engine = Arc::new(create_test_engine().await);
    let mut handles = vec![];

    for _ in 0..5 {
        let engine_clone = Arc::clone(&engine);
        let handle = tokio::spawn(async move {
            let request = create_test_request(
                if cfg!(windows) { "cmd" } else { "/bin/echo" },
                vec!["test".to_string()],
            );
            engine_clone.execute(request).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_failure_handling() {
    let engine = create_test_engine().await;
    let request = create_test_request("/bin/false", vec![]);

    let response = engine.execute(request).await.unwrap();

    match response.status {
        ExecutionStatus::Failed { .. } => {}
        _ => panic!("Expected failed status for /bin/false"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_default_capabilities() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(
        capabilities
            .supported_workloads
            .contains(&WorkloadType::Native)
    );
    assert!(capabilities.max_concurrent_executions.is_some());
    assert!(capabilities.max_concurrent_executions.unwrap() > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_engine_default_construction() {
    let engine1 = NativeRuntimeEngine::new();
    let engine2 = NativeRuntimeEngine::default();

    assert_eq!(engine1.config.settings.len(), engine2.config.settings.len());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_debug_trait() {
    let engine = NativeRuntimeEngine::new();
    let debug_str = format!("{engine:?}");

    assert!(debug_str.contains("NativeRuntimeEngine"));
    assert!(debug_str.contains("config"));
}
