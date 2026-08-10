// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime type selection and engine registration.
//!
//! Parses runtime hints, infers runtime from workload spec, and registers
//! available runtime engines with the orchestrator.

use std::sync::Arc;

use crate::{CliContextExt, Result};
use tracing::{debug, info};

use toadstool::{execution::RuntimeType, runtime::RuntimeOrchestrator, workload::WorkloadSpec};
use toadstool_server::RuntimeEngineDispatch;

use toadstool_runtime_native::NativeRuntimeEngine;
#[cfg(feature = "wasm")]
use toadstool_runtime_wasm::WasmRuntimeEngine;

#[cfg(feature = "gpu")]
use toadstool_runtime_gpu::UniversalGpuEngine;

/// Parse a runtime hint string (e.g. from CLI) into RuntimeType.
pub(super) fn parse_runtime_hint(hint: &str) -> Result<RuntimeType> {
    match hint.to_lowercase().as_str() {
        "native" => Ok(RuntimeType::Native),
        "python" => Ok(RuntimeType::Python),
        "wasm" | "webassembly" => Ok(RuntimeType::Wasm),
        "container" | "docker" => Ok(RuntimeType::Container),
        "gpu" => Ok(RuntimeType::Gpu),
        _ => Err(crate::CliError::Other(format!(
            "Unknown runtime type: {hint}"
        ))),
    }
}

/// Infer the appropriate runtime type from a workload specification.
pub(crate) const fn infer_runtime_type(workload: &WorkloadSpec) -> RuntimeType {
    match workload {
        WorkloadSpec::Native { .. } => RuntimeType::Native,
        WorkloadSpec::Python { .. } => RuntimeType::Python,
        WorkloadSpec::Wasm { .. } => RuntimeType::Wasm,
        WorkloadSpec::Container { .. } => RuntimeType::Container,
        WorkloadSpec::Gpu { .. } => RuntimeType::Gpu,
        WorkloadSpec::AiMl { .. } => RuntimeType::Gpu, // AI/ML uses GPU runtime with backend selector
        WorkloadSpec::Cuda { .. } => RuntimeType::Gpu, // CUDA uses GPU runtime with compat layer
    }
}

/// Register all available runtime engines with the orchestrator.
pub(super) async fn register_runtime_engines(
    orchestrator: &RuntimeOrchestrator<RuntimeEngineDispatch>,
) -> Result<()> {
    // Native runtime (always available)
    let native_engine = NativeRuntimeEngine::new();
    orchestrator
        .register_engine(
            RuntimeType::Native,
            Arc::new(RuntimeEngineDispatch::Native(native_engine)),
        )
        .context("Failed to register native runtime")?;
    info!("   ✅ Native runtime registered");

    // Python runtime removed — pyo3 FFI violates ecoBin v3.0.
    // Python workloads route to the AI/routing capability provider via IPC.
    debug!("   ℹ️  Python runtime: delegate to AI/routing service via IPC");

    // WASM runtime - Optional (has zstd C dependency)
    #[cfg(feature = "wasm")]
    {
        let wasm_config = toadstool_runtime_wasm::WasmRuntimeConfig::default();
        match WasmRuntimeEngine::new(wasm_config) {
            Ok(wasm_engine) => {
                orchestrator
                    .register_engine(
                        RuntimeType::Wasm,
                        Arc::new(RuntimeEngineDispatch::Wasm(wasm_engine)),
                    )
                    .context("Failed to register WASM runtime")?;
                info!("   ✅ WASM runtime registered");
            }
            Err(e) => {
                debug!("   ⚠️  WASM runtime not available: {}", e);
            }
        }
    }
    #[cfg(not(feature = "wasm"))]
    {
        debug!("   ⚠️  WASM runtime not enabled (pure-rust build)");
    }

    // GPU runtime (optional, feature-gated)
    #[cfg(feature = "gpu")]
    {
        match UniversalGpuEngine::new().await {
            Ok(gpu_engine) => {
                orchestrator
                    .register_engine(
                        RuntimeType::Gpu,
                        Arc::new(RuntimeEngineDispatch::Gpu(gpu_engine)),
                    )
                    .context("Failed to register GPU runtime")?;
                info!("   ✅ GPU runtime registered");
            }
            Err(e) => {
                debug!("   ⚠️  GPU runtime not available: {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_runtime_hint_native() {
        let result = parse_runtime_hint("native");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), RuntimeType::Native));
    }

    #[test]
    fn test_parse_runtime_hint_python() {
        let result = parse_runtime_hint("PYTHON");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), RuntimeType::Python));
    }

    #[test]
    fn test_parse_runtime_hint_wasm() {
        assert!(parse_runtime_hint("wasm").is_ok());
        assert!(parse_runtime_hint("webassembly").is_ok());
    }

    #[test]
    fn test_parse_runtime_hint_container() {
        assert!(parse_runtime_hint("container").is_ok());
        assert!(parse_runtime_hint("docker").is_ok());
    }

    #[test]
    fn test_parse_runtime_hint_gpu() {
        let result = parse_runtime_hint("gpu");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), RuntimeType::Gpu));
    }

    #[test]
    fn test_parse_runtime_hint_invalid() {
        let result = parse_runtime_hint("invalid_runtime");
        assert!(result.is_err());
    }

    #[test]
    fn test_infer_runtime_type_native() {
        let spec = WorkloadSpec::Native {
            executable: toadstool::workload::ExecutableSource::File {
                path: "/bin/echo".into(),
            },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        };

        let runtime = infer_runtime_type(&spec);
        assert!(matches!(runtime, RuntimeType::Native));
    }

    #[test]
    fn test_infer_runtime_type_python() {
        let spec = WorkloadSpec::Python {
            source: toadstool::workload::PythonSource::Code {
                code: "print('test')".to_string(),
            },
            python_version: None,
            requirements: vec![],
            env_vars: HashMap::new(),
        };

        let runtime = infer_runtime_type(&spec);
        assert!(matches!(runtime, RuntimeType::Python));
    }
}
