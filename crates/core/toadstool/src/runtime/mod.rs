// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime engine orchestration for `ToadStool`

mod engine_registry;

pub use engine_registry::RuntimeSelectionStrategy;

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::{
    ExecutionRequest, ExecutionResponse, RuntimeEngine, RuntimeType, ToadStoolError,
    ToadStoolResult, WorkloadType,
};

use crate::workload::{BackendSelector, WorkloadAnalyzer};

use engine_registry::EngineRegistry;

/// Runtime orchestrator that manages multiple runtime engines
pub struct RuntimeOrchestrator {
    /// Registered runtime engines
    registry: EngineRegistry,
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
            registry: EngineRegistry::new(),
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
            registry: EngineRegistry::new(),
            selection_strategy,
            workload_analyzer: Arc::new(WorkloadAnalyzer::new()),
            backend_selector: Arc::new(backend_selector),
        }
    }

    /// Register a runtime engine
    ///
    /// # Errors
    ///
    /// Returns error if registration fails.
    pub async fn register_engine(
        &self,
        runtime_type: RuntimeType,
        engine: Box<dyn RuntimeEngine>,
    ) -> ToadStoolResult<()> {
        self.registry.register_engine(runtime_type, engine).await
    }

    /// Execute a workload using the appropriate runtime
    ///
    /// # Errors
    ///
    /// Returns error if validation fails, no engine is available for the selected runtime, or execution fails.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )]
    pub async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        let execution_id = request.execution_id;
        info!("Starting execution: {}", execution_id);

        request.workload.validate()?;

        request.security_context.validate()?;

        let runtime_type = self.select_runtime(&request).await?;
        debug!("Selected runtime: {:?}", runtime_type);

        let engines = self.registry.engines().read().await;
        let engine = engines.get(&runtime_type).ok_or_else(|| {
            ToadStoolError::not_found(format!("Runtime engine {runtime_type:?} not available"))
        })?;

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

    async fn select_runtime(&self, request: &ExecutionRequest) -> ToadStoolResult<RuntimeType> {
        if let Some(hint) = &request.runtime_hint {
            if let Some(engine) = self.registry.engines().read().await.get(hint) {
                if engine.supports_workload(&request.workload.workload_type()) {
                    return Ok(hint.clone());
                }
            }
        }

        let workload_type = request.workload.workload_type();
        match workload_type {
            WorkloadType::AiMl | WorkloadType::Cuda => {
                self.select_intelligent_backend(request).await
            }
            _ => {
                self.selection_strategy
                    .select_runtime(request, self.registry.engines())
                    .await
            }
        }
    }

    async fn select_intelligent_backend(
        &self,
        request: &ExecutionRequest,
    ) -> ToadStoolResult<RuntimeType> {
        let characteristics = self.workload_analyzer.analyze(&request.workload);

        debug!(
            "Workload characteristics: compute={:?}, memory={:?}, parallelism={:?}, gpu_advantage={:?}, cpu_viable={}",
            characteristics.compute_intensity,
            characteristics.memory_requirement,
            characteristics.parallelism_level,
            characteristics.gpu_advantage,
            characteristics.cpu_viable
        );

        if matches!(request.workload.workload_type(), WorkloadType::Cuda) {
            let decision = self.backend_selector.select_cuda_backend(&characteristics);

            info!(
                "Selected backend: {:?} (confidence: {:.0}%) - {}",
                decision.cuda_backend,
                decision.confidence * 100.0,
                decision.reasoning
            );

            if self
                .registry
                .engines()
                .read()
                .await
                .contains_key(&RuntimeType::Gpu)
            {
                Ok(RuntimeType::Gpu)
            } else {
                warn!("GPU runtime not available for CUDA workload, falling back to Native");
                Ok(RuntimeType::Native)
            }
        } else {
            let engines = self.registry.engines().read().await;
            let result = if engines.contains_key(&RuntimeType::Gpu) {
                info!("AI/ML workload: using GPU runtime");
                Ok(RuntimeType::Gpu)
            } else if engines.contains_key(&RuntimeType::Python) {
                info!("AI/ML workload: using Python runtime (GPU not available)");
                Ok(RuntimeType::Python)
            } else {
                info!("AI/ML workload: using Native runtime (fallback)");
                Ok(RuntimeType::Native)
            };
            drop(engines);
            result
        }
    }
}

#[cfg(test)]
mod tests;
