// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability discovery and error-path tests.

use std::collections::HashMap;
use std::sync::Arc;

use toadstool::universal::{
    JobPriority, PrimalCapability, UniversalPrimalRegistry, UniversalScheduler,
};

use super::helpers::{create_test_context, FailingMockProvider};

// ── Capability discovery ─────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_primals_by_native_capability() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::new()))
        .await
        .unwrap();
    let capability = PrimalCapability::NativeExecution {
        architectures: vec!["x86_64".to_string()],
    };
    // Returns empty list when no providers registered — the API must not panic.
    let primals = scheduler.find_primals_by_capability(&capability).await;
    let _ = primals.len();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_wasm_capability() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::new()))
        .await
        .unwrap();
    let capability = PrimalCapability::WasmExecution { wasi_support: true };
    let primals = scheduler.find_primals_by_capability(&capability).await;
    let _ = primals.len();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_container_runtime_capability() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::new()))
        .await
        .unwrap();
    let capability = PrimalCapability::ContainerRuntime {
        orchestrators: vec!["docker".to_string()],
    };
    let _ = scheduler.find_primals_by_capability(&capability).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_gpu_capability() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::new()))
        .await
        .unwrap();
    let capability = PrimalCapability::GpuAcceleration { cuda_support: true };
    let _ = scheduler.find_primals_by_capability(&capability).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_find_custom_capability() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::new()))
        .await
        .unwrap();
    let capability = PrimalCapability::Custom {
        name: "custom-analytics".to_string(),
        attributes: HashMap::new(),
    };
    let _ = scheduler.find_primals_by_capability(&capability).await;
}

// ── Error paths ──────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_native_job_fails_when_provider_returns_error() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    registry
        .register_primal(Arc::new(FailingMockProvider {
            instance_id: "failing-native".to_string(),
            context: create_test_context(),
        }))
        .await
        .unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let result = scheduler
        .schedule_job(super::helpers::create_test_native_job(JobPriority::Normal))
        .await;
    assert!(
        result.is_err(),
        "Schedule should fail when provider returns error"
    );
}
