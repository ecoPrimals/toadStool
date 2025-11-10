//! Runtime engine orchestration for `ToadStool`

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::{
    ExecutionRequest, ExecutionResponse, RuntimeEngine, RuntimeType, ToadStoolError,
    ToadStoolResult,
};

/// Runtime orchestrator that manages multiple runtime engines
pub struct RuntimeOrchestrator {
    /// Registered runtime engines
    engines: Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,
    /// Runtime selection strategy
    selection_strategy: RuntimeSelectionStrategy,
}

impl RuntimeOrchestrator {
    /// Create a new runtime orchestrator
    #[must_use]
    pub fn new(selection_strategy: RuntimeSelectionStrategy) -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
            selection_strategy,
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

        // Use selection strategy to choose runtime
        self.selection_strategy
            .select_runtime(request, &self.engines)
            .await
    }
}

/// Runtime selection strategies
#[derive(Debug, Clone)]
pub enum RuntimeSelectionStrategy {
    /// Always use the first available runtime
    FirstAvailable,
    /// Use the runtime with the lowest load
    LoadBalanced,
    /// Use the runtime best suited for the workload
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
