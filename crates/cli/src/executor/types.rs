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
#[allow(
    dead_code,
    reason = "Inner names used by tests; HealthCheck reserved for future health-check wiring"
)]
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
}

impl BiomeProcess {
    pub(super) const fn process_type_name(&self) -> &str {
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
    fn test_process_type_type_labels_match_process_type_name() {
        let primal = BiomeProcess {
            name: "p".to_string(),
            process_type: ProcessType::Primal("x".to_string()),
            execution_id: Uuid::new_v4(),
            pid: None,
            _started_at: std::time::SystemTime::now(),
        };
        assert_eq!(primal.process_type_name(), "primal");

        let svc = BiomeProcess {
            name: "s".to_string(),
            process_type: ProcessType::Service("x".to_string()),
            execution_id: Uuid::new_v4(),
            pid: None,
            _started_at: std::time::SystemTime::now(),
        };
        assert_eq!(svc.process_type_name(), "service");

        let hc = BiomeProcess {
            name: "h".to_string(),
            process_type: ProcessType::HealthCheck("x".to_string()),
            execution_id: Uuid::new_v4(),
            pid: None,
            _started_at: std::time::SystemTime::now(),
        };
        assert_eq!(hc.process_type_name(), "healthcheck");
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

/// WASM module metadata for execution
#[derive(Debug, Clone)]
pub struct WasmModule {
    /// Module ID
    pub id: Uuid,
    /// Source path or URL
    pub source: String,
    /// Module size in bytes
    pub size_bytes: usize,
    /// Whether the module passed validation
    pub validated: bool,
    /// SHA256 or similar checksum
    pub checksum: String,
    /// When the module was compiled/loaded
    pub compiled_at: std::time::SystemTime,
}

/// Info for an in-flight WASM execution
#[derive(Debug, Clone)]
pub struct WasmExecutionInfo {
    /// Execution ID
    pub execution_id: Uuid,
    /// Module ID
    pub module_id: Uuid,
    /// Optional WASI runtime config
    pub wasi_config: Option<WasiExecutionConfig>,
    /// Memory limit in MB
    pub memory_limit_mb: u64,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// When execution started
    pub started_at: std::time::SystemTime,
}

/// WASI runtime configuration for WASM execution
#[derive(Debug, Clone)]
pub struct WasiExecutionConfig {
    /// Stdin content (if any)
    pub stdin: Option<String>,
    /// Whether to capture stdout
    pub stdout_capture: bool,
    /// Whether to capture stderr
    pub stderr_capture: bool,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Command-line arguments
    pub arguments: Vec<String>,
    /// Working directory
    pub working_directory: Option<PathBuf>,
    /// Allowed filesystem paths
    pub filesystem_access: Vec<PathBuf>,
    /// Whether network access is allowed
    pub network_access: bool,
}
