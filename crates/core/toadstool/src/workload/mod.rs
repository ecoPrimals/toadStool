// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload types and specifications

mod types;

pub mod ai_ml;
pub mod analyzer;
pub mod cuda;
#[cfg(test)]
pub mod integration_tests;
pub mod selector;

pub use ai_ml::{AiFramework, AiMlWorkload, AiOperation, ModelSize, Precision};
pub use analyzer::{
    ComputeIntensity, GpuAdvantage, MemoryRequirement, ParallelismLevel, WorkloadAnalyzer,
    WorkloadCharacteristics,
};
pub use cuda::{CudaBackend, CudaLaunchConfig, CudaSource, CudaWorkload};
pub use selector::{BackendDecision, BackendSelector, GpuDevice, GpuVendor, HardwareCapabilities};
pub use types::*;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{ToadStoolError, ToadStoolResult};

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
    pub fn validate(&self) -> ToadStoolResult<()> {
        match self {
            Self::Native { executable, .. } => {
                self.validate_executable(executable)?;
            }
            Self::Wasm { module, .. } => {
                self.validate_wasm_module(module)?;
            }
            Self::Container { image, .. } => {
                if image.is_empty() {
                    return Err(ToadStoolError::validation(
                        "Container image cannot be empty",
                    ));
                }
            }
            Self::Gpu { program, .. } => {
                self.validate_gpu_program(program)?;
            }
            Self::Python { source, .. } => {
                self.validate_python_source(source)?;
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

    /// Validate executable source
    ///
    /// Inlined for performance - called on every native workload execution
    #[inline]
    fn validate_executable(&self, executable: &ExecutableSource) -> ToadStoolResult<()> {
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
    fn validate_wasm_module(&self, module: &WasmModuleSource) -> ToadStoolResult<()> {
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
    fn validate_gpu_program(&self, program: &GpuProgramSource) -> ToadStoolResult<()> {
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
    fn validate_python_source(&self, source: &PythonSource) -> ToadStoolResult<()> {
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
}

/// Types of workloads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkloadType {
    /// Native executable
    Native,
    /// WebAssembly module
    Wasm,
    /// Container
    Container,
    /// GPU program
    Gpu,
    /// Python script
    Python,
    /// AI/ML workload
    AiMl,
    /// CUDA workload
    Cuda,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_spec_default() {
        let spec = WorkloadSpec::default();
        assert_eq!(spec.workload_type(), WorkloadType::Native);
        if let WorkloadSpec::Native {
            executable, args, ..
        } = &spec
        {
            assert!(matches!(executable, ExecutableSource::File { .. }));
            assert_eq!(args.as_ref().unwrap()[0], "Hello, World!");
        }
    }

    #[test]
    fn test_workload_type_for_all_variants() {
        let native = WorkloadSpec::Native {
            executable: ExecutableSource::Url {
                url: "https://example.com/bin".to_string(),
            },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        };
        assert_eq!(native.workload_type(), WorkloadType::Native);

        let wasm = WorkloadSpec::Wasm {
            module: WasmModuleSource::Bytes {
                data: bytes::Bytes::from(vec![0, 97, 115, 109]),
            },
            args: None,
            wasi_config: None,
            env_vars: HashMap::new(),
        };
        assert_eq!(wasm.workload_type(), WorkloadType::Wasm);

        let container = WorkloadSpec::Container {
            image: "ubuntu:22.04".to_string(),
            command: None,
            args: None,
            env_vars: HashMap::new(),
            working_dir: None,
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        };
        assert_eq!(container.workload_type(), WorkloadType::Container);

        let gpu = WorkloadSpec::Gpu {
            program: GpuProgramSource::OpenCL {
                source: "kernel void k() {}".to_string(),
            },
            kernel_name: "k".to_string(),
            work_group_size: None,
            global_work_size: (1, 1, 1),
            args: vec![],
        };
        assert_eq!(gpu.workload_type(), WorkloadType::Gpu);

        let python = WorkloadSpec::Python {
            source: PythonSource::Code {
                code: "print(1)".to_string(),
            },
            python_version: None,
            requirements: vec![],
            env_vars: HashMap::new(),
        };
        assert_eq!(python.workload_type(), WorkloadType::Python);

        let ai_ml = WorkloadSpec::AiMl {
            workload: ai_ml::AiMlWorkload::new(
                AiFramework::PyTorch,
                AiOperation::Inference,
                ModelSize::Small,
                1,
            ),
        };
        assert_eq!(ai_ml.workload_type(), WorkloadType::AiMl);

        let cuda = WorkloadSpec::Cuda {
            workload: cuda::CudaWorkload::new(
                cuda::CudaSource::CudaCpp {
                    source: "extern \"C\" __global__ void k() {}".to_string(),
                    entry_point: "k".to_string(),
                },
                cuda::CudaLaunchConfig::new((1, 1, 1), (1, 1, 1)),
            ),
        };
        assert_eq!(cuda.workload_type(), WorkloadType::Cuda);
    }

    #[test]
    fn test_workload_type_enum_variants() {
        assert_eq!(WorkloadType::Native, WorkloadType::Native);
        assert_ne!(WorkloadType::Native, WorkloadType::Wasm);
        let mut s = std::collections::HashSet::new();
        s.insert(WorkloadType::Native);
        s.insert(WorkloadType::Cuda);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_validate_container_empty_image() {
        let spec = WorkloadSpec::Container {
            image: String::new(),
            command: None,
            args: None,
            env_vars: HashMap::new(),
            working_dir: None,
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        };
        assert!(spec.validate().is_err());
        assert!(spec.validate().unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_container_ok() {
        let spec = WorkloadSpec::Container {
            image: "alpine:latest".to_string(),
            command: None,
            args: None,
            env_vars: HashMap::new(),
            working_dir: None,
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        };
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_validate_executable_empty_url() {
        let spec = WorkloadSpec::Native {
            executable: ExecutableSource::Url { url: String::new() },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_validate_executable_empty_bytes() {
        let spec = WorkloadSpec::Native {
            executable: ExecutableSource::Bytes {
                data: bytes::Bytes::new(),
            },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_validate_executable_url_ok() {
        let spec = WorkloadSpec::Native {
            executable: ExecutableSource::Url {
                url: "https://example.com/app".to_string(),
            },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        };
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_validate_wasm_empty_bytes() {
        let spec = WorkloadSpec::Wasm {
            module: WasmModuleSource::Bytes {
                data: bytes::Bytes::new(),
            },
            args: None,
            wasi_config: None,
            env_vars: HashMap::new(),
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_validate_wasm_empty_url() {
        let spec = WorkloadSpec::Wasm {
            module: WasmModuleSource::Url { url: String::new() },
            args: None,
            wasi_config: None,
            env_vars: HashMap::new(),
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_validate_wasm_bytes_ok() {
        let spec = WorkloadSpec::Wasm {
            module: WasmModuleSource::Bytes {
                data: bytes::Bytes::from(vec![0, 97, 115, 109]),
            },
            args: None,
            wasi_config: None,
            env_vars: HashMap::new(),
        };
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_validate_gpu_empty_opencl() {
        let spec = WorkloadSpec::Gpu {
            program: GpuProgramSource::OpenCL {
                source: String::new(),
            },
            kernel_name: "k".to_string(),
            work_group_size: None,
            global_work_size: (1, 1, 1),
            args: vec![],
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_validate_gpu_empty_cuda() {
        let spec = WorkloadSpec::Gpu {
            program: GpuProgramSource::Cuda {
                source: String::new(),
            },
            kernel_name: "k".to_string(),
            work_group_size: None,
            global_work_size: (1, 1, 1),
            args: vec![],
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_validate_gpu_empty_vulkan() {
        let spec = WorkloadSpec::Gpu {
            program: GpuProgramSource::Vulkan { spirv: vec![] },
            kernel_name: "k".to_string(),
            work_group_size: None,
            global_work_size: (1, 1, 1),
            args: vec![],
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_validate_gpu_ok() {
        let spec = WorkloadSpec::Gpu {
            program: GpuProgramSource::OpenCL {
                source: "kernel void k() {}".to_string(),
            },
            kernel_name: "k".to_string(),
            work_group_size: None,
            global_work_size: (1, 1, 1),
            args: vec![],
        };
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_validate_python_empty_code() {
        let spec = WorkloadSpec::Python {
            source: PythonSource::Code {
                code: String::new(),
            },
            python_version: None,
            requirements: vec![],
            env_vars: HashMap::new(),
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_validate_python_empty_module() {
        let spec = WorkloadSpec::Python {
            source: PythonSource::Module {
                name: String::new(),
            },
            python_version: None,
            requirements: vec![],
            env_vars: HashMap::new(),
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_validate_python_code_ok() {
        let spec = WorkloadSpec::Python {
            source: PythonSource::Code {
                code: "print(1)".to_string(),
            },
            python_version: None,
            requirements: vec![],
            env_vars: HashMap::new(),
        };
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_validate_ai_ml_ok() {
        let spec = WorkloadSpec::AiMl {
            workload: ai_ml::AiMlWorkload::new(
                AiFramework::Candle,
                AiOperation::Inference,
                ModelSize::Small,
                8,
            ),
        };
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_validate_cuda_ok() {
        let spec = WorkloadSpec::Cuda {
            workload: cuda::CudaWorkload::new(
                cuda::CudaSource::CudaCpp {
                    source: "void k() {}".to_string(),
                    entry_point: "k".to_string(),
                },
                cuda::CudaLaunchConfig::new((2, 1, 1), (64, 1, 1)),
            ),
        };
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_volume_mount_type_serialization() {
        let bind = VolumeMountType::Bind;
        let json = serde_json::to_string(&bind).unwrap();
        let parsed: VolumeMountType = serde_json::from_str(&json).unwrap();
        assert_eq!(bind, parsed);
    }

    #[test]
    fn test_port_protocol_serialization() {
        for proto in [PortProtocol::Tcp, PortProtocol::Udp] {
            let json = serde_json::to_string(&proto).unwrap();
            let parsed: PortProtocol = serde_json::from_str(&json).unwrap();
            assert_eq!(proto, parsed);
        }
    }

    #[test]
    fn test_workload_type_serialization() {
        for wtype in [
            WorkloadType::Native,
            WorkloadType::Wasm,
            WorkloadType::Container,
            WorkloadType::Gpu,
            WorkloadType::Python,
            WorkloadType::AiMl,
            WorkloadType::Cuda,
        ] {
            let json = serde_json::to_string(&wtype).unwrap();
            let parsed: WorkloadType = serde_json::from_str(&json).unwrap();
            assert_eq!(wtype, parsed);
        }
    }

    #[test]
    fn test_gpu_argument_variants() {
        let buf = GpuArgument::Buffer {
            data: vec![1, 2, 3],
        };
        let json_buf = serde_json::to_string(&buf).unwrap();
        let parsed: GpuArgument = serde_json::from_str(&json_buf).unwrap();
        assert!(matches!(parsed, GpuArgument::Buffer { data } if data == vec![1, 2, 3]));
    }

    #[test]
    fn test_wasi_config_serialization() {
        let config = WasiConfig {
            inherit_env: true,
            inherit_stdio: false,
            allowed_dirs: vec![PathBuf::from("/tmp")],
            preopened_dirs: vec![],
            args: vec!["a".to_string()],
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: WasiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.inherit_env, parsed.inherit_env);
        assert_eq!(config.args, parsed.args);
    }
}
