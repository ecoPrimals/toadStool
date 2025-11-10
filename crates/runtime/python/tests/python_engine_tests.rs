//! Additional tests for Python runtime engine behavior

use toadstool::{RuntimeEngine, WorkloadType};
use toadstool_runtime_python::*;

// ============================================================================
// Engine Creation and Configuration Tests
// ============================================================================

#[test]
fn test_python_engine_default_creation() {
    let result = PythonRuntimeEngine::new();
    assert!(result.is_ok());
}

#[test]
fn test_python_engine_with_default_config() {
    let config = PythonRuntimeConfig::default();
    let result = PythonRuntimeEngine::with_config(config);
    assert!(result.is_ok());
}

#[test]
fn test_python_engine_with_minimal_config() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 128,
        execution_timeout_secs: 10,
        requirements: vec![],
    };

    let result = PythonRuntimeEngine::with_config(config);
    assert!(result.is_ok());
}

#[test]
fn test_python_engine_with_large_config() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 32768,         // 32 GB
        execution_timeout_secs: 7200, // 2 hours
        requirements: vec!["tensorflow".to_string()],
    };

    let result = PythonRuntimeEngine::with_config(config);
    assert!(result.is_ok());
}

// ============================================================================
// Workload Support Tests
// ============================================================================

#[test]
fn test_python_engine_supports_python_workload() {
    let engine = PythonRuntimeEngine::new().unwrap();
    assert!(engine.supports_workload(&WorkloadType::Python));
}

#[test]
fn test_python_engine_not_supports_wasm_workload() {
    let engine = PythonRuntimeEngine::new().unwrap();
    assert!(!engine.supports_workload(&WorkloadType::Wasm));
}

#[test]
fn test_python_engine_not_supports_container_workload() {
    let engine = PythonRuntimeEngine::new().unwrap();
    assert!(!engine.supports_workload(&WorkloadType::Container));
}

#[test]
fn test_python_engine_not_supports_native_workload() {
    let engine = PythonRuntimeEngine::new().unwrap();
    assert!(!engine.supports_workload(&WorkloadType::Native));
}

#[test]
fn test_python_engine_not_supports_gpu_workload() {
    let engine = PythonRuntimeEngine::new().unwrap();
    assert!(!engine.supports_workload(&WorkloadType::Gpu));
}

// ============================================================================
// Capabilities Tests
// ============================================================================

#[test]
fn test_python_engine_capabilities_has_version() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let caps = engine.get_capabilities();

    assert!(!caps.version.is_empty());
}

#[test]
fn test_python_engine_capabilities_max_concurrent() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let caps = engine.get_capabilities();

    assert!(caps.max_concurrent_executions.is_some());
    assert_eq!(caps.max_concurrent_executions.unwrap(), 10);
}

#[test]
fn test_python_engine_capabilities_supported_workloads_count() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let caps = engine.get_capabilities();

    assert_eq!(caps.supported_workloads.len(), 1);
}

#[test]
fn test_python_engine_capabilities_supported_architectures() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let caps = engine.get_capabilities();

    assert!(!caps.supported_architectures.is_empty());
    assert!(caps.supported_architectures.len() >= 2);
}

#[test]
fn test_python_engine_capabilities_x86_64_support() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let caps = engine.get_capabilities();

    assert!(caps.supported_architectures.contains(&"x86_64".to_string()));
}

#[test]
fn test_python_engine_capabilities_aarch64_support() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let caps = engine.get_capabilities();

    assert!(caps
        .supported_architectures
        .contains(&"aarch64".to_string()));
}

// ============================================================================
// Debug and Display Tests
// ============================================================================

#[test]
fn test_python_engine_debug_format_contains_name() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let debug_str = format!("{:?}", engine);

    assert!(debug_str.contains("PythonRuntimeEngine"));
}

#[test]
fn test_python_engine_debug_format_contains_config() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let debug_str = format!("{:?}", engine);

    assert!(debug_str.contains("config"));
}

// ============================================================================
// Default Implementation Tests
// ============================================================================

#[test]
fn test_python_engine_default_trait() {
    let engine = PythonRuntimeEngine::default();
    let debug_str = format!("{:?}", engine);

    assert!(debug_str.contains("PythonRuntimeEngine"));
}

#[test]
fn test_python_engine_default_has_capabilities() {
    let engine = PythonRuntimeEngine::default();
    let caps = engine.get_capabilities();

    assert!(!caps.version.is_empty());
}

// ============================================================================
// Multiple Engines Tests
// ============================================================================

#[test]
fn test_create_multiple_python_engines() {
    let engine1 = PythonRuntimeEngine::new();
    let engine2 = PythonRuntimeEngine::new();
    let engine3 = PythonRuntimeEngine::new();

    assert!(engine1.is_ok());
    assert!(engine2.is_ok());
    assert!(engine3.is_ok());
}

#[test]
fn test_python_engines_independent_configs() {
    let config1 = PythonRuntimeConfig {
        interpreter_path: "python3.9".to_string(),
        virtual_env: None,
        max_memory_mb: 512,
        execution_timeout_secs: 60,
        requirements: vec![],
    };

    let config2 = PythonRuntimeConfig {
        interpreter_path: "python3.11".to_string(),
        virtual_env: None,
        max_memory_mb: 2048,
        execution_timeout_secs: 300,
        requirements: vec!["numpy".to_string()],
    };

    let engine1 = PythonRuntimeEngine::with_config(config1);
    let engine2 = PythonRuntimeEngine::with_config(config2);

    assert!(engine1.is_ok());
    assert!(engine2.is_ok());
}
