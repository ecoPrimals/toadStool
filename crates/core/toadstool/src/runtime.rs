//! Runtime engine orchestration for `ToadStool`

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{
    ExecutionRequest, ExecutionResponse, RuntimeEngine, RuntimeType, ToadStoolError,
    ToadStoolResult, WorkloadType,
};

use crate::workload::{BackendSelector, WorkloadAnalyzer};

/// Runtime orchestrator that manages multiple runtime engines
pub struct RuntimeOrchestrator {
    /// Registered runtime engines
    engines: Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,
    /// Runtime selection strategy
    selection_strategy: RuntimeSelectionStrategy,
    /// Workload analyzer for intelligent routing (AI/ML, CUDA)
    workload_analyzer: Arc<WorkloadAnalyzer>,
    /// Backend selector for intelligent backend routing
    backend_selector: Arc<BackendSelector>,
}

impl RuntimeOrchestrator {
    /// Create a new runtime orchestrator
    #[must_use]
    pub fn new(selection_strategy: RuntimeSelectionStrategy) -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
            selection_strategy,
            workload_analyzer: Arc::new(WorkloadAnalyzer::new()),
            backend_selector: Arc::new(BackendSelector::new()),
        }
    }

    /// Create orchestrator with custom backend selector (for testing)
    #[must_use]
    pub fn with_backend_selector(
        selection_strategy: RuntimeSelectionStrategy,
        backend_selector: BackendSelector,
    ) -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
            selection_strategy,
            workload_analyzer: Arc::new(WorkloadAnalyzer::new()),
            backend_selector: Arc::new(backend_selector),
        }
    }

    /// Register a runtime engine
    pub async fn register_engine(
        &self,
        runtime_type: RuntimeType,
        engine: Box<dyn RuntimeEngine>,
    ) -> ToadStoolResult<()> {
        info!("Registering runtime engine: {:?}", runtime_type);

        let mut engines = self.engines.write().await;
        engines.insert(runtime_type, engine);
        info!("Successfully registered runtime engine");
        Ok(())
    }

    /// Execute a workload using the appropriate runtime
    pub async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        let execution_id = request.execution_id;
        info!("Starting execution: {}", execution_id);

        // Validate the workload specification
        request.workload.validate()?;

        // Validate the security context
        request.security_context.validate()?;

        // Select the appropriate runtime
        let runtime_type = self.select_runtime(&request).await?;
        debug!("Selected runtime: {:?}", runtime_type);

        // Get the runtime engine
        let engines = self.engines.read().await;
        let engine = engines.get(&runtime_type).ok_or_else(|| {
            ToadStoolError::not_found(format!("Runtime engine {runtime_type:?} not available"))
        })?;

        // Execute the workload
        let result = engine.execute(request).await;

        match &result {
            Ok(response) => {
                info!(
                    "Execution {} completed successfully in {:?}",
                    execution_id, response.duration
                );
            }
            Err(e) => {
                error!("Execution {} failed: {}", execution_id, e);
            }
        }

        result
    }

    /// Select the appropriate runtime for a workload
    async fn select_runtime(&self, request: &ExecutionRequest) -> ToadStoolResult<RuntimeType> {
        // If a runtime hint is provided, try to use it
        if let Some(hint) = &request.runtime_hint {
            let engines = self.engines.read().await;
            if let Some(engine) = engines.get(hint) {
                if engine.supports_workload(&request.workload.workload_type()) {
                    return Ok(hint.clone());
                }
            }
        }

        // For AI/ML and CUDA workloads, use intelligent backend selection
        let workload_type = request.workload.workload_type();
        match workload_type {
            WorkloadType::AiMl | WorkloadType::Cuda => {
                self.select_intelligent_backend(request).await
            }
            _ => {
                // Use standard selection strategy for other workloads
                self.selection_strategy
                    .select_runtime(request, &self.engines)
                    .await
            }
        }
    }

    /// Intelligent backend selection for AI/ML and CUDA workloads
    async fn select_intelligent_backend(
        &self,
        request: &ExecutionRequest,
    ) -> ToadStoolResult<RuntimeType> {
        // Analyze workload characteristics
        let characteristics = self.workload_analyzer.analyze(&request.workload);

        debug!(
            "Workload characteristics: compute={:?}, memory={:?}, parallelism={:?}, gpu_advantage={:?}, cpu_viable={}",
            characteristics.compute_intensity,
            characteristics.memory_requirement,
            characteristics.parallelism_level,
            characteristics.gpu_advantage,
            characteristics.cpu_viable
        );

        // For CUDA workloads, use backend selector to choose optimal backend
        if matches!(request.workload.workload_type(), WorkloadType::Cuda) {
            let decision = self.backend_selector.select_cuda_backend(&characteristics);

            info!(
                "Selected backend: {:?} (confidence: {:.0}%) - {}",
                decision.cuda_backend,
                decision.confidence * 100.0,
                decision.reasoning
            );

            // Map CUDA backend to runtime type
            // For now, all CUDA backends use the GPU runtime
            // In the future, we might have separate runtimes for CPU fallback
            let engines = self.engines.read().await;
            if engines.contains_key(&RuntimeType::Gpu) {
                Ok(RuntimeType::Gpu)
            } else {
                warn!("GPU runtime not available for CUDA workload, falling back to Native");
                Ok(RuntimeType::Native)
            }
        } else {
            // For AI/ML workloads, prefer GPU if available, otherwise use Native
            let engines = self.engines.read().await;
            if engines.contains_key(&RuntimeType::Gpu) {
                info!("AI/ML workload: using GPU runtime");
                Ok(RuntimeType::Gpu)
            } else if engines.contains_key(&RuntimeType::Python) {
                info!("AI/ML workload: using Python runtime (GPU not available)");
                Ok(RuntimeType::Python)
            } else {
                info!("AI/ML workload: using Native runtime (fallback)");
                Ok(RuntimeType::Native)
            }
        }
    }
}

/// Runtime selection strategies
#[derive(Debug, Clone)]
pub enum RuntimeSelectionStrategy {
    /// Always use the first available runtime
    FirstAvailable,
    /// Use the runtime with the lowest load
    LoadBalanced,
    /// Use the runtime best suited for the workload (with intelligent backend selection for AI/ML and CUDA)
    OptimalMatch,
}

impl RuntimeSelectionStrategy {
    async fn select_runtime(
        &self,
        request: &ExecutionRequest,
        engines: &Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,
    ) -> ToadStoolResult<RuntimeType> {
        let engines_guard = engines.read().await;

        match self {
            RuntimeSelectionStrategy::FirstAvailable => {
                // Try engines in a deterministic order for consistency
                // First check if the workload type suggests a preferred runtime
                let workload_type = request.workload.workload_type();

                // Try to find an engine that supports the workload
                for (runtime_type, engine) in engines_guard.iter() {
                    if engine.supports_workload(&workload_type) {
                        return Ok(runtime_type.clone());
                    }
                }

                // If no engine explicitly supports it, return the first available
                engines_guard
                    .keys()
                    .next()
                    .cloned()
                    .ok_or_else(|| ToadStoolError::not_found("No runtime engines available"))
            }
            RuntimeSelectionStrategy::LoadBalanced => {
                // For now, just return the first available that supports the workload
                // In a real implementation, this would check load metrics
                let workload_type = request.workload.workload_type();

                for (runtime_type, engine) in engines_guard.iter() {
                    if engine.supports_workload(&workload_type) {
                        return Ok(runtime_type.clone());
                    }
                }

                engines_guard
                    .keys()
                    .next()
                    .cloned()
                    .ok_or_else(|| ToadStoolError::not_found("No runtime engines available"))
            }
            RuntimeSelectionStrategy::OptimalMatch => {
                // Find the best runtime for the workload type
                let workload_type = request.workload.workload_type();

                for (runtime_type, engine) in engines_guard.iter() {
                    if engine.supports_workload(&workload_type) {
                        return Ok(runtime_type.clone());
                    }
                }

                Err(ToadStoolError::not_found(format!(
                    "No runtime engine supports workload type: {workload_type:?}"
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::time::Duration;
    use uuid::Uuid;

    use crate::execution::RuntimeConfig;
    use crate::workload::{AiFramework, AiMlWorkload, AiOperation, CudaLaunchConfig, CudaSource};
    use crate::workload::{CudaWorkload, ModelSize, WorkloadSpec};

    /// Mock RuntimeEngine for testing - supports configurable workload types and returns
    /// configurable success/failure from execute.
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
        ) -> Pin<Box<dyn Future<Output = ToadStoolResult<crate::RuntimeMetrics>> + Send + '_>>
        {
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
                data: vec![0x00, 0x61, 0x73, 0x6d],
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
        // Request with no engines registered - use OptimalMatch + workload no engine supports
        let orch_no_engine = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);
        let request = ExecutionRequest {
            workload: wasm_workload_spec(),
            ..ExecutionRequest::default()
        };
        let result = orch_no_engine.execute(request).await;
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("not available") || err_str.contains("not found"),
            "Expected 'not available' or 'not found', got: {err_str}"
        );
        // With registered engine, execute succeeds
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
        assert!(result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("not found"));
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

        let mut ctx = crate::SecurityContext::default();
        ctx.capabilities = vec![];
        let request = ExecutionRequest {
            workload: ai_ml_workload_spec(),
            security_context: ctx,
            ..ExecutionRequest::default()
        };
        let result = orchestrator.execute(request).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("capability"));
    }
}
