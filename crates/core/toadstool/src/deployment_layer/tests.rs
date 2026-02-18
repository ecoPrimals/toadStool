// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::detector::LayerDetector;
    use super::super::*;

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
        assert!(DeploymentLayer::ContainerLayer {
            runtime: ContainerRuntime::Docker,
            container_id: None,
        }
        .is_virtualized());
    }

    #[test]
    fn test_deployment_layer_has_direct_hardware_access() {
        assert!(DeploymentLayer::BareMetalOS.has_direct_hardware_access());
        assert!(DeploymentLayer::VMLayer {
            hypervisor: "QEMU/KVM".to_string(),
            gpu_passthrough: true,
        }
        .has_direct_hardware_access());
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
}
