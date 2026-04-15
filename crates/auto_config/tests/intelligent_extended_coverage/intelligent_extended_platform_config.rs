// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashSet;

use toadstool_auto_config::intelligent::{PlatformConfig, PlatformOptimization, PlatformSupport};

#[test]
fn test_platform_config_supports_containers() {
    let mut config = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: HashSet::new(),
        optimizations: Vec::new(),
    };

    config
        .supported_features
        .insert(PlatformSupport::Containers);

    assert!(config.supports_containers());
}

#[test]
fn test_platform_config_supports_sandboxing() {
    let mut config = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: HashSet::new(),
        optimizations: Vec::new(),
    };

    config
        .supported_features
        .insert(PlatformSupport::Sandboxing);

    assert!(config.supports_sandboxing());
}

#[test]
fn test_platform_config_supports_process_isolation() {
    let mut config = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: HashSet::new(),
        optimizations: Vec::new(),
    };

    config
        .supported_features
        .insert(PlatformSupport::ProcessIsolation);

    assert!(config.supports_process_isolation());
}

#[test]
fn test_platform_config_supports_network_isolation() {
    let mut config = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: HashSet::new(),
        optimizations: Vec::new(),
    };

    config
        .supported_features
        .insert(PlatformSupport::NetworkIsolation);

    assert!(config.supports_network_isolation());
}

#[test]
fn test_platform_config_supports_generic() {
    let mut config = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: HashSet::new(),
        optimizations: Vec::new(),
    };

    config
        .supported_features
        .insert(PlatformSupport::Containers);

    assert!(config.supports(&PlatformSupport::Containers));
    assert!(!config.supports(&PlatformSupport::Sandboxing));
}

#[test]
fn test_platform_config_clone() {
    let config = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: HashSet::new(),
        optimizations: vec![PlatformOptimization {
            optimization_type: "test".to_string(),
            description: "test optimization".to_string(),
            performance_gain: 0.1,
        }],
    };

    let cloned = config.clone();
    assert_eq!(config.platform_name, cloned.platform_name);
    assert_eq!(config.optimizations.len(), cloned.optimizations.len());
}

#[test]
fn test_platform_optimization_creation() {
    let opt = PlatformOptimization {
        optimization_type: "memory_mapping".to_string(),
        description: "Use mmap for large files".to_string(),
        performance_gain: 0.15,
    };

    assert_eq!(opt.optimization_type, "memory_mapping");
    assert_eq!(opt.performance_gain, 0.15);
}

#[test]
fn test_platform_optimization_clone() {
    let opt = PlatformOptimization {
        optimization_type: "async_io".to_string(),
        description: "Use io_uring".to_string(),
        performance_gain: 0.25,
    };

    let cloned = opt.clone();
    assert_eq!(opt.optimization_type, cloned.optimization_type);
    assert_eq!(opt.performance_gain, cloned.performance_gain);
}

#[test]
fn test_platform_optimization_types() {
    let types = vec![
        "memory_mapping",
        "async_io",
        "vector_instructions",
        "numa_awareness",
        "parallel_compilation",
        "large_buffer",
    ];

    for opt_type in types {
        let opt = PlatformOptimization {
            optimization_type: opt_type.to_string(),
            description: format!("Test {opt_type}"),
            performance_gain: 0.1,
        };

        assert!(!opt.optimization_type.is_empty());
        assert!(opt.performance_gain >= 0.0 && opt.performance_gain <= 1.0);
    }
}
