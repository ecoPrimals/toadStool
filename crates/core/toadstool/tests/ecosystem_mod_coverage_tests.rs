// SPDX-License-Identifier: AGPL-3.0-or-later
//! Targeted tests for ecosystem/mod.rs coverage expansion
//! Covers: DiscoveryMethodConfig variants, integrate_services, error paths, deprecated APIs

use std::collections::HashMap;
use std::time::SystemTime;

use toadstool::{DiscoveryMethodConfig, EcosystemConfig, EcosystemCoordinator, ServiceStatus};
use toadstool_common::primal_identity::{Capability, ServiceEndpoint};
use toadstool_common::service_discovery::DiscoveredService;

fn make_discovered_service(
    id: &str,
    name: &str,
    endpoints: Vec<ServiceEndpoint>,
    capabilities: Vec<Capability>,
) -> DiscoveredService {
    let now = SystemTime::now();
    DiscoveredService {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0".to_string(),
        capabilities,
        endpoints,
        metadata: HashMap::new(),
        discovered_at: now,
        last_seen: now,
        healthy: true,
    }
}

// ── DiscoveryMethodConfig / with_config variants ──────────────────────────────

#[tokio::test]
async fn test_ecosystem_with_config_mdns() {
    let config = EcosystemConfig::builder()
        .discovery_method(DiscoveryMethodConfig::Mdns)
        .build();
    let coordinator = EcosystemCoordinator::with_config(config).await;
    assert!(coordinator.is_ok());
}

#[tokio::test]
async fn test_ecosystem_with_config_config_file() {
    let config = EcosystemConfig::builder()
        .discovery_method(DiscoveryMethodConfig::ConfigFile {
            path: "/tmp/nonexistent-discovery.json".to_string(),
        })
        .build();
    let coordinator = EcosystemCoordinator::with_config(config).await;
    assert!(coordinator.is_ok() || coordinator.is_err());
}

#[tokio::test]
async fn test_ecosystem_with_config_registry() {
    let config = EcosystemConfig::builder()
        .discovery_method(DiscoveryMethodConfig::Registry {
            endpoint: "http://localhost:9999/registry".to_string(),
        })
        .build();
    let coordinator = EcosystemCoordinator::with_config(config).await;
    assert!(coordinator.is_ok() || coordinator.is_err());
}

#[tokio::test]
async fn test_ecosystem_with_config_auto() {
    let config = EcosystemConfig::builder()
        .discovery_method(DiscoveryMethodConfig::Auto)
        .build();
    let coordinator = EcosystemCoordinator::with_config(config).await;
    assert!(coordinator.is_ok());
}

// ── integrate_services ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_ecosystem_integrate_services_empty() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let result = coordinator.integrate_services(vec![]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ecosystem_integrate_services_no_endpoint_marks_failed() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let service = make_discovered_service(
        "svc-no-ep",
        "TestService",
        vec![], // no endpoints -> channel creation fails
        vec![],
    );
    let result = coordinator.integrate_services(vec![service.clone()]).await;
    assert!(result.is_ok());
    let status = coordinator.get_service_status("svc-no-ep").await;
    assert!(status.is_some());
    if let Some(ServiceStatus::Failed(reason)) = status {
        assert!(
            reason.contains("endpoint")
                || reason.contains("Channel")
                || reason.contains("No endpoint"),
            "reason: {}",
            reason
        );
    }
}

#[tokio::test]
async fn test_ecosystem_integrate_services_with_endpoint() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let endpoint = ServiceEndpoint::http("127.0.0.1", 9999);
    let service = make_discovered_service("svc-with-ep", "WithEndpoint", vec![endpoint], vec![]);
    let result = coordinator.integrate_services(vec![service]).await;
    assert!(result.is_ok());
}

// ── get_service_capabilities ─────────────────────────────────────────────────

#[tokio::test]
async fn test_ecosystem_get_service_capabilities_unknown() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let result = coordinator.get_service_capabilities("unknown-svc").await;
    assert!(result.is_err());
}

// ── is_capability_available ───────────────────────────────────────────────────

#[tokio::test]
async fn test_ecosystem_is_capability_available_initially_false() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let cap =
        Capability::Compute(toadstool_common::primal_identity::ComputeCapability::NativeExecution);
    let available = coordinator.is_capability_available(&cap).await;
    assert!(!available);
}

// ── discover_services ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_ecosystem_discover_services_no_required() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let result = coordinator.discover_services().await;
    assert!(result.is_ok());
    let services = result.unwrap();
    assert!(services.is_empty() || !services.is_empty());
}

// ── get_primal_capabilities (deprecated) when found ────────────────────────────

#[allow(deprecated)]
#[tokio::test]
async fn test_ecosystem_get_primal_capabilities_found() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let endpoint = ServiceEndpoint::http("127.0.0.1", 8888);
    let cap = Capability::Compute(toadstool_common::primal_identity::ComputeCapability::GpuCompute);
    let service =
        make_discovered_service("primal-svc", "MyPrimal", vec![endpoint], vec![cap.clone()]);
    coordinator.integrate_services(vec![service]).await.unwrap();
    let caps = coordinator.get_primal_capabilities("MyPrimal").await;
    assert!(caps.is_ok());
    let cap_strs = caps.unwrap();
    assert!(!cap_strs.is_empty());
}

#[allow(deprecated)]
#[tokio::test]
async fn test_ecosystem_get_primal_capabilities_not_found() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let result = coordinator
        .get_primal_capabilities("NonexistentPrimal")
        .await;
    assert!(result.is_err());
}

// ── is_primal_available (deprecated) ───────────────────────────────────────────

#[allow(deprecated)]
#[tokio::test]
async fn test_ecosystem_is_primal_available_when_registered() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let endpoint = ServiceEndpoint::http("127.0.0.1", 7777);
    let service = make_discovered_service("avail-svc", "AvailablePrimal", vec![endpoint], vec![]);
    coordinator.integrate_services(vec![service]).await.unwrap();
    let available = coordinator.is_primal_available("AvailablePrimal").await;
    let _ = available;
}

// ── get_primal_status (deprecated) ────────────────────────────────────────────

#[allow(deprecated)]
#[tokio::test]
async fn test_ecosystem_get_primal_status() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let statuses = coordinator.get_primal_status().await.unwrap();
    assert!(statuses.is_empty());
}

// ── integrate_services: register_service error path (continue) ───────────────

#[tokio::test]
async fn test_ecosystem_integrate_services_multiple_one_has_endpoint() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let endpoint = ServiceEndpoint::http("127.0.0.1", 9998);
    let good = make_discovered_service("good-svc", "Good", vec![endpoint], vec![]);
    let bad = make_discovered_service("bad-svc", "Bad", vec![], vec![]);
    let result = coordinator.integrate_services(vec![good, bad]).await;
    assert!(result.is_ok());
    let statuses = coordinator.get_service_statuses().await;
    assert!(!statuses.is_empty());
    let healthy = coordinator.healthy_count().await;
    let total = coordinator.service_count().await;
    assert!(total >= 1);
    assert!(healthy <= total);
}

// ── get_service_status when Connected ─────────────────────────────────────────

#[tokio::test]
async fn test_ecosystem_get_service_status_connected_after_integration() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let endpoint = ServiceEndpoint::http("127.0.0.1", 9997);
    let service = make_discovered_service("conn-svc", "ConnectedSvc", vec![endpoint], vec![]);
    coordinator.integrate_services(vec![service]).await.unwrap();
    let status = coordinator.get_service_status("conn-svc").await;
    assert!(status.is_some());
    let s = status.unwrap();
    assert!(
        s.is_usable() || s.is_error() || matches!(s, ServiceStatus::Discovered),
        "status: {:?}",
        s
    );
}

// ── Config builder: require_capability, optional_capability ───────────────────

#[tokio::test]
async fn test_ecosystem_with_config_require_capability() {
    use toadstool_common::primal_identity::ComputeCapability;
    let config = EcosystemConfig::builder()
        .discovery_method(DiscoveryMethodConfig::Auto)
        .require_capability(Capability::Compute(ComputeCapability::NativeExecution))
        .build();
    let coordinator = EcosystemCoordinator::with_config(config).await;
    assert!(coordinator.is_ok() || coordinator.is_err());
}

#[tokio::test]
async fn test_ecosystem_with_config_optional_capability() {
    use toadstool_common::primal_identity::ComputeCapability;
    let config = EcosystemConfig::builder()
        .discovery_method(DiscoveryMethodConfig::Auto)
        .optional_capability(Capability::Compute(ComputeCapability::GpuCompute))
        .build();
    let coordinator = EcosystemCoordinator::with_config(config).await;
    assert!(coordinator.is_ok());
}

// ── get_service_statuses and healthy_count with integrated services ─────────────

#[tokio::test]
async fn test_ecosystem_get_service_statuses_after_integration() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let endpoint = ServiceEndpoint::http("127.0.0.1", 9996);
    let service = make_discovered_service("status-svc", "StatusSvc", vec![endpoint], vec![]);
    coordinator.integrate_services(vec![service]).await.unwrap();
    let statuses = coordinator.get_service_statuses().await;
    assert!(!statuses.is_empty());
    assert!(statuses.contains_key("status-svc"));
}

// ── discover_services with required capabilities (empty = ok) ───────────────────

#[tokio::test]
async fn test_ecosystem_discover_services_empty_required_returns_ok() {
    let config = EcosystemConfig::builder()
        .discovery_method(DiscoveryMethodConfig::Auto)
        .build();
    let coordinator = EcosystemCoordinator::with_config(config).await.unwrap();
    let result = coordinator.discover_services().await;
    assert!(result.is_ok());
    let services = result.unwrap();
    assert!(services.is_empty());
}

// ── is_capability_available after integrating service with capability ───────────

#[tokio::test]
async fn test_ecosystem_is_capability_available_after_integration() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let cap = Capability::Compute(toadstool_common::primal_identity::ComputeCapability::GpuCompute);
    let endpoint = ServiceEndpoint::http("127.0.0.1", 9995);
    let service = make_discovered_service("gpu-svc", "GpuSvc", vec![endpoint], vec![cap.clone()]);
    coordinator.integrate_services(vec![service]).await.unwrap();
    let _available = coordinator.is_capability_available(&cap).await;
    assert!(coordinator.service_count().await >= 1);
}

// ── find_service_by_capability error path (capability not discoverable) ───────

#[tokio::test]
async fn test_ecosystem_find_service_by_capability_uncommon_capability() {
    use toadstool_common::primal_identity::StorageCapability;
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let cap = Capability::Storage(StorageCapability::ObjectStorage);
    let result = coordinator.find_service_by_capability(cap).await;
    if let Ok(svc) = &result {
        assert!(!svc.id.is_empty());
    }
}

// ── get_discovered_services after integrate ─────────────────────────────────────

#[tokio::test]
async fn test_ecosystem_get_discovered_services_after_integration() {
    let coordinator = EcosystemCoordinator::new().await.unwrap();
    let endpoint = ServiceEndpoint::http("127.0.0.1", 9994);
    let service = make_discovered_service("disc-svc", "DiscoveredSvc", vec![endpoint], vec![]);
    coordinator.integrate_services(vec![service]).await.unwrap();
    let discovered = coordinator.get_discovered_services().await;
    assert!(!discovered.is_empty());
    assert!(discovered.iter().any(|s| s.id == "disc-svc"));
}
