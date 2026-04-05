// SPDX-License-Identifier: AGPL-3.0-or-later
//! Per-domain validation helpers for [`super::WorkloadSpec`].

use super::types::{ExecutableSource, GpuProgramSource, PythonSource, WasmModuleSource};

use crate::{ToadStoolError, ToadStoolResult};

/// Validate executable source
///
/// Inlined for performance - called on every native workload execution
#[inline]
pub(super) fn validate_executable(executable: &ExecutableSource) -> ToadStoolResult<()> {
    match executable {
        ExecutableSource::File { path } => {
            if !path.exists() {
                return Err(ToadStoolError::validation(format!(
                    "Executable file not found: {}",
                    path.display()
                )));
            }
        }
        ExecutableSource::Url { url } => {
            if url.is_empty() {
                return Err(ToadStoolError::validation("Executable URL cannot be empty"));
            }
        }
        ExecutableSource::Bytes { data } => {
            if data.is_empty() {
                return Err(ToadStoolError::validation(
                    "Executable data cannot be empty",
                ));
            }
        }
    }
    Ok(())
}

/// Validate WASM module source
///
/// Inlined for performance - called on every WASM workload execution
#[inline]
pub(super) fn validate_wasm_module(module: &WasmModuleSource) -> ToadStoolResult<()> {
    match module {
        WasmModuleSource::File { path } => {
            if !path.exists() {
                return Err(ToadStoolError::validation(format!(
                    "WASM module file not found: {}",
                    path.display()
                )));
            }
        }
        WasmModuleSource::Bytes { data } => {
            if data.is_empty() {
                return Err(ToadStoolError::validation(
                    "WASM module data cannot be empty",
                ));
            }
        }
        WasmModuleSource::Url { url } => {
            if url.is_empty() {
                return Err(ToadStoolError::validation(
                    "WASM module URL cannot be empty",
                ));
            }
        }
    }
    Ok(())
}

/// Validate GPU program source
pub(super) fn validate_gpu_program(program: &GpuProgramSource) -> ToadStoolResult<()> {
    match program {
        GpuProgramSource::OpenCL { source } => {
            if source.is_empty() {
                return Err(ToadStoolError::validation("OpenCL source cannot be empty"));
            }
        }
        GpuProgramSource::Cuda { source } => {
            if source.is_empty() {
                return Err(ToadStoolError::validation("CUDA source cannot be empty"));
            }
        }
        GpuProgramSource::Vulkan { spirv } => {
            if spirv.is_empty() {
                return Err(ToadStoolError::validation("Vulkan SPIR-V cannot be empty"));
            }
        }
    }
    Ok(())
}

/// Validate Python source
pub(super) fn validate_python_source(source: &PythonSource) -> ToadStoolResult<()> {
    match source {
        PythonSource::Code { code } => {
            if code.is_empty() {
                return Err(ToadStoolError::validation("Python code cannot be empty"));
            }
        }
        PythonSource::File { path } => {
            if !path.exists() {
                return Err(ToadStoolError::validation(format!(
                    "Python file not found: {}",
                    path.display()
                )));
            }
        }
        PythonSource::Module { name } => {
            if name.is_empty() {
                return Err(ToadStoolError::validation(
                    "Python module name cannot be empty",
                ));
            }
        }
    }
    Ok(())
}
