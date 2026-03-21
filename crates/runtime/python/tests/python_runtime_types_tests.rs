// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for Python runtime types

use std::path::PathBuf;
use std::time::Duration;
use toadstool_common::config_bases::TimeoutConfig;
use toadstool_runtime_python::PythonRuntimeConfig;

// ============================================================================
// PythonRuntimeConfig Tests
// ============================================================================

#[test]
fn test_python_runtime_config_default() {
    let config = PythonRuntimeConfig::default();

    assert_eq!(config.interpreter_path, "python3");
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
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/opt/venv")),
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert!(config.virtual_env.is_some());
    assert_eq!(config.virtual_env.unwrap(), PathBuf::from("/opt/venv"));
}

#[test]
fn test_python_runtime_config_with_requirements() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![
            "numpy==1.24.0".to_string(),
            "pandas==2.0.0".to_string(),
            "scikit-learn==1.2.0".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 3);
    assert!(config.requirements.contains(&"numpy==1.24.0".to_string()));
}

#[test]
fn test_python_runtime_config_high_memory() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 8192,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(3600),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.max_memory_mb, 8192);
    assert_eq!(config.timeouts.request_timeout.as_secs(), 3600);
}

#[test]
fn test_python_runtime_config_minimal() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 256,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(60),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.max_memory_mb, 256);
    assert_eq!(config.timeouts.request_timeout.as_secs(), 60);
}

#[test]
fn test_python_runtime_config_with_multiple_requirements() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/app/venv")),
        max_memory_mb: 4096,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(1800),
            ..Default::default()
        },
        requirements: vec![
            "django==4.2.0".to_string(),
            "celery==5.3.0".to_string(),
            "redis==4.5.0".to_string(),
            "psycopg2-binary==2.9.0".to_string(),
            "requests==2.31.0".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 5);
}

#[test]
fn test_python_runtime_config_clone() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![],
    };

    let cloned = config.clone();
    assert_eq!(config.interpreter_path, cloned.interpreter_path);
    assert_eq!(config.max_memory_mb, cloned.max_memory_mb);
}

#[test]
fn test_python_runtime_config_serialization() {
    let config = PythonRuntimeConfig {
        interpreter_path: "/usr/local/bin/python3.10".to_string(),
        virtual_env: Some(PathBuf::from("/home/user/venv")),
        max_memory_mb: 2048,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(900),
            ..Default::default()
        },
        requirements: vec!["flask==2.3.0".to_string()],
    };

    let json = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: PythonRuntimeConfig =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(config.interpreter_path, deserialized.interpreter_path);
    assert_eq!(config.max_memory_mb, deserialized.max_memory_mb);
}

#[test]
fn test_python_runtime_config_long_timeout() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(86400),
            ..Default::default()
        }, // 24 hours
        requirements: vec![],
    };

    assert_eq!(config.timeouts.request_timeout.as_secs(), 86400);
}

#[test]
fn test_python_runtime_config_with_ml_requirements() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/ml/venv")),
        max_memory_mb: 16384, // 16GB for ML workloads
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(7200),
            ..Default::default()
        },
        requirements: vec![
            "tensorflow==2.13.0".to_string(),
            "torch==2.0.0".to_string(),
            "transformers==4.30.0".to_string(),
            "numpy==1.24.0".to_string(),
            "scipy==1.11.0".to_string(),
        ],
    };

    assert_eq!(config.max_memory_mb, 16384);
    assert!(config.requirements.iter().any(|r| r.contains("tensorflow")));
    assert!(config.requirements.iter().any(|r| r.contains("torch")));
}

#[test]
fn test_python_runtime_config_data_science() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/data/venv")),
        max_memory_mb: 8192,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(3600),
            ..Default::default()
        },
        requirements: vec![
            "pandas==2.0.0".to_string(),
            "numpy==1.24.0".to_string(),
            "matplotlib==3.7.0".to_string(),
            "seaborn==0.12.0".to_string(),
            "jupyter==1.0.0".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 5);
}

#[test]
fn test_python_runtime_config_web_dev() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/web/venv")),
        max_memory_mb: 2048,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(600),
            ..Default::default()
        },
        requirements: vec![
            "django==4.2.0".to_string(),
            "djangorestframework==3.14.0".to_string(),
            "gunicorn==20.1.0".to_string(),
        ],
    };

    assert!(config.requirements.iter().any(|r| r.contains("django")));
}

#[test]
fn test_python_runtime_config_async_work() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/async/venv")),
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(1800),
            ..Default::default()
        },
        requirements: vec![
            "celery==5.3.0".to_string(),
            "redis==4.5.0".to_string(),
            "aiohttp==3.8.0".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 3);
}

#[test]
fn test_python_runtime_config_testing() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/test/venv")),
        max_memory_mb: 512,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![
            "pytest==7.4.0".to_string(),
            "pytest-cov==4.1.0".to_string(),
            "pytest-asyncio==0.21.0".to_string(),
        ],
    };

    assert!(config.requirements.iter().all(|r| r.contains("pytest")));
}

#[test]
fn test_python_runtime_config_minimal_memory() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: None,
        max_memory_mb: 128, // Minimal for simple scripts
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(30),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert_eq!(config.max_memory_mb, 128);
}

#[test]
fn test_python_runtime_config_cloud_sdk() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/cloud/venv")),
        max_memory_mb: 2048,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(900),
            ..Default::default()
        },
        requirements: vec![
            "boto3==1.28.0".to_string(),
            "google-cloud-storage==2.10.0".to_string(),
            "azure-storage-blob==12.17.0".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 3);
}

#[test]
fn test_python_runtime_config_scientific() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/sci/venv")),
        max_memory_mb: 8192,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(3600),
            ..Default::default()
        },
        requirements: vec![
            "scipy==1.11.0".to_string(),
            "numpy==1.24.0".to_string(),
            "sympy==1.12.0".to_string(),
            "matplotlib==3.7.0".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 4);
}

#[test]
fn test_python_runtime_config_automation() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/auto/venv")),
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(600),
            ..Default::default()
        },
        requirements: vec![
            "selenium==4.11.0".to_string(),
            "beautifulsoup4==4.12.0".to_string(),
            "requests==2.31.0".to_string(),
        ],
    };

    assert!(config.requirements.iter().any(|r| r.contains("selenium")));
}

#[test]
fn test_python_runtime_config_database() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/db/venv")),
        max_memory_mb: 2048,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(1200),
            ..Default::default()
        },
        requirements: vec![
            "psycopg2-binary==2.9.0".to_string(),
            "pymongo==4.4.0".to_string(),
            "redis==4.5.0".to_string(),
            "sqlalchemy==2.0.0".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 4);
}

#[test]
fn test_python_runtime_config_api_client() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/api/venv")),
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec![
            "requests==2.31.0".to_string(),
            "httpx==0.24.0".to_string(),
            "aiohttp==3.8.0".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 3);
}

#[test]
fn test_python_runtime_config_crypto() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/crypto/venv")),
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(600),
            ..Default::default()
        },
        requirements: vec![
            "cryptography==41.0.0".to_string(),
            "pycryptodome==3.18.0".to_string(),
        ],
    };

    assert!(
        config
            .requirements
            .iter()
            .any(|r| r.contains("cryptography"))
    );
}

#[test]
fn test_python_runtime_config_no_venv() {
    let config = PythonRuntimeConfig {
        interpreter_path: "/usr/bin/python3".to_string(),
        virtual_env: None,
        max_memory_mb: 1024,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(300),
            ..Default::default()
        },
        requirements: vec!["requests==2.31.0".to_string()],
    };

    assert!(config.virtual_env.is_none());
}

#[test]
fn test_python_runtime_config_pypy() {
    let config = PythonRuntimeConfig {
        interpreter_path: "/usr/bin/pypy3".to_string(),
        virtual_env: Some(PathBuf::from("/pypy/venv")),
        max_memory_mb: 2048,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(600),
            ..Default::default()
        },
        requirements: vec![],
    };

    assert!(config.interpreter_path.contains("pypy"));
}

#[test]
fn test_python_runtime_config_jupyter() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/jupyter/venv")),
        max_memory_mb: 4096,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(7200),
            ..Default::default()
        },
        requirements: vec![
            "jupyter==1.0.0".to_string(),
            "ipykernel==6.25.0".to_string(),
            "nbformat==5.9.0".to_string(),
        ],
    };

    assert!(config.requirements.iter().any(|r| r.contains("jupyter")));
}

#[test]
fn test_python_runtime_config_nlp() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/nlp/venv")),
        max_memory_mb: 8192,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(3600),
            ..Default::default()
        },
        requirements: vec![
            "transformers==4.30.0".to_string(),
            "spacy==3.6.0".to_string(),
            "nltk==3.8.0".to_string(),
        ],
    };

    assert_eq!(config.requirements.len(), 3);
}

#[test]
fn test_python_runtime_config_computer_vision() {
    let config = PythonRuntimeConfig {
        interpreter_path: "python3".to_string(),
        virtual_env: Some(PathBuf::from("/cv/venv")),
        max_memory_mb: 16384,
        timeouts: TimeoutConfig {
            request_timeout: Duration::from_secs(7200),
            ..Default::default()
        },
        requirements: vec![
            "opencv-python==4.8.0".to_string(),
            "pillow==10.0.0".to_string(),
            "scikit-image==0.21.0".to_string(),
        ],
    };

    assert!(config.requirements.iter().any(|r| r.contains("opencv")));
}
