// SPDX-License-Identifier: AGPL-3.0-only
//! Biome Executor - Type Definitions

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
    pub(super) _started_at: std::time::SystemTime,
}

#[derive(Debug, Clone)]
#[allow(dead_code, reason = "enum used by BiomeProcess; internal use only")]
pub(super) enum ProcessType {
    Primal(String),
    Service(String),
    HealthCheck(String),
}

impl ProcessType {
    #[cfg(test)]
    pub(super) fn name(&self) -> &str {
        match self {
            ProcessType::Primal(name) => name,
            ProcessType::Service(name) => name,
            ProcessType::HealthCheck(name) => name,
        }
    }

    #[allow(dead_code, reason = "used in tests")]
    pub(super) fn type_str(&self) -> &str {
        match self {
            ProcessType::Primal(_) => "primal",
            ProcessType::Service(_) => "service",
            ProcessType::HealthCheck(_) => "healthcheck",
        }
    }
}

impl BiomeProcess {
    pub(super) fn process_type_name(&self) -> &str {
        match &self.process_type {
            ProcessType::Primal(_) => "primal",
            ProcessType::Service(_) => "service",
            ProcessType::HealthCheck(_) => "healthcheck",
        }
    }

    #[cfg(test)]
    pub(super) fn display_name(&self) -> String {
        format!("{}:{}", self.process_type.name(), self.execution_id)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_type_primal_name() {
        let pt = ProcessType::Primal("toadstool".to_string());
        assert_eq!(pt.name(), "toadstool");
    }

    #[test]
    fn test_process_type_service_name() {
        let pt = ProcessType::Service("nginx".to_string());
        assert_eq!(pt.name(), "nginx");
    }

    #[test]
    fn test_process_type_type_str() {
        assert_eq!(ProcessType::Primal("x".to_string()).type_str(), "primal");
        assert_eq!(ProcessType::Service("x".to_string()).type_str(), "service");
        assert_eq!(
            ProcessType::HealthCheck("x".to_string()).type_str(),
            "healthcheck"
        );
    }

    #[test]
    fn test_biome_process_type_name() {
        let bp = BiomeProcess {
            name: "web".to_string(),
            process_type: ProcessType::Service("web-service".to_string()),
            execution_id: Uuid::new_v4(),
            pid: Some(1234),
            _started_at: std::time::SystemTime::now(),
        };
        assert_eq!(bp.process_type_name(), "service");
    }

    #[test]
    fn test_biome_process_display_name() {
        let id = Uuid::new_v4();
        let bp = BiomeProcess {
            name: "api".to_string(),
            process_type: ProcessType::Primal("toadstool".to_string()),
            execution_id: id,
            pid: None,
            _started_at: std::time::SystemTime::now(),
        };
        let display = bp.display_name();
        assert!(display.contains("toadstool"));
        assert!(display.contains(&id.to_string()));
    }

    #[test]
    fn test_biome_process_no_pid() {
        let bp = BiomeProcess {
            name: "worker".to_string(),
            process_type: ProcessType::Service("worker".to_string()),
            execution_id: Uuid::new_v4(),
            pid: None,
            _started_at: std::time::SystemTime::now(),
        };
        assert!(bp.pid.is_none());
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
