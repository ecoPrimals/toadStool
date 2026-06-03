// SPDX-License-Identifier: AGPL-3.0-or-later
//! Default [`super::RuntimeEngine`] placeholder for generic platform/orchestrator types
//! when no concrete runtime engines are registered.

use std::future::Future;

use crate::ExecutionError;
use crate::ToadStoolResult;
use crate::execution::{
    ExecutionRequest, ExecutionResponse, RuntimeCapabilities, RuntimeConfig, RuntimeEngine,
};

/// Sentinel [`RuntimeEngine`] — null-object default for generic orchestrator,
/// scheduler, and platform types before real engines are discovered at runtime.
///
/// This is **not** a test mock. It is the complete implementation of the
/// "no engine registered" state. [`execute`](RuntimeEngine::execute) probes
/// host backends (WGPU, VFIO, WASM) and reports the first available backend
/// that still needs registration via `compute.engine.register`. When no
/// backend is present, it returns a diagnostic listing everything probed.
/// [`initialize`](RuntimeEngine::initialize) and [`shutdown`](RuntimeEngine::shutdown)
/// succeed as no-ops. [`get_metrics`](RuntimeEngine::get_metrics) returns
/// [`ExecutionError::NoEngineRegistered`].
#[derive(Debug, Default, Clone, Copy)]
pub struct StubRuntimeEngine;

/// Outcome of a single runtime-backend probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BackendProbe {
    name: &'static str,
    available: bool,
    detail: &'static str,
}

/// Aggregated probe of WGPU, VFIO, and WASM availability on this host.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeBackendProbe {
    wgpu: BackendProbe,
    vfio: BackendProbe,
    wasm: BackendProbe,
}

impl RuntimeBackendProbe {
    fn probe() -> Self {
        Self {
            wgpu: probe_wgpu(),
            vfio: probe_vfio(),
            wasm: probe_wasm(),
        }
    }

    /// First available backend in dispatch priority order (WGPU → VFIO → WASM).
    fn first_available(&self) -> Option<&BackendProbe> {
        [&self.wgpu, &self.vfio, &self.wasm]
            .into_iter()
            .find(|b| b.available)
    }

    fn probe_report(&self) -> String {
        format!(
            "WGPU: {} ({}) | VFIO: {} ({}) | WASM: {} ({})",
            yes_no(self.wgpu.available),
            self.wgpu.detail,
            yes_no(self.vfio.available),
            self.vfio.detail,
            yes_no(self.wasm.available),
            self.wasm.detail,
        )
    }
}

fn yes_no(v: bool) -> &'static str {
    if v { "available" } else { "unavailable" }
}

fn no_engine_err_response(reason: impl Into<String>) -> ToadStoolResult<ExecutionResponse> {
    Err(ExecutionError::no_engine_registered(reason).into())
}

fn probe_wgpu() -> BackendProbe {
    #[cfg(feature = "wgpu")]
    {
        match wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        }) {
            Ok(_instance) => BackendProbe {
                name: "WGPU",
                available: true,
                detail: "wgpu instance created (feature enabled)",
            },
            Err(e) => BackendProbe {
                name: "WGPU",
                available: false,
                detail: match e {
                    wgpu::InstanceError::BackendNotFound => {
                        "wgpu feature enabled but no GPU backend found"
                    }
                    wgpu::InstanceError::InvalidConfiguration => {
                        "wgpu feature enabled but instance config invalid"
                    }
                },
            },
        }
    }
    #[cfg(not(feature = "wgpu"))]
    {
        #[cfg(target_os = "linux")]
        let dri_present = std::path::Path::new("/dev/dri").exists();
        #[cfg(not(target_os = "linux"))]
        let dri_present = false;

        if dri_present {
            BackendProbe {
                name: "WGPU",
                available: true,
                detail: "/dev/dri present (enable `wgpu` feature for full probe)",
            }
        } else {
            BackendProbe {
                name: "WGPU",
                available: false,
                detail: "wgpu feature not enabled and /dev/dri absent",
            }
        }
    }
}

fn probe_vfio() -> BackendProbe {
    let vfio_path = std::path::Path::new("/dev/vfio/vfio");
    if vfio_path.exists() {
        BackendProbe {
            name: "VFIO",
            available: true,
            detail: "/dev/vfio/vfio present",
        }
    } else {
        BackendProbe {
            name: "VFIO",
            available: false,
            detail: "/dev/vfio/vfio not found",
        }
    }
}

fn probe_wasm() -> BackendProbe {
    #[cfg(feature = "wasm-runtime")]
    {
        BackendProbe {
            name: "WASM",
            available: true,
            detail: "wasm-runtime feature enabled",
        }
    }
    #[cfg(not(feature = "wasm-runtime"))]
    {
        BackendProbe {
            name: "WASM",
            available: false,
            detail: "wasm-runtime feature not enabled",
        }
    }
}

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
            let probe = RuntimeBackendProbe::probe();

            if let Some(backend) = probe.first_available() {
                tracing::info!(
                    backend = backend.name,
                    detail = backend.detail,
                    "StubRuntimeEngine: backend detected but no engine registered"
                );
                return no_engine_err_response(format!(
                    "{} backend available ({}) but no runtime engine registered; \
                     register via compute.engine.register capability",
                    backend.name, backend.detail
                ));
            }

            no_engine_err_response(format!(
                "no runtime engine registered and no backend available; probed: {}",
                probe.probe_report()
            ))
        }
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        let probe = RuntimeBackendProbe::probe();
        let mut platform_features = std::collections::HashMap::new();
        platform_features.insert("wgpu".to_string(), probe.wgpu.available);
        platform_features.insert("vfio".to_string(), probe.vfio.available);
        platform_features.insert("wasm".to_string(), probe.wasm.available);

        RuntimeCapabilities {
            supported_workloads: vec![],
            max_concurrent_executions: Some(0),
            supported_architectures: vec![],
            platform_features,
            version: "unregistered".to_string(),
        }
    }

    fn supports_workload(&self, _workload_type: &crate::WorkloadType) -> bool {
        false
    }

    fn get_metrics(
        &self,
    ) -> impl Future<Output = ToadStoolResult<crate::RuntimeMetrics>> + Send + '_ {
        async {
            Err(ExecutionError::no_engine_registered(
                "no runtime engine registered; register via compute.engine.register capability",
            )
            .into())
        }
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
    async fn execute_reports_probe_when_no_backend() {
        let engine = StubRuntimeEngine;
        let request = ExecutionRequest::default();
        let result = engine.execute(request).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("No runtime engine registered"),
            "unexpected error: {err}"
        );
        // When a host backend is present, error names it; otherwise lists all probes.
        assert!(
            err.contains("backend available") || err.contains("WGPU:"),
            "unexpected error: {err}"
        );
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
    fn capabilities_include_probe_features() {
        let engine = StubRuntimeEngine;
        let caps = engine.get_capabilities();
        assert!(caps.supported_workloads.is_empty());
        assert_eq!(caps.max_concurrent_executions, Some(0));
        assert!(caps.supported_architectures.is_empty());
        assert_eq!(caps.version, "unregistered");
        assert!(caps.platform_features.contains_key("wgpu"));
        assert!(caps.platform_features.contains_key("vfio"));
        assert!(caps.platform_features.contains_key("wasm"));
    }

    #[tokio::test]
    async fn get_metrics_returns_no_engine_registered() {
        let engine = StubRuntimeEngine;
        let result = engine.get_metrics().await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("No runtime engine registered"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn copy_and_debug() {
        let a = StubRuntimeEngine;
        let b = a;
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    #[test]
    fn probe_report_includes_all_backends() {
        let probe = RuntimeBackendProbe::probe();
        let report = probe.probe_report();
        assert!(report.contains("WGPU:"));
        assert!(report.contains("VFIO:"));
        assert!(report.contains("WASM:"));
    }
}
