// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capacity and can_handle tests

use super::common::{make_availability, make_orchestrator_config, make_requirements};
use crate::cloud::UniversalCloudOrchestrator;

#[tokio::test]
async fn test_can_handle_full_job_sufficient_resources() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(8.0, 16.0, 100.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
    assert!(orch.can_handle_full_job(&availability, &requirements));
}

#[tokio::test]
async fn test_can_handle_full_job_insufficient_cpu() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(2.0, 64.0, 500.0);
    let requirements = make_requirements(8.0, 1024 * 1024 * 1024, 1024 * 1024 * 1024);
    assert!(!orch.can_handle_full_job(&availability, &requirements));
}

#[tokio::test]
async fn test_can_handle_full_job_insufficient_memory() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(16.0, 2.0, 500.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 1024 * 1024 * 1024);
    assert!(!orch.can_handle_full_job(&availability, &requirements));
}

#[tokio::test]
async fn test_calculate_provider_capacity_exact_fit() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(4.0, 8.0, 100.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!((cap - 1.0).abs() < 0.01);
}

#[tokio::test]
async fn test_calculate_provider_capacity_cpu_limited() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(8.0, 32.0, 500.0);
    let requirements = make_requirements(16.0, 1024 * 1024 * 1024, 1024 * 1024 * 1024);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!((cap - 0.5).abs() < 0.01);
}

#[tokio::test]
async fn test_calculate_provider_capacity_capped_at_one() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(32.0, 128.0, 1000.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!(cap <= 1.0);
}

#[tokio::test]
async fn test_can_handle_full_job_exact_resources() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(4.0, 8.0, 100.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
    assert!(orch.can_handle_full_job(&availability, &requirements));
}

#[tokio::test]
async fn test_can_handle_full_job_insufficient_storage() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(16.0, 32.0, 10.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
    assert!(!orch.can_handle_full_job(&availability, &requirements));
}

#[tokio::test]
async fn test_calculate_provider_capacity_memory_limited() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(16.0, 4.0, 500.0);
    let requirements = make_requirements(4.0, 16 * 1024 * 1024 * 1024, 1024 * 1024 * 1024);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!((cap - 0.25).abs() < 0.01);
}

#[tokio::test]
async fn test_calculate_provider_capacity_storage_limited() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(32.0, 128.0, 25.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!(cap < 1.0);
}

#[tokio::test]
async fn test_calculate_provider_capacity_zero_requirements() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(1.0, 1.0, 1.0);
    let requirements = make_requirements(0.0, 0, 0);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!(cap <= 1.0);
}

#[tokio::test]
async fn test_calculate_provider_capacity_all_above_requirements() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(100.0, 200.0, 1000.0);
    let requirements = make_requirements(2.0, 1024, 100);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!((cap - 1.0).abs() < 0.01);
}

#[tokio::test]
async fn test_provider_selection_cpu_bottleneck() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(2.0, 128.0, 1000.0);
    let requirements = make_requirements(8.0, 4 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!((cap - 0.25).abs() < 0.01);
}

#[tokio::test]
async fn test_provider_selection_storage_bottleneck() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(64.0, 256.0, 25.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
    assert!(!orch.can_handle_full_job(&availability, &requirements));
}

#[tokio::test]
async fn test_resource_capacity_management_distribute_work() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(8.0, 16.0, 100.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!((cap - 1.0).abs() < 0.01);
}

#[tokio::test]
async fn test_can_handle_full_job_boundary_exactly() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(4.0, 8.0, 100.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
    assert!(orch.can_handle_full_job(&availability, &requirements));
}

#[tokio::test]
async fn test_calculate_provider_capacity_storage_ratio() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(8.0, 32.0, 50.0);
    let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 200 * 1024 * 1024 * 1024);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!(cap < 0.3);
}

#[tokio::test]
async fn test_calculate_provider_capacity_zero_requirements_avoids_division_by_zero() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(8.0, 16.0, 100.0);
    let requirements = make_requirements(0.0, 0, 0);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!(cap.is_finite());
    assert!(cap <= 1.0);
}

#[tokio::test]
async fn test_can_handle_full_job_minimum_boundary() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(1.0, 0.000000001, 0.000000001);
    let requirements = make_requirements(
        1.0, 1, // 1 byte
        1, // 1 byte
    );
    assert!(orch.can_handle_full_job(&availability, &requirements));
}

#[tokio::test]
async fn test_calculate_provider_capacity_inf_avoids_panic() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let availability = make_availability(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let requirements = make_requirements(1.0, 1024, 1024);
    let cap = orch.calculate_provider_capacity(&availability, &requirements);
    assert!(cap <= 1.0);
    assert!(cap.is_finite());
}
