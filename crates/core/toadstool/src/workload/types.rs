// SPDX-License-Identifier: AGPL-3.0-or-later
//! Supporting types for workload specifications
//!
//! Types used by multiple `WorkloadSpec` variants. Domain separation allows
//! changes to container/execution types without recompiling workload validation logic.

use bytes::Bytes;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Source of an executable.
///
/// The `Bytes` variant uses [`bytes::Bytes`] (an `Arc<[u8]>`) so that in-memory
/// binary payloads flowing from the RPC layer to the executor are never copied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableSource {
    /// File on disk
    File { path: PathBuf },
    /// URL to download from
    Url { url: String },
    /// Raw bytes (zero-copy: clone bumps refcount, not a memcpy)
    Bytes { data: Bytes },
}

/// Source of a WASM module.
///
/// The `Bytes` variant uses [`bytes::Bytes`] so that compiled WASM payloads
/// can be shared across concurrent executions without copying.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmModuleSource {
    /// File on disk
    File { path: PathBuf },
    /// Raw bytes (zero-copy: clone bumps refcount, not a memcpy)
    Bytes { data: Bytes },
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
