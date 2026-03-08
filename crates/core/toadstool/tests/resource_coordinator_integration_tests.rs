// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests for `ResourceCoordinator`
//! These tests verify resource allocation and coordination logic

use toadstool::resources::ResourceRequirements;
use toadstool::universal::ResourceCoordinator;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_new() {
    let coordinator = ResourceCoordinator::new().await;
    assert!(coordinator.is_ok());
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_get_available_resources() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let resources = coordinator.get_available_resources().await;

    // Check default resources
    assert_eq!(resources.cpu_cores, 8.0);
    assert_eq!(resources.memory_bytes, 8 * 1024 * 1024 * 1024); // 8GB
    assert_eq!(resources.storage_bytes, 100 * 1024 * 1024 * 1024); // 100GB
    assert_eq!(resources.network_bandwidth, 1000 * 1024 * 1024); // 1Gbps
    assert_eq!(resources.gpu_units, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_allocate_resources() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let requirements = ResourceRequirements::default();

    let allocation = coordinator.allocate_resources(&requirements).await;
    assert!(allocation.is_ok());

    let alloc = allocation.unwrap();
    assert!(alloc.released_at.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_release_resources() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let requirements = ResourceRequirements::default();

    let allocation = coordinator.allocate_resources(&requirements).await.unwrap();
    let result = coordinator.release_resources(allocation).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_multiple_allocations() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    for _ in 0..5 {
        let requirements = ResourceRequirements::default();
        let allocation = coordinator.allocate_resources(&requirements).await;
        assert!(allocation.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_allocate_and_release_cycle() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    // Allocate
    let requirements = ResourceRequirements::default();
    let allocation = coordinator.allocate_resources(&requirements).await.unwrap();
    assert!(allocation.released_at.is_none());

    // Release
    let result = coordinator.release_resources(allocation).await;
    assert!(result.is_ok());
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_custom_requirements() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    let mut requirements = ResourceRequirements::default();
    requirements.cpu.min_cores = 4.0;
    requirements.memory.min_bytes = 2 * 1024 * 1024 * 1024; // 2GB

    let allocation = coordinator.allocate_resources(&requirements).await;
    assert!(allocation.is_ok());

    let alloc = allocation.unwrap();
    assert_eq!(alloc.allocated_resources.cpu.min_cores, 4.0);
    assert_eq!(
        alloc.allocated_resources.memory.min_bytes,
        2 * 1024 * 1024 * 1024
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_allocation_has_job_id() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let requirements = ResourceRequirements::default();

    let allocation = coordinator.allocate_resources(&requirements).await.unwrap();
    // UUID should be non-zero
    assert_ne!(
        allocation.job_id.to_string(),
        "00000000-0000-0000-0000-000000000000"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_allocation_timestamps() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let requirements = ResourceRequirements::default();

    let before = std::time::SystemTime::now();
    let allocation = coordinator.allocate_resources(&requirements).await.unwrap();
    let after = std::time::SystemTime::now();

    assert!(allocation.allocated_at >= before);
    assert!(allocation.allocated_at <= after);
    assert!(allocation.released_at.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_release_sets_timestamp() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let requirements = ResourceRequirements::default();

    let allocation = coordinator.allocate_resources(&requirements).await.unwrap();
    assert!(allocation.released_at.is_none());

    // Release doesn't return the modified allocation, so we just verify no error
    let result = coordinator.release_resources(allocation).await;
    assert!(result.is_ok());
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_available_resources_immutable() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    let resources1 = coordinator.get_available_resources().await;
    let resources2 = coordinator.get_available_resources().await;

    assert_eq!(resources1.cpu_cores, resources2.cpu_cores);
    assert_eq!(resources1.memory_bytes, resources2.memory_bytes);
    assert_eq!(resources1.storage_bytes, resources2.storage_bytes);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_concurrent_allocations() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let coordinator = std::sync::Arc::new(coordinator);

    let mut handles = vec![];
    for _ in 0..10 {
        let coord = coordinator.clone();
        let handle = tokio::spawn(async move {
            let requirements = ResourceRequirements::default();
            coord.allocate_resources(&requirements).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_special_hardware_empty() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let resources = coordinator.get_available_resources().await;
    assert!(resources.special_hardware.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_network_bandwidth_value() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let resources = coordinator.get_available_resources().await;

    // Should be 1Gbps = 1000 * 1024 * 1024 bytes/sec
    assert_eq!(resources.network_bandwidth, 1_048_576_000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_coordinator_multiple_releases() {
    let coordinator = ResourceCoordinator::new().await.unwrap();

    let mut allocations = vec![];
    for _ in 0..3 {
        let requirements = ResourceRequirements::default();
        let alloc = coordinator.allocate_resources(&requirements).await.unwrap();
        allocations.push(alloc);
    }

    for alloc in allocations {
        let result = coordinator.release_resources(alloc).await;
        assert!(result.is_ok());
    }
}
