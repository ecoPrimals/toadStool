// SPDX-License-Identifier: AGPL-3.0-or-later
//! Default [`super::RuntimeEngine`] placeholder for generic platform/orchestrator types
//! when no concrete runtime engines are registered.

use std::future::Future;

use crate::ToadStoolResult;
use crate::execution::{
    ExecutionRequest, ExecutionResponse, RuntimeCapabilities, RuntimeConfig, RuntimeEngine,
};

/// Sentinel [`RuntimeEngine`] — null-object default for generic orchestrator,
/// scheduler, and platform types before real engines are discovered at runtime.
///
/// This is **not** a test mock. It is the complete implementation of the
/// "no engine registered" state. [`execute`](RuntimeEngine::execute) returns
/// [`ToadStoolError::configuration`] with capability-based guidance directing
/// callers to register an engine. [`initialize`](RuntimeEngine::initialize)
/// and [`shutdown`](RuntimeEngine::shutdown) succeed as no-ops.
#[derive(Debug, Default, Clone, Copy)]
pub struct StubRuntimeEngine;

const ENGINE_MSG: &str =
    "no runtime engine registered; register engines via compute.engine.register capability";

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
        async { Err(crate::ToadStoolError::configuration(ENGINE_MSG)) }
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![],
            max_concurrent_executions: Some(0),
            supported_architectures: vec![],
            platform_features: std::collections::HashMap::new(),
            version: "unregistered".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorkloadType;

    #[tokio::test]
    async fn execute_returns_configuration_error() {
        let engine = StubRuntimeEngine;
        let request = ExecutionRequest::default();
        let result = engine.execute(request).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("no runtime engine registered"));
        assert!(err.contains("compute.engine.register"));
    }

    #[tokio::test]
    async fn initialize_succeeds() {
        let mut engine = StubRuntimeEngine;
        engine.initialize(RuntimeConfig::default()).await.unwrap();
    }

    #[tokio::test]
    async fn shutdown_succeeds() {
        let mut engine = StubRuntimeEngine;
        engine.shutdown().await.unwrap();
    }

    #[test]
    fn supports_no_workload_types() {
        let engine = StubRuntimeEngine;
        assert!(!engine.supports_workload(&WorkloadType::Native));
        assert!(!engine.supports_workload(&WorkloadType::Container));
    }

    #[test]
    fn capabilities_are_empty() {
        let engine = StubRuntimeEngine;
        let caps = engine.get_capabilities();
        assert!(caps.supported_workloads.is_empty());
        assert_eq!(caps.max_concurrent_executions, Some(0));
        assert!(caps.supported_architectures.is_empty());
        assert!(caps.platform_features.is_empty());
        assert_eq!(caps.version, "unregistered");
    }

    #[tokio::test]
    async fn get_metrics_returns_defaults() {
        let engine = StubRuntimeEngine;
        let metrics = engine.get_metrics().await.unwrap();
        assert!(metrics.cpu.usage_percent.abs() < f64::EPSILON);
        assert_eq!(metrics.memory.used_bytes, 0);
    }

    #[test]
    fn copy_and_debug() {
        let a = StubRuntimeEngine;
        let b = a;
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }
}
