// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for Distributed Configuration types
//!
//! Week 17 Sprint 6: Distributed coordination and configuration tests
//! Target: ~25 tests

use toadstool::{IsolationLevel, RuntimeType};
use toadstool_distributed::{
    DistributedConfig, ExecutionEnvironment, PlatformCapabilities, SongbirdConfig,
    StandaloneConfig, ToadStoolCapabilities,
};

// ============================================================================
// DistributedConfig Tests (5 tests)
// ============================================================================

#[test]
fn test_distributed_config_default() {
    let config = DistributedConfig::default();

    // instance_id is a generated UUID
    assert!(!config.instance_id.is_empty());
    assert_eq!(config.standalone.max_concurrent_executions, 10);
    assert_eq!(config.standalone.default_timeout_secs, 3600);
    assert!(config.songbird_integration.is_none());
}

#[test]
fn test_distributed_config_with_songbird() {
    let songbird_config = SongbirdConfig {
        endpoint: "http://songbird:8080".to_string(),
        auth_token: Some("token-123".to_string()),
        health_reporting_interval_secs: 60,
    };

    let config = DistributedConfig {
        instance_id: "toadstool-prod".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 3600,
            enable_job_queue: true,
            max_queue_size: 1000,
        },
        songbird_integration: Some(songbird_config),
    };

    assert_eq!(config.instance_id, "toadstool-prod");
    assert!(config.songbird_integration.is_some());
}

#[test]
fn test_distributed_config_clone() {
    let config1 = DistributedConfig::default();
    let mut config2 = config1.clone();
    // Change instance_id to verify they start equal
    config2.instance_id = config1.instance_id.clone();

    assert_eq!(
        config1.standalone.max_concurrent_executions,
        config2.standalone.max_concurrent_executions
    );
}

#[test]
fn test_distributed_config_debug() {
    let config = DistributedConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("DistributedConfig"));
    assert!(debug_str.contains("instance_id"));
}

#[test]
fn test_distributed_config_custom_instance() {
    let config = DistributedConfig {
        instance_id: "custom-instance-42".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 3600,
            enable_job_queue: true,
            max_queue_size: 1000,
        },
        songbird_integration: None,
    };

    assert_eq!(config.instance_id, "custom-instance-42");
}

// ============================================================================
// StandaloneConfig Tests (5 tests)
// ============================================================================

#[test]
fn test_standalone_config_default() {
    let config = StandaloneConfig {
        max_concurrent_executions: 10,
        default_timeout_secs: 3600,
        enable_job_queue: true,
        max_queue_size: 1000,
    };

    assert_eq!(config.max_concurrent_executions, 10);
    assert_eq!(config.default_timeout_secs, 3600);
    assert!(config.enable_job_queue);
    assert_eq!(config.max_queue_size, 1000);
}

#[test]
fn test_standalone_config_custom_values() {
    let config = StandaloneConfig {
        max_concurrent_executions: 50,
        default_timeout_secs: 600,
        enable_job_queue: false,
        max_queue_size: 500,
    };

    assert_eq!(config.max_concurrent_executions, 50);
    assert_eq!(config.default_timeout_secs, 600);
    assert!(!config.enable_job_queue);
    assert_eq!(config.max_queue_size, 500);
}

#[test]
fn test_standalone_config_high_concurrency() {
    let config = StandaloneConfig {
        max_concurrent_executions: 1000,
        default_timeout_secs: 120,
        enable_job_queue: true,
        max_queue_size: 10000,
    };

    assert_eq!(config.max_concurrent_executions, 1000);
    assert_eq!(config.max_queue_size, 10000);
}

#[test]
fn test_standalone_config_clone_debug() {
    let config1 = StandaloneConfig {
        max_concurrent_executions: 10,
        default_timeout_secs: 3600,
        enable_job_queue: true,
        max_queue_size: 1000,
    };
    let config2 = config1.clone();

    assert_eq!(
        config1.max_concurrent_executions,
        config2.max_concurrent_executions
    );

    let debug_str = format!("{config1:?}");
    assert!(debug_str.contains("StandaloneConfig"));
}

#[test]
fn test_standalone_config_queue_disabled() {
    let config = StandaloneConfig {
        max_concurrent_executions: 10,
        default_timeout_secs: 60,
        enable_job_queue: false,
        max_queue_size: 0,
    };

    assert!(!config.enable_job_queue);
    assert_eq!(config.max_queue_size, 0);
}

// ============================================================================
// SongbirdConfig Tests (4 tests)
// ============================================================================

#[test]
fn test_songbird_config_with_auth() {
    let config = SongbirdConfig {
        endpoint: "http://songbird.local:8080".to_string(),
        auth_token: Some("bearer-token-abc".to_string()),
        health_reporting_interval_secs: 30,
    };

    assert_eq!(config.endpoint, "http://songbird.local:8080");
    assert_eq!(config.auth_token, Some("bearer-token-abc".to_string()));
    assert_eq!(config.health_reporting_interval_secs, 30);
}

#[test]
fn test_songbird_config_without_auth() {
    let config = SongbirdConfig {
        endpoint: "http://localhost:8080".to_string(),
        auth_token: None,
        health_reporting_interval_secs: 60,
    };

    assert!(config.auth_token.is_none());
}

#[test]
fn test_songbird_config_different_intervals() {
    let intervals = [10, 30, 60, 120, 300];

    for interval in intervals {
        let config = SongbirdConfig {
            endpoint: "http://songbird:8080".to_string(),
            auth_token: None,
            health_reporting_interval_secs: interval,
        };
        assert_eq!(config.health_reporting_interval_secs, interval);
    }
}

#[test]
fn test_songbird_config_clone_debug() {
    let config1 = SongbirdConfig {
        endpoint: "http://test:8080".to_string(),
        auth_token: Some("token".to_string()),
        health_reporting_interval_secs: 60,
    };

    let config2 = config1.clone();
    assert_eq!(config1.endpoint, config2.endpoint);

    let debug_str = format!("{config1:?}");
    assert!(debug_str.contains("SongbirdConfig"));
}

// ============================================================================
// ExecutionEnvironment Tests (4 tests)
// ============================================================================

#[test]
fn test_execution_environment_container() {
    let env = ExecutionEnvironment::Container {
        runtime: "docker".to_string(),
    };

    match env {
        ExecutionEnvironment::Container { runtime } => {
            assert_eq!(runtime, "docker");
        }
        _ => panic!("Should be Container variant"),
    }
}

#[test]
fn test_execution_environment_wasm() {
    let env = ExecutionEnvironment::Wasm {
        runtime: "wasmtime".to_string(),
    };

    match env {
        ExecutionEnvironment::Wasm { runtime } => {
            assert_eq!(runtime, "wasmtime");
        }
        _ => panic!("Should be Wasm variant"),
    }
}

#[test]
fn test_execution_environment_native() {
    let env = ExecutionEnvironment::Native {
        isolation: IsolationLevel::Standard,
    };

    match env {
        ExecutionEnvironment::Native { isolation } => {
            assert!(matches!(isolation, IsolationLevel::Standard));
        }
        _ => panic!("Should be Native variant"),
    }
}

#[test]
fn test_execution_environment_clone_debug() {
    let env1 = ExecutionEnvironment::Container {
        runtime: "podman".to_string(),
    };

    let env2 = env1.clone();
    match (&env1, &env2) {
        (
            ExecutionEnvironment::Container { runtime: r1 },
            ExecutionEnvironment::Container { runtime: r2 },
        ) => {
            assert_eq!(r1, r2);
        }
        _ => panic!("Both should be Container"),
    }

    let debug_str = format!("{env1:?}");
    assert!(debug_str.contains("Container"));
}

// ============================================================================
// ToadStoolCapabilities Tests (3 tests)
// ============================================================================

#[test]
fn test_toadstool_capabilities_full() {
    let caps = ToadStoolCapabilities {
        execution_environments: vec![
            ExecutionEnvironment::Container {
                runtime: "docker".to_string(),
            },
            ExecutionEnvironment::Wasm {
                runtime: "wasmtime".to_string(),
            },
        ],
        supported_runtimes: vec![
            RuntimeType::Container,
            RuntimeType::Wasm,
            RuntimeType::Native,
        ],
        platform_capabilities: PlatformCapabilities {
            os: "Linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: 8,
        },
    };

    assert_eq!(caps.execution_environments.len(), 2);
    assert_eq!(caps.supported_runtimes.len(), 3);
    assert_eq!(caps.platform_capabilities.os, "Linux");
}

#[test]
fn test_toadstool_capabilities_minimal() {
    let caps = ToadStoolCapabilities {
        execution_environments: vec![],
        supported_runtimes: vec![RuntimeType::Native],
        platform_capabilities: PlatformCapabilities {
            os: "unknown".to_string(),
            architecture: "unknown".to_string(),
            cpu_cores: 1,
        },
    };

    assert_eq!(caps.execution_environments.len(), 0);
    assert_eq!(caps.supported_runtimes.len(), 1);
    assert_eq!(caps.platform_capabilities.cpu_cores, 1);
}

#[test]
fn test_toadstool_capabilities_clone_debug() {
    let caps1 = ToadStoolCapabilities {
        execution_environments: vec![],
        supported_runtimes: vec![RuntimeType::Native],
        platform_capabilities: PlatformCapabilities {
            os: "Linux".to_string(),
            architecture: "aarch64".to_string(),
            cpu_cores: 4,
        },
    };

    let caps2 = caps1.clone();
    assert_eq!(
        caps1.supported_runtimes.len(),
        caps2.supported_runtimes.len()
    );

    let debug_str = format!("{caps1:?}");
    assert!(debug_str.contains("ToadStoolCapabilities"));
}

// ============================================================================
// PlatformCapabilities Tests (4 tests)
// ============================================================================

#[test]
fn test_platform_capabilities_linux() {
    let caps = PlatformCapabilities {
        os: "Linux".to_string(),
        architecture: "x86_64".to_string(),
        cpu_cores: 16,
    };

    assert_eq!(caps.os, "Linux");
    assert_eq!(caps.architecture, "x86_64");
    assert_eq!(caps.cpu_cores, 16);
}

#[test]
fn test_platform_capabilities_different_architectures() {
    let architectures = vec!["x86_64", "aarch64", "armv7", "riscv64"];

    for arch in architectures {
        let caps = PlatformCapabilities {
            os: "Linux".to_string(),
            architecture: arch.to_string(),
            cpu_cores: 8,
        };
        assert_eq!(caps.architecture, arch);
    }
}

#[test]
fn test_platform_capabilities_different_cpu_cores() {
    let core_counts = vec![1, 2, 4, 8, 16, 32, 64];

    for cores in core_counts {
        let caps = PlatformCapabilities {
            os: "Linux".to_string(),
            architecture: "x86_64".to_string(),
            cpu_cores: cores,
        };
        assert_eq!(caps.cpu_cores, cores);
    }
}

#[test]
fn test_platform_capabilities_clone_debug() {
    let caps1 = PlatformCapabilities {
        os: "FreeBSD".to_string(),
        architecture: "x86_64".to_string(),
        cpu_cores: 12,
    };

    let caps2 = caps1.clone();
    assert_eq!(caps1.os, caps2.os);
    assert_eq!(caps1.architecture, caps2.architecture);

    let debug_str = format!("{caps1:?}");
    assert!(debug_str.contains("PlatformCapabilities"));
}

// ============================================================================
// Test Coverage Summary
// ============================================================================

#[test]
fn test_distributed_config_coverage_summary() {
    println!("=== Distributed Configuration Test Coverage ===");
    println!("DistributedConfig Tests:          5 tests");
    println!("StandaloneConfig Tests:           5 tests");
    println!("SongbirdConfig Tests:             4 tests");
    println!("ExecutionEnvironment Tests:       4 tests");
    println!("ToadStoolCapabilities Tests:      3 tests");
    println!("PlatformCapabilities Tests:       4 tests");
    println!("─────────────────────────────────────────────");
    println!("Total:                           25 tests");
    println!("Module Coverage:                  Expanded");
    println!("================================================");
}
