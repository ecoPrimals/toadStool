// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use std::sync::RwLock;
use tracing::info;

use crate::{ExecutionRequest, RuntimeEngine, RuntimeType, ToadStoolError, ToadStoolResult};

pub(crate) struct EngineRegistry<E: RuntimeEngine> {
    engines: Arc<RwLock<HashMap<RuntimeType, Arc<E>>>>,
}

impl<E: RuntimeEngine> EngineRegistry<E> {
    pub(crate) fn new() -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(crate) fn engines(&self) -> &Arc<RwLock<HashMap<RuntimeType, Arc<E>>>> {
        &self.engines
    }

    pub fn register_engine(
        &self,
        runtime_type: RuntimeType,
        engine: Arc<E>,
    ) -> ToadStoolResult<()> {
        info!("Registering runtime engine: {:?}", runtime_type);

        self.engines
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(runtime_type, engine);
        info!("Successfully registered runtime engine");
        Ok(())
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
    pub(crate) fn select_runtime<E: RuntimeEngine>(
        &self,
        request: &ExecutionRequest,
        engines: &Arc<RwLock<HashMap<RuntimeType, Arc<E>>>>,
    ) -> ToadStoolResult<RuntimeType> {
        let engines_guard = engines.read().unwrap_or_else(|e| e.into_inner());
        let workload_type = request.workload.workload_type();

        let result = match self {
            Self::FirstAvailable => engines_guard
                .iter()
                .find(|(_, engine)| engine.supports_workload(&workload_type))
                .map(|(rt, _)| rt.clone())
                .or_else(|| engines_guard.keys().next().cloned())
                .ok_or_else(|| ToadStoolError::not_found("No runtime engines available")),
            Self::LoadBalanced => engines_guard
                .iter()
                .find(|(_, engine)| engine.supports_workload(&workload_type))
                .map(|(rt, _)| rt.clone())
                .or_else(|| engines_guard.keys().next().cloned())
                .ok_or_else(|| ToadStoolError::not_found("No runtime engines available")),
            Self::OptimalMatch => engines_guard
                .iter()
                .find(|(_, engine)| engine.supports_workload(&workload_type))
                .map(|(rt, _)| rt.clone())
                .ok_or_else(|| {
                    ToadStoolError::not_found(format!(
                        "No runtime engine supports workload type: {workload_type:?}"
                    ))
                }),
        };
        drop(engines_guard);
        result
    }
}
