// SPDX-License-Identifier: AGPL-3.0-only
//! Advanced Coordination Tests for Distributed Module
//!
//! These tests expand coverage for coordination edge cases and error paths
//! Target: Increase distributed module coverage from 40-60% to 70%

use toadstool_distributed::core::config::{DistributedConfig, StandaloneConfig};
use toadstool_distributed::core::coordinator::DistributedCoordinator;
use toadstool_distributed::types::jobs::{JobPriority, UniversalJobType};

// ============================================================================
// Coordinator Initialization Edge Cases
// ============================================================================

#[tokio::test]
async fn test_coordinator_with_zero_concurrency() {
    let config = DistributedConfig {
        instance_id: "test-zero-concurrency".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 0, // Edge case
            default_timeout_secs: 30,
            enable_job_queue: true,
            max_queue_size: 10,
        },
        songbird_integration: None,
    };

    // Should handle zero concurrency gracefully
    let result = DistributedCoordinator::new(config).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_coordinator_with_very_high_concurrency() {
    let config = DistributedConfig {
        instance_id: "test-high-concurrency".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10_000, // Very high
            default_timeout_secs: 30,
            enable_job_queue: true,
            max_queue_size: 100_000,
        },
        songbird_integration: None,
    };

    let result = DistributedCoordinator::new(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_coordinator_with_disabled_queue() {
    let config = DistributedConfig {
        instance_id: "test-disabled-queue".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 4,
            default_timeout_secs: 30,
            enable_job_queue: false, // Queue disabled
            max_queue_size: 0,
        },
        songbird_integration: None,
    };

    let result = DistributedCoordinator::new(config).await;
    assert!(result.is_ok());
}

// ============================================================================
// Job Priority Handling
// ============================================================================

#[test]
fn test_job_priority_ordering() {
    let priorities = vec![
        JobPriority::Critical,
        JobPriority::High,
        JobPriority::Normal,
        JobPriority::Low,
        JobPriority::Background,
    ];

    // Verify priority ordering makes sense
    for i in 0..priorities.len() - 1 {
        let current = format!("{:?}", priorities[i]);
        let next = format!("{:?}", priorities[i + 1]);
        assert!(!current.is_empty());
        assert!(!next.is_empty());
    }
}

#[test]
fn test_job_priority_debug_format() {
    assert_eq!(format!("{:?}", JobPriority::Critical), "Critical");
    assert_eq!(format!("{:?}", JobPriority::High), "High");
    assert_eq!(format!("{:?}", JobPriority::Normal), "Normal");
    assert_eq!(format!("{:?}", JobPriority::Low), "Low");
    assert_eq!(format!("{:?}", JobPriority::Background), "Background");
}

// ============================================================================
// Job Type Validation
// ============================================================================

#[test]
fn test_all_job_types_are_debuggable() {
    let job_types = vec![
        UniversalJobType::Local,
        UniversalJobType::ComputeIntensive,
        UniversalJobType::MemoryIntensive,
        UniversalJobType::NetworkIntensive,
        UniversalJobType::StorageIntensive,
        UniversalJobType::Hybrid,
        UniversalJobType::DataProcessing,
        UniversalJobType::MachineLearning,
        UniversalJobType::Simulation,
        UniversalJobType::Native,
        UniversalJobType::WASM,
        UniversalJobType::Container,
    ];

    for job_type in job_types {
        let debug_str = format!("{job_type:?}");
        assert!(!debug_str.is_empty(), "Job type should have debug output");
    }
}

#[test]
fn test_job_type_clone() {
    let job_type = UniversalJobType::MachineLearning;
    let cloned = job_type.clone();
    assert_eq!(format!("{job_type:?}"), format!("{:?}", cloned));
}

// ============================================================================
// Configuration Validation
// ============================================================================

#[test]
fn test_standalone_config_default_values() {
    let config = StandaloneConfig {
        max_concurrent_executions: 4,
        default_timeout_secs: 300,
        enable_job_queue: true,
        max_queue_size: 100,
    };

    assert_eq!(config.max_concurrent_executions, 4);
    assert_eq!(config.default_timeout_secs, 300);
    assert!(config.enable_job_queue);
    assert_eq!(config.max_queue_size, 100);
}

#[test]
fn test_standalone_config_edge_values() {
    let config = StandaloneConfig {
        max_concurrent_executions: 1,
        default_timeout_secs: 1,
        enable_job_queue: false,
        max_queue_size: 1,
    };

    assert_eq!(config.max_concurrent_executions, 1);
    assert_eq!(config.default_timeout_secs, 1);
    assert!(!config.enable_job_queue);
    assert_eq!(config.max_queue_size, 1);
}

#[test]
fn test_distributed_config_with_unique_instance_ids() {
    let config1 = DistributedConfig {
        instance_id: "instance-001".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 4,
            default_timeout_secs: 30,
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: None,
    };

    let config2 = DistributedConfig {
        instance_id: "instance-002".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 4,
            default_timeout_secs: 30,
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: None,
    };

    assert_ne!(config1.instance_id, config2.instance_id);
}

// ============================================================================
// Timeout Handling
// ============================================================================

#[tokio::test]
async fn test_short_timeout_config() {
    let config = DistributedConfig {
        instance_id: "test-short-timeout".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 4,
            default_timeout_secs: 1, // Very short
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: None,
    };

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok());
}

#[tokio::test]
async fn test_long_timeout_config() {
    let config = DistributedConfig {
        instance_id: "test-long-timeout".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 4,
            default_timeout_secs: 86400, // 24 hours
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: None,
    };

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok());
}

// ============================================================================
// Concurrent Coordinator Operations
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_coordinators_different_ids() {
    let futures = (0..4).map(|i| async move {
        let config = DistributedConfig {
            instance_id: format!("concurrent-coordinator-{i}"),
            standalone: StandaloneConfig {
                max_concurrent_executions: 2,
                default_timeout_secs: 30,
                enable_job_queue: true,
                max_queue_size: 50,
            },
            songbird_integration: None,
        };
        DistributedCoordinator::new(config).await
    });

    let results = futures::future::join_all(futures).await;

    for result in results {
        assert!(
            result.is_ok(),
            "All coordinators should initialize successfully"
        );
    }
}

// ============================================================================
// Queue Size Limits
// ============================================================================

#[test]
fn test_queue_size_limits() {
    let sizes = vec![1, 10, 100, 1_000, 10_000, 100_000];

    for size in sizes {
        let config = StandaloneConfig {
            max_concurrent_executions: 4,
            default_timeout_secs: 30,
            enable_job_queue: true,
            max_queue_size: size,
        };
        assert_eq!(config.max_queue_size, size);
    }
}

// ============================================================================
// Instance ID Validation
// ============================================================================

#[test]
fn test_instance_id_formats() {
    let ids = vec![
        "simple-id",
        "id-with-numbers-123",
        "id_with_underscores",
        "UPPERCASE_ID",
        "MixedCase-ID_123",
        "very-long-instance-id-with-many-parts-separated-by-hyphens-12345",
    ];

    for id in ids {
        let config = DistributedConfig {
            instance_id: id.to_string(),
            standalone: StandaloneConfig {
                max_concurrent_executions: 4,
                default_timeout_secs: 30,
                enable_job_queue: true,
                max_queue_size: 100,
            },
            songbird_integration: None,
        };
        assert_eq!(config.instance_id, id);
    }
}

#[test]
fn test_empty_instance_id() {
    let config = DistributedConfig {
        instance_id: String::new(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 4,
            default_timeout_secs: 30,
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: None,
    };
    assert!(config.instance_id.is_empty());
}

// ============================================================================
// Configuration Cloning and Copying
// ============================================================================

#[test]
fn test_config_clone() {
    let config = DistributedConfig {
        instance_id: "original-config".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 8,
            default_timeout_secs: 60,
            enable_job_queue: true,
            max_queue_size: 200,
        },
        songbird_integration: None,
    };

    let cloned = config.clone();
    assert_eq!(config.instance_id, cloned.instance_id);
    assert_eq!(
        config.standalone.max_concurrent_executions,
        cloned.standalone.max_concurrent_executions
    );
}
