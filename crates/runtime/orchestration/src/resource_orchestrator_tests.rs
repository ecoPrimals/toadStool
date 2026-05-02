// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn two_gpu_devices() -> Vec<AvailableDevice> {
    vec![
        AvailableDevice {
            index: 0,
            total_vram_bytes: 16_000_000_000,
            allocated_vram_bytes: 0,
            current_tenant: None,
        },
        AvailableDevice {
            index: 1,
            total_vram_bytes: 24_000_000_000,
            allocated_vram_bytes: 0,
            current_tenant: None,
        },
    ]
}

fn test_request(tenant: &str, priority: u8) -> ResourceRequest {
    ResourceRequest {
        tenant_id: tenant.into(),
        priority,
        preferred_devices: vec![],
        min_vram_bytes: 1_000_000_000,
        estimated_duration: Duration::from_secs(60),
    }
}

#[test]
fn test_local_direct_gives_largest_device() {
    let orch = ResourceOrchestrator::new(DeploymentModel::LocalDirect, two_gpu_devices());
    let alloc = orch.allocate(&test_request("hotspring", 3)).unwrap();
    assert_eq!(alloc.device_index, 1);
    assert_eq!(alloc.vram_bytes, 24_000_000_000);
    assert!(alloc.exclusive);
}

#[test]
fn test_local_direct_preferred_device() {
    let orch = ResourceOrchestrator::new(DeploymentModel::LocalDirect, two_gpu_devices());
    let mut req = test_request("hotspring", 3);
    req.preferred_devices = vec![0];
    let alloc = orch.allocate(&req).unwrap();
    assert_eq!(alloc.device_index, 0);
}

#[test]
fn test_local_multi_shared_allocation() {
    let orch = ResourceOrchestrator::new(DeploymentModel::LocalMulti, two_gpu_devices());
    let alloc1 = orch.allocate(&test_request("hotspring", 3)).unwrap();
    let alloc2 = orch.allocate(&test_request("wetspring", 3)).unwrap();
    assert!(!alloc1.exclusive);
    assert!(!alloc2.exclusive);
}

#[test]
fn test_quota_enforcement_max_workloads() {
    let orch = ResourceOrchestrator::new(DeploymentModel::CloudRental, two_gpu_devices());
    orch.register_tenant(
        "tenant-a",
        TenantQuota {
            max_concurrent_workloads: 1,
            ..Default::default()
        },
    )
    .unwrap();

    let req = test_request("tenant-a", 3);
    let _alloc1 = orch.allocate(&req).unwrap();
    let result = orch.allocate(&req);
    assert!(result.is_err());
}

#[test]
fn test_quota_enforcement_max_vram() {
    let orch = ResourceOrchestrator::new(DeploymentModel::CloudRental, two_gpu_devices());
    orch.register_tenant(
        "tenant-a",
        TenantQuota {
            max_vram_bytes: 500_000_000,
            ..Default::default()
        },
    )
    .unwrap();

    let req = test_request("tenant-a", 3);
    let result = orch.allocate(&req);
    assert!(result.is_err());
}

#[test]
fn test_release_frees_resources() {
    let orch = ResourceOrchestrator::new(DeploymentModel::LocalMulti, two_gpu_devices());
    let alloc = orch.allocate(&test_request("hotspring", 3)).unwrap();

    let usage = orch.tenant_usage("hotspring").unwrap().unwrap();
    assert_eq!(usage.active_workloads, 1);

    orch.release("hotspring", alloc.device_index).unwrap();

    let usage = orch.tenant_usage("hotspring").unwrap().unwrap();
    assert_eq!(usage.active_workloads, 0);
}

#[test]
fn test_deployment_model_default() {
    assert_eq!(DeploymentModel::default(), DeploymentModel::LocalDirect);
}

#[test]
fn test_device_count() {
    let orch = ResourceOrchestrator::new(DeploymentModel::LocalDirect, two_gpu_devices());
    assert_eq!(orch.device_count().unwrap(), 2);
}

#[test]
fn test_all_usage_empty() {
    let orch = ResourceOrchestrator::new(DeploymentModel::LocalDirect, two_gpu_devices());
    assert!(orch.all_usage().unwrap().is_empty());
}

#[test]
fn test_unregistered_tenant_no_quota_check() {
    let orch = ResourceOrchestrator::new(DeploymentModel::LocalMulti, two_gpu_devices());
    let result = orch.allocate(&test_request("unknown-tenant", 3));
    assert!(result.is_ok());
}

#[test]
fn test_free_vram_calculation() {
    let dev = AvailableDevice {
        index: 0,
        total_vram_bytes: 16_000_000_000,
        allocated_vram_bytes: 4_000_000_000,
        current_tenant: Some("test".into()),
    };
    assert_eq!(dev.free_vram_bytes(), 12_000_000_000);
}

#[test]
fn test_free_vram_saturates() {
    let dev = AvailableDevice {
        index: 0,
        total_vram_bytes: 0,
        allocated_vram_bytes: 100,
        current_tenant: None,
    };
    assert_eq!(dev.free_vram_bytes(), 0);
}
