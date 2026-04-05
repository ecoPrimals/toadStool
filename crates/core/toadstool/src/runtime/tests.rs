// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use uuid::Uuid;

use crate::execution::RuntimeConfig;
use crate::workload::{AiFramework, AiMlWorkload, AiOperation, CudaLaunchConfig, CudaSource};
use crate::workload::{CudaWorkload, ModelSize, WorkloadSpec};

struct MockRuntimeEngine {
    supported_types: Vec<WorkloadType>,
    execute_result: Option<ToadStoolResult<ExecutionResponse>>,
}

impl MockRuntimeEngine {
    fn new(supported_types: Vec<WorkloadType>) -> Self {
        Self {
            supported_types,
            execute_result: None,
        }
    }

    fn with_execute_success(mut self, execution_id: Uuid, runtime_used: RuntimeType) -> Self {
        self.execute_result = Some(Ok(ExecutionResponse {
            execution_id,
            status: crate::ExecutionStatus::Success,
            output: crate::ExecutionOutput::default(),
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::from_millis(42),
            runtime_used,
            warnings: vec![],
        }));
        self
    }

    fn with_execute_error(mut self, msg: &str) -> Self {
        self.execute_result = Some(Err(ToadStoolError::execution(msg.to_string())));
        self
    }
}

impl RuntimeEngine for MockRuntimeEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        let default_response = ExecutionResponse {
            execution_id: request.execution_id,
            status: crate::ExecutionStatus::Success,
            output: crate::ExecutionOutput::default(),
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::from_millis(1),
            runtime_used: RuntimeType::Native,
            warnings: vec![],
        };
        let result = self
            .execute_result
            .as_ref()
            .map_or(Ok(default_response), |r| match r {
                Ok(resp) => Ok(resp.clone()),
                Err(e) => Err(ToadStoolError::execution(e.to_string())),
            });
        Box::pin(async move { result })
    }

    fn get_capabilities(&self) -> crate::RuntimeCapabilities {
        crate::RuntimeCapabilities {
            supported_workloads: self.supported_types.clone(),
            max_concurrent_executions: Some(16),
            supported_architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
            platform_features: std::collections::HashMap::new(),
            version: "mock-1.0".to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        self.supported_types.contains(workload_type)
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<crate::RuntimeMetrics>> + Send + '_>> {
        Box::pin(async { Ok(crate::RuntimeMetrics::default()) })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

fn ai_ml_workload_spec() -> WorkloadSpec {
    let workload = AiMlWorkload::new(
        AiFramework::PyTorch,
        AiOperation::Inference,
        ModelSize::Small,
        32,
    );
    WorkloadSpec::AiMl { workload }
}

fn cuda_workload_spec() -> WorkloadSpec {
    let launch_config = CudaLaunchConfig::new((256, 1, 1), (256, 1, 1));
    let workload = CudaWorkload::new(
        CudaSource::CudaCpp {
            source: "__global__ void k() {}".to_string(),
            entry_point: "k".to_string(),
        },
        launch_config,
    );
    WorkloadSpec::Cuda { workload }
}

fn wasm_workload_spec() -> WorkloadSpec {
    WorkloadSpec::Wasm {
        module: crate::WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0x00, 0x61, 0x73, 0x6d]),
        },
        args: None,
        wasi_config: None,
        env_vars: std::collections::HashMap::new(),
    }
}

#[tokio::test]
async fn test_orchestrator_construction() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    assert!(std::mem::size_of_val(&orchestrator) > 0);
}

#[tokio::test]
async fn test_orchestrator_with_backend_selector() {
    let selector = crate::workload::BackendSelector::new();
    let orchestrator = RuntimeOrchestrator::with_backend_selector(
        RuntimeSelectionStrategy::OptimalMatch,
        selector,
    );
    assert!(std::mem::size_of_val(&orchestrator) > 0);
}

#[tokio::test]
async fn test_runtime_selection_strategy_variants() {
    let _ = RuntimeSelectionStrategy::FirstAvailable;
    let _ = RuntimeSelectionStrategy::LoadBalanced;
    let _ = RuntimeSelectionStrategy::OptimalMatch;
    assert_eq!(
        format!("{:?}", RuntimeSelectionStrategy::FirstAvailable),
        "FirstAvailable"
    );
}

#[tokio::test]
async fn test_register_engine_and_lookup() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let engine = MockRuntimeEngine::new(vec![WorkloadType::Wasm]);
    orchestrator
        .register_engine(RuntimeType::Wasm, Box::new(engine))
        .await
        .unwrap();
    let orch_no_engine = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);
    let request = ExecutionRequest {
        workload: wasm_workload_spec(),
        ..ExecutionRequest::default()
    };
    let result = orch_no_engine.execute(request).await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string().to_lowercase();
    assert!(
        err_str.contains("not available") || err_str.contains("not found"),
        "Expected 'not available' or 'not found', got: {err_str}"
    );
    let request2 = ExecutionRequest {
        workload: wasm_workload_spec(),
        ..ExecutionRequest::default()
    };
    let result2 = orchestrator.execute(request2).await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_execute_with_mock_engine_success() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let exec_id = Uuid::new_v4();
    let engine = MockRuntimeEngine::new(vec![WorkloadType::AiMl])
        .with_execute_success(exec_id, RuntimeType::Native);
    orchestrator
        .register_engine(RuntimeType::Native, Box::new(engine))
        .await
        .unwrap();

    let request = ExecutionRequest {
        execution_id: exec_id,
        workload: ai_ml_workload_spec(),
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.execution_id, exec_id);
    assert_eq!(response.runtime_used, RuntimeType::Native);
    assert_eq!(response.duration, Duration::from_millis(42));
}

#[tokio::test]
async fn test_execute_with_mock_engine_failure() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let engine =
        MockRuntimeEngine::new(vec![WorkloadType::AiMl]).with_execute_error("mock failure");
    orchestrator
        .register_engine(RuntimeType::Native, Box::new(engine))
        .await
        .unwrap();

    let request = ExecutionRequest {
        workload: ai_ml_workload_spec(),
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("mock failure"));
}

#[tokio::test]
async fn test_selection_logic_ai_ml_prefers_gpu() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let exec_id = Uuid::new_v4();
    orchestrator
        .register_engine(
            RuntimeType::Gpu,
            Box::new(
                MockRuntimeEngine::new(vec![WorkloadType::AiMl])
                    .with_execute_success(exec_id, RuntimeType::Gpu),
            ),
        )
        .await
        .unwrap();
    orchestrator
        .register_engine(
            RuntimeType::Native,
            Box::new(
                MockRuntimeEngine::new(vec![WorkloadType::AiMl])
                    .with_execute_success(exec_id, RuntimeType::Native),
            ),
        )
        .await
        .unwrap();

    let request = ExecutionRequest {
        execution_id: exec_id,
        workload: ai_ml_workload_spec(),
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.runtime_used, RuntimeType::Gpu);
}

#[tokio::test]
async fn test_selection_logic_cuda_with_gpu_available() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let exec_id = Uuid::new_v4();
    orchestrator
        .register_engine(
            RuntimeType::Gpu,
            Box::new(
                MockRuntimeEngine::new(vec![WorkloadType::Cuda])
                    .with_execute_success(exec_id, RuntimeType::Gpu),
            ),
        )
        .await
        .unwrap();

    let request = ExecutionRequest {
        execution_id: exec_id,
        workload: cuda_workload_spec(),
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().runtime_used, RuntimeType::Gpu);
}

#[tokio::test]
async fn test_selection_logic_cuda_fallback_to_native_when_no_gpu() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    orchestrator
        .register_engine(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine::new(vec![WorkloadType::Cuda])),
        )
        .await
        .unwrap();

    let request = ExecutionRequest {
        workload: cuda_workload_spec(),
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().runtime_used, RuntimeType::Native);
}

#[tokio::test]
async fn test_runtime_hint_respected_when_supported() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let exec_id = Uuid::new_v4();
    orchestrator
        .register_engine(
            RuntimeType::Python,
            Box::new(
                MockRuntimeEngine::new(vec![WorkloadType::AiMl])
                    .with_execute_success(exec_id, RuntimeType::Python),
            ),
        )
        .await
        .unwrap();
    orchestrator
        .register_engine(
            RuntimeType::Gpu,
            Box::new(
                MockRuntimeEngine::new(vec![WorkloadType::AiMl])
                    .with_execute_success(exec_id, RuntimeType::Gpu),
            ),
        )
        .await
        .unwrap();

    let request = ExecutionRequest {
        execution_id: exec_id,
        workload: ai_ml_workload_spec(),
        runtime_hint: Some(RuntimeType::Python),
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().runtime_used, RuntimeType::Python);
}

#[tokio::test]
async fn test_runtime_hint_ignored_when_unsupported() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let exec_id = Uuid::new_v4();
    orchestrator
        .register_engine(
            RuntimeType::Gpu,
            Box::new(
                MockRuntimeEngine::new(vec![WorkloadType::AiMl])
                    .with_execute_success(exec_id, RuntimeType::Gpu),
            ),
        )
        .await
        .unwrap();

    let request = ExecutionRequest {
        execution_id: exec_id,
        workload: ai_ml_workload_spec(),
        runtime_hint: Some(RuntimeType::Python),
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().runtime_used, RuntimeType::Gpu);
}

#[tokio::test]
async fn test_optimal_match_returns_error_when_no_engine_supports() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);
    orchestrator
        .register_engine(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine::new(vec![WorkloadType::Native])),
        )
        .await
        .unwrap();

    let request = ExecutionRequest {
        workload: wasm_workload_spec(),
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("supports workload type") || err_msg.contains("Wasm"),
        "Expected workload type error, got: {err_msg}"
    );
}

#[tokio::test]
async fn test_execute_validates_workload() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    orchestrator
        .register_engine(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine::new(vec![WorkloadType::Native])),
        )
        .await
        .unwrap();

    let request = ExecutionRequest {
        workload: WorkloadSpec::Native {
            executable: crate::ExecutableSource::File {
                path: std::path::PathBuf::from("/nonexistent/path/12345"),
            },
            args: None,
            working_dir: None,
            env_vars: std::collections::HashMap::new(),
            user: None,
        },
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("not found")
    );
}

#[tokio::test]
async fn test_execute_validates_security_context() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    orchestrator
        .register_engine(
            RuntimeType::Gpu,
            Box::new(MockRuntimeEngine::new(vec![WorkloadType::AiMl])),
        )
        .await
        .unwrap();

    let ctx = crate::SecurityContext {
        capabilities: vec![],
        ..Default::default()
    };
    let request = ExecutionRequest {
        workload: ai_ml_workload_spec(),
        security_context: ctx,
        ..ExecutionRequest::default()
    };
    let result = orchestrator.execute(request).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("capability")
    );
}
