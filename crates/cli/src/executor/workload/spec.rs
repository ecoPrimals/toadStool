// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload specification file format types.
//!
//! Defines the structure of workload files (TOML/JSON) used by `toadstool execute`.

use std::collections::HashMap;

/// Workload specification file format
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WorkloadFile {
    /// Workload name, description, version
    pub metadata: WorkloadMetadata,
    /// Execution type and parameters (native, python, wasm, container, gpu)
    pub execution: ExecutionSpec,
    /// Optional resource limits
    pub resources: Option<ResourceSpec>,
    /// Optional security settings
    pub security: Option<SecuritySpec>,
}

/// Workload metadata (name, description, version)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WorkloadMetadata {
    /// Workload name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional version string
    pub version: Option<String>,
}

/// Execution specification by runtime type
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExecutionSpec {
    /// Native binary execution
    Native {
        /// Command to run
        command: String,
        /// Command-line arguments
        args: Option<Vec<String>>,
        /// Working directory
        working_dir: Option<String>,
        /// Environment variables
        env: Option<HashMap<String, String>>,
    },
    /// Python script execution
    Python {
        /// Inline script content
        script: Option<String>,
        /// Path to script file
        file: Option<String>,
        /// Script arguments
        args: Option<Vec<String>>,
        /// Environment variables
        env: Option<HashMap<String, String>>,
    },
    /// WebAssembly module execution
    Wasm {
        /// Path or URL to WASM module
        module: String,
        /// Arguments passed to WASI
        args: Option<Vec<String>>,
        /// Environment variables
        env: Option<HashMap<String, String>>,
    },
    /// OCI container execution
    Container {
        /// Container image (registry/name:tag)
        image: String,
        /// Override entrypoint
        command: Option<Vec<String>>,
        /// Container args
        args: Option<Vec<String>>,
        /// Environment variables
        env: Option<HashMap<String, String>>,
    },
    /// GPU kernel execution
    Gpu {
        /// Kernel function name
        kernel_name: String,
        /// Source file or module path
        source: String,
        /// Input data for the kernel
        input_data: Option<serde_json::Value>,
        /// Keys to extract from output
        output_data_keys: Option<Vec<String>>,
    },
}

/// Resource limits for workload execution
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ResourceSpec {
    /// CPU cores limit
    pub cpu_cores: Option<f64>,
    /// Memory limit in MB
    pub memory_mb: Option<u64>,
    /// Disk limit in MB
    pub disk_mb: Option<u64>,
    /// Whether GPU is required
    pub gpu: Option<bool>,
}

/// Security settings for workload execution
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SecuritySpec {
    /// Isolation level: "none", "basic", "standard", "enhanced", "maximum"
    pub isolation: Option<String>,
    /// Directories the workload is allowed to use as `working_dir` even under
    /// Basic/Standard isolation (acts as an allowlist beyond temp_dir).
    pub trusted_directories: Option<Vec<String>>,
}
