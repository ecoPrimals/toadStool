//! Comprehensive tests for Python runtime configuration

use std::path::PathBuf;
use std::time::Duration;
use toadstool_common::config_bases::TimeoutConfig;
use toadstool_runtime_python::*;

// ============================================================================
// PythonRuntimeConfig Tests
// ============================================================================

#[test]
fn test_python_runtime_config_default() {
    let config = PythonRuntimeConfig::default();

    assert_eq!(config.interpreter_path, "python3");
    assert_eq!(config.virtual_env, None);
    assert_eq!(config.max_memory_mb, 1024);
    assert_eq!(config.timeouts.request_timeout.as_secs(), 300);
    assert_eq!(config.requirements.len(), 0);
}

#[test]
fn test_python_runtime_config_interpreter_path() {
    let config = PythonRuntimeConfig::default();

    assert_eq!(config.interpreter_path, "python3");
}

#[test]
fn test_python_runtime_config_virtual_env_none() {
    let config = PythonRuntimeConfig::default();

    assert!(config.virtual_env.is_none());
}

#[test]
fn test_python_runtime_config_max_memory() {
    let config = PythonRuntimeConfig::default();

    assert_eq!(config.max_memory_mb, 1024);
}

#[test]
fn test_python_runtime_config_execution_timeout() {
    let config = PythonRuntimeConfig::default();

    assert_eq!(config.timeouts.request_timeout.as_secs(), 300);
}

#[test]
fn test_python_runtime_config_no_requirements() {
    let config = PythonRuntimeConfig::default();

    assert!(config.requirements.is_empty());
}

#[test]
fn test_python_runtime_config_custom() {
    let timeouts = TimeoutConfig {
        request_timeout: Duration::from_secs(600),
        ..Default::default()
    };

    let config = PythonRuntimeConfig {
        interpreter_path: "python3.11".to_string(),
        virtual_env: Some(PathBuf::from("/opt/venv")),
        max_memory_mb: 2048,
        timeouts,
        requirements: vec!["numpy".to_string(), "pandas".to_string()],
    };

    assert_eq!(config.interpreter_path, "python3.11");
    assert!(config.virtual_env.is_some());
    assert_eq!(config.max_memory_mb, 2048);
    assert_eq!(config.timeouts.request_timeout.as_secs(), 600);
    assert_eq!(config.requirements.len(), 2);
}

#[test]
fn test_python_runtime_config_with_virtual_env() {
    let timeouts = TimeoutConfig {
        request_timeout: Duration::from_secs(120),
        ..Default::default()
    };

    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/home/user/.venv")),
        max_memory_mb: 512,
        timeouts,
        requirements: vec![],
    };

    assert!(config.virtual_env.is_some());
    if let Some(venv) = &config.virtual_env {
        assert_eq!(venv, &PathBuf::from("/home/user/.venv"));
    }
}

#[test]
fn test_python_runtime_config_with_requirements() {
    let requirements = vec![
        "requests==2.31.0".to_string(),
        "beautifulsoup4".to_string(),
        "lxml".to_string(),
    ];

    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig::default(),
        requirements: requirements.clone(),
    };

    assert_eq!(config.requirements.len(), 3);
    assert_eq!(config.requirements[0], "requests==2.31.0");
    assert_eq!(config.requirements[1], "beautifulsoup4");
    assert_eq!(config.requirements[2], "lxml");
}

#[test]
fn test_python_runtime_config_clone() {
    let config1 = PythonRuntimeConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.interpreter_path, config2.interpreter_path);
    assert_eq!(config1.max_memory_mb, config2.max_memory_mb);
    assert_eq!(
        config1.timeouts.request_timeout,
        config2.timeouts.request_timeout
    );
}

#[test]
fn test_python_runtime_config_debug() {
    let config = PythonRuntimeConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("PythonRuntimeConfig"));
    assert!(debug_str.contains("interpreter_path"));
}

#[test]
fn test_python_runtime_config_serialization() {
    let config = PythonRuntimeConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();

    assert!(!serialized.is_empty());
    assert!(serialized.contains("python3"));
}

#[test]
#[ignore = "Duration deserialization format needs investigation"]
fn test_python_runtime_config_deserialization() {
    // Use integer seconds for Duration fields
    let json = r#"{
        "interpreter_path": "python3",
        "virtual_env": null,
        "max_memory_mb": 1024,
        "timeouts": {
            "connect_timeout": 30,
            "request_timeout": 300,
            "idle_timeout": 60
        },
        "requirements": []
    }"#;

    let config: PythonRuntimeConfig = serde_json::from_str(json).unwrap();

    assert_eq!(config.interpreter_path, "python3");
    assert_eq!(config.max_memory_mb, 1024);
}

#[test]
fn test_python_runtime_config_round_trip() {
    let timeouts = TimeoutConfig {
        request_timeout: Duration::from_secs(60),
        ..Default::default()
    };

    let original = PythonRuntimeConfig {
        interpreter_path: "python3.9".to_string(),
        virtual_env: Some(PathBuf::from("/tmp/venv")),
        max_memory_mb: 512,
        timeouts,
        requirements: vec!["pytest".to_string()],
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PythonRuntimeConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(original.interpreter_path, deserialized.interpreter_path);
    assert_eq!(original.max_memory_mb, deserialized.max_memory_mb);
    assert_eq!(
        original.timeouts.request_timeout,
        deserialized.timeouts.request_timeout
    );
    assert_eq!(original.requirements.len(), deserialized.requirements.len());
}

// ============================================================================
// Configuration Validation Tests
// ============================================================================

#[test]
fn test_python_runtime_config_zero_memory() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 0,
        timeouts: TimeoutConfig::default(),
        requirements: vec![],
    };

    assert_eq!(config.max_memory_mb, 0);
}

#[test]
fn test_python_runtime_config_large_memory() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 16384, // 16GB
        timeouts: TimeoutConfig::default(),
        requirements: vec![],
    };

    assert_eq!(config.max_memory_mb, 16384);
}

#[test]
fn test_python_runtime_config_short_timeout() {
    let timeouts = TimeoutConfig {
        request_timeout: Duration::from_secs(1),
        ..Default::default()
    };

    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts,
        requirements: vec![],
    };

    assert_eq!(config.timeouts.request_timeout.as_secs(), 1);
}

#[test]
fn test_python_runtime_config_long_timeout() {
    let timeouts = TimeoutConfig {
        request_timeout: Duration::from_secs(3600),
        ..Default::default()
    };

    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts,
        requirements: vec![],
    };

    assert_eq!(config.timeouts.request_timeout.as_secs(), 3600);
}

#[test]
fn test_python_runtime_config_custom_interpreter() {
    let custom_interpreters = vec![
        "python3.8",
        "python3.9",
        "python3.10",
        "python3.11",
        "python3.12",
        "/usr/bin/python3",
        "/opt/python/bin/python3",
    ];

    for interpreter in custom_interpreters {
        let config = PythonRuntimeConfig {
            interpreter_path: interpreter.to_string(),
            virtual_env: None,
            max_memory_mb: 1024,
            timeouts: TimeoutConfig::default(),
            requirements: vec![],
        };

        assert_eq!(config.interpreter_path, interpreter);
    }
}

#[test]
fn test_python_runtime_config_multiple_requirements() {
    let requirements = vec![
        "numpy>=1.21.0".to_string(),
        "pandas==1.5.3".to_string(),
        "scikit-learn".to_string(),
        "matplotlib".to_string(),
        "scipy".to_string(),
    ];

    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 2048,
        timeouts: TimeoutConfig::default(),
        requirements: requirements.clone(),
    };

    assert_eq!(config.requirements.len(), 5);
    assert!(config.requirements.contains(&"numpy>=1.21.0".to_string()));
    assert!(config.requirements.contains(&"pandas==1.5.3".to_string()));
}

#[test]
fn test_python_runtime_config_empty_interpreter_path() {
    let config = PythonRuntimeConfig {
        interpreter_path: "".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig::default(),
        requirements: vec![],
    };

    assert_eq!(config.interpreter_path, "");
}

#[test]
fn test_python_runtime_config_path_separators() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/home/user/projects/my_project/.venv")),
        max_memory_mb: 1024,
        timeouts: TimeoutConfig::default(),
        requirements: vec![],
    };

    if let Some(venv) = &config.virtual_env {
        assert_eq!(venv, &PathBuf::from("/home/user/projects/my_project/.venv"));
    }
}

// ============================================================================
// Configuration Comparison Tests
// ============================================================================

#[test]
fn test_python_runtime_config_default_vs_custom() {
    let default_config = PythonRuntimeConfig::default();
    let custom_timeouts = TimeoutConfig {
        request_timeout: Duration::from_secs(600),
        ..Default::default()
    };

    let custom_config = PythonRuntimeConfig {
        interpreter_path: "python3.11".to_string(),
        virtual_env: Some(PathBuf::from("/opt/venv")),
        max_memory_mb: 2048,
        timeouts: custom_timeouts,
        requirements: vec!["numpy".to_string()],
    };

    assert_ne!(
        default_config.interpreter_path,
        custom_config.interpreter_path
    );
    assert_ne!(default_config.max_memory_mb, custom_config.max_memory_mb);
    assert_ne!(
        default_config.timeouts.request_timeout,
        custom_config.timeouts.request_timeout
    );
    assert_ne!(
        default_config.requirements.len(),
        custom_config.requirements.len()
    );
}

#[test]
fn test_python_runtime_config_modify_after_creation() {
    let mut config = PythonRuntimeConfig::default();

    assert_eq!(config.max_memory_mb, 1024);

    config.max_memory_mb = 4096;

    assert_eq!(config.max_memory_mb, 4096);
}

#[test]
fn test_python_runtime_config_add_requirements() {
    let mut config = PythonRuntimeConfig::default();

    assert_eq!(config.requirements.len(), 0);

    config.requirements.push("flask".to_string());
    config.requirements.push("django".to_string());

    assert_eq!(config.requirements.len(), 2);
}
