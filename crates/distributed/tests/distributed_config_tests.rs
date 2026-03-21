// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for distributed config types

use toadstool::{IsolationLevel, RuntimeType};
use toadstool_distributed::core::*;

// ============================================================================
// StandaloneConfig Tests
// ============================================================================

#[test]
fn test_standalone_config_default_values() {
    let config = StandaloneConfig {
        max_concurrent_executions: 10,
        default_timeout_secs: 3600,
        enable_job_queue: true,
        max_queue_size: 1000,
    };

    assert_eq!(config.max_concurrent_executions, 10);
    assert_eq!(config.default_timeout_secs, 3600);
}

#[test]
fn test_standalone_config_high_capacity() {
    let config = StandaloneConfig {
        max_concurrent_executions: 100,
        default_timeout_secs: 7200,
        enable_job_queue: true,
        max_queue_size: 10000,
    };

    assert_eq!(config.max_concurrent_executions, 100);
    assert_eq!(config.max_queue_size, 10000);
}

#[test]
fn test_standalone_config_no_queue() {
    let config = StandaloneConfig {
        max_concurrent_executions: 5,
        default_timeout_secs: 1800,
        enable_job_queue: false,
        max_queue_size: 0,
    };

    assert!(!config.enable_job_queue);
    assert_eq!(config.max_queue_size, 0);
}

#[test]
fn test_standalone_config_minimal() {
    let config = StandaloneConfig {
        max_concurrent_executions: 1,
        default_timeout_secs: 60,
        enable_job_queue: false,
        max_queue_size: 0,
    };

    assert_eq!(config.max_concurrent_executions, 1);
}

// ============================================================================
// SongbirdConfig Tests
// ============================================================================

#[test]
fn test_songbird_config_with_auth() {
    let config = SongbirdConfig {
        endpoint: "http://songbird:8080".to_string(),
        auth_token: Some("token123".to_string()),
        health_reporting_interval_secs: 60,
    };

    assert_eq!(config.endpoint, "http://songbird:8080");
    assert!(config.auth_token.is_some());
}

#[test]
fn test_songbird_config_no_auth() {
    let config = SongbirdConfig {
        endpoint: "http://localhost:8080".to_string(),
        auth_token: None,
        health_reporting_interval_secs: 30,
    };

    assert!(config.auth_token.is_none());
}

#[test]
fn test_songbird_config_custom_interval() {
    let config = SongbirdConfig {
        endpoint: "https://prod-songbird:443".to_string(),
        auth_token: Some("prod-token".to_string()),
        health_reporting_interval_secs: 120,
    };

    assert_eq!(config.health_reporting_interval_secs, 120);
}

#[test]
fn test_songbird_config_https() {
    let config = SongbirdConfig {
        endpoint: "https://secure-songbird:8443".to_string(),
        auth_token: Some("secure-token".to_string()),
        health_reporting_interval_secs: 90,
    };

    assert!(config.endpoint.starts_with("https://"));
}

// ============================================================================
// ExecutionEnvironment Tests
// ============================================================================

#[test]
fn test_execution_environment_container() {
    let env = ExecutionEnvironment::Container {
        runtime: "docker".to_string(),
    };

    if let ExecutionEnvironment::Container { runtime } = env {
        assert_eq!(runtime, "docker");
    } else {
        panic!("Expected Container variant");
    }
}

#[test]
fn test_execution_environment_wasm() {
    let env = ExecutionEnvironment::Wasm {
        runtime: "wasmtime".to_string(),
    };

    if let ExecutionEnvironment::Wasm { runtime } = env {
        assert_eq!(runtime, "wasmtime");
    } else {
        panic!("Expected Wasm variant");
    }
}

#[test]
fn test_execution_environment_native() {
    let env = ExecutionEnvironment::Native {
        isolation: IsolationLevel::Standard,
    };

    if let ExecutionEnvironment::Native { isolation } = env {
        assert!(matches!(isolation, IsolationLevel::Standard));
    } else {
        panic!("Expected Native variant");
    }
}

#[test]
fn test_execution_environment_clone() {
    let env = ExecutionEnvironment::Container {
        runtime: "podman".to_string(),
    };

    let cloned = env.clone();

    if let ExecutionEnvironment::Container { runtime } = cloned {
        assert_eq!(runtime, "podman");
    } else {
        panic!("Expected Container variant");
    }
}

// ============================================================================
// PlatformCapabilities Tests
// ============================================================================

#[test]
fn test_platform_capabilities_linux() {
    let caps = PlatformCapabilities {
        os: "linux".to_string(),
        architecture: "x86_64".to_string(),
        cpu_cores: 8,
    };

    assert_eq!(caps.os, "linux");
    assert_eq!(caps.architecture, "x86_64");
    assert_eq!(caps.cpu_cores, 8);
}

#[test]
fn test_platform_capabilities_macos() {
    let caps = PlatformCapabilities {
        os: "macos".to_string(),
        architecture: "aarch64".to_string(),
        cpu_cores: 10,
    };

    assert_eq!(caps.os, "macos");
    assert_eq!(caps.architecture, "aarch64");
}

#[test]
fn test_platform_capabilities_minimal() {
    let caps = PlatformCapabilities {
        os: "linux".to_string(),
        architecture: "x86_64".to_string(),
        cpu_cores: 1,
    };

    assert_eq!(caps.cpu_cores, 1);
}

#[test]
fn test_platform_capabilities_high_core_count() {
    let caps = PlatformCapabilities {
        os: "linux".to_string(),
        architecture: "x86_64".to_string(),
        cpu_cores: 128,
    };

    assert_eq!(caps.cpu_cores, 128);
}

// ============================================================================
// ToadStoolCapabilities Tests
// ============================================================================

#[test]
fn test_toadstool_capabilities_with_all_environments() {
    let caps = ToadStoolCapabilities {
        execution_environments: vec![
            ExecutionEnvironment::Native {
                isolation: IsolationLevel::Standard,
            },
            ExecutionEnvironment::Container {
                runtime: "docker".to_string(),
            },
            ExecutionEnvironment::Wasm {
                runtime: "wasmtime".to_string(),
            },
        ],
        supported_runtimes: vec![
            RuntimeType::Native,
            RuntimeType::Container,
            RuntimeType::Wasm,
        ],
        platform_capabilities: PlatformCapabilities {
            os: "linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: 16,
        },
    };

    assert_eq!(caps.execution_environments.len(), 3);
    assert_eq!(caps.supported_runtimes.len(), 3);
}

#[test]
fn test_toadstool_capabilities_minimal() {
    let caps = ToadStoolCapabilities {
        execution_environments: vec![ExecutionEnvironment::Native {
            isolation: IsolationLevel::Standard,
        }],
        supported_runtimes: vec![RuntimeType::Native],
        platform_capabilities: PlatformCapabilities {
            os: "linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: 2,
        },
    };

    assert_eq!(caps.execution_environments.len(), 1);
    assert_eq!(caps.supported_runtimes.len(), 1);
}

#[test]
fn test_toadstool_capabilities_clone() {
    let caps = ToadStoolCapabilities {
        execution_environments: vec![],
        supported_runtimes: vec![],
        platform_capabilities: PlatformCapabilities {
            os: "linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: 4,
        },
    };

    let cloned = caps.clone();
    assert_eq!(
        caps.platform_capabilities.cpu_cores,
        cloned.platform_capabilities.cpu_cores
    );
}

// ============================================================================
// DistributedConfig Tests
// ============================================================================

#[test]
fn test_distributed_config_standalone_only() {
    let config = DistributedConfig {
        instance_id: "toadstool-1".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 3600,
            enable_job_queue: true,
            max_queue_size: 1000,
        },
        songbird_integration: None,
    };

    assert_eq!(config.instance_id, "toadstool-1");
    assert!(config.songbird_integration.is_none());
}

#[test]
fn test_distributed_config_with_songbird() {
    let config = DistributedConfig {
        instance_id: "toadstool-2".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 20,
            default_timeout_secs: 3600,
            enable_job_queue: true,
            max_queue_size: 2000,
        },
        songbird_integration: Some(SongbirdConfig {
            endpoint: "http://songbird:8080".to_string(),
            auth_token: Some("token".to_string()),
            health_reporting_interval_secs: 60,
        }),
    };

    assert!(config.songbird_integration.is_some());
}

#[test]
fn test_distributed_config_default() {
    let config = DistributedConfig::default();

    assert!(!config.instance_id.is_empty());
    assert_eq!(config.standalone.max_concurrent_executions, 10);
    assert!(config.songbird_integration.is_none());
}

#[test]
fn test_distributed_config_clone() {
    let config = DistributedConfig {
        instance_id: "test".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 5,
            default_timeout_secs: 1800,
            enable_job_queue: false,
            max_queue_size: 0,
        },
        songbird_integration: None,
    };

    let cloned = config.clone();
    assert_eq!(config.instance_id, cloned.instance_id);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_all_execution_environments() {
    let envs = [
        ExecutionEnvironment::Native {
            isolation: IsolationLevel::Standard,
        },
        ExecutionEnvironment::Container {
            runtime: "docker".to_string(),
        },
        ExecutionEnvironment::Wasm {
            runtime: "wasmtime".to_string(),
        },
    ];

    assert_eq!(envs.len(), 3);
}

#[test]
fn test_config_hierarchy() {
    let distributed_config = DistributedConfig {
        instance_id: "hierarchy-test".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 15,
            default_timeout_secs: 3600,
            enable_job_queue: true,
            max_queue_size: 1500,
        },
        songbird_integration: Some(SongbirdConfig {
            endpoint: "http://songbird:8080".to_string(),
            auth_token: None,
            health_reporting_interval_secs: 45,
        }),
    };

    // Verify config hierarchy
    assert!(!distributed_config.instance_id.is_empty());
    assert!(distributed_config.standalone.enable_job_queue);
    assert!(distributed_config.songbird_integration.is_some());
}

#[test]
fn test_capabilities_completeness() {
    let caps = ToadStoolCapabilities {
        execution_environments: vec![
            ExecutionEnvironment::Native {
                isolation: IsolationLevel::Standard,
            },
            ExecutionEnvironment::Container {
                runtime: "docker".to_string(),
            },
            ExecutionEnvironment::Container {
                runtime: "podman".to_string(),
            },
            ExecutionEnvironment::Wasm {
                runtime: "wasmtime".to_string(),
            },
            ExecutionEnvironment::Wasm {
                runtime: "wasmer".to_string(),
            },
        ],
        supported_runtimes: vec![
            RuntimeType::Native,
            RuntimeType::Container,
            RuntimeType::Wasm,
        ],
        platform_capabilities: PlatformCapabilities {
            os: "linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: 32,
        },
    };

    assert_eq!(caps.execution_environments.len(), 5);
    assert_eq!(caps.supported_runtimes.len(), 3);
}
