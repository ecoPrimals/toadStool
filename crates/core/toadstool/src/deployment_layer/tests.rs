// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

use super::detector::LayerDetector;
use super::*;

#[tokio::test]
async fn test_layer_detector_creation() {
    let detector = LayerDetector::new();
    assert!(detector.cached_layer.is_none());
}

#[test]
fn test_layer_detector_default() {
    let detector = LayerDetector::default();
    assert!(detector.cached_layer.is_none());
}

#[tokio::test]
async fn test_layer_detector_reset() {
    let mut detector = LayerDetector::new();
    assert!(detector.cached_layer.is_none());
    detector.cached_layer = Some(DeploymentLayer::BareMetalOS);
    assert!(detector.cached_layer.is_some());
    detector.reset();
    assert!(detector.cached_layer.is_none());
}

#[test]
fn test_deployment_layer_description_all_variants() {
    assert_eq!(
        DeploymentLayer::BareMetalOS.description(),
        "Base OS on bare metal"
    );
    assert_eq!(
        DeploymentLayer::MiddlewareLayer {
            host_os: "Ubuntu".to_string(),
            host_version: None,
        }
        .description(),
        "Middleware on host OS"
    );
    assert_eq!(
        DeploymentLayer::CloudLayer {
            provider: CloudProvider::AWS,
            instance_type: None,
            region: None,
        }
        .description(),
        "Cloud environment"
    );
}

#[test]
fn test_deployment_layer_host_os() {
    assert_eq!(DeploymentLayer::BareMetalOS.host_os(), None);
    assert_eq!(
        DeploymentLayer::MiddlewareLayer {
            host_os: "Pop!_OS".to_string(),
            host_version: Some("22.04".to_string()),
        }
        .host_os(),
        Some("Pop!_OS")
    );
}

#[test]
fn test_deployment_layer_is_virtualized() {
    assert!(!DeploymentLayer::BareMetalOS.is_virtualized());
    assert!(
        DeploymentLayer::ContainerLayer {
            runtime: ContainerRuntime::Docker,
            container_id: None,
        }
        .is_virtualized()
    );
}

#[test]
fn test_deployment_layer_has_direct_hardware_access() {
    assert!(DeploymentLayer::BareMetalOS.has_direct_hardware_access());
    assert!(
        DeploymentLayer::VMLayer {
            hypervisor: "QEMU/KVM".to_string(),
            gpu_passthrough: true,
        }
        .has_direct_hardware_access()
    );
}

#[test]
fn test_deployment_layer_display() {
    assert_eq!(format!("{}", DeploymentLayer::BareMetalOS), "BareMetalOS");
    assert_eq!(
        format!(
            "{}",
            DeploymentLayer::CloudLayer {
                provider: CloudProvider::AWS,
                instance_type: Some("t3.micro".to_string()),
                region: Some("us-east-1".to_string()),
            }
        ),
        "Cloud (AWS)"
    );
}

#[test]
fn test_container_runtime_variants() {
    assert_eq!(ContainerRuntime::Docker, ContainerRuntime::Docker);
    assert_eq!(
        ContainerRuntime::Other("custom".to_string()),
        ContainerRuntime::Other("custom".to_string())
    );
}

#[test]
fn test_cloud_provider_variants() {
    assert_eq!(CloudProvider::AWS, CloudProvider::AWS);
    assert_eq!(
        CloudProvider::Custom("Linode".to_string()),
        CloudProvider::Custom("Linode".to_string())
    );
}

#[test]
fn test_deployment_layer_serde_json_roundtrip_bare_metal() {
    let layer = DeploymentLayer::BareMetalOS;
    let json = serde_json::to_string(&layer).unwrap();
    let decoded: DeploymentLayer = serde_json::from_str(&json).unwrap();
    assert_eq!(layer, decoded);
}

#[test]
fn test_detection_error_display() {
    let err = DetectionError::ContainerIdNotFound;
    assert!(err.to_string().contains("Container ID not found"));
    let err = DetectionError::DetectionFailed("custom msg".to_string());
    assert!(err.to_string().contains("custom msg"));
}

#[test]
fn test_detection_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let det_err: DetectionError = io_err.into();
    match &det_err {
        DetectionError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        _ => panic!("expected Io variant"),
    }
}

// ── Full detection pipeline ───────────────────────────────────────────────────

#[tokio::test]
async fn test_detect_returns_a_layer() {
    // On any real machine, detect() must succeed (never panic, never return Err
    // for normal filesystem enumeration).
    let mut detector = LayerDetector::new();
    let layer = detector.detect().await.expect("detect should not fail");
    // Verify the result is one of the known variants.
    let description = layer.description();
    assert!(!description.is_empty());
}

#[tokio::test]
async fn test_detect_caches_result() {
    let mut detector = LayerDetector::new();
    assert!(detector.cached_layer.is_none());

    let layer1 = detector
        .detect()
        .await
        .expect("first detect should succeed");
    assert!(detector.cached_layer.is_some(), "cache should be populated");

    // Second call must return the same variant without re-detecting.
    let layer2 = detector
        .detect()
        .await
        .expect("second detect should succeed");
    assert_eq!(layer1.description(), layer2.description());
}

#[tokio::test]
async fn test_detect_reset_then_redetect() {
    let mut detector = LayerDetector::new();
    let _ = detector.detect().await.expect("detect should succeed");
    assert!(detector.cached_layer.is_some());

    detector.reset();
    assert!(detector.cached_layer.is_none());

    // Detecting again after reset must still work.
    let _ = detector
        .detect()
        .await
        .expect("re-detect after reset should succeed");
    assert!(detector.cached_layer.is_some());
}

// ── Cloud detection via environment variables ─────────────────────────────────
// Uses temp_env for safe, isolated env var testing.

#[tokio::test]
async fn test_detect_aws_via_env() {
    temp_env::async_with_vars([("AWS_EXECUTION_ENV", Some("AWS_ECS_EC2"))], async {
        let mut detector = LayerDetector::new();
        let layer = detector.detect().await.expect("detect should succeed");

        // On a machine that doesn't have /.dockerenv etc., AWS should win.
        if let DeploymentLayer::CloudLayer { provider, .. } = &layer {
            assert!(matches!(provider, CloudProvider::AWS));
        }
        // (On a machine that IS in a container, the container layer wins first —
        // that's correct behaviour; just ensure no panic occurred.)
    })
    .await;
}

#[tokio::test]
async fn test_detect_gcp_via_env() {
    temp_env::async_with_vars([("GCP_PROJECT", Some("my-test-project"))], async {
        let mut detector = LayerDetector::new();
        let _ = detector
            .detect()
            .await
            .expect("detect with GCP env should succeed");
    })
    .await;
}

#[tokio::test]
async fn test_detect_azure_via_env() {
    temp_env::async_with_vars(
        [(
            "AZURE_SUBSCRIPTION_ID",
            Some("aaaaaaaa-0000-0000-0000-aaaaaaaaaaaa"),
        )],
        async {
            let mut detector = LayerDetector::new();
            let _ = detector
                .detect()
                .await
                .expect("detect with Azure env should succeed");
        },
    )
    .await;
}

// ── DeploymentLayer helper methods ────────────────────────────────────────────

#[test]
fn test_deployment_layer_guest_os() {
    let layer = DeploymentLayer::ServiceLayer {
        guest_os: vec![
            "QEMU/KVM guests".to_string(),
            "Docker containers".to_string(),
        ],
    };
    let guests = layer.guest_os().expect("ServiceLayer should have guest_os");
    assert_eq!(guests.len(), 2);
    assert_eq!(guests[0], "QEMU/KVM guests");

    assert!(DeploymentLayer::BareMetalOS.guest_os().is_none());
}

#[test]
fn test_deployment_layer_is_not_virtualized_bare_metal() {
    assert!(!DeploymentLayer::BareMetalOS.is_virtualized());
    assert!(
        !DeploymentLayer::MiddlewareLayer {
            host_os: "Ubuntu".to_string(),
            host_version: None,
        }
        .is_virtualized()
    );
    assert!(
        !DeploymentLayer::ServiceLayer {
            guest_os: vec!["Docker".to_string()],
        }
        .is_virtualized()
    );
}

#[test]
fn test_deployment_layer_vm_no_gpu_passthrough_lacks_direct_hardware() {
    let layer = DeploymentLayer::VMLayer {
        hypervisor: "VirtualBox".to_string(),
        gpu_passthrough: false,
    };
    assert!(!layer.has_direct_hardware_access());
    assert!(layer.is_virtualized());
}

#[test]
fn test_container_layer_is_virtualized_and_lacks_direct_hardware() {
    let layer = DeploymentLayer::ContainerLayer {
        runtime: ContainerRuntime::Podman,
        container_id: Some("abc123".to_string()),
    };
    assert!(layer.is_virtualized());
    assert!(!layer.has_direct_hardware_access());
}

#[test]
fn test_deployment_layer_display_all_variants() {
    assert_eq!(
        format!(
            "{}",
            DeploymentLayer::MiddlewareLayer {
                host_os: "Pop!_OS".to_string(),
                host_version: None,
            }
        ),
        "Middleware on Pop!_OS"
    );
    assert_eq!(
        format!(
            "{}",
            DeploymentLayer::ServiceLayer {
                guest_os: vec!["Docker".to_string(), "QEMU".to_string()],
            }
        ),
        "ServiceLayer (serving: Docker, QEMU)"
    );
    assert_eq!(
        format!(
            "{}",
            DeploymentLayer::ContainerLayer {
                runtime: ContainerRuntime::Docker,
                container_id: None,
            }
        ),
        "Container (Docker)"
    );
    assert_eq!(
        format!(
            "{}",
            DeploymentLayer::VMLayer {
                hypervisor: "QEMU/KVM".to_string(),
                gpu_passthrough: false,
            }
        ),
        "VM (QEMU/KVM)"
    );
}

#[test]
fn test_deployment_layer_serde_roundtrip_all_variants() {
    let layers = vec![
        DeploymentLayer::MiddlewareLayer {
            host_os: "Ubuntu".to_string(),
            host_version: Some("22.04".to_string()),
        },
        DeploymentLayer::ServiceLayer {
            guest_os: vec!["Docker".to_string()],
        },
        DeploymentLayer::ContainerLayer {
            runtime: ContainerRuntime::Containerd,
            container_id: Some("deadbeef".to_string()),
        },
        DeploymentLayer::VMLayer {
            hypervisor: "KVM".to_string(),
            gpu_passthrough: true,
        },
        DeploymentLayer::CloudLayer {
            provider: CloudProvider::GCP,
            instance_type: Some("n1-standard-1".to_string()),
            region: Some("us-central1".to_string()),
        },
    ];
    for layer in layers {
        let json = serde_json::to_string(&layer).unwrap();
        let decoded: DeploymentLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(layer, decoded);
    }
}

#[test]
fn test_detection_error_http_disabled_display() {
    let err = DetectionError::ExternalHttpDisabled;
    assert!(err.to_string().contains("External HTTP detection disabled"));
}
