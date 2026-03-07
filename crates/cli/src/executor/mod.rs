// SPDX-License-Identifier: AGPL-3.0-or-later
//! Biome Executor - Core Universal Compute Operations
//!
//! Implements the essential biome lifecycle management commands:
//! - run: Start biome in foreground
//! - up: Start biome in background (detached)
//! - down: Stop running biome
//! - ps: List all biomes
//! - logs: View biome/service logs
//!
//! ## Module Structure (Refactored by Concern)
//!
//! - `types`: Type definitions (BiomeProcess, RunningBiome, WasmModule, etc.)
//! - `display`: UI and pretty-printing
//! - `resources`: Resource cleanup and PID management
//! - `signals`: Unix signal handling
//! - `lifecycle`: Biome start/stop operations
//! - `workload`: Direct workload execution
//! - `executor_impl`: BiomeExecutor implementation (all orchestration logic)

// Submodules
mod types;

// Refactored domain modules
mod display;
mod lifecycle;
mod resources;
mod signals;

// Internal use
use types::{BiomeProcess, ProcessType, RunningBiome};

// Public re-exports
pub use types::{WasiExecutionConfig, WasmExecutionInfo, WasmModule};

// Direct workload execution module
pub mod workload;

// Core imports
use crate::{CliContextExt, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
use tokio::fs;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use toadstool::{
    ExecutionInput, ExecutionRequest, ResourceRequirements, RuntimeType, SecurityContext,
    WorkloadSpec,
};
use toadstool_config::ToadStoolConfig;
use toadstool_distributed::{DistributedConfig, DistributedCoordinator};

use crate::{
    load_biome_manifest, validate_manifest, BiomeInfo, BiomeManifest, BiomeStatus, CliContext,
    ResourceUsage, ServiceInfo, WorkloadSource,
};

/// Options for running a biome in foreground mode
#[derive(Debug, Clone)]
pub struct RunBiomeOptions {
    /// Path to biome.yaml manifest file
    pub manifest_path: PathBuf,
    /// Override biome name (default: from manifest)
    pub name: Option<String>,
    /// Environment variables to set
    pub env: Vec<String>,
    /// Enable debug mode
    pub debug: bool,
    /// CPU limit override
    pub cpu_limit: Option<f64>,
    /// Memory limit override
    pub memory_limit: Option<String>,
    /// Security level (low, medium, high, maximum)
    pub security: String,
}

/// Options for starting a biome in background (detached) mode
#[derive(Debug, Clone)]
pub struct UpBiomeOptions {
    /// Path to biome.yaml manifest file
    pub manifest_path: PathBuf,
    /// Run in detached mode (background)
    pub detach: bool,
    /// Override biome name (default: from manifest)
    pub name: Option<String>,
    /// Environment variables to set
    pub env: Vec<String>,
    /// Auto-restart on failure
    pub restart: bool,
    /// Health check interval in seconds
    pub health_interval: u64,
}

/// Biome execution engine
pub struct BiomeExecutor {
    /// Distributed coordinator for ecosystem integration
    distributed: Arc<DistributedCoordinator>,
    /// Running biomes registry
    biomes: Arc<tokio::sync::RwLock<HashMap<String, RunningBiome>>>,
    /// Configuration
    _config: ToadStoolConfig,
}

// ✅ REFACTORED: Split by logical domains (Smart Refactoring - Deep Debt!)
// Domain modules contain `impl BiomeExecutor { ... }` for their specific concerns
mod commands; // Public CLI commands (new, run, up, down, list, logs)
mod display_ops; // Display & logging (table printing, log viewing)
mod lifecycle_ops; // Internal lifecycle (start/stop biomes, primals, services)
mod wasm_ops; // WASM operations (loading, verification, execution)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_type_variants() {
        let primal = ProcessType::Primal("songbird".to_string());
        assert!(matches!(primal, ProcessType::Primal(_)));
        if let ProcessType::Primal(name) = primal {
            assert_eq!(name, "songbird");
        }

        let service = ProcessType::Service("postgres".to_string());
        assert!(matches!(service, ProcessType::Service(_)));
        if let ProcessType::Service(name) = service {
            assert_eq!(name, "postgres");
        }
    }

    #[test]
    fn test_wasm_module_creation() {
        let module = WasmModule {
            id: Uuid::new_v4(),
            source: "path/to/module.wasm".to_string(),
            size_bytes: 1024,
            validated: true,
            checksum: "abc123".to_string(),
            compiled_at: std::time::SystemTime::now(),
        };

        assert!(!module.source.is_empty());
        assert_eq!(module.source, "path/to/module.wasm");
        assert_eq!(module.size_bytes, 1024);
        assert!(module.validated);
    }

    #[test]
    fn test_wasm_module_with_different_sources() {
        let local_module = WasmModule {
            id: Uuid::new_v4(),
            source: "/app/modules/compute.wasm".to_string(),
            size_bytes: 2048,
            validated: false,
            checksum: "def456".to_string(),
            compiled_at: std::time::SystemTime::now(),
        };
        assert!(local_module.source.starts_with('/'));
        assert!(!local_module.validated);

        let url_module = WasmModule {
            id: Uuid::new_v4(),
            source: "https://example.com/module.wasm".to_string(),
            size_bytes: 4096,
            validated: true,
            checksum: "ghi789".to_string(),
            compiled_at: std::time::SystemTime::now(),
        };
        assert!(url_module.source.starts_with("https://"));
        assert_eq!(url_module.size_bytes, 4096);
    }

    #[test]
    fn test_wasm_execution_info_creation() {
        let config = WasiExecutionConfig {
            stdin: None,
            stdout_capture: false,
            stderr_capture: false,
            environment: HashMap::new(),
            arguments: vec![],
            working_directory: None,
            filesystem_access: vec![],
            network_access: false,
        };

        let info = WasmExecutionInfo {
            execution_id: Uuid::new_v4(),
            module_id: Uuid::new_v4(),
            wasi_config: Some(config),
            memory_limit_mb: 128,
            timeout_ms: 30000,
            started_at: std::time::SystemTime::now(),
        };

        assert_eq!(info.memory_limit_mb, 128);
        assert_eq!(info.timeout_ms, 30000);
        assert!(info.wasi_config.is_some());
    }

    #[test]
    fn test_wasm_execution_info_with_timeout() {
        let config = WasiExecutionConfig {
            stdin: None,
            stdout_capture: true,
            stderr_capture: true,
            environment: HashMap::new(),
            arguments: vec![],
            working_directory: None,
            filesystem_access: vec![],
            network_access: false,
        };

        let info = WasmExecutionInfo {
            execution_id: Uuid::new_v4(),
            module_id: Uuid::new_v4(),
            wasi_config: Some(config),
            memory_limit_mb: 256,
            timeout_ms: 60000,
            started_at: std::time::SystemTime::now(),
        };

        assert_eq!(info.memory_limit_mb, 256);
        assert_eq!(info.timeout_ms, 60000);
        assert!(info.wasi_config.is_some());
        if let Some(cfg) = &info.wasi_config {
            assert!(cfg.stdout_capture);
        }
    }

    #[test]
    fn test_wasi_execution_config_default() {
        let config = WasiExecutionConfig {
            stdin: None,
            stdout_capture: false,
            stderr_capture: false,
            environment: HashMap::new(),
            arguments: vec![],
            working_directory: None,
            filesystem_access: vec![],
            network_access: false,
        };

        assert!(config.stdin.is_none());
        assert!(!config.stdout_capture);
        assert!(!config.stderr_capture);
        assert!(config.environment.is_empty());
        assert!(config.arguments.is_empty());
        assert!(!config.network_access);
    }

    #[test]
    fn test_wasi_execution_config_with_environment() {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin".to_string());
        env.insert(
            "HOME".to_string(),
            std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()),
        );

        let config = WasiExecutionConfig {
            stdin: None,
            stdout_capture: true,
            stderr_capture: true,
            environment: env.clone(),
            arguments: vec!["--verbose".to_string()],
            working_directory: Some(PathBuf::from("/app")),
            filesystem_access: vec![PathBuf::from("/data")],
            network_access: true,
        };

        assert_eq!(config.environment.len(), 2);
        assert_eq!(
            config.environment.get("PATH"),
            Some(&"/usr/bin".to_string())
        );
        assert_eq!(config.arguments.len(), 1);
        assert!(config.network_access);
    }

    #[test]
    fn test_wasi_execution_config_with_arguments() {
        let config = WasiExecutionConfig {
            stdin: Some("input data".to_string()),
            stdout_capture: true,
            stderr_capture: false,
            environment: HashMap::new(),
            arguments: vec![
                "--config".to_string(),
                "app.toml".to_string(),
                "--verbose".to_string(),
            ],
            working_directory: None,
            filesystem_access: vec![],
            network_access: false,
        };

        assert_eq!(config.arguments.len(), 3);
        assert_eq!(config.arguments[0], "--config");
        assert_eq!(config.stdin, Some("input data".to_string()));
    }

    #[test]
    fn test_wasi_execution_config_filesystem_access() {
        let config = WasiExecutionConfig {
            stdin: None,
            stdout_capture: true,
            stderr_capture: true,
            environment: HashMap::new(),
            arguments: vec![],
            working_directory: Some(PathBuf::from("/workspace")),
            filesystem_access: vec![
                PathBuf::from("/data"),
                PathBuf::from("/config"),
                PathBuf::from("/logs"),
            ],
            network_access: false,
        };

        assert_eq!(config.filesystem_access.len(), 3);
        assert!(config.filesystem_access.contains(&PathBuf::from("/data")));
        assert_eq!(config.working_directory, Some(PathBuf::from("/workspace")));
    }

    #[test]
    fn test_wasi_execution_config_network_enabled() {
        let config = WasiExecutionConfig {
            stdin: None,
            stdout_capture: true,
            stderr_capture: true,
            environment: HashMap::new(),
            arguments: vec![],
            working_directory: None,
            filesystem_access: vec![],
            network_access: true,
        };

        assert!(config.network_access);
        assert!(config.stdout_capture);
    }

    #[test]
    fn test_biome_process_creation() {
        let process = BiomeProcess {
            name: "test-service".to_string(),
            process_type: ProcessType::Service("web".to_string()),
            execution_id: Uuid::new_v4(),
            pid: Some(12345),
            _started_at: std::time::SystemTime::now(),
        };

        assert_eq!(process.name, "test-service");
        assert_eq!(process.pid, Some(12345));
    }

    #[test]
    fn test_biome_process_without_pid() {
        let process = BiomeProcess {
            name: "pending-service".to_string(),
            process_type: ProcessType::Primal("songbird".to_string()),
            execution_id: Uuid::new_v4(),
            pid: None,
            _started_at: std::time::SystemTime::now(),
        };

        assert_eq!(process.name, "pending-service");
        assert!(process.pid.is_none());
    }

    #[tokio::test]
    async fn test_down_biome_nonexistent_returns_error() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let result = executor
            .down_biome("nonexistent-biome-12345", false, 30, false)
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not running"));
    }

    #[tokio::test]
    async fn test_list_biomes_empty() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let result = executor.list_biomes(false, "table", false, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_biomes_with_resources() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let result = executor.list_biomes(false, "table", true, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_biomes_json_format() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let result = executor.list_biomes(false, "json", false, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_logs_nonexistent_biome() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let result = executor
            .show_logs("nonexistent-biome", false, 10, false, None, None)
            .await;
        assert!(result.is_err());
    }
}
