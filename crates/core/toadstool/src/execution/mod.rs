// SPDX-License-Identifier: AGPL-3.0-or-later
//! Execution types and runtime engine interface

use std::future::Future;

use crate::ToadStoolResult;

// Re-export all pure data types from toadstool-core
pub use toadstool_core::execution::*;

/// Core trait for all runtime execution engines in the ToadStool universal compute platform.
///
/// This trait defines the interface that all runtime engines must implement to participate
/// in workload execution. Implementations can support various execution environments:
/// Native binaries, WASM modules, containers, GPU compute, or custom runtimes.
pub trait RuntimeEngine: Send + Sync {
    /// Initialize the runtime engine with the provided configuration.
    fn initialize(
        &mut self,
        config: RuntimeConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_;

    /// Execute a workload and return the result.
    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_;

    /// Get the capabilities supported by this runtime.
    fn get_capabilities(&self) -> RuntimeCapabilities;

    /// Check if this runtime supports a specific workload type.
    fn supports_workload(&self, workload_type: &crate::WorkloadType) -> bool;

    /// Get current runtime metrics.
    fn get_metrics(
        &self,
    ) -> impl Future<Output = ToadStoolResult<crate::RuntimeMetrics>> + Send + '_;

    /// Shutdown the runtime engine and clean up resources.
    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_;
}

mod stub_runtime_engine;

pub use stub_runtime_engine::StubRuntimeEngine;

pub use crate::runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy};

#[cfg(test)]
mod tests;
