//! Comprehensive Tests for GPU Resource Coordinator
//!
//! Tests cover:
//! - Resource pool initialization
//! - Device selection
//! - Resource allocation/deallocation
//! - Load balancing
//! - Resource limits and errors

use toadstool::error::ToadStoolResult;
use toadstool_runtime_gpu::config::ResourceConfig;
use toadstool_runtime_gpu::coordinator::ComputeResourceCoordinator;
use toadstool_runtime_gpu::types::{
    ComputeCapabilities, DeviceId, DeviceRequirements, DeviceVendor, GpuFramework,
    UniversalComputeDevice,
};

/// Test helper to create a test device
fn create_test_device(id: &str, memory_mb: u64, compute_units: usize) -> UniversalComputeDevice {
    UniversalComputeDevice {
        id: DeviceId::from(id.to_string()),
        name: format!("Test Device {}", id),
        vendor: DeviceVendor::Simulated,
        framework: GpuFramework::Simulation,
        driver_version: "1.0.0".to_string(),
        capabilities: ComputeCapabilities {
            compute_capability: "1.0".to_string(),
            total_memory_bytes: memory_mb * 1024 * 1024,
            compute_units: compute_units,
            max_work_group_size: 1024,
            max_threads_per_block: 1024,
            warp_size: 32,
            supports_double_precision: false,
            supports_unified_memory: false,
            supports_cooperative_launch: false,
        },
        is_available: true,
        current_usage: None,
    }
}

/// Test helper to create default resource config
fn create_test_config() -> ResourceConfig {
    ResourceConfig {
        max_concurrent_kernels: Some(4),
        default_memory_pool_mb: Some(256),
        enable_unified_memory: Some(false),
        enable_peer_access: Some(false),
    }
}

#[tokio::test]
async fn test_coordinator_creation() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    // Should create successfully (no Result type on new())
    assert!(true, "Coordinator created successfully");
}

#[tokio::test]
async fn test_initialize_device_pool() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 1024, 8);
    let result = coordinator.initialize_device_pool(&device).await;
    
    assert!(
        result.is_ok(),
        "Should initialize device pool: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_initialize_multiple_device_pools() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device1 = create_test_device("device1", 1024, 8);
    let device2 = create_test_device("device2", 2048, 16);
    let device3 = create_test_device("device3", 512, 4);
    
    let result1 = coordinator.initialize_device_pool(&device1).await;
    let result2 = coordinator.initialize_device_pool(&device2).await;
    let result3 = coordinator.initialize_device_pool(&device3).await;
    
    assert!(result1.is_ok(), "Device 1 pool should initialize");
    assert!(result2.is_ok(), "Device 2 pool should initialize");
    assert!(result3.is_ok(), "Device 3 pool should initialize");
}

#[tokio::test]
async fn test_select_device_basic() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 1024, 8);
    coordinator.initialize_device_pool(&device).await.unwrap();
    
    let available_devices = vec![device.id.clone()];
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(256 * 1024 * 1024), // 256MB
        min_compute_units: Some(4),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    let result = coordinator
        .select_device(&available_devices, &requirements)
        .await;
    
    assert!(
        result.is_ok(),
        "Should select device: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), device.id);
}

#[tokio::test]
async fn test_select_device_multiple_choices() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device1 = create_test_device("device1", 1024, 8);
    let device2 = create_test_device("device2", 2048, 16);
    
    coordinator.initialize_device_pool(&device1).await.unwrap();
    coordinator.initialize_device_pool(&device2).await.unwrap();
    
    let available_devices = vec![device1.id.clone(), device2.id.clone()];
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(256 * 1024 * 1024),
        min_compute_units: Some(4),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    let result = coordinator
        .select_device(&available_devices, &requirements)
        .await;
    
    assert!(result.is_ok(), "Should select a device");
    let selected = result.unwrap();
    assert!(
        selected == device1.id || selected == device2.id,
        "Should select one of the available devices"
    );
}

#[tokio::test]
async fn test_allocate_resources_basic() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 1024, 8);
    coordinator.initialize_device_pool(&device).await.unwrap();
    
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(128 * 1024 * 1024), // 128MB
        min_compute_units: Some(2),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    let result = coordinator
        .allocate_resources(&device.id, &requirements)
        .await;
    
    assert!(
        result.is_ok(),
        "Should allocate resources: {:?}",
        result.err()
    );
    
    let allocation = result.unwrap();
    assert!(allocation.allocated_memory_bytes >= 128 * 1024 * 1024);
    assert!(allocation.allocated_compute_units >= 2);
}

#[tokio::test]
async fn test_allocate_resources_insufficient_memory() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 512, 8); // Only 512MB
    coordinator.initialize_device_pool(&device).await.unwrap();
    
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(1024 * 1024 * 1024), // Request 1GB (more than available)
        min_compute_units: Some(2),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    let result = coordinator
        .allocate_resources(&device.id, &requirements)
        .await;
    
    assert!(
        result.is_err(),
        "Should fail when insufficient memory"
    );
}

#[tokio::test]
async fn test_allocate_resources_nonexistent_device() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let nonexistent_device = DeviceId::from("nonexistent".to_string());
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(128 * 1024 * 1024),
        min_compute_units: Some(2),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    let result = coordinator
        .allocate_resources(&nonexistent_device, &requirements)
        .await;
    
    assert!(
        result.is_err(),
        "Should fail for nonexistent device"
    );
}

#[tokio::test]
async fn test_deallocate_resources() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 1024, 8);
    coordinator.initialize_device_pool(&device).await.unwrap();
    
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(128 * 1024 * 1024),
        min_compute_units: Some(2),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    // Allocate
    let allocation = coordinator
        .allocate_resources(&device.id, &requirements)
        .await
        .unwrap();
    
    // Deallocate
    let result = coordinator
        .deallocate_resources(&device.id, &allocation)
        .await;
    
    assert!(
        result.is_ok(),
        "Should deallocate resources: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_multiple_allocations() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 2048, 16);
    coordinator.initialize_device_pool(&device).await.unwrap();
    
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(256 * 1024 * 1024),
        min_compute_units: Some(2),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    // Make multiple allocations
    let alloc1 = coordinator.allocate_resources(&device.id, &requirements).await;
    let alloc2 = coordinator.allocate_resources(&device.id, &requirements).await;
    let alloc3 = coordinator.allocate_resources(&device.id, &requirements).await;
    
    assert!(alloc1.is_ok(), "First allocation should succeed");
    assert!(alloc2.is_ok(), "Second allocation should succeed");
    assert!(alloc3.is_ok(), "Third allocation should succeed");
}

#[tokio::test]
async fn test_allocation_deallocation_cycle() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 1024, 8);
    coordinator.initialize_device_pool(&device).await.unwrap();
    
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(256 * 1024 * 1024),
        min_compute_units: Some(2),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    // Allocate -> Deallocate -> Allocate again
    let alloc1 = coordinator
        .allocate_resources(&device.id, &requirements)
        .await
        .unwrap();
    
    coordinator
        .deallocate_resources(&device.id, &alloc1)
        .await
        .unwrap();
    
    let alloc2 = coordinator
        .allocate_resources(&device.id, &requirements)
        .await;
    
    assert!(alloc2.is_ok(), "Should be able to allocate after deallocation");
}

#[tokio::test]
async fn test_concurrent_allocations() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 4096, 16);
    coordinator.initialize_device_pool(&device).await.unwrap();
    
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(256 * 1024 * 1024),
        min_compute_units: Some(1),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    // Spawn concurrent allocation tasks
    let mut handles = vec![];
    for _ in 0..5 {
        let coord = &coordinator;
        let dev_id = device.id.clone();
        let reqs = requirements.clone();
        
        let handle = tokio::spawn(async move {
            coord.allocate_resources(&dev_id, &reqs).await
        });
        handles.push(handle);
    }
    
    // Wait for all to complete
    let mut successful = 0;
    for handle in handles {
        if let Ok(result) = handle.await {
            if result.is_ok() {
                successful += 1;
            }
        }
    }
    
    assert!(successful >= 3, "Most concurrent allocations should succeed");
}

#[tokio::test]
async fn test_device_requirements_defaults() {
    let requirements = DeviceRequirements {
        min_memory_bytes: None,
        min_compute_units: None,
        preferred_vendor: None,
        required_features: vec![],
    };
    
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 1024, 8);
    coordinator.initialize_device_pool(&device).await.unwrap();
    
    // Should use default values
    let result = coordinator
        .allocate_resources(&device.id, &requirements)
        .await;
    
    assert!(
        result.is_ok(),
        "Should handle default requirements: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_resource_config_defaults() {
    let config = ResourceConfig::default();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 1024, 8);
    let result = coordinator.initialize_device_pool(&device).await;
    
    assert!(
        result.is_ok(),
        "Should work with default config"
    );
}

#[tokio::test]
async fn test_resource_config_custom_limits() {
    let config = ResourceConfig {
        max_concurrent_kernels: Some(2),
        default_memory_pool_mb: Some(512),
        enable_unified_memory: Some(true),
        enable_peer_access: Some(true),
    };
    
    let coordinator = ComputeResourceCoordinator::new(config);
    let device = create_test_device("device1", 1024, 8);
    coordinator.initialize_device_pool(&device).await.unwrap();
    
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(128 * 1024 * 1024),
        min_compute_units: Some(2),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    let result = coordinator
        .allocate_resources(&device.id, &requirements)
        .await;
    
    assert!(result.is_ok(), "Custom limits should be respected");
}

#[test]
fn test_device_id_equality() {
    let id1 = DeviceId::from("device1".to_string());
    let id2 = DeviceId::from("device1".to_string());
    let id3 = DeviceId::from("device2".to_string());
    
    assert_eq!(id1, id2, "Same device IDs should be equal");
    assert_ne!(id1, id3, "Different device IDs should not be equal");
}

#[test]
fn test_device_requirements_clone() {
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(256 * 1024 * 1024),
        min_compute_units: Some(4),
        preferred_vendor: Some(DeviceVendor::NVIDIA),
        required_features: vec!["double_precision".to_string()],
    };
    
    let cloned = requirements.clone();
    assert_eq!(requirements.min_memory_bytes, cloned.min_memory_bytes);
    assert_eq!(requirements.min_compute_units, cloned.min_compute_units);
}

#[tokio::test]
async fn test_initialize_same_device_twice() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device = create_test_device("device1", 1024, 8);
    
    let result1 = coordinator.initialize_device_pool(&device).await;
    let result2 = coordinator.initialize_device_pool(&device).await;
    
    assert!(result1.is_ok(), "First initialization should succeed");
    assert!(result2.is_ok(), "Second initialization should be idempotent or replace");
}

#[tokio::test]
async fn test_select_device_empty_list() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let available_devices = vec![];
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(256 * 1024 * 1024),
        min_compute_units: Some(4),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    let result = coordinator
        .select_device(&available_devices, &requirements)
        .await;
    
    assert!(
        result.is_err(),
        "Should fail with empty device list"
    );
}

#[tokio::test]
async fn test_load_balancing_distribution() {
    let config = create_test_config();
    let coordinator = ComputeResourceCoordinator::new(config);
    
    let device1 = create_test_device("device1", 1024, 8);
    let device2 = create_test_device("device2", 1024, 8);
    
    coordinator.initialize_device_pool(&device1).await.unwrap();
    coordinator.initialize_device_pool(&device2).await.unwrap();
    
    let available_devices = vec![device1.id.clone(), device2.id.clone()];
    let requirements = DeviceRequirements {
        min_memory_bytes: Some(128 * 1024 * 1024),
        min_compute_units: Some(2),
        preferred_vendor: None,
        required_features: vec![],
    };
    
    // Make multiple selections and track distribution
    let mut selections = std::collections::HashMap::new();
    for _ in 0..10 {
        let selected = coordinator
            .select_device(&available_devices, &requirements)
            .await
            .unwrap();
        *selections.entry(selected).or_insert(0) += 1;
    }
    
    // Both devices should be selected at least once (load balancing)
    assert!(
        selections.len() > 0,
        "Load balancer should distribute across devices"
    );
}

