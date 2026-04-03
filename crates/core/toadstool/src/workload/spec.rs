// SPDX-License-Identifier: AGPL-3.0-only
//! [`WorkloadSpec`] — workload description and validation

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{ToadStoolError, ToadStoolResult};

use super::ai_ml;
use super::cuda;
use super::types::*;
use super::validators;
use super::workload_type::WorkloadType;

/// Workload specification containing all information needed to execute a workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadSpec {
    /// Native executable
    Native {
        /// Executable source
        executable: ExecutableSource,
        /// Command line arguments
        args: Option<Vec<String>>,
        /// Working directory
        working_dir: Option<PathBuf>,
        /// Environment variables
        env_vars: HashMap<String, String>,
        /// User to run as
        user: Option<String>,
    },
    /// WebAssembly module
    Wasm {
        /// WASM module source
        module: WasmModuleSource,
        /// Arguments to pass to the module
        args: Option<Vec<String>>,
        /// WASI configuration
        wasi_config: Option<WasiConfig>,
        /// Environment variables
        env_vars: HashMap<String, String>,
    },
    /// Container workload
    Container {
        /// Container image
        image: String,
        /// Command to run
        command: Option<Vec<String>>,
        /// Arguments
        args: Option<Vec<String>>,
        /// Environment variables
        env_vars: HashMap<String, String>,
        /// Working directory
        working_dir: Option<String>,
        /// Volume mounts
        volumes: Vec<VolumeMount>,
        /// Port mappings
        ports: Vec<PortMapping>,
        /// Registry authentication
        registry_auth: Option<RegistryAuth>,
    },
    /// GPU workload
    Gpu {
        /// GPU program source
        program: GpuProgramSource,
        /// Kernel function name
        kernel_name: String,
        /// Work group size
        work_group_size: Option<(u32, u32, u32)>,
        /// Global work size
        global_work_size: (u32, u32, u32),
        /// Program arguments
        args: Vec<GpuArgument>,
    },
    /// Python workload
    Python {
        /// Python source code
        source: PythonSource,
        /// Python version requirement
        python_version: Option<String>,
        /// Required packages
        requirements: Vec<String>,
        /// Environment variables
        env_vars: HashMap<String, String>,
    },
    /// AI/ML workload (intelligent backend selection)
    AiMl {
        /// AI/ML workload specification
        workload: ai_ml::AiMlWorkload,
    },
    /// CUDA workload (compatibility layer)
    Cuda {
        /// CUDA workload specification
        workload: cuda::CudaWorkload,
    },
}

impl Default for WorkloadSpec {
    fn default() -> Self {
        Self::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("echo"),
            },
            args: Some(vec!["Hello, World!".to_string()]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        }
    }
}

impl WorkloadSpec {
    /// Get the workload type
    ///
    /// This is called frequently in runtime selection, so it's marked inline
    /// for performance optimization.
    #[inline]
    #[must_use]
    pub const fn workload_type(&self) -> WorkloadType {
        match self {
            Self::Native { .. } => WorkloadType::Native,
            Self::Wasm { .. } => WorkloadType::Wasm,
            Self::Container { .. } => WorkloadType::Container,
            Self::Gpu { .. } => WorkloadType::Gpu,
            Self::Python { .. } => WorkloadType::Python,
            Self::AiMl { .. } => WorkloadType::AiMl,
            Self::Cuda { .. } => WorkloadType::Cuda,
        }
    }

    /// Validate the workload specification
    ///
    /// # Errors
    ///
    /// Returns error if any required field is invalid for the workload variant.
    pub fn validate(&self) -> ToadStoolResult<()> {
        match self {
            Self::Native { executable, .. } => {
                validators::validate_executable(executable)?;
            }
            Self::Wasm { module, .. } => {
                validators::validate_wasm_module(module)?;
            }
            Self::Container { image, .. } => {
                if image.is_empty() {
                    return Err(ToadStoolError::validation(
                        "Container image cannot be empty",
                    ));
                }
            }
            Self::Gpu { program, .. } => {
                validators::validate_gpu_program(program)?;
            }
            Self::Python { source, .. } => {
                validators::validate_python_source(source)?;
            }
            Self::AiMl { workload } => {
                // AI/ML workloads are self-validating
                let _ = workload.estimate_total_memory_bytes(); // Verify it computes
            }
            Self::Cuda { workload } => {
                // CUDA workloads are self-validating
                let _ = workload.launch_config.total_threads(); // Verify it computes
            }
        }
        Ok(())
    }
}
