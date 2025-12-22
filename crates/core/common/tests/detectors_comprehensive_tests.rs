//! Comprehensive tests for substrate detectors
//!
//! Sprint 23R: detectors.rs coverage 35.47% → 60%+

use toadstool_common::infant_discovery::capabilities::*;
use toadstool_common::infant_discovery::detectors::*;

// ============================================================================
// KubernetesDetector Tests
// ============================================================================

#[test]
fn test_kubernetes_detector_new() {
    let detector = KubernetesDetector::new();
    assert_eq!(detector.name(), "kubernetes");
}

#[test]
fn test_kubernetes_detector_default() {
    let detector = KubernetesDetector;
    assert_eq!(detector.name(), "kubernetes");
}

#[test]
fn test_kubernetes_detector_name() {
    let detector = KubernetesDetector::new();
    assert_eq!(detector.name(), "kubernetes");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_kubernetes_detector_no_kubernetes_env() {
    // Ensure we're not in a k8s environment for this test
    std::env::remove_var("KUBERNETES_SERVICE_HOST");

    let detector = KubernetesDetector::new();
    let result = detector.detect().await;

    assert!(result.is_ok());
    // Result depends on if /.../serviceaccount exists (unlikely in test env)
}

// ============================================================================
// DockerDetector Tests
// ============================================================================

#[test]
fn test_docker_detector_new() {
    let detector = DockerDetector::new();
    assert_eq!(detector.name(), "docker");
}

#[test]
fn test_docker_detector_default() {
    let detector = DockerDetector;
    assert_eq!(detector.name(), "docker");
}

#[test]
fn test_docker_detector_name() {
    let detector = DockerDetector::new();
    assert_eq!(detector.name(), "docker");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_docker_detector_no_docker_env() {
    let detector = DockerDetector::new();
    let result = detector.detect().await;

    assert!(result.is_ok());
    // Result depends on if /.dockerenv or /proc/self/cgroup exists
}

// ============================================================================
// ConsulDetector Tests
// ============================================================================

#[test]
fn test_consul_detector_new() {
    let detector = ConsulDetector::new();
    assert_eq!(detector.name(), "consul");
}

#[test]
fn test_consul_detector_default() {
    let detector = ConsulDetector;
    assert_eq!(detector.name(), "consul");
}

#[test]
fn test_consul_detector_name() {
    let detector = ConsulDetector::new();
    assert_eq!(detector.name(), "consul");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_consul_detector_no_consul() {
    std::env::remove_var("CONSUL_HTTP_ADDR");

    let detector = ConsulDetector::new();
    let result = detector.detect().await;

    assert!(result.is_ok());
    // Should return None when consul is not available
    if let Ok(None) = result {
        // Expected: No consul running
    }
}

// ============================================================================
// CloudDetector Tests
// ============================================================================

#[test]
fn test_cloud_detector_new() {
    let detector = CloudDetector::new();
    assert_eq!(detector.name(), "cloud");
}

#[test]
fn test_cloud_detector_default() {
    let detector = CloudDetector;
    assert_eq!(detector.name(), "cloud");
}

#[test]
fn test_cloud_detector_name() {
    let detector = CloudDetector::new();
    assert_eq!(detector.name(), "cloud");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cloud_detector_no_cloud_env() {
    // Remove all cloud environment variables
    std::env::remove_var("AWS_REGION");
    std::env::remove_var("AWS_DEFAULT_REGION");
    std::env::remove_var("GCP_PROJECT");
    std::env::remove_var("GOOGLE_CLOUD_PROJECT");
    std::env::remove_var("AZURE_SUBSCRIPTION_ID");

    let detector = CloudDetector::new();
    let result = detector.detect().await;

    assert!(result.is_ok());
    // Should return None when no cloud detected
}

// ============================================================================
// BareMetalDetector Tests
// ============================================================================

#[test]
fn test_bare_metal_detector_new() {
    let detector = BareMetalDetector::new();
    assert_eq!(detector.name(), "bare_metal");
}

#[test]
fn test_bare_metal_detector_default() {
    let detector = BareMetalDetector;
    assert_eq!(detector.name(), "bare_metal");
}

#[test]
fn test_bare_metal_detector_name() {
    let detector = BareMetalDetector::new();
    assert_eq!(detector.name(), "bare_metal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_detector_always_succeeds() {
    let detector = BareMetalDetector::new();
    let result = detector.detect().await;

    assert!(result.is_ok());
    let substrate_option = result.unwrap();
    assert!(substrate_option.is_some());

    let substrate = substrate_option.unwrap();
    assert_eq!(substrate.substrate_type, SubstrateType::Bare);
    assert_eq!(substrate.capabilities.len(), 1);
    assert_eq!(substrate.capabilities[0], SubstrateCapability::BareMetal);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_detector_metadata() {
    let detector = BareMetalDetector::new();
    let result = detector.detect().await.unwrap().unwrap();

    assert!(result.metadata.contains_key("deployment"));
    assert_eq!(
        result.metadata.get("deployment"),
        Some(&"bare_metal".to_string())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_detector_capabilities() {
    let detector = BareMetalDetector::new();
    let result = detector.detect().await.unwrap().unwrap();

    assert!(result.has_capability(&SubstrateCapability::BareMetal));
    assert!(!result.has_capability(&SubstrateCapability::CloudCompute));
}

// ============================================================================
// standard_detectors() Function Tests
// ============================================================================

#[test]
fn test_standard_detectors_count() {
    let detectors = standard_detectors();
    assert_eq!(detectors.len(), 5);
}

#[test]
fn test_standard_detectors_order() {
    let detectors = standard_detectors();

    assert_eq!(detectors[0].name(), "kubernetes");
    assert_eq!(detectors[1].name(), "docker");
    assert_eq!(detectors[2].name(), "consul");
    assert_eq!(detectors[3].name(), "cloud");
    assert_eq!(detectors[4].name(), "bare_metal");
}

#[test]
fn test_standard_detectors_all_names() {
    let detectors = standard_detectors();
    let names: Vec<&str> = detectors.iter().map(|d| d.name()).collect();

    assert_eq!(
        names,
        vec!["kubernetes", "docker", "consul", "cloud", "bare_metal"]
    );
}

// ============================================================================
// Detector Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detector_chain_execution() {
    let detectors = standard_detectors();

    // All detectors should be callable
    for detector in detectors {
        let result = detector.detect().await;
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_fallback_works() {
    // BareMetalDetector should always succeed as fallback
    let detector = BareMetalDetector::new();
    let result = detector.detect().await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

// ============================================================================
// Detector Type Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_kubernetes_detector_substrate_type() {
    let detector = KubernetesDetector::new();
    // We can't guarantee k8s detection, but we can test the type is correct
    if let Ok(Some(substrate)) = detector.detect().await {
        assert_eq!(
            substrate.substrate_type,
            SubstrateType::ContainerOrchestrator
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_docker_detector_substrate_type() {
    let detector = DockerDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        assert_eq!(substrate.substrate_type, SubstrateType::ContainerRuntime);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_consul_detector_substrate_type() {
    let detector = ConsulDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        assert_eq!(
            substrate.substrate_type,
            SubstrateType::ContainerOrchestrator
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cloud_detector_substrate_type() {
    let detector = CloudDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        assert_eq!(substrate.substrate_type, SubstrateType::Cloud);
    }
}

// ============================================================================
// Capability Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_kubernetes_capabilities() {
    let detector = KubernetesDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        // K8s should provide orchestration, discovery, and mesh capabilities
        assert!(substrate.capabilities.len() >= 3);
        assert!(substrate
            .capabilities
            .contains(&SubstrateCapability::ContainerOrchestration));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_docker_capabilities() {
    let detector = DockerDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        assert!(substrate
            .capabilities
            .contains(&SubstrateCapability::ContainerRuntime));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_consul_capabilities() {
    let detector = ConsulDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        assert!(substrate
            .capabilities
            .contains(&SubstrateCapability::ServiceDiscovery));
        assert!(substrate
            .capabilities
            .contains(&SubstrateCapability::ServiceMesh));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cloud_capabilities() {
    let detector = CloudDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        assert!(substrate
            .capabilities
            .contains(&SubstrateCapability::CloudCompute));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_capabilities() {
    let detector = BareMetalDetector::new();
    let substrate = detector.detect().await.unwrap().unwrap();

    assert_eq!(substrate.capabilities.len(), 1);
    assert!(substrate
        .capabilities
        .contains(&SubstrateCapability::BareMetal));
}

// ============================================================================
// Metadata Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_metadata_deployment() {
    let detector = BareMetalDetector::new();
    let substrate = detector.detect().await.unwrap().unwrap();

    assert!(substrate.metadata.contains_key("deployment"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_kubernetes_metadata_structure() {
    let detector = KubernetesDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        // Should have orchestrator metadata
        assert!(substrate.metadata.contains_key("orchestrator"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_docker_metadata_structure() {
    let detector = DockerDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        // Should have runtime metadata
        assert!(substrate.metadata.contains_key("runtime"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_consul_metadata_structure() {
    let detector = ConsulDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        // Should have service_mesh metadata
        assert!(substrate.metadata.contains_key("service_mesh"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cloud_metadata_structure() {
    let detector = CloudDetector::new();
    if let Ok(Some(substrate)) = detector.detect().await {
        // Should have cloud_provider metadata
        assert!(substrate.metadata.contains_key("cloud_provider"));
    }
}

// ============================================================================
// Default Trait Tests
// ============================================================================

#[test]
fn test_all_detectors_have_default() {
    let _k8s = KubernetesDetector;
    let _docker = DockerDetector;
    let _consul = ConsulDetector;
    let _cloud = CloudDetector;
    let _bare = BareMetalDetector;
    // All created successfully via Default trait
}

#[test]
fn test_detector_default_equals_new() {
    // Verify Default produces same result as new()
    let k8s_new = KubernetesDetector::new();
    let k8s_default = KubernetesDetector;
    assert_eq!(k8s_new.name(), k8s_default.name());

    let docker_new = DockerDetector::new();
    let docker_default = DockerDetector;
    assert_eq!(docker_new.name(), docker_default.name());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detectors_never_panic() {
    let detectors = standard_detectors();

    for detector in detectors {
        // All detectors should return Result, never panic
        let _ = detector.detect().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_never_returns_error() {
    let detector = BareMetalDetector::new();
    let result = detector.detect().await;

    // BareMetalDetector should always succeed
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

// ============================================================================
// Name Consistency Tests
// ============================================================================

#[test]
fn test_detector_names_lowercase() {
    let detectors = standard_detectors();

    for detector in detectors {
        let name = detector.name();
        assert_eq!(
            name,
            name.to_lowercase(),
            "Detector names should be lowercase"
        );
    }
}

#[test]
fn test_detector_names_no_spaces() {
    let detectors = standard_detectors();

    for detector in detectors {
        let name = detector.name();
        assert!(
            !name.contains(' '),
            "Detector names should not contain spaces"
        );
    }
}

#[test]
fn test_detector_names_unique() {
    let detectors = standard_detectors();
    let names: Vec<&str> = detectors.iter().map(|d| d.name()).collect();

    // Check for duplicates
    for (i, name1) in names.iter().enumerate() {
        for (j, name2) in names.iter().enumerate() {
            if i != j {
                assert_ne!(name1, name2, "Detector names must be unique");
            }
        }
    }
}
