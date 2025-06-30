//! Runtime orchestration and management
//!
//! This module provides the main ToadStool runtime orchestrator that manages
//! different runtime engines and provides a unified interface for workload execution.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::error::{ToadStoolError, ToadStoolResult};
use crate::execution::{
    ExecutionRequest, ExecutionResponse,
    RuntimeEngine, RuntimeType,
};
use crate::resources::ResourceMonitor;

/// Runtime orchestrator that manages multiple runtime engines
#[derive(Debug)]
pub struct RuntimeOrchestrator {
    /// Registered runtime engines
    engines: Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,
    /// Runtime selection strategy
    selection_strategy: RuntimeSelectionStrategy,
    /// Resource monitor
    resource_monitor: Option<Arc<dyn ResourceMonitor>>,
}

impl RuntimeOrchestrator {
    /// Create a new runtime orchestrator
    pub fn new(selection_strategy: RuntimeSelectionStrategy) -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
            selection_strategy,
            resource_monitor: None,
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
            ToadStoolError::not_found(format!("Runtime engine {:?} not available", runtime_type))
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
        self.selection_strategy.select_runtime(request, &self.engines).await
    }
}

/// Runtime selection strategies
#[derive(Debug, Clone)]
pub enum RuntimeSelectionStrategy {
    /// Use the first available runtime that supports the workload
    FirstAvailable,
    /// Prefer specific runtime types in order
    PreferenceList(Vec<RuntimeType>),
}

impl RuntimeSelectionStrategy {
    async fn select_runtime(
        &self,
        request: &ExecutionRequest,
        engines: &Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,
    ) -> ToadStoolResult<RuntimeType> {
        match self {
            Self::FirstAvailable => {
                let engines = engines.read().await;
                let workload_type = request.workload.workload_type();
                
                for (runtime_type, engine) in engines.iter() {
                    if engine.supports_workload(&workload_type) {
                        return Ok(runtime_type.clone());
                    }
                }
                
                Err(ToadStoolError::not_found(format!(
                    "No runtime available for workload type: {:?}",
                    workload_type
                )))
            }
            Self::PreferenceList(preferences) => {
                let engines = engines.read().await;
                let workload_type = request.workload.workload_type();
                
                for preferred in preferences {
                    if let Some(engine) = engines.get(preferred) {
                        if engine.supports_workload(&workload_type) {
                            return Ok(preferred.clone());
                        }
                    }
                }
                
                // Fallback to any available runtime
                for (runtime_type, engine) in engines.iter() {
                    if engine.supports_workload(&workload_type) {
                        return Ok(runtime_type.clone());
                    }
                }
                
                Err(ToadStoolError::not_found(format!(
                    "No runtime available for workload type: {:?}",
                    workload_type
                )))
            }
        }
    }
} 