// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! CLI types coverage tests - calling actual production code
//!
//! These tests directly instantiate and use types from cli/src
//! to increase llvm-cov coverage

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use toadstool_cli::CliError;
use toadstool_cli::executor::{WasiExecutionConfig, WasmExecutionInfo, WasmModule};
use uuid::Uuid;

// ============================================================================
// WasmModule Tests (calls struct construction)
// ============================================================================

#[test]
fn test_wasm_module_creation() {
    let module = WasmModule {
        id: Uuid::new_v4(),
        source: "/path/to/module.wasm".to_string(),
        size_bytes: 1024,
        validated: true,
        checksum: "abc123def456".to_string(),
        compiled_at: SystemTime::now(),
    };

    assert!(!module.id.is_nil());
    assert_eq!(module.source, "/path/to/module.wasm");
    assert_eq!(module.size_bytes, 1024);
    assert!(module.validated);
    assert_eq!(module.checksum, "abc123def456");
}

#[test]
fn test_wasm_module_not_validated() {
    let module = WasmModule {
        id: Uuid::new_v4(),
        source: "untrusted.wasm".to_string(),
        size_bytes: 512,
        validated: false,
        checksum: String::new(),
        compiled_at: SystemTime::now(),
    };

    assert!(!module.validated);
    assert!(module.checksum.is_empty());
}

#[test]
fn test_wasm_module_clone() {
    let module1 = WasmModule {
        id: Uuid::new_v4(),
        source: "test.wasm".to_string(),
        size_bytes: 2048,
        validated: true,
        checksum: "hash".to_string(),
        compiled_at: SystemTime::now(),
    };

    let module2 = module1.clone();
    assert_eq!(module1.id, module2.id);
    assert_eq!(module1.source, module2.source);
    assert_eq!(module1.size_bytes, module2.size_bytes);
}

#[test]
fn test_wasm_module_debug() {
    let module = WasmModule {
        id: Uuid::new_v4(),
        source: "debug.wasm".to_string(),
        size_bytes: 1024,
        validated: true,
        checksum: "test".to_string(),
        compiled_at: SystemTime::now(),
    };

    let debug_str = format!("{module:?}");
    assert!(debug_str.contains("WasmModule"));
    assert!(debug_str.contains("debug.wasm"));
}

#[test]
fn test_wasm_module_large_size() {
    let module = WasmModule {
        id: Uuid::new_v4(),
        source: "large.wasm".to_string(),
        size_bytes: 100_000_000, // 100 MB
        validated: true,
        checksum: "largefile".to_string(),
        compiled_at: SystemTime::now(),
    };

    assert_eq!(module.size_bytes, 100_000_000);
}

// ============================================================================
// WasmExecutionInfo Tests
// ============================================================================

#[test]
fn test_wasm_execution_info_creation() {
    let info = WasmExecutionInfo {
        execution_id: Uuid::new_v4(),
        module_id: Uuid::new_v4(),
        wasi_config: None,
        memory_limit_mb: 256,
        timeout_ms: 5000,
        started_at: SystemTime::now(),
    };

    assert!(!info.execution_id.is_nil());
    assert!(!info.module_id.is_nil());
    assert!(info.wasi_config.is_none());
    assert_eq!(info.memory_limit_mb, 256);
    assert_eq!(info.timeout_ms, 5000);
}

#[test]
fn test_wasm_execution_info_with_wasi() {
    let wasi_config = WasiExecutionConfig {
        stdin: Some("input data".to_string()),
        stdout_capture: true,
        stderr_capture: true,
        environment: HashMap::new(),
        arguments: vec!["--help".to_string()],
        working_directory: Some(PathBuf::from("/tmp")),
        filesystem_access: vec![],
        network_access: false,
    };

    let info = WasmExecutionInfo {
        execution_id: Uuid::new_v4(),
        module_id: Uuid::new_v4(),
        wasi_config: Some(wasi_config),
        memory_limit_mb: 512,
        timeout_ms: 10000,
        started_at: SystemTime::now(),
    };

    assert!(info.wasi_config.is_some());
    assert_eq!(info.memory_limit_mb, 512);
}

#[test]
fn test_wasm_execution_info_clone() {
    let info1 = WasmExecutionInfo {
        execution_id: Uuid::new_v4(),
        module_id: Uuid::new_v4(),
        wasi_config: None,
        memory_limit_mb: 128,
        timeout_ms: 3000,
        started_at: SystemTime::now(),
    };

    let info2 = info1.clone();
    assert_eq!(info1.execution_id, info2.execution_id);
    assert_eq!(info1.memory_limit_mb, info2.memory_limit_mb);
}

#[test]
fn test_wasm_execution_info_debug() {
    let info = WasmExecutionInfo {
        execution_id: Uuid::new_v4(),
        module_id: Uuid::new_v4(),
        wasi_config: None,
        memory_limit_mb: 256,
        timeout_ms: 5000,
        started_at: SystemTime::now(),
    };

    let debug_str = format!("{info:?}");
    assert!(debug_str.contains("WasmExecutionInfo"));
}

// ============================================================================
// WasiExecutionConfig Tests
// ============================================================================

#[test]
fn test_wasi_config_creation() {
    let config = WasiExecutionConfig {
        stdin: Some("test input".to_string()),
        stdout_capture: true,
        stderr_capture: false,
        environment: HashMap::new(),
        arguments: vec![],
        working_directory: None,
        filesystem_access: vec![],
        network_access: false,
    };

    assert_eq!(config.stdin, Some("test input".to_string()));
    assert!(config.stdout_capture);
    assert!(!config.stderr_capture);
    assert!(!config.network_access);
}

#[test]
fn test_wasi_config_with_environment() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin".to_string());
    env.insert("HOME".to_string(), "/home/user".to_string());

    let config = WasiExecutionConfig {
        stdin: None,
        stdout_capture: true,
        stderr_capture: true,
        environment: env.clone(),
        arguments: vec!["arg1".to_string(), "arg2".to_string()],
        working_directory: Some(PathBuf::from("/workspace")),
        filesystem_access: vec![PathBuf::from("/data")],
        network_access: true,
    };

    assert_eq!(config.environment.len(), 2);
    assert_eq!(config.arguments.len(), 2);
    assert_eq!(config.filesystem_access.len(), 1);
    assert!(config.network_access);
}

#[test]
fn test_wasi_config_clone() {
    let config1 = WasiExecutionConfig {
        stdin: Some("data".to_string()),
        stdout_capture: true,
        stderr_capture: false,
        environment: HashMap::new(),
        arguments: vec!["test".to_string()],
        working_directory: None,
        filesystem_access: vec![],
        network_access: false,
    };

    let config2 = config1.clone();
    assert_eq!(config1.stdin, config2.stdin);
    assert_eq!(config1.arguments.len(), config2.arguments.len());
}

#[test]
fn test_wasi_config_debug() {
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

    let debug_str = format!("{config:?}");
    assert!(debug_str.contains("WasiExecutionConfig"));
}

#[test]
fn test_wasi_config_filesystem_access() {
    let config = WasiExecutionConfig {
        stdin: None,
        stdout_capture: false,
        stderr_capture: false,
        environment: HashMap::new(),
        arguments: vec![],
        working_directory: None,
        filesystem_access: vec![
            PathBuf::from("/data"),
            PathBuf::from("/tmp"),
            PathBuf::from("/output"),
        ],
        network_access: false,
    };

    assert_eq!(config.filesystem_access.len(), 3);
}

#[test]
fn test_wasi_config_with_working_directory() {
    let config = WasiExecutionConfig {
        stdin: None,
        stdout_capture: true,
        stderr_capture: true,
        environment: HashMap::new(),
        arguments: vec![],
        working_directory: Some(PathBuf::from("/workspace/project")),
        filesystem_access: vec![],
        network_access: false,
    };

    assert!(config.working_directory.is_some());
    assert_eq!(
        config.working_directory.unwrap(),
        PathBuf::from("/workspace/project")
    );
}

// ============================================================================
// CliError Tests (calls enum constructors and trait implementations)
// ============================================================================

#[test]
fn test_cli_error_biome_not_found() {
    let error = CliError::BiomeNotFound("test-biome".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Biome not found"));
    assert!(msg.contains("test-biome"));
}

#[test]
fn test_cli_error_biome_already_exists() {
    let error = CliError::BiomeAlreadyExists("existing-biome".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Biome already exists"));
    assert!(msg.contains("existing-biome"));
}

#[test]
fn test_cli_error_invalid_config() {
    let error = CliError::InvalidConfig("missing field".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Invalid configuration"));
    assert!(msg.contains("missing field"));
}

#[test]
fn test_cli_error_system() {
    let error = CliError::System("system failure".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("System error"));
    assert!(msg.contains("system failure"));
}

#[test]
fn test_cli_error_other() {
    let error = CliError::Other("unknown error".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Other error"));
    assert!(msg.contains("unknown error"));
}

#[test]
fn test_cli_error_from_io() {
    use std::io::{Error, ErrorKind};

    let io_error = Error::new(ErrorKind::NotFound, "file not found");
    let cli_error: CliError = io_error.into();

    let msg = format!("{cli_error}");
    assert!(msg.contains("IO error"));
}

#[test]
fn test_cli_error_from_serialization() {
    // Create a serialization error by trying to serialize invalid JSON
    let json_str = "{invalid json";
    let parse_result: Result<serde_json::Value, _> = serde_json::from_str(json_str);
    let json_error = parse_result.unwrap_err();
    let cli_error: CliError = json_error.into();

    let msg = format!("{cli_error}");
    assert!(msg.contains("Serialization error"));
}

#[test]
fn test_cli_error_debug() {
    let error = CliError::BiomeNotFound("test".to_string());
    let debug_str = format!("{error:?}");
    assert!(debug_str.contains("BiomeNotFound"));
}

#[test]
fn test_cli_error_as_source() {
    use std::error::Error;

    let io_error = std::io::Error::other("test");
    let cli_error = CliError::Io(io_error);

    // Calls Error trait implementation
    assert!(cli_error.source().is_some());
}

// ============================================================================
// Integration Tests with Multiple Types
// ============================================================================

#[test]
fn test_complete_wasm_execution_scenario() {
    // Create a module
    let module = WasmModule {
        id: Uuid::new_v4(),
        source: "calculator.wasm".to_string(),
        size_bytes: 4096,
        validated: true,
        checksum: "checksum123".to_string(),
        compiled_at: SystemTime::now(),
    };

    // Create WASI config
    let mut env = HashMap::new();
    env.insert("PRECISION".to_string(), "high".to_string());

    let wasi_config = WasiExecutionConfig {
        stdin: Some("2 + 2".to_string()),
        stdout_capture: true,
        stderr_capture: true,
        environment: env,
        arguments: vec!["--mode".to_string(), "interactive".to_string()],
        working_directory: Some(PathBuf::from("/tmp/calc")),
        filesystem_access: vec![PathBuf::from("/tmp/calc")],
        network_access: false,
    };

    // Create execution info
    let exec_info = WasmExecutionInfo {
        execution_id: Uuid::new_v4(),
        module_id: module.id,
        wasi_config: Some(wasi_config),
        memory_limit_mb: 128,
        timeout_ms: 5000,
        started_at: SystemTime::now(),
    };

    assert_eq!(exec_info.module_id, module.id);
    assert!(exec_info.wasi_config.is_some());
    assert!(module.validated);
}

#[test]
fn test_wasm_module_lifecycle() {
    let module = WasmModule {
        id: Uuid::new_v4(),
        source: "app.wasm".to_string(),
        size_bytes: 8192,
        validated: false,
        checksum: String::new(),
        compiled_at: SystemTime::now(),
    };

    // Simulate validation
    let validated_module = WasmModule {
        validated: true,
        checksum: "computed_hash".to_string(),
        ..module
    };

    assert!(validated_module.validated);
    assert!(!validated_module.checksum.is_empty());
}

// Coverage: These tests call actual production code in cli/src:
// - WasmModule struct construction and Clone/Debug traits
// - WasmExecutionInfo struct construction and Clone/Debug traits
// - WasiExecutionConfig struct construction and Clone/Debug traits
// - CliError enum variants construction
// - CliError Display trait implementation
// - CliError From<io::Error> conversion
// - CliError From<serde_json::Error> conversion
// - CliError Error trait implementation (source())
