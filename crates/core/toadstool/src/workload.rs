//! Workload types and specifications

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
    #[must_use]
    pub fn workload_type(&self) -> WorkloadType {
        match self {
            WorkloadSpec::Native { .. } => WorkloadType::Native,
            WorkloadSpec::Wasm { .. } => WorkloadType::Wasm,
            WorkloadSpec::Container { .. } => WorkloadType::Container,
            WorkloadSpec::Gpu { .. } => WorkloadType::Gpu,
            WorkloadSpec::Python { .. } => WorkloadType::Python,
        }
    }

    /// Validate the workload specification
    pub fn validate(&self) -> ToadStoolResult<()> {
        match self {
            WorkloadSpec::Native { executable, .. } => {
                self.validate_executable(executable)?;
            }
            WorkloadSpec::Wasm { module, .. } => {
                self.validate_wasm_module(module)?;
            }
            WorkloadSpec::Container { image, .. } => {
                if image.is_empty() {
                    return Err(ToadStoolError::validation(
                        "Container image cannot be empty",
                    ));
                }
            }
            WorkloadSpec::Gpu { program, .. } => {
                self.validate_gpu_program(program)?;
            }
            WorkloadSpec::Python { source, .. } => {
                self.validate_python_source(source)?;
            }
        }
        Ok(())
    }

    /// Validate executable source
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
}

/// Source of an executable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableSource {
    /// File on disk
    File { path: PathBuf },
    /// URL to download from
    Url { url: String },
    /// Raw bytes
    Bytes { data: Vec<u8> },
}

/// Source of a WASM module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmModuleSource {
    /// File on disk
    File { path: PathBuf },
    /// Raw bytes
    Bytes { data: Vec<u8> },
    /// URL to download from
    Url { url: String },
}

/// WASI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasiConfig {
    /// Inherit environment variables
    pub inherit_env: bool,
    /// Inherit standard I/O
    pub inherit_stdio: bool,
    /// Allowed directories
    pub allowed_dirs: Vec<PathBuf>,
    /// Pre-opened directories
    pub preopened_dirs: Vec<PathBuf>,
    /// Arguments to pass to the module
    pub args: Vec<String>,
}

/// Volume mount specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Source path (host)
    pub source: PathBuf,
    /// Target path (container)
    pub target: PathBuf,
    /// Mount type
    pub mount_type: VolumeMountType,
    /// Read-only flag
    pub read_only: bool,
}

/// Types of volume mounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolumeMountType {
    /// Bind mount
    Bind,
    /// Volume mount
    Volume,
    /// Tmpfs mount
    Tmpfs,
}

/// Port mapping specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port
    pub host_port: u16,
    /// Protocol
    pub protocol: PortProtocol,
}

/// Network protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortProtocol {
    /// TCP
    Tcp,
    /// UDP
    Udp,
}

/// Registry authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryAuth {
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Server URL
    pub server_url: String,
}

/// Source of a GPU program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuProgramSource {
    /// `OpenCL` source code
    OpenCL { source: String },
    /// CUDA source code
    Cuda { source: String },
    /// Vulkan SPIR-V bytecode
    Vulkan { spirv: Vec<u8> },
}

/// GPU program argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuArgument {
    /// Buffer argument
    Buffer { data: Vec<u8> },
    /// Scalar argument
    Scalar { value: f64 },
    /// Integer argument
    Integer { value: i64 },
}

/// Source of Python code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PythonSource {
    /// Inline Python code
    Code { code: String },
    /// Python file
    File { path: PathBuf },
    /// Python module name
    Module { name: String },
}
