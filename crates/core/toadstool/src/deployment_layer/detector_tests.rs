// SPDX-License-Identifier: AGPL-3.0-or-later


use super::*;
use crate::deployment_layer::{CloudProvider, DeploymentLayer};

#[test]
fn test_layer_detector_new() {
    let detector = LayerDetector::new();
    assert!(detector.cached_layer.is_none());
}

#[test]
fn test_layer_detector_default() {
    let detector = LayerDetector::default();
    assert!(detector.cached_layer.is_none());
}

#[test]
fn test_layer_detector_reset() {
    let mut detector = LayerDetector::new();
    detector.cached_layer = Some(DeploymentLayer::BareMetalOS);
    detector.reset();
    assert!(detector.cached_layer.is_none());
}

#[tokio::test]
async fn test_layer_detector_detect_returns_valid_layer() {
    let mut detector = LayerDetector::new();
    let result = detector.detect().await;
    assert!(result.is_ok());
    let layer = result.unwrap();
    assert!(matches!(
        layer,
        DeploymentLayer::BareMetalOS
            | DeploymentLayer::ContainerLayer { .. }
            | DeploymentLayer::CloudLayer { .. }
            | DeploymentLayer::VMLayer { .. }
            | DeploymentLayer::MiddlewareLayer { .. }
            | DeploymentLayer::ServiceLayer { .. }
    ));
}

#[tokio::test]
async fn test_layer_detector_caches_result() {
    let mut detector = LayerDetector::new();
    let first = detector.detect().await.unwrap();
    let second = detector.detect().await.unwrap();
    assert_eq!(
        std::mem::discriminant(&first),
        std::mem::discriminant(&second)
    );
}

#[test]
fn test_parse_os_release_extracts_name_and_version() {
    let detector = LayerDetector::new();
    let content = r#"NAME="Ubuntu"
VERSION="22.04 LTS""#;
    let (name, version) = detector.parse_os_release(content);
    assert_eq!(name, "Ubuntu");
    assert_eq!(version, Some("22.04 LTS".to_string()));
}

#[test]
fn test_parse_os_release_name_only() {
    let detector = LayerDetector::new();
    let content = r#"NAME="Pop!_OS"
ID=pop"#;
    let (name, version) = detector.parse_os_release(content);
    assert_eq!(name, "Pop!_OS");
    assert_eq!(version, None);
}

#[test]
fn test_parse_os_release_empty() {
    let detector = LayerDetector::new();
    let (name, version) = detector.parse_os_release("");
    assert_eq!(name, "");
    assert_eq!(version, None);
}

#[test]
fn test_parse_os_release_version_only() {
    let detector = LayerDetector::new();
    let content = r#"VERSION="20.04"
ID=ubuntu"#;
    let (name, version) = detector.parse_os_release(content);
    assert_eq!(name, "");
    assert_eq!(version, Some("20.04".to_string()));
}

#[test]
fn test_parse_os_release_quoted_values() {
    let detector = LayerDetector::new();
    let content = r#"NAME="Fedora Linux"
VERSION="39 (Container Image)""#;
    let (name, version) = detector.parse_os_release(content);
    assert_eq!(name, "Fedora Linux");
    assert_eq!(version, Some("39 (Container Image)".to_string()));
}

#[tokio::test]
async fn test_detect_aws_cloud_layer_via_env() {
    temp_env::async_with_vars([("AWS_EXECUTION_ENV", Some("AWS_Lambda_rust"))], async {
        let mut detector = LayerDetector::new();
        let result = detector.detect().await;
        assert!(result.is_ok());
        let layer = result.unwrap();
        assert!(
            matches!(
                layer,
                DeploymentLayer::CloudLayer {
                    provider: CloudProvider::AWS,
                    ..
                }
            ),
            "expected AWS CloudLayer, got {layer:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn test_detect_gcp_cloud_layer_via_env() {
    temp_env::async_with_vars(
        [
            ("AWS_EXECUTION_ENV", None::<&str>),
            ("AWS_LAMBDA_FUNCTION_NAME", None::<&str>),
            ("ECS_CONTAINER_METADATA_URI", None::<&str>),
            ("GCP_PROJECT", Some("my-project")),
        ],
        async {
            let mut detector = LayerDetector::new();
            let result = detector.detect().await;
            assert!(result.is_ok());
            let layer = result.unwrap();
            assert!(
                matches!(
                    layer,
                    DeploymentLayer::CloudLayer {
                        provider: CloudProvider::GCP,
                        ..
                    }
                ),
                "expected GCP CloudLayer, got {layer:?}"
            );
        },
    )
    .await;
}

#[tokio::test]
async fn test_detect_azure_cloud_layer_via_env() {
    temp_env::async_with_vars(
        [
            ("AWS_EXECUTION_ENV", None::<&str>),
            ("GCP_PROJECT", None::<&str>),
            ("AZURE_SUBSCRIPTION_ID", Some("sub-123")),
        ],
        async {
            let mut detector = LayerDetector::new();
            let result = detector.detect().await;
            assert!(result.is_ok());
            let layer = result.unwrap();
            assert!(
                matches!(
                    layer,
                    DeploymentLayer::CloudLayer {
                        provider: CloudProvider::Azure,
                        ..
                    }
                ),
                "expected Azure CloudLayer, got {layer:?}"
            );
        },
    )
    .await;
}

#[test]
fn test_detect_reset_clears_cache() {
    let mut detector = LayerDetector::new();
    detector.cached_layer = Some(DeploymentLayer::BareMetalOS);
    detector.reset();
    assert!(detector.cached_layer.is_none());
}
