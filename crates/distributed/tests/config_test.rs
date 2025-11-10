//! Configuration tests for distributed module
//!
//! These tests focus on configuration serialization, deserialization,
//! validation, and defaults for the distributed coordination system.

use toadstool::execution::RuntimeType;
use toadstool_distributed::core::{
    DistributedConfig, ExecutionEnvironment, PlatformCapabilities, SongbirdConfig,
    StandaloneConfig, ToadStoolCapabilities,
};

// ============================================================================
// StandaloneConfig Tests
// ============================================================================

#[test]
fn test_standalone_config_serialization() {
    let config = StandaloneConfig {
        max_concurrent_executions: 5,
        default_timeout_secs: 120,
        enable_job_queue: false,
        max_queue_size: 50,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: StandaloneConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(
        config.max_concurrent_executions,
        deserialized.max_concurrent_executions
    );
    assert_eq!(
        config.default_timeout_secs,
        deserialized.default_timeout_secs
    );
    assert_eq!(config.enable_job_queue, deserialized.enable_job_queue);
    assert_eq!(config.max_queue_size, deserialized.max_queue_size);
}

#[test]
fn test_standalone_config_debug() {
    let config = StandaloneConfig {
        max_concurrent_executions: 10,
        default_timeout_secs: 300,
        enable_job_queue: true,
        max_queue_size: 100,
    };

    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("10"));
    assert!(debug_str.contains("300"));
}

#[test]
fn test_standalone_config_validation() {
    let config = StandaloneConfig {
        max_concurrent_executions: 1,
        default_timeout_secs: 1,
        enable_job_queue: false,
        max_queue_size: 1,
    };

    assert!(config.max_concurrent_executions > 0);
    assert!(config.default_timeout_secs > 0);
    assert!(config.max_queue_size > 0);
}

#[test]
fn test_standalone_config_high_values() {
    let config = StandaloneConfig {
        max_concurrent_executions: 1000,
        default_timeout_secs: 86400, // 24 hours
        enable_job_queue: true,
        max_queue_size: 100000,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("1000"));
    assert!(json.contains("86400"));
}

// ============================================================================
// SongbirdConfig Tests
// ============================================================================

#[test]
fn test_songbird_config_serialization() {
    let config = SongbirdConfig {
        endpoint: "https://songbird.example.com".to_string(),
        auth_token: Some("secret-token".to_string()),
        health_reporting_interval_secs: 60,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: SongbirdConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.endpoint, deserialized.endpoint);
    assert_eq!(config.auth_token, deserialized.auth_token);
    assert_eq!(
        config.health_reporting_interval_secs,
        deserialized.health_reporting_interval_secs
    );
}

#[test]
fn test_songbird_config_without_auth() {
    let config = SongbirdConfig {
        endpoint: "http://localhost:8080".to_string(),
        auth_token: None,
        health_reporting_interval_secs: 30,
    };

    assert!(config.auth_token.is_none());

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: SongbirdConfig = serde_json::from_str(&json).unwrap();
    assert!(deserialized.auth_token.is_none());
}

#[test]
fn test_songbird_config_various_endpoints() {
    let endpoints = vec![
        "http://localhost:8080",
        "https://songbird.prod.example.com",
        "http://192.168.1.100:9000",
        "https://songbird:8443",
    ];

    for endpoint in endpoints {
        let config = SongbirdConfig {
            endpoint: endpoint.to_string(),
            auth_token: None,
            health_reporting_interval_secs: 30,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains(endpoint));
    }
}

#[test]
fn test_songbird_config_intervals() {
    let intervals = vec![1, 10, 30, 60, 300, 3600];

    for interval in intervals {
        let config = SongbirdConfig {
            endpoint: "http://localhost:8080".to_string(),
            auth_token: None,
            health_reporting_interval_secs: interval,
        };

        assert_eq!(config.health_reporting_interval_secs, interval);
    }
}

// ============================================================================
// DistributedConfig Tests
// ============================================================================

#[test]
fn test_distributed_config_default() {
    let config = DistributedConfig::default();

    assert!(!config.instance_id.is_empty());
    assert_eq!(config.standalone.max_concurrent_executions, 10);
    assert_eq!(config.standalone.default_timeout_secs, 3600);
    assert!(config.standalone.enable_job_queue);
    assert_eq!(config.standalone.max_queue_size, 1000);
    assert!(config.songbird_integration.is_none());
}

#[test]
fn test_distributed_config_serialization() {
    let config = DistributedConfig {
        instance_id: "test-node-1".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 5,
            default_timeout_secs: 600,
            enable_job_queue: true,
            max_queue_size: 200,
        },
        songbird_integration: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: DistributedConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.instance_id, deserialized.instance_id);
    assert_eq!(
        config.standalone.max_concurrent_executions,
        deserialized.standalone.max_concurrent_executions
    );
}

#[test]
fn test_distributed_config_with_songbird() {
    let config = DistributedConfig {
        instance_id: "node-with-songbird".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 300,
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: Some(SongbirdConfig {
            endpoint: "http://songbird:8080".to_string(),
            auth_token: Some("token-123".to_string()),
            health_reporting_interval_secs: 30,
        }),
    };

    assert!(config.songbird_integration.is_some());

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: DistributedConfig = serde_json::from_str(&json).unwrap();

    assert!(deserialized.songbird_integration.is_some());
    let songbird = deserialized.songbird_integration.unwrap();
    assert_eq!(songbird.endpoint, "http://songbird:8080");
    assert_eq!(songbird.auth_token, Some("token-123".to_string()));
}

#[test]
fn test_distributed_config_clone() {
    let config = DistributedConfig::default();
    let cloned = config.clone();

    assert_eq!(config.instance_id, cloned.instance_id);
    assert_eq!(
        config.standalone.max_concurrent_executions,
        cloned.standalone.max_concurrent_executions
    );
}

// ============================================================================
// ExecutionEnvironment Tests
// ============================================================================

#[test]
fn test_execution_environment_container_variant() {
    let env = ExecutionEnvironment::Container {
        runtime: "docker".to_string(),
    };

    let json = serde_json::to_string(&env).unwrap();
    let deserialized: ExecutionEnvironment = serde_json::from_str(&json).unwrap();

    match deserialized {
        ExecutionEnvironment::Container { runtime } => {
            assert_eq!(runtime, "docker");
        }
        _ => panic!("Expected Container variant"),
    }
}

#[test]
fn test_execution_environment_wasm_variant() {
    let env = ExecutionEnvironment::Wasm {
        runtime: "wasmtime".to_string(),
    };

    let json = serde_json::to_string(&env).unwrap();
    let deserialized: ExecutionEnvironment = serde_json::from_str(&json).unwrap();

    match deserialized {
        ExecutionEnvironment::Wasm { runtime } => {
            assert_eq!(runtime, "wasmtime");
        }
        _ => panic!("Expected Wasm variant"),
    }
}

#[test]
fn test_execution_environment_native_variant() {
    use toadstool::IsolationLevel;

    let env = ExecutionEnvironment::Native {
        isolation: IsolationLevel::Standard,
    };

    let json = serde_json::to_string(&env).unwrap();
    let deserialized: ExecutionEnvironment = serde_json::from_str(&json).unwrap();

    match deserialized {
        ExecutionEnvironment::Native { isolation } => {
            assert!(matches!(isolation, IsolationLevel::Standard));
        }
        _ => panic!("Expected Native variant"),
    }
}

#[test]
fn test_execution_environment_multiple_runtimes() {
    let runtimes = vec!["docker", "podman", "containerd", "cri-o"];

    for runtime in runtimes {
        let env = ExecutionEnvironment::Container {
            runtime: runtime.to_string(),
        };

        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains(runtime));
    }
}

#[test]
fn test_execution_environment_wasm_runtimes() {
    let runtimes = vec!["wasmtime", "wasmer", "wasm3", "wamr"];

    for runtime in runtimes {
        let env = ExecutionEnvironment::Wasm {
            runtime: runtime.to_string(),
        };

        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains(runtime));
    }
}

// ============================================================================
// PlatformCapabilities Tests
// ============================================================================

#[test]
fn test_platform_capabilities_creation() {
    let caps = PlatformCapabilities {
        os: "Linux".to_string(),
        architecture: "x86_64".to_string(),
        cpu_cores: 8,
    };

    assert_eq!(caps.os, "Linux");
    assert_eq!(caps.architecture, "x86_64");
    assert_eq!(caps.cpu_cores, 8);
}

#[test]
fn test_platform_capabilities_serialization() {
    let caps = PlatformCapabilities {
        os: "Darwin".to_string(),
        architecture: "aarch64".to_string(),
        cpu_cores: 12,
    };

    let json = serde_json::to_string(&caps).unwrap();
    let deserialized: PlatformCapabilities = serde_json::from_str(&json).unwrap();

    assert_eq!(caps.os, deserialized.os);
    assert_eq!(caps.architecture, deserialized.architecture);
    assert_eq!(caps.cpu_cores, deserialized.cpu_cores);
}

#[test]
fn test_platform_capabilities_various_platforms() {
    let platforms = vec![
        ("Linux", "x86_64", 4),
        ("Darwin", "aarch64", 8),
        ("Windows", "x86_64", 16),
        ("FreeBSD", "x86_64", 32),
    ];

    for (os, arch, cores) in platforms {
        let caps = PlatformCapabilities {
            os: os.to_string(),
            architecture: arch.to_string(),
            cpu_cores: cores,
        };

        assert_eq!(caps.os, os);
        assert_eq!(caps.architecture, arch);
        assert_eq!(caps.cpu_cores, cores);
    }
}

// ============================================================================
// ToadStoolCapabilities Tests
// ============================================================================

#[test]
fn test_toadstool_capabilities_creation() {
    let caps = ToadStoolCapabilities {
        execution_environments: vec![],
        supported_runtimes: vec![RuntimeType::Native, RuntimeType::Container],
        platform_capabilities: PlatformCapabilities {
            os: "Linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: 8,
        },
    };

    assert_eq!(caps.supported_runtimes.len(), 2);
    assert!(caps.supported_runtimes.contains(&RuntimeType::Native));
    assert!(caps.supported_runtimes.contains(&RuntimeType::Container));
}

#[test]
fn test_toadstool_capabilities_serialization() {
    let caps = ToadStoolCapabilities {
        execution_environments: vec![
            ExecutionEnvironment::Native {
                isolation: toadstool::IsolationLevel::Standard,
            },
            ExecutionEnvironment::Container {
                runtime: "docker".to_string(),
            },
        ],
        supported_runtimes: vec![RuntimeType::Native, RuntimeType::Container],
        platform_capabilities: PlatformCapabilities {
            os: "Linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: 16,
        },
    };

    let json = serde_json::to_string(&caps).unwrap();
    let deserialized: ToadStoolCapabilities = serde_json::from_str(&json).unwrap();

    assert_eq!(
        caps.execution_environments.len(),
        deserialized.execution_environments.len()
    );
    assert_eq!(
        caps.supported_runtimes.len(),
        deserialized.supported_runtimes.len()
    );
}

#[test]
fn test_toadstool_capabilities_all_runtime_types() {
    let all_runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Python,
        RuntimeType::Gpu,
    ];

    let caps = ToadStoolCapabilities {
        execution_environments: vec![],
        supported_runtimes: all_runtimes.clone(),
        platform_capabilities: PlatformCapabilities {
            os: "Linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: 32,
        },
    };

    assert_eq!(caps.supported_runtimes.len(), all_runtimes.len());

    for runtime in all_runtimes {
        assert!(caps.supported_runtimes.contains(&runtime));
    }
}

#[tokio::test]
async fn test_toadstool_capabilities_detect_current() {
    let result = ToadStoolCapabilities::detect_current().await;
    assert!(result.is_ok());

    let caps = result.unwrap();
    assert!(!caps.execution_environments.is_empty());
    assert!(!caps.supported_runtimes.is_empty());
    assert!(caps.platform_capabilities.cpu_cores > 0);
    assert!(!caps.platform_capabilities.os.is_empty());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_config_roundtrip_json() {
    let config = DistributedConfig {
        instance_id: "integration-test".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 15,
            default_timeout_secs: 900,
            enable_job_queue: true,
            max_queue_size: 500,
        },
        songbird_integration: Some(SongbirdConfig {
            endpoint: "https://songbird.internal:8443".to_string(),
            auth_token: Some("bearer-token-xyz".to_string()),
            health_reporting_interval_secs: 45,
        }),
    };

    // JSON roundtrip
    let json = serde_json::to_string_pretty(&config).unwrap();
    let deserialized: DistributedConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.instance_id, deserialized.instance_id);
    assert_eq!(
        config.standalone.max_concurrent_executions,
        deserialized.standalone.max_concurrent_executions
    );

    let orig_songbird = config.songbird_integration.unwrap();
    let deser_songbird = deserialized.songbird_integration.unwrap();
    assert_eq!(orig_songbird.endpoint, deser_songbird.endpoint);
}

#[test]
fn test_full_config_clone() {
    let config = DistributedConfig {
        instance_id: "clone-test".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 20,
            default_timeout_secs: 1200,
            enable_job_queue: false,
            max_queue_size: 250,
        },
        songbird_integration: None,
    };

    let cloned = config.clone();

    assert_eq!(config.instance_id, cloned.instance_id);
    assert_eq!(
        config.standalone.enable_job_queue,
        cloned.standalone.enable_job_queue
    );
}

#[test]
fn test_config_edge_cases() {
    // Test with very long instance ID
    let long_id = "a".repeat(256);
    let config = DistributedConfig {
        instance_id: long_id.clone(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 1,
            default_timeout_secs: 1,
            enable_job_queue: false,
            max_queue_size: 1,
        },
        songbird_integration: None,
    };

    assert_eq!(config.instance_id.len(), 256);

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: DistributedConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.instance_id, long_id);
}
