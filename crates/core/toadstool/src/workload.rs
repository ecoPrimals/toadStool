//! Workload specification and configuration
//!
//! This module defines how workloads are specified, configured, and prepared for execution
//! across different runtime types.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::execution::WorkloadType;

/// Universal workload specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadSpec {
    /// Container-based workload
    Container {
        /// Container image reference
        image: String,
        /// Command to execute
        command: Option<Vec<String>>,
        /// Command arguments
        args: Option<Vec<String>>,
        /// Working directory
        working_dir: Option<String>,
        /// User to run as
        user: Option<String>,
        /// Volume mounts
        volumes: Vec<VolumeMount>,
        /// Port mappings
        ports: Vec<PortMapping>,
        /// Registry authentication
        registry_auth: Option<RegistryAuth>,
    },

    /// WebAssembly workload
    Wasm {
        /// WASM module source
        module_source: WasmModuleSource,
        /// WASI configuration
        wasi_config: Option<WasiConfig>,
        /// Host function imports
        host_functions: Vec<String>,
        /// Memory limits
        memory_limit: Option<u64>,
    },

    /// Native executable workload
    Native {
        /// Executable path or source
        executable: ExecutableSource,
        /// Command arguments
        args: Option<Vec<String>>,
        /// Working directory
        working_dir: Option<PathBuf>,
        /// Environment variables
        env_vars: HashMap<String, String>,
        /// User to run as
        user: Option<String>,
    },

    /// GPU compute workload
    Gpu {
        /// Compute kernel source
        kernel_source: GpuKernelSource,
        /// Kernel language/framework
        framework: GpuFramework,
        /// Device requirements
        device_requirements: GpuDeviceRequirements,
        /// Compute parameters
        compute_params: HashMap<String, serde_json::Value>,
    },

    /// Script workload (interpreted)
    Script {
        /// Script source code
        source: ScriptSource,
        /// Interpreter to use
        interpreter: String,
        /// Script arguments
        args: Option<Vec<String>>,
        /// Required packages/dependencies
        dependencies: Vec<String>,
    },
}

impl WorkloadSpec {
    /// Get the workload type
    pub fn workload_type(&self) -> WorkloadType {
        match self {
            Self::Container { .. } => WorkloadType::Container,
            Self::Wasm { .. } => WorkloadType::Wasm,
            Self::Native { .. } => WorkloadType::Native,
            Self::Gpu { .. } => WorkloadType::Gpu,
            Self::Script { interpreter, .. } => WorkloadType::Script {
                interpreter: interpreter.clone(),
            },
        }
    }

    /// Validate the workload specification
    pub fn validate(&self) -> crate::error::ToadStoolResult<()> {
        match self {
            Self::Container { image, .. } => {
                if image.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "Container image cannot be empty",
                    ));
                }
            }
            Self::Wasm { module_source, .. } => {
                module_source.validate()?;
            }
            Self::Native { executable, .. } => {
                executable.validate()?;
            }
            Self::Gpu { kernel_source, .. } => {
                kernel_source.validate()?;
            }
            Self::Script {
                source,
                interpreter,
                ..
            } => {
                if interpreter.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "Script interpreter cannot be empty",
                    ));
                }
                source.validate()?;
            }
        }
        Ok(())
    }
}

/// Volume mount specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Host path or volume name
    pub source: String,
    /// Container path
    pub target: String,
    /// Mount type (bind, volume, tmpfs)
    pub mount_type: VolumeMountType,
    /// Read-only flag
    pub read_only: bool,
}

/// Volume mount types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolumeMountType {
    /// Bind mount from host
    Bind,
    /// Named volume
    Volume,
    /// Temporary filesystem
    Tmpfs,
}

/// Port mapping specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Host port
    pub host_port: u16,
    /// Container port
    pub container_port: u16,
    /// Protocol (TCP/UDP)
    pub protocol: String,
}

/// Registry authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryAuth {
    /// Registry server
    pub server: String,
    /// Username
    pub username: String,
    /// Password or token
    pub password: String,
}

/// WASM module source specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmModuleSource {
    /// Module from file path
    File { path: PathBuf },
    /// Module from URL
    Url { url: String },
    /// Module from raw bytes
    Bytes { data: Vec<u8> },
    /// Module from registry
    Registry {
        registry: String,
        name: String,
        version: String,
    },
}

impl WasmModuleSource {
    fn validate(&self) -> crate::error::ToadStoolResult<()> {
        match self {
            Self::File { path } => {
                if path.as_os_str().is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "WASM module file path cannot be empty",
                    ));
                }
            }
            Self::Url { url } => {
                if url.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "WASM module URL cannot be empty",
                    ));
                }
            }
            Self::Bytes { data } => {
                if data.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "WASM module bytes cannot be empty",
                    ));
                }
            }
            Self::Registry { name, .. } => {
                if name.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "WASM module name cannot be empty",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// WASI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasiConfig {
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Command line arguments
    pub args: Vec<String>,
    /// Stdin content
    pub stdin: Option<Vec<u8>>,
    /// Directory mappings
    pub dir_mappings: Vec<DirMapping>,
}

/// Directory mapping for WASI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirMapping {
    /// Guest path
    pub guest_path: String,
    /// Host path
    pub host_path: PathBuf,
    /// Read-only flag
    pub read_only: bool,
}

/// Executable source specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableSource {
    /// Executable from file path
    File { path: PathBuf },
    /// Executable from URL
    Url { url: String },
    /// Executable from raw bytes
    Bytes { data: Vec<u8> },
}

impl ExecutableSource {
    fn validate(&self) -> crate::error::ToadStoolResult<()> {
        match self {
            Self::File { path } => {
                if path.as_os_str().is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "Executable file path cannot be empty",
                    ));
                }
            }
            Self::Url { url } => {
                if url.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "Executable URL cannot be empty",
                    ));
                }
            }
            Self::Bytes { data } => {
                if data.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "Executable bytes cannot be empty",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// GPU kernel source specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuKernelSource {
    /// Kernel from file
    File { path: PathBuf },
    /// Kernel from source code
    Source { code: String },
    /// Precompiled kernel
    Binary { data: Vec<u8> },
}

impl GpuKernelSource {
    fn validate(&self) -> crate::error::ToadStoolResult<()> {
        match self {
            Self::File { path } => {
                if path.as_os_str().is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "GPU kernel file path cannot be empty",
                    ));
                }
            }
            Self::Source { code } => {
                if code.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "GPU kernel source code cannot be empty",
                    ));
                }
            }
            Self::Binary { data } => {
                if data.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "GPU kernel binary cannot be empty",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// GPU computing frameworks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GpuFramework {
    /// CUDA framework
    Cuda,
    /// OpenCL framework
    OpenCl,
    /// Vulkan compute
    Vulkan,
    /// ROCm/HIP
    Rocm,
    /// Custom framework
    Custom(String),
}

/// GPU device requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDeviceRequirements {
    /// Minimum compute capability
    pub min_compute_capability: Option<String>,
    /// Minimum memory in MB
    pub min_memory_mb: Option<u64>,
    /// Required device count
    pub device_count: Option<u32>,
    /// Specific device IDs to use
    pub device_ids: Option<Vec<u32>>,
}

/// Script source specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptSource {
    /// Script from file
    File { path: PathBuf },
    /// Script from inline source
    Inline { code: String },
    /// Script from URL
    Url { url: String },
}

impl ScriptSource {
    fn validate(&self) -> crate::error::ToadStoolResult<()> {
        match self {
            Self::File { path } => {
                if path.as_os_str().is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "Script file path cannot be empty",
                    ));
                }
            }
            Self::Inline { code } => {
                if code.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "Script source code cannot be empty",
                    ));
                }
            }
            Self::Url { url } => {
                if url.is_empty() {
                    return Err(crate::error::ToadStoolError::validation(
                        "Script URL cannot be empty",
                    ));
                }
            }
        }
        Ok(())
    }
}
