//! Workload specification file format types.
//!
//! Defines the structure of workload files (TOML/JSON) used by `toadstool execute`.

use std::collections::HashMap;

/// Workload specification file format
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WorkloadFile {
    pub metadata: WorkloadMetadata,
    pub execution: ExecutionSpec,
    pub resources: Option<ResourceSpec>,
    pub security: Option<SecuritySpec>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WorkloadMetadata {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExecutionSpec {
    Native {
        command: String,
        args: Option<Vec<String>>,
        working_dir: Option<String>,
        env: Option<HashMap<String, String>>,
    },
    Python {
        script: Option<String>,
        file: Option<String>,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
    },
    Wasm {
        module: String,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
    },
    Container {
        image: String,
        command: Option<Vec<String>>,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
    },
    Gpu {
        kernel_name: String,
        source: String,
        input_data: Option<serde_json::Value>,
        output_data_keys: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ResourceSpec {
    pub cpu_cores: Option<f64>,
    pub memory_mb: Option<u64>,
    pub disk_mb: Option<u64>,
    pub gpu: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SecuritySpec {
    pub isolation: Option<String>,
}
