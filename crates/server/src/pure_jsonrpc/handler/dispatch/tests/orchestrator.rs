// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for ResourceOrchestrator integration with DispatchHandler.

use std::sync::Arc;
use std::time::Duration;

use super::{DispatchHandler, test_handler};
use toadstool_common::constants::jsonrpc::error_codes;
use toadstool_runtime_orchestration::{
    AvailableDevice, DeploymentModel, GuestLoadPolicy, ResourceOrchestrator, TenantQuota,
    YieldStrategy,
};

fn multi_handler(devices: Vec<AvailableDevice>) -> DispatchHandler {
    let mut handler = test_handler();
    let orchestrator = ResourceOrchestrator::new(DeploymentModel::LocalMulti, devices);
    handler.set_resource_orchestrator(Arc::new(orchestrator));
    handler
}

fn fake_gpu(index: u32) -> AvailableDevice {
    AvailableDevice {
        index,
        total_vram_bytes: 8 * 1024 * 1024 * 1024,
        allocated_vram_bytes: 0,
        current_tenant: None,
    }
}

// --- no-op when orchestrator is None ---

#[tokio::test]
async fn no_orchestrator_pre_dispatch_is_noop() {
    let handler = test_handler();
    let result = handler.pre_dispatch_resource_check("0000:03:00.0");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

// --- basic allocation succeeds ---

#[tokio::test]
async fn orchestrator_allows_dispatch_within_quota() {
    let handler = multi_handler(vec![fake_gpu(0)]);
    let orch = handler.resource_orchestrator.as_ref().unwrap();
    orch.register_tenant("anonymous", TenantQuota::default())
        .unwrap();

    let result = handler.pre_dispatch_resource_check("0000:03:00.0");
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

// --- guest load reject yields -32003 ---

#[tokio::test]
async fn guest_load_reject_returns_capability_not_available() {
    let handler = multi_handler(vec![fake_gpu(0)]);
    let orch = handler.resource_orchestrator.as_ref().unwrap();
    orch.register_tenant(
        "anonymous",
        TenantQuota {
            max_guest_load: Some(GuestLoadPolicy {
                max_concurrent_gpu: 0,
                yield_strategy: YieldStrategy::Reject,
            }),
            ..TenantQuota::default()
        },
    )
    .unwrap();

    let err = handler
        .pre_dispatch_resource_check("0000:03:00.0")
        .expect_err("should reject");
    assert_eq!(err.code, error_codes::CAPABILITY_NOT_AVAILABLE);
}

// --- guest load queue yields -32003 (queued, not an allocation) ---

#[tokio::test]
async fn guest_load_queue_returns_capability_not_available() {
    let handler = multi_handler(vec![fake_gpu(0)]);
    let orch = handler.resource_orchestrator.as_ref().unwrap();
    orch.register_tenant(
        "anonymous",
        TenantQuota {
            max_guest_load: Some(GuestLoadPolicy {
                max_concurrent_gpu: 0,
                yield_strategy: YieldStrategy::Queue,
            }),
            ..TenantQuota::default()
        },
    )
    .unwrap();

    let err = handler
        .pre_dispatch_resource_check("0000:03:00.0")
        .expect_err("should queue");
    assert_eq!(err.code, error_codes::CAPABILITY_NOT_AVAILABLE);
}

// --- quota exceeded yields -32004 ---

#[tokio::test]
async fn quota_exceeded_returns_resource_exhausted() {
    let handler = multi_handler(vec![fake_gpu(0)]);
    let orch = handler.resource_orchestrator.as_ref().unwrap();
    orch.register_tenant(
        "anonymous",
        TenantQuota {
            max_devices: 0,
            max_vram_bytes: 0,
            max_concurrent_workloads: 0,
            max_compute_time: Duration::ZERO,
            max_guest_load: None,
        },
    )
    .unwrap();

    let err = handler
        .pre_dispatch_resource_check("0000:03:00.0")
        .expect_err("should reject quota");
    assert_eq!(err.code, error_codes::RESOURCE_EXHAUSTED);
}

// --- local direct handler has no orchestrator overhead ---

#[tokio::test]
async fn local_direct_handler_has_no_orchestrator() {
    let handler = test_handler();
    assert!(handler.resource_orchestrator.is_none());
    let result = handler.pre_dispatch_resource_check("0000:03:00.0");
    assert!(result.unwrap().is_none());
}

// --- deployment model is accessible through orchestrator ---

#[tokio::test]
async fn multi_handler_reports_local_multi_model() {
    let handler = multi_handler(vec![fake_gpu(0)]);
    let orch = handler.resource_orchestrator.as_ref().unwrap();
    assert_eq!(orch.deployment_model(), DeploymentModel::LocalMulti);
}

// --- guest load under threshold allows dispatch ---

#[tokio::test]
async fn guest_load_under_threshold_allows_dispatch() {
    let handler = multi_handler(vec![fake_gpu(0), fake_gpu(1)]);
    let orch = handler.resource_orchestrator.as_ref().unwrap();
    orch.register_tenant(
        "anonymous",
        TenantQuota {
            max_guest_load: Some(GuestLoadPolicy {
                max_concurrent_gpu: 2,
                yield_strategy: YieldStrategy::Reject,
            }),
            ..TenantQuota::default()
        },
    )
    .unwrap();

    let result = handler.pre_dispatch_resource_check("0000:03:00.0");
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

// --- DeferUntilPowerCycle yields -32003 ---

#[tokio::test]
async fn guest_load_defer_power_cycle_returns_error() {
    let handler = multi_handler(vec![fake_gpu(0)]);
    let orch = handler.resource_orchestrator.as_ref().unwrap();
    orch.register_tenant(
        "anonymous",
        TenantQuota {
            max_guest_load: Some(GuestLoadPolicy {
                max_concurrent_gpu: 0,
                yield_strategy: YieldStrategy::DeferUntilPowerCycle,
            }),
            ..TenantQuota::default()
        },
    )
    .unwrap();

    let err = handler
        .pre_dispatch_resource_check("0000:03:00.0")
        .expect_err("should defer");
    assert_eq!(err.code, error_codes::CAPABILITY_NOT_AVAILABLE);
}
