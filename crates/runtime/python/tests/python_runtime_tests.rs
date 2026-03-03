// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Python runtime engine

use std::path::PathBuf;
use std::time::Duration;
use toadstool::{RuntimeEngine, WorkloadType};
use toadstool_common::config_bases::TimeoutConfig;
use toadstool_runtime_python::*;

// ============================================================================
// PythonRuntimeConfig Tests
// ============================================================================

#[test]
fn test_python_runtime_config_default() {
    let config = PythonRuntimeConfig::default();

    assert_eq!(config.interpreter_path, "python3");
    assert!(config.virtual_env.is_none());
    assert_eq!(config.max_memory_mb, 1024);
    assert_eq!(config.timeouts.request_timeout.as_secs(), 300);
    assert!(config.requirements.is_empty());
}

#[test]
fn test_python_runtime_config_custom_interpreter() {
    let config = PythonRuntimeConfig {
        interpreter_path: "/usr/bin/python3.11".to_string(),
        virtual_env: None,
        max_memory_mb: 2048,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(600),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.interpreter_path, "/usr/bin/python3.11");
    assert_eq!(config.max_memory_mb, 2048);
}

#[test]
fn test_python_runtime_config_with_venv() {
    let venv_path = PathBuf::from("/opt/venvs/myenv");
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(venv_path.clone()),
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert!(config.virtual_env.is_some());
    assert_eq!(config.virtual_env.unwrap(), venv_path);
}

#[test]
fn test_python_runtime_config_with_requirements() {
    let requirements = vec![
        "numpy==1.24.0".to_string(),
        "pandas>=2.0.0".to_string(),
        "scikit-learn".to_string(),
    ];

    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: requirements.clone(),
    };

    assert_eq!(config.requirements.len(), 3);
    assert_eq!(config.requirements[0], "numpy==1.24.0");
}

#[test]
fn test_python_runtime_config_large_memory() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 16384, // 16 GB
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(3600),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.max_memory_mb, 16384);
    assert_eq!(config.timeouts.request_timeout.as_secs(), 3600);
}

#[test]
fn test_python_runtime_config_minimal_resources() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 128, // Minimal
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(30),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.max_memory_mb, 128);
    assert_eq!(config.timeouts.request_timeout.as_secs(), 30);
}

#[test]
fn test_python_runtime_config_serialization() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3.10".to_string(),
        virtual_env: Some(PathBuf::from("/venv")),
        max_memory_mb: 2048,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(600),
            ..Default::default()
        },
        requirements: vec!["requests".to_string()],
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: PythonRuntimeConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.interpreter_path, "python3.10");
    assert_eq!(deserialized.max_memory_mb, 2048);
}

#[test]
fn test_python_runtime_config_clone() {
    let config = PythonRuntimeConfig::default();
    let cloned = config.clone();

    assert_eq!(config.interpreter_path, cloned.interpreter_path);
    assert_eq!(config.max_memory_mb, cloned.max_memory_mb);
}

// ============================================================================
// PythonRuntimeEngine Tests
// ============================================================================

#[test]
fn test_python_runtime_engine_creation() {
    let engine = PythonRuntimeEngine::new();
    assert!(engine.is_ok());
}

#[test]
fn test_python_runtime_engine_with_custom_config() {
    let config = PythonRuntimeConfig {
        interpreter_path: "/usr/local/bin/python3".to_string(),
        virtual_env: None,
        max_memory_mb: 4096,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(1200),
            ..Default::default()
        },
        requirements: vec!["numpy".to_string()],
    };

    let engine = PythonRuntimeEngine::with_config(config);
    assert!(engine.is_ok());
}

#[test]
fn test_python_runtime_engine_get_capabilities() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let capabilities = engine.get_capabilities();

    assert!(capabilities
        .supported_workloads
        .contains(&WorkloadType::Python));
    assert_eq!(capabilities.max_concurrent_executions, Some(10));
    assert!(capabilities
        .supported_architectures
        .contains(&"x86_64".to_string()));
}

#[test]
fn test_python_runtime_engine_capabilities_architectures() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let capabilities = engine.get_capabilities();

    assert!(capabilities.supported_architectures.len() >= 2);
    assert!(capabilities
        .supported_architectures
        .contains(&"x86_64".to_string()));
    assert!(capabilities
        .supported_architectures
        .contains(&"aarch64".to_string()));
}

#[test]
fn test_python_runtime_engine_capabilities_version() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let capabilities = engine.get_capabilities();

    assert!(!capabilities.version.is_empty());
    assert_eq!(capabilities.version, "1.0.0");
}

// Runtime type is returned in ExecutionResponse, not as a separate method

#[test]
fn test_python_runtime_engine_debug_format() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let debug_str = format!("{:?}", engine);

    assert!(debug_str.contains("PythonRuntimeEngine"));
}

// ============================================================================
// PythonRuntimeConfig Edge Cases
// ============================================================================

#[test]
fn test_python_runtime_config_empty_interpreter() {
    let config = PythonRuntimeConfig {
        interpreter_path: String::new(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert!(config.interpreter_path.is_empty());
}

#[test]
fn test_python_runtime_config_many_requirements() {
    let requirements: Vec<String> = (0..100).map(|i| format!("package{}", i)).collect();

    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: requirements.clone(),
    };

    assert_eq!(config.requirements.len(), 100);
}

#[test]
fn test_python_runtime_config_zero_timeout() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(0),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.timeouts.request_timeout.as_secs(), 0);
}

#[test]
fn test_python_runtime_config_max_memory_zero() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 0,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.max_memory_mb, 0);
}

#[test]
fn test_python_runtime_config_venv_absolute_path() {
    let venv_path = PathBuf::from("/home/user/.venvs/project");
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(venv_path.clone()),
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.virtual_env.unwrap(), venv_path);
}

#[test]
fn test_python_runtime_config_venv_relative_path() {
    let venv_path = PathBuf::from("./venv");
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(venv_path.clone()),
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.virtual_env.unwrap(), venv_path);
}

// ============================================================================
// Multiple Engine Instances
// ============================================================================

#[test]
fn test_multiple_python_engines() {
    let engine1 = PythonRuntimeEngine::new();
    let engine2 = PythonRuntimeEngine::new();

    assert!(engine1.is_ok());
    assert!(engine2.is_ok());
}

#[test]
fn test_python_engines_with_different_configs() {
    let config1 = PythonRuntimeConfig {
        interpreter_path: "python3.9".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![],
    };

    let config2 = PythonRuntimeConfig {
        interpreter_path: "python3.11".to_string(),
        virtual_env: None,
        max_memory_mb: 2048,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(600),
            ..Default::default()
        },
        requirements: vec![],
    };

    let engine1 = PythonRuntimeEngine::with_config(config1);
    let engine2 = PythonRuntimeEngine::with_config(config2);

    assert!(engine1.is_ok());
    assert!(engine2.is_ok());
}

// ============================================================================
// Configuration Scenarios
// ============================================================================

#[test]
fn test_python_runtime_config_data_science() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/opt/data-science-venv")),
        max_memory_mb: 8192, // 8 GB for data processing
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(1800),
            ..Default::default()
        }, // 30 minutes
        requirements: vec![
            "numpy".to_string(),
            "pandas".to_string(),
            "scikit-learn".to_string(),
            "matplotlib".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 4);
    assert_eq!(config.max_memory_mb, 8192);
}

#[test]
fn test_python_runtime_config_web_scraping() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 512,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(120),
            ..Default::default()
        },
        requirements: vec![
            "requests".to_string(),
            "beautifulsoup4".to_string(),
            "selenium".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 3);
    assert_eq!(config.timeouts.request_timeout.as_secs(), 120);
}

#[test]
fn test_python_runtime_config_ml_inference() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/ml/inference-env")),
        max_memory_mb: 4096,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(60),
            ..Default::default()
        },
        requirements: vec![
            "torch".to_string(),
            "tensorflow".to_string(),
            "transformers".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 3);
    assert_eq!(config.max_memory_mb, 4096);
}

#[test]
fn test_python_runtime_config_simple_script() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 256,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(30),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert!(config.requirements.is_empty());
    assert_eq!(config.max_memory_mb, 256);
}

// ============================================================================
// Capability Validation
// ============================================================================

#[test]
fn test_python_capabilities_workload_type() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let capabilities = engine.get_capabilities();

    assert_eq!(capabilities.supported_workloads.len(), 1);
    assert!(capabilities
        .supported_workloads
        .contains(&WorkloadType::Python));
}

#[test]
fn test_python_capabilities_concurrent_limit() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let capabilities = engine.get_capabilities();

    assert!(capabilities.max_concurrent_executions.is_some());
    let limit = capabilities.max_concurrent_executions.unwrap();
    assert_eq!(limit, 10);
}

#[test]
fn test_python_capabilities_platform_features() {
    let engine = PythonRuntimeEngine::new().unwrap();
    let capabilities = engine.get_capabilities();

    // Should have empty platform features for basic implementation
    assert_eq!(capabilities.platform_features.len(), 0);
}
