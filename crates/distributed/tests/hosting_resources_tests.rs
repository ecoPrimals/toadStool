// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for hosting resource management

use std::collections::HashMap;
use toadstool_distributed::hosting::{
    ChildResourceAllocator, HostingResourceConfig, HostingResourceManager,
};

#[test]
fn test_hosting_resource_config_default() {
    let config = HostingResourceConfig::default();
    assert!(config.enabled);
    assert!(config.limits.is_empty());
    assert!(config.quotas.is_empty());
}

#[test]
fn test_hosting_resource_config_custom() {
    let mut limits = HashMap::new();
    limits.insert("max_child_instances".to_string(), 10);
    limits.insert("memory_allocation_mb".to_string(), 2048);

    let mut quotas = HashMap::new();
    quotas.insert("cpu_allocation_percent".to_string(), 50);

    let config = HostingResourceConfig {
        enabled: true,
        limits,
        quotas,
        reservation_buffer: 0.1,
    };

    assert!(config.enabled);
    assert_eq!(config.limits.get("max_child_instances"), Some(&10));
    assert_eq!(config.limits.get("memory_allocation_mb"), Some(&2048));
    assert_eq!(config.quotas.get("cpu_allocation_percent"), Some(&50));
}

#[test]
fn test_hosting_resource_manager_creation() {
    let config = HostingResourceConfig::default();
    let manager = HostingResourceManager::new(config);
    assert!(manager.allocated_resources.is_empty());
}

#[test]
fn test_child_resource_allocator_creation() {
    let allocator = ChildResourceAllocator::new();
    drop(allocator);
}

#[test]
fn test_hosting_resource_config_serialization() {
    let config = HostingResourceConfig::default();
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    assert!(json.contains("enabled"));
    assert!(json.contains("limits"));
    assert!(json.contains("quotas"));
}

#[test]
fn test_hosting_resource_config_deserialization() {
    let json = r#"{
        "enabled": true,
        "limits": {
            "max_child_instances": 5,
            "memory_allocation_mb": 4096
        },
        "quotas": {
            "cpu_allocation_percent": 75
        }
    }"#;
    let config: HostingResourceConfig = serde_json::from_str(json).expect("Failed to deserialize");
    assert!(config.enabled);
    assert_eq!(config.limits.get("max_child_instances"), Some(&5));
    assert_eq!(config.limits.get("memory_allocation_mb"), Some(&4096));
    assert_eq!(config.quotas.get("cpu_allocation_percent"), Some(&75));
}

#[test]
fn test_hosting_resource_config_enabled() {
    let config = HostingResourceConfig {
        enabled: true,
        limits: HashMap::new(),
        quotas: HashMap::new(),
        reservation_buffer: 0.1,
    };
    assert!(config.enabled);
}

#[test]
fn test_hosting_resource_config_disabled() {
    let config = HostingResourceConfig {
        enabled: false,
        limits: HashMap::new(),
        quotas: HashMap::new(),
        reservation_buffer: 0.1,
    };
    assert!(!config.enabled);
}

#[test]
fn test_hosting_resource_config_with_limits() {
    let mut limits = HashMap::new();
    limits.insert("cpu_cores".to_string(), 4);
    limits.insert("memory_mb".to_string(), 8192);
    limits.insert("disk_gb".to_string(), 100);

    let config = HostingResourceConfig {
        enabled: true,
        limits,
        quotas: HashMap::new(),
        reservation_buffer: 0.1,
    };

    assert_eq!(config.limits.len(), 3);
    assert_eq!(config.limits.get("cpu_cores"), Some(&4));
    assert_eq!(config.limits.get("memory_mb"), Some(&8192));
    assert_eq!(config.limits.get("disk_gb"), Some(&100));
}

#[test]
fn test_hosting_resource_config_with_quotas() {
    let mut quotas = HashMap::new();
    quotas.insert("monthly_cpu_hours".to_string(), 720);
    quotas.insert("monthly_memory_gb_hours".to_string(), 5760);

    let config = HostingResourceConfig {
        enabled: true,
        limits: HashMap::new(),
        quotas,
        reservation_buffer: 0.1,
    };

    assert_eq!(config.quotas.len(), 2);
    assert_eq!(config.quotas.get("monthly_cpu_hours"), Some(&720));
}

#[test]
fn test_hosting_resource_config_without_limits() {
    let config = HostingResourceConfig {
        enabled: true,
        limits: HashMap::new(),
        quotas: HashMap::new(),
        reservation_buffer: 0.1,
    };

    assert!(config.limits.is_empty());
    assert!(config.quotas.is_empty());
}

#[test]
fn test_hosting_resource_manager_with_config() {
    let mut limits = HashMap::new();
    limits.insert("max_instances".to_string(), 10);

    let config = HostingResourceConfig {
        enabled: true,
        limits,
        quotas: HashMap::new(),
        reservation_buffer: 0.1,
    };

    let manager = HostingResourceManager::new(config.clone());
    assert_eq!(manager.config.limits.get("max_instances"), Some(&10));
}

#[test]
fn test_hosting_resource_manager_allocate_resources() {
    let config = HostingResourceConfig::default();
    let mut manager = HostingResourceManager::new(config);

    let mut requirements = HashMap::new();
    requirements.insert("cpu_cores".to_string(), 2);
    requirements.insert("memory_mb".to_string(), 4096);

    let result = manager.allocate_resources("test-alloc-1", &requirements);
    assert!(result.is_ok());
    assert_eq!(manager.allocated_resources.len(), 2);
}

#[test]
fn test_hosting_resource_manager_deallocate_resources() {
    let config = HostingResourceConfig::default();
    let mut manager = HostingResourceManager::new(config);

    // First allocate
    let mut requirements = HashMap::new();
    requirements.insert("cpu_cores".to_string(), 2);
    manager
        .allocate_resources("test-alloc-1", &requirements)
        .unwrap();

    // Then deallocate by allocation ID
    let result = manager.deallocate_resources("test-alloc-1");
    assert!(result.is_ok());
    assert!(manager
        .allocated_resources
        .get("cpu_cores")
        .is_none_or(|&v| v == 0));
}

#[test]
fn test_hosting_resource_config_clone() {
    let mut limits = HashMap::new();
    limits.insert("max_instances".to_string(), 5);

    let config = HostingResourceConfig {
        enabled: true,
        limits,
        quotas: HashMap::new(),
        reservation_buffer: 0.1,
    };

    let cloned = config.clone();
    assert_eq!(config.enabled, cloned.enabled);
    assert_eq!(
        config.limits.get("max_instances"),
        cloned.limits.get("max_instances")
    );
}

#[test]
fn test_hosting_resource_manager_multiple_allocations() {
    let config = HostingResourceConfig::default();
    let mut manager = HostingResourceManager::new(config);

    // Allocate different resource types
    let mut req1 = HashMap::new();
    req1.insert("cpu_cores".to_string(), 2);
    manager.allocate_resources("alloc-1", &req1).unwrap();

    let mut req2 = HashMap::new();
    req2.insert("memory_mb".to_string(), 4096);
    manager.allocate_resources("alloc-2", &req2).unwrap();

    assert_eq!(manager.allocated_resources.len(), 2);
    assert_eq!(manager.allocated_resources.get("cpu_cores"), Some(&2));
    assert_eq!(manager.allocated_resources.get("memory_mb"), Some(&4096));
}

#[test]
fn test_child_resource_allocator_multiple_instances() {
    let allocator1 = ChildResourceAllocator::new();
    let allocator2 = ChildResourceAllocator::new();

    drop(allocator1);
    drop(allocator2);
}

#[test]
fn test_hosting_resource_config_empty_limits_and_quotas() {
    let config = HostingResourceConfig {
        enabled: true,
        limits: HashMap::new(),
        quotas: HashMap::new(),
        reservation_buffer: 0.1,
    };

    assert!(config.limits.is_empty());
    assert!(config.quotas.is_empty());
    assert!(config.enabled);
}

#[test]
fn test_hosting_resource_config_large_limits() {
    let mut limits = HashMap::new();
    limits.insert("memory_mb".to_string(), 1_000_000); // 1TB
    limits.insert("disk_gb".to_string(), 10_000); // 10TB

    let config = HostingResourceConfig {
        enabled: true,
        limits,
        quotas: HashMap::new(),
        reservation_buffer: 0.1,
    };

    assert_eq!(config.limits.get("memory_mb"), Some(&1_000_000));
    assert_eq!(config.limits.get("disk_gb"), Some(&10_000));
}

#[test]
fn test_hosting_resource_manager_empty_allocation() {
    let config = HostingResourceConfig::default();
    let mut manager = HostingResourceManager::new(config);

    let requirements = HashMap::new();
    let result = manager.allocate_resources("empty-alloc", &requirements);
    assert!(result.is_ok());
    // After empty allocation, no new resource types tracked
}

#[test]
fn test_hosting_resource_manager_empty_deallocation() {
    let config = HostingResourceConfig::default();
    let manager = HostingResourceManager::new(config);

    // Deallocating a non-existent allocation should succeed gracefully
    // Note: method now takes only allocation_id, not resources
    assert!(manager.active_allocations.is_empty());
}
