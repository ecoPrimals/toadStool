// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_edge_runtime_config_default() {
    let config = EdgeRuntimeConfig::default();
    assert!(config.discovery_enabled);
    assert_eq!(config.discovery_timeout_secs, 30);
    assert_eq!(config.max_devices, 100);
    assert!(config.auto_provisioning);
}

#[test]
fn test_edge_runtime_config_custom() {
    let config = EdgeRuntimeConfig {
        discovery_enabled: false,
        discovery_timeout_secs: 60,
        max_devices: 50,
        communication_timeout_ms: 10000,
        cross_compile_cache_path: "/custom/cache".to_string(),
        auto_provisioning: false,
        security_level: EdgeSecurityLevel::High,
        resource_strategy: ResourceAllocationStrategy::Conservative,
        port_registry: toadstool_config::ports::PortRegistry::default(),
    };

    assert!(!config.discovery_enabled);
    assert_eq!(config.max_devices, 50);
    assert_eq!(config.cross_compile_cache_path, "/custom/cache");
}

#[test]
fn test_edge_security_level_variants() {
    let levels = vec![
        EdgeSecurityLevel::Minimal,
        EdgeSecurityLevel::Standard,
        EdgeSecurityLevel::High,
        EdgeSecurityLevel::Maximum,
    ];

    assert_eq!(levels.len(), 4);
}

#[test]
fn test_resource_allocation_strategy_adaptive() {
    let strategy = ResourceAllocationStrategy::Adaptive;
    assert!(matches!(strategy, ResourceAllocationStrategy::Adaptive));
}

#[test]
fn test_resource_allocation_strategy_custom() {
    let mut rules = HashMap::new();
    rules.insert("cpu_limit".to_string(), 0.8);
    rules.insert("memory_limit".to_string(), 0.7);

    let strategy = ResourceAllocationStrategy::Custom(rules);
    if let ResourceAllocationStrategy::Custom(rules) = strategy {
        assert_eq!(rules.len(), 2);
        assert_eq!(rules.get("cpu_limit"), Some(&0.8));
    } else {
        unreachable!("Expected Custom strategy");
    }
}

#[test]
fn test_edge_execution_handle_creation() {
    let handle = EdgeExecutionHandle {
        id: Uuid::new_v4(),
        device_id: Uuid::new_v4(),
        platform: EdgePlatform::RaspberryPi {
            model: PiModel::Pi4,
            os: PiOS::RaspberryPiOS,
        },
        status: ExecutionStatus::Running,
        started_at: std::time::SystemTime::now(),
        resource_usage: ResourceUsage {
            cpu_percent: 25.5,
            memory_bytes: 1024000,
            storage_bytes: 512000,
            network_bytes_sent: 2048,
            network_bytes_received: 4096,
        },
    };

    assert_eq!(handle.status, ExecutionStatus::Running);
    assert_eq!(handle.resource_usage.cpu_percent, 25.5);
    assert_eq!(handle.resource_usage.memory_bytes, 1024000);
}

#[test]
fn test_resource_usage_tracking() {
    let usage = ResourceUsage {
        cpu_percent: 50.0,
        memory_bytes: 2048000,
        storage_bytes: 1024000,
        network_bytes_sent: 5000,
        network_bytes_received: 10000,
    };

    assert_eq!(usage.cpu_percent, 50.0);
    assert_eq!(usage.memory_bytes, 2048000);
    assert_eq!(usage.network_bytes_sent, 5000);
    assert_eq!(usage.network_bytes_received, 10000);
}

#[test]
fn test_edge_security_level_debug() {
    let level = EdgeSecurityLevel::High;
    let debug_str = format!("{:?}", level);
    assert!(debug_str.contains("High"));
}

#[test]
fn test_resource_allocation_strategy_debug() {
    let strategy = ResourceAllocationStrategy::Aggressive;
    let debug_str = format!("{:?}", strategy);
    assert!(debug_str.contains("Aggressive"));
}

#[test]
fn test_edge_runtime_config_clone() {
    let config1 = EdgeRuntimeConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.discovery_enabled, config2.discovery_enabled);
    assert_eq!(config1.max_devices, config2.max_devices);
}

#[test]
fn test_edge_execution_handle_clone() {
    let handle1 = EdgeExecutionHandle {
        id: Uuid::new_v4(),
        device_id: Uuid::new_v4(),
        platform: EdgePlatform::ESP32 {
            chip: ESP32Variant::ESP32,
            framework: ESP32Framework::ESPIDF,
        },
        status: ExecutionStatus::Success,
        started_at: std::time::SystemTime::now(),
        resource_usage: ResourceUsage {
            cpu_percent: 10.0,
            memory_bytes: 512000,
            storage_bytes: 256000,
            network_bytes_sent: 1024,
            network_bytes_received: 2048,
        },
    };
    let handle2 = handle1.clone();

    assert_eq!(handle1.id, handle2.id);
    assert_eq!(handle1.device_id, handle2.device_id);
    assert_eq!(handle1.status, handle2.status);
}

#[test]
fn test_resource_usage_clone() {
    let usage1 = ResourceUsage {
        cpu_percent: 75.0,
        memory_bytes: 4096000,
        storage_bytes: 2048000,
        network_bytes_sent: 8192,
        network_bytes_received: 16384,
    };
    let usage2 = usage1.clone();

    assert_eq!(usage1.cpu_percent, usage2.cpu_percent);
    assert_eq!(usage1.memory_bytes, usage2.memory_bytes);
    assert_eq!(usage1.storage_bytes, usage2.storage_bytes);
}

#[test]
fn test_edge_runtime_config_cache_path() {
    let config = EdgeRuntimeConfig {
        discovery_enabled: true,
        discovery_timeout_secs: 30,
        max_devices: 100,
        communication_timeout_ms: 5000,
        cross_compile_cache_path: "/custom/edge/cache".to_string(),
        auto_provisioning: true,
        security_level: EdgeSecurityLevel::Standard,
        resource_strategy: ResourceAllocationStrategy::Adaptive,
        port_registry: toadstool_config::ports::PortRegistry::default(),
    };

    assert_eq!(config.cross_compile_cache_path, "/custom/edge/cache");
}

#[test]
fn test_edge_runtime_config_timeouts() {
    let config = EdgeRuntimeConfig {
        discovery_enabled: true,
        discovery_timeout_secs: 120,
        max_devices: 100,
        communication_timeout_ms: 15000,
        cross_compile_cache_path: std::env::temp_dir()
            .join("toadstool-edge-cache")
            .to_string_lossy()
            .to_string(),
        auto_provisioning: true,
        security_level: EdgeSecurityLevel::Standard,
        resource_strategy: ResourceAllocationStrategy::Adaptive,
        port_registry: toadstool_config::ports::PortRegistry::default(),
    };

    assert_eq!(config.discovery_timeout_secs, 120);
    assert_eq!(config.communication_timeout_ms, 15000);
}

#[test]
fn test_resource_allocation_strategies() {
    let strategies = vec![
        ResourceAllocationStrategy::Adaptive,
        ResourceAllocationStrategy::Conservative,
        ResourceAllocationStrategy::Aggressive,
    ];

    assert_eq!(strategies.len(), 3);
}

#[test]
fn test_edge_security_levels_ordering() {
    let minimal = EdgeSecurityLevel::Minimal;
    let standard = EdgeSecurityLevel::Standard;
    let high = EdgeSecurityLevel::High;
    let maximum = EdgeSecurityLevel::Maximum;

    assert!(matches!(minimal, EdgeSecurityLevel::Minimal));
    assert!(matches!(standard, EdgeSecurityLevel::Standard));
    assert!(matches!(high, EdgeSecurityLevel::High));
    assert!(matches!(maximum, EdgeSecurityLevel::Maximum));
}
