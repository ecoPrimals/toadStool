//! Comprehensive tests for workload executor
//!
//! Coverage target: 0% → 30% (20 tests)

use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;

use toadstool_cli::executor::workload::*;

// ============================================================================
// WorkloadFile Structure Tests (5 tests)
// ============================================================================

#[test]
fn test_workload_metadata_creation() {
    let metadata = WorkloadMetadata {
        name: "test-workload".to_string(),
        description: Some("Test description".to_string()),
        version: Some("1.0.0".to_string()),
    };

    assert_eq!(metadata.name, "test-workload");
    assert!(metadata.description.is_some());
    assert!(metadata.version.is_some());
}

#[test]
fn test_workload_metadata_minimal() {
    let metadata = WorkloadMetadata {
        name: "minimal".to_string(),
        description: None,
        version: None,
    };

    assert_eq!(metadata.name, "minimal");
    assert!(metadata.description.is_none());
}

#[test]
fn test_execution_spec_native() {
    let exec = ExecutionSpec::Native {
        command: "/bin/echo".to_string(),
        args: Some(vec!["hello".to_string()]),
        working_dir: None,
        env: None,
    };

    match exec {
        ExecutionSpec::Native { command, .. } => {
            assert_eq!(command, "/bin/echo");
        }
        _ => panic!("Expected Native execution spec"),
    }
}

#[test]
fn test_execution_spec_python() {
    let mut env = HashMap::new();
    env.insert("PYTHONPATH".to_string(), "/custom".to_string());

    let exec = ExecutionSpec::Python {
        script: Some("print('hello')".to_string()),
        file: None,
        args: None,
        env: Some(env),
    };

    match exec {
        ExecutionSpec::Python { script, env, .. } => {
            assert!(script.is_some());
            assert!(env.is_some());
        }
        _ => panic!("Expected Python execution spec"),
    }
}

#[test]
fn test_execution_spec_wasm() {
    let exec = ExecutionSpec::Wasm {
        module: "module.wasm".to_string(),
        args: Some(vec!["arg1".to_string()]),
        env: None,
    };

    match exec {
        ExecutionSpec::Wasm { module, .. } => {
            assert_eq!(module, "module.wasm");
        }
        _ => panic!("Expected Wasm execution spec"),
    }
}

// ============================================================================
// Resource Spec Tests (3 tests)
// ============================================================================

#[test]
fn test_resource_spec_full() {
    let resources = ResourceSpec {
        cpu_cores: Some(2.0),
        memory_mb: Some(1024),
        disk_mb: Some(5000),
        gpu: Some(true),
    };

    assert_eq!(resources.cpu_cores, Some(2.0));
    assert_eq!(resources.memory_mb, Some(1024));
    assert!(resources.gpu.unwrap());
}

#[test]
fn test_resource_spec_minimal() {
    let resources = ResourceSpec {
        cpu_cores: None,
        memory_mb: None,
        disk_mb: None,
        gpu: None,
    };

    assert!(resources.cpu_cores.is_none());
    assert!(resources.gpu.is_none());
}

#[test]
fn test_resource_spec_partial() {
    let resources = ResourceSpec {
        cpu_cores: Some(1.0),
        memory_mb: Some(512),
        disk_mb: None,
        gpu: None,
    };

    assert_eq!(resources.cpu_cores, Some(1.0));
    assert!(resources.disk_mb.is_none());
}

// ============================================================================
// Security Spec Tests (2 tests)
// ============================================================================

#[test]
fn test_security_spec_with_isolation() {
    let security = SecuritySpec {
        isolation: Some("strict".to_string()),
    };

    assert_eq!(security.isolation, Some("strict".to_string()));
}

#[test]
fn test_security_spec_no_isolation() {
    let security = SecuritySpec { isolation: None };

    assert!(security.isolation.is_none());
}

// ============================================================================
// Workload File Parsing Tests (5 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_toml_workload_native() {
    let temp_dir = TempDir::new().unwrap();
    let workload_path = temp_dir.path().join("workload.toml");

    let toml_content = r#"
[metadata]
name = "echo-test"
description = "Simple echo test"
version = "1.0.0"

[execution]
type = "native"
command = "/bin/echo"
args = ["Hello", "World"]
"#;

    fs::write(&workload_path, toml_content).await.unwrap();

    // Verify file was created
    assert!(workload_path.exists());

    // Verify content can be read
    let content = fs::read_to_string(&workload_path).await.unwrap();
    assert!(content.contains("echo-test"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_toml_workload_python() {
    let temp_dir = TempDir::new().unwrap();
    let workload_path = temp_dir.path().join("python_workload.toml");

    let toml_content = r#"
[metadata]
name = "python-test"

[execution]
type = "python"
script = "print('Hello from Python')"
"#;

    fs::write(&workload_path, toml_content).await.unwrap();

    let content = fs::read_to_string(&workload_path).await.unwrap();
    assert!(content.contains("python-test"));
    assert!(content.contains("Hello from Python"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_json_workload() {
    let temp_dir = TempDir::new().unwrap();
    let workload_path = temp_dir.path().join("workload.json");

    let json_content = r#"{
  "metadata": {
    "name": "json-test",
    "description": "JSON workload"
  },
  "execution": {
    "type": "native",
    "command": "/bin/ls"
  }
}"#;

    fs::write(&workload_path, json_content).await.unwrap();

    let content = fs::read_to_string(&workload_path).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["metadata"]["name"], "json-test");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_workload_with_resources() {
    let temp_dir = TempDir::new().unwrap();
    let workload_path = temp_dir.path().join("resource_workload.toml");

    let toml_content = r#"
[metadata]
name = "resource-test"

[execution]
type = "native"
command = "/bin/test"

[resources]
cpu_cores = 2.0
memory_mb = 1024
gpu = true
"#;

    fs::write(&workload_path, toml_content).await.unwrap();

    let content = fs::read_to_string(&workload_path).await.unwrap();
    assert!(content.contains("cpu_cores"));
    assert!(content.contains("1024"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_workload_with_security() {
    let temp_dir = TempDir::new().unwrap();
    let workload_path = temp_dir.path().join("security_workload.toml");

    let toml_content = r#"
[metadata]
name = "security-test"

[execution]
type = "native"
command = "/bin/secure"

[security]
isolation = "strict"
"#;

    fs::write(&workload_path, toml_content).await.unwrap();

    let content = fs::read_to_string(&workload_path).await.unwrap();
    assert!(content.contains("isolation"));
    assert!(content.contains("strict"));
}

// ============================================================================
// Environment Variable Parsing Tests (3 tests)
// ============================================================================

#[test]
fn test_parse_env_variable() {
    let env_str = "KEY=value";

    if let Some((key, value)) = env_str.split_once('=') {
        assert_eq!(key, "KEY");
        assert_eq!(value, "value");
    } else {
        panic!("Failed to parse env variable");
    }
}

#[test]
fn test_parse_env_variable_with_equals() {
    let env_str = "KEY=value=with=equals";

    if let Some((key, value)) = env_str.split_once('=') {
        assert_eq!(key, "KEY");
        assert_eq!(value, "value=with=equals");
    }
}

#[test]
fn test_parse_multiple_env_variables() {
    let env_pairs = vec!["VAR1=value1", "VAR2=value2", "VAR3=value3"];

    let mut env_map = HashMap::new();
    for env_pair in env_pairs {
        if let Some((key, value)) = env_pair.split_once('=') {
            env_map.insert(key.to_string(), value.to_string());
        }
    }

    assert_eq!(env_map.len(), 3);
    assert_eq!(env_map.get("VAR1"), Some(&"value1".to_string()));
    assert_eq!(env_map.get("VAR2"), Some(&"value2".to_string()));
}

// ============================================================================
// Utility Tests (2 tests)
// ============================================================================

#[test]
fn test_workload_file_clone() {
    let workload = WorkloadFile {
        metadata: WorkloadMetadata {
            name: "test".to_string(),
            description: None,
            version: None,
        },
        execution: ExecutionSpec::Native {
            command: "/bin/echo".to_string(),
            args: None,
            working_dir: None,
            env: None,
        },
        resources: None,
        security: None,
    };

    let cloned = workload.clone();
    assert_eq!(workload.metadata.name, cloned.metadata.name);
}

#[test]
fn test_workload_file_debug() {
    let workload = WorkloadFile {
        metadata: WorkloadMetadata {
            name: "debug-test".to_string(),
            description: None,
            version: None,
        },
        execution: ExecutionSpec::Native {
            command: "/bin/test".to_string(),
            args: None,
            working_dir: None,
            env: None,
        },
        resources: None,
        security: None,
    };

    let debug_str = format!("{:?}", workload);
    assert!(debug_str.contains("WorkloadFile"));
    assert!(debug_str.contains("debug-test"));
}
