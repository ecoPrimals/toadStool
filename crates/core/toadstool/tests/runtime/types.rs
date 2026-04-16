// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for runtime.rs - Sprint 24
//!
//! Target: 45% → 65% coverage (~50 tests)
//! Focus: RuntimeOrchestrator, RuntimeSelectionStrategy

use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use toadstool::execution::*;
use toadstool::{RuntimeMetrics, ToadStoolError, ToadStoolResult};

// ============================================================================
// Mock Runtime Engine for Testing
// ============================================================================

#[derive(Debug, Clone)]
struct MockRuntimeEngine {
    supports: Vec<String>,
    should_fail: bool,
}

impl RuntimeEngine for MockRuntimeEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move { Ok(()) }
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        let should_fail = self.should_fail;
        async move {
            if should_fail {
                return Err(ToadStoolError::execution("Mock failure"));
            }

            Ok(ExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::Success,
                output: ExecutionOutput::default(),
                metrics: RuntimeMetrics::default(),
                duration: Duration::from_secs(1),
                runtime_used: RuntimeType::Container,
                warnings: Vec::new(),
            })
        }
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        use toadstool::WorkloadType;
        RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Container, WorkloadType::Native],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string()],
            platform_features: HashMap::new(),
            version: "1.0.0".to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &toadstool::WorkloadType) -> bool {
        // If supports list is empty, support all common types
        if self.supports.is_empty() {
            use toadstool::WorkloadType;
            return matches!(
                workload_type,
                WorkloadType::Container | WorkloadType::Native | WorkloadType::Wasm
            );
        }

        // Otherwise, check if the workload type string is in supports list
        let workload_str = format!("{:?}", workload_type).to_lowercase();
        self.supports
            .iter()
            .any(|s| workload_str.contains(&s.to_lowercase()))
    }

    fn get_metrics(
        &self,
    ) -> impl Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        async move { Ok(RuntimeMetrics::default()) }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move { Ok(()) }
    }
}

