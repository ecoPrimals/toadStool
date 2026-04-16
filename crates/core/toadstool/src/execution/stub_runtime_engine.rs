// SPDX-License-Identifier: AGPL-3.0-or-later
//! Default [`super::RuntimeEngine`] placeholder for generic platform/orchestrator types
//! when no concrete runtime engines are registered.

use std::future::Future;

use crate::ToadStoolResult;
use crate::execution::{
    ExecutionRequest, ExecutionResponse, RuntimeCapabilities, RuntimeConfig, RuntimeEngine,
};

/// Minimal placeholder [`RuntimeEngine`] for generic orchestrator and scheduler types.
///
/// Used when the deployment does not register real engines yet (demos, tests, empty orchestrators).
#[derive(Debug, Default, Clone, Copy)]
pub struct StubRuntimeEngine;

impl RuntimeEngine for StubRuntimeEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn execute(
        &self,
        _request: ExecutionRequest,
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        async {
            Err(crate::ToadStoolError::not_found(
                "No runtime engine registered (stub engine)",
            ))
        }
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![],
            max_concurrent_executions: Some(0),
            supported_architectures: vec![],
            platform_features: std::collections::HashMap::new(),
            version: "stub".to_string(),
        }
    }

    fn supports_workload(&self, _workload_type: &crate::WorkloadType) -> bool {
        false
    }

    fn get_metrics(
        &self,
    ) -> impl Future<Output = ToadStoolResult<crate::RuntimeMetrics>> + Send + '_ {
        async { Ok(crate::RuntimeMetrics::default()) }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
}
