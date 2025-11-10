//! Biome Executor - Type Definitions

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use crate::{BiomeInfo, BiomeManifest};

/// Running biome state
#[derive(Debug, Clone)]
pub(super) struct RunningBiome {
    pub(super) info: BiomeInfo,
    pub(super) _manifest: BiomeManifest,
    pub(super) process_handles: Vec<BiomeProcess>,
    pub(super) log_files: HashMap<String, PathBuf>,
}

#[derive(Debug, Clone)]
pub(super) struct BiomeProcess {
    pub(super) name: String,
    pub(super) process_type: ProcessType,
    pub(super) execution_id: Uuid,
    pub(super) pid: Option<u32>,
    pub(super) _started_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) enum ProcessType {
    Primal(String),
    Service(String),
    _HealthCheck(String),
}

impl ProcessType {
    #[allow(dead_code)]
    pub(super) fn name(&self) -> &str {
        match self {
            ProcessType::Primal(name) => name,
            ProcessType::Service(name) => name,
            ProcessType::_HealthCheck(name) => name,
        }
    }

    pub(super) fn _type_str(&self) -> &str {
        match self {
            ProcessType::Primal(_) => "primal",
            ProcessType::Service(_) => "service",
            ProcessType::_HealthCheck(_) => "healthcheck",
        }
    }
}

impl BiomeProcess {
    pub(super) fn process_type_name(&self) -> &str {
        match &self.process_type {
            ProcessType::Primal(_) => "primal",
            ProcessType::Service(_) => "service",
            ProcessType::_HealthCheck(_) => "healthcheck",
        }
    }

    pub(super) fn _display_name(&self) -> String {
        format!("{}:{}", self.process_type.name(), self.execution_id)
    }
}

// Additional structs for improved functionality
#[derive(Debug, Clone)]
pub struct WasmModule {
    pub id: Uuid,
    pub source: String,
    pub size_bytes: usize,
    pub validated: bool,
    pub checksum: String,
    pub compiled_at: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct WasmExecutionInfo {
    pub execution_id: Uuid,
    pub module_id: Uuid,
    pub wasi_config: Option<WasiExecutionConfig>,
    pub memory_limit_mb: u64,
    pub timeout_ms: u64,
    pub started_at: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct WasiExecutionConfig {
    pub stdin: Option<String>,
    pub stdout_capture: bool,
    pub stderr_capture: bool,
    pub environment: HashMap<String, String>,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub filesystem_access: Vec<PathBuf>,
    pub network_access: bool,
}
