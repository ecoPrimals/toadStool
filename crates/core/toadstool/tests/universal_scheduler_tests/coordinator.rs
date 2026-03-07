// SPDX-License-Identifier: AGPL-3.0-or-later
//! `ResourceCoordinator` tests — allocation, release, and multi-allocation.

use toadstool::resources::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements,
    ResourceRequirements, StorageRequirements,
};
use toadstool::universal::ResourceCoordinator;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation() {
    let result = ResourceCoordinator::new().await;
    assert!(
        result.is_ok(),
        "ResourceCoordinator creation should succeed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_get_available_resources() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let resources = coordinator.get_available_resources().await;

    assert!(
        resources.cpu_cores > 0.0,
        "CPU cores should be positive (got {})",
        resources.cpu_cores
    );
    assert!(
        resources.memory_bytes > 0,
        "Memory should be positive (got {})",
        resources.memory_bytes
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_allocate_resources() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 1.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 512 * 1024 * 1024,
            max_bytes: None,
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    let result = coordinator.allocate_resources(&requirements).await;
    assert!(
        result.is_ok(),
        "Resource allocation should succeed for modest requirements"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_allocate_resources_with_gpu() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let requirements = ResourceRequirements {
        cpu: CpuRequirements::default(),
        memory: MemoryRequirements::default(),
        storage: StorageRequirements::default(),
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: Some(2),
            gpu_type: None,
            min_memory_bytes: Some(4 * 1024 * 1024 * 1024),
        }),
        network: NetworkRequirements::default(),
    };

    let result = coordinator.allocate_resources(&requirements).await;
    assert!(result.is_ok(), "GPU allocation should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_release_resources() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 1.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 256 * 1024 * 1024,
            max_bytes: None,
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    let allocation = coordinator.allocate_resources(&requirements).await.unwrap();
    let result = coordinator.release_resources(allocation).await;
    assert!(result.is_ok(), "Resource release should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_multiple_allocations() {
    let coordinator = ResourceCoordinator::new().await.unwrap();
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.5,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 128 * 1024 * 1024,
            max_bytes: None,
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    let alloc1 = coordinator.allocate_resources(&requirements).await;
    let alloc2 = coordinator.allocate_resources(&requirements).await;
    let alloc3 = coordinator.allocate_resources(&requirements).await;

    assert!(alloc1.is_ok());
    assert!(alloc2.is_ok());
    assert!(alloc3.is_ok());
}
