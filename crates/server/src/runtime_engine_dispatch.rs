// SPDX-License-Identifier: AGPL-3.0-or-later
//! Enum dispatch for production [`toadstool::execution::RuntimeEngine`] implementations.

use toadstool::execution::RuntimeConfig;
use toadstool::{
    ExecutionRequest, ExecutionResponse, RuntimeCapabilities, RuntimeEngine, RuntimeMetrics,
    ToadStoolResult, WorkloadType,
};
use toadstool_runtime_container::ContainerRuntimeEngine;
#[cfg(target_os = "linux")]
use toadstool_runtime_gpu::UniversalGpuEngine;
use toadstool_runtime_native::NativeRuntimeEngine;
use toadstool_runtime_specialty::SpecialtyRuntimeEngine;
use toadstool_runtime_wasm::WasmRuntimeEngine;

/// Production GPU engine type (universal GPU engine).
#[cfg(target_os = "linux")]
pub type GpuRuntimeEngine = UniversalGpuEngine;

/// Bundles all first-party runtime engines for use with [`toadstool::runtime::EngineRegistry`],
/// [`crate::state::ServerState`], and related generic APIs.
pub enum RuntimeEngineDispatch {
    /// Native process runtime.
    Native(NativeRuntimeEngine),
    /// OCI/container runtime.
    Container(ContainerRuntimeEngine),
    /// GPU compute runtime (Linux only).
    #[cfg(target_os = "linux")]
    Gpu(GpuRuntimeEngine),
    /// WebAssembly (`wasmi`) runtime.
    Wasm(WasmRuntimeEngine),
    /// Specialty / legacy hardware runtime.
    Specialty(SpecialtyRuntimeEngine),
}

impl std::fmt::Debug for RuntimeEngineDispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native(e) => f.debug_tuple("Native").field(e).finish(),
            Self::Container(e) => f.debug_tuple("Container").field(e).finish(),
            #[cfg(target_os = "linux")]
            Self::Gpu(_) => f.write_str("Gpu(...)"),
            Self::Wasm(e) => f.debug_tuple("Wasm").field(e).finish(),
            Self::Specialty(e) => f.debug_tuple("Specialty").field(e).finish(),
        }
    }
}

impl RuntimeEngine for RuntimeEngineDispatch {
    fn initialize(
        &mut self,
        config: RuntimeConfig,
    ) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                Self::Native(e) => e.initialize(config).await,
                Self::Container(e) => e.initialize(config).await,
                #[cfg(target_os = "linux")]
                Self::Gpu(e) => e.initialize(config).await,
                Self::Wasm(e) => e.initialize(config).await,
                Self::Specialty(e) => RuntimeEngine::initialize(e, config).await,
            }
        }
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> impl std::future::Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        async move {
            match self {
                Self::Native(e) => e.execute(request).await,
                Self::Container(e) => e.execute(request).await,
                #[cfg(target_os = "linux")]
                Self::Gpu(e) => e.execute(request).await,
                Self::Wasm(e) => e.execute(request).await,
                Self::Specialty(e) => e.execute(request).await,
            }
        }
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        match self {
            Self::Native(e) => e.get_capabilities(),
            Self::Container(e) => e.get_capabilities(),
            #[cfg(target_os = "linux")]
            Self::Gpu(e) => e.get_capabilities(),
            Self::Wasm(e) => e.get_capabilities(),
            Self::Specialty(e) => e.get_capabilities(),
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        match self {
            Self::Native(e) => e.supports_workload(workload_type),
            Self::Container(e) => e.supports_workload(workload_type),
            #[cfg(target_os = "linux")]
            Self::Gpu(e) => e.supports_workload(workload_type),
            Self::Wasm(e) => e.supports_workload(workload_type),
            Self::Specialty(e) => e.supports_workload(workload_type),
        }
    }

    fn get_metrics(
        &self,
    ) -> impl std::future::Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        async move {
            match self {
                Self::Native(e) => e.get_metrics().await,
                Self::Container(e) => e.get_metrics().await,
                #[cfg(target_os = "linux")]
                Self::Gpu(e) => e.get_metrics().await,
                Self::Wasm(e) => e.get_metrics().await,
                Self::Specialty(e) => RuntimeEngine::get_metrics(e).await,
            }
        }
    }

    fn shutdown(&mut self) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                Self::Native(e) => e.shutdown().await,
                Self::Container(e) => e.shutdown().await,
                #[cfg(target_os = "linux")]
                Self::Gpu(e) => e.shutdown().await,
                Self::Wasm(e) => e.shutdown().await,
                Self::Specialty(e) => e.shutdown().await,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use toadstool::WorkloadType;
    use toadstool_runtime_container::ContainerRuntimeEngine;
    #[cfg(target_os = "linux")]
    use toadstool_runtime_gpu::UniversalGpuEngine;
    use toadstool_runtime_native::NativeRuntimeEngine;
    use toadstool_runtime_specialty::{SpecialtyRuntimeConfig, SpecialtyRuntimeEngine};
    use toadstool_runtime_wasm::{WasmRuntimeConfig, WasmRuntimeEngine};

    use super::RuntimeEngineDispatch;
    use toadstool::RuntimeEngine;

    #[test]
    fn native_variant_delegates_supports_workload() {
        let dispatch = RuntimeEngineDispatch::Native(NativeRuntimeEngine::new());
        assert!(dispatch.supports_workload(&WorkloadType::Native));
        assert!(!dispatch.supports_workload(&WorkloadType::Wasm));
    }

    #[test]
    fn container_variant_delegates_supports_workload() {
        let inner = ContainerRuntimeEngine::new().expect("container engine");
        let dispatch = RuntimeEngineDispatch::Container(inner);
        assert!(dispatch.supports_workload(&WorkloadType::Container));
        assert!(!dispatch.supports_workload(&WorkloadType::Native));
    }

    #[test]
    fn wasm_variant_delegates_supports_workload() {
        let inner = WasmRuntimeEngine::new(WasmRuntimeConfig::default()).expect("wasm engine");
        let dispatch = RuntimeEngineDispatch::Wasm(inner);
        assert!(dispatch.supports_workload(&WorkloadType::Wasm));
        assert!(!dispatch.supports_workload(&WorkloadType::Container));
    }

    #[test]
    fn specialty_variant_delegates_supports_workload() {
        let inner = SpecialtyRuntimeEngine::new(SpecialtyRuntimeConfig::default());
        let dispatch = RuntimeEngineDispatch::Specialty(inner);
        let caps = dispatch.get_capabilities();
        assert!(!caps.supported_workloads.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn gpu_variant_delegates_get_capabilities() {
        let inner = UniversalGpuEngine::new().await.expect("gpu engine");
        let dispatch = RuntimeEngineDispatch::Gpu(inner);
        let caps = dispatch.get_capabilities();
        assert!(caps.supported_workloads.contains(&WorkloadType::Gpu));
    }

    #[tokio::test]
    async fn native_variant_delegates_get_metrics() {
        let dispatch = RuntimeEngineDispatch::Native(NativeRuntimeEngine::new());
        let metrics = dispatch.get_metrics().await.expect("metrics");
        assert!((metrics.cpu.usage_percent - 0.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn wasm_variant_delegates_shutdown() {
        let inner = WasmRuntimeEngine::new(WasmRuntimeConfig::default()).expect("wasm engine");
        let mut dispatch = RuntimeEngineDispatch::Wasm(inner);
        dispatch.shutdown().await.expect("shutdown");
    }
}
