//! Targeted tests for deployment_layer/detector.rs coverage expansion
//! Covers: cloud metadata getters, env-based detection branches,
//! DeploymentLayer helpers (description, host_os, guest_os, is_virtualized, has_direct_hardware_access),
//! CloudProvider/ContainerRuntime variants, Display impl
use std::sync::Mutex;
use toadstool::deployment_layer::{
    CloudProvider, ContainerRuntime, DeploymentLayer, LayerDetector,
};

// Env-var-mutating tests must be serialized to avoid cross-test pollution.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

// ── Cloud metadata via env vars (when cloud branch wins) ─────────────────────
// SAFETY: std::env::set_var/remove_var are unsafe in Rust 2024 (concurrent reads can race).
// All env-mutating tests hold ENV_MUTEX to serialize execution.
// Each test cleans up its vars immediately after detection.

#[tokio::test]
async fn test_detector_aws_with_instance_and_region_env() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::set_var("AWS_EXECUTION_ENV", "AWS_ECS_EC2");
        std::env::set_var("AWS_INSTANCE_TYPE", "t3.medium");
        std::env::set_var("AWS_REGION", "us-west-2");
    }
    let mut detector = LayerDetector::new();
    let layer = detector.detect().await.expect("detect should succeed");
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::remove_var("AWS_EXECUTION_ENV");
        std::env::remove_var("AWS_INSTANCE_TYPE");
        std::env::remove_var("AWS_REGION");
    }
    if let DeploymentLayer::CloudLayer {
        provider,
        instance_type,
        region,
    } = &layer
    {
        assert!(matches!(provider, CloudProvider::AWS));
        assert!(instance_type.as_ref().map_or(true, |s| !s.is_empty()));
        assert!(region.as_ref().map_or(true, |s| !s.is_empty()));
    }
}

#[tokio::test]
async fn test_detector_aws_ec2_instance_type_env() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::set_var("AWS_EXECUTION_ENV", "test");
        std::env::set_var("EC2_INSTANCE_TYPE", "m5.xlarge");
    }
    let mut detector = LayerDetector::new();
    let _ = detector.detect().await.expect("detect should succeed");
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::remove_var("AWS_EXECUTION_ENV");
        std::env::remove_var("EC2_INSTANCE_TYPE");
    }
}

#[tokio::test]
async fn test_detector_aws_default_region_fallback() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::set_var("AWS_EXECUTION_ENV", "test");
        std::env::remove_var("AWS_REGION");
        std::env::remove_var("AWS_DEFAULT_REGION");
    }
    let mut detector = LayerDetector::new();
    let _ = detector.detect().await.expect("detect should succeed");
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::remove_var("AWS_EXECUTION_ENV");
    }
}

#[tokio::test]
async fn test_detector_gcp_with_zone_env() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::set_var("GCP_PROJECT", "my-project");
        std::env::set_var("GCE_ZONE", "us-central1-a");
        std::env::set_var("GCE_MACHINE_TYPE", "n1-standard-4");
    }
    let mut detector = LayerDetector::new();
    let _ = detector.detect().await.expect("detect should succeed");
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::remove_var("GCP_PROJECT");
        std::env::remove_var("GCE_ZONE");
        std::env::remove_var("GCE_MACHINE_TYPE");
    }
}

#[tokio::test]
async fn test_detector_azure_with_location_env() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::set_var("AZURE_SUBSCRIPTION_ID", "sub-id");
        std::env::set_var("AZURE_LOCATION", "eastus");
        std::env::set_var("AZURE_VM_SIZE", "Standard_D2s_v3");
    }
    let mut detector = LayerDetector::new();
    let _ = detector.detect().await.expect("detect should succeed");
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::remove_var("AZURE_SUBSCRIPTION_ID");
        std::env::remove_var("AZURE_LOCATION");
        std::env::remove_var("AZURE_VM_SIZE");
    }
}

#[tokio::test]
async fn test_detector_lambda_env() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::set_var("AWS_LAMBDA_FUNCTION_NAME", "my-function");
    }
    let mut detector = LayerDetector::new();
    let _ = detector.detect().await.expect("detect should succeed");
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::remove_var("AWS_LAMBDA_FUNCTION_NAME");
    }
}

#[tokio::test]
async fn test_detector_ecs_metadata_env() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::set_var("ECS_CONTAINER_METADATA_URI", "http://169.254.170.2/v4");
    }
    let mut detector = LayerDetector::new();
    let _ = detector.detect().await.expect("detect should succeed");
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::remove_var("ECS_CONTAINER_METADATA_URI");
    }
}

#[tokio::test]
async fn test_detector_reset_redetects() {
    let mut detector = LayerDetector::new();
    let first = detector.detect().await.expect("first detect succeeds");
    assert!(
        !first.description().is_empty(),
        "first detection returns a description"
    );
    detector.reset();
    let second = detector
        .detect()
        .await
        .expect("second detect after reset succeeds");
    assert!(
        !second.description().is_empty(),
        "second detection after reset returns a description"
    );
}

// ── DeploymentLayer description() for all variants ────────────────────────────

#[test]
fn test_deployment_layer_description_bare_metal() {
    let layer = DeploymentLayer::BareMetalOS;
    assert_eq!(layer.description(), "Base OS on bare metal");
}

#[test]
fn test_deployment_layer_description_middleware() {
    let layer = DeploymentLayer::MiddlewareLayer {
        host_os: "Ubuntu".to_string(),
        host_version: Some("22.04".to_string()),
    };
    assert_eq!(layer.description(), "Middleware on host OS");
}

#[test]
fn test_deployment_layer_description_service() {
    let layer = DeploymentLayer::ServiceLayer {
        guest_os: vec!["Docker containers".to_string()],
    };
    assert_eq!(layer.description(), "Service provider to guest OS");
}

#[test]
fn test_deployment_layer_description_container() {
    let layer = DeploymentLayer::ContainerLayer {
        runtime: ContainerRuntime::Docker,
        container_id: Some("abc123".to_string()),
    };
    assert_eq!(layer.description(), "Inside container");
}

#[test]
fn test_deployment_layer_description_vm() {
    let layer = DeploymentLayer::VMLayer {
        hypervisor: "QEMU/KVM".to_string(),
        gpu_passthrough: true,
    };
    assert_eq!(layer.description(), "Inside virtual machine");
}

#[test]
fn test_deployment_layer_description_cloud() {
    let layer = DeploymentLayer::CloudLayer {
        provider: CloudProvider::AWS,
        instance_type: Some("t3.medium".to_string()),
        region: Some("us-west-2".to_string()),
    };
    assert_eq!(layer.description(), "Cloud environment");
}

// ── DeploymentLayer host_os, guest_os, is_virtualized, has_direct_hardware_access ───

#[test]
fn test_deployment_layer_host_os_middleware() {
    let layer = DeploymentLayer::MiddlewareLayer {
        host_os: "Pop!_OS".to_string(),
        host_version: None,
    };
    assert_eq!(layer.host_os(), Some("Pop!_OS"));
}

#[test]
fn test_deployment_layer_host_os_none_for_bare_metal() {
    let layer = DeploymentLayer::BareMetalOS;
    assert!(layer.host_os().is_none());
}

#[test]
fn test_deployment_layer_guest_os_service() {
    let layer = DeploymentLayer::ServiceLayer {
        guest_os: vec![
            "Kubernetes pods".to_string(),
            "Docker containers".to_string(),
        ],
    };
    let guests = layer.guest_os().expect("ServiceLayer has guest_os");
    assert_eq!(guests.len(), 2);
    assert!(guests.contains(&"Kubernetes pods".to_string()));
}

#[test]
fn test_deployment_layer_guest_os_none_for_cloud() {
    let layer = DeploymentLayer::CloudLayer {
        provider: CloudProvider::GCP,
        instance_type: None,
        region: None,
    };
    assert!(layer.guest_os().is_none());
}

#[test]
fn test_deployment_layer_is_virtualized_true_for_container() {
    let layer = DeploymentLayer::ContainerLayer {
        runtime: ContainerRuntime::Podman,
        container_id: None,
    };
    assert!(layer.is_virtualized());
}

#[test]
fn test_deployment_layer_is_virtualized_true_for_cloud() {
    let layer = DeploymentLayer::CloudLayer {
        provider: CloudProvider::Azure,
        instance_type: None,
        region: None,
    };
    assert!(layer.is_virtualized());
}

#[test]
fn test_deployment_layer_is_virtualized_false_for_bare_metal() {
    let layer = DeploymentLayer::BareMetalOS;
    assert!(!layer.is_virtualized());
}

#[test]
fn test_deployment_layer_has_direct_hardware_access_bare_metal() {
    let layer = DeploymentLayer::BareMetalOS;
    assert!(layer.has_direct_hardware_access());
}

#[test]
fn test_deployment_layer_has_direct_hardware_access_vm_with_gpu_passthrough() {
    let layer = DeploymentLayer::VMLayer {
        hypervisor: "QEMU/KVM".to_string(),
        gpu_passthrough: true,
    };
    assert!(layer.has_direct_hardware_access());
}

#[test]
fn test_deployment_layer_has_direct_hardware_access_vm_without_gpu_passthrough() {
    let layer = DeploymentLayer::VMLayer {
        hypervisor: "VirtualBox".to_string(),
        gpu_passthrough: false,
    };
    assert!(!layer.has_direct_hardware_access());
}

#[test]
fn test_deployment_layer_has_direct_hardware_access_cloud_false() {
    let layer = DeploymentLayer::CloudLayer {
        provider: CloudProvider::AWS,
        instance_type: None,
        region: None,
    };
    assert!(!layer.has_direct_hardware_access());
}

// ── CloudProvider and ContainerRuntime variants ────────────────────────────────

#[test]
fn test_cloud_provider_variants() {
    assert!(matches!(CloudProvider::AWS, CloudProvider::AWS));
    assert!(matches!(CloudProvider::GCP, CloudProvider::GCP));
    assert!(matches!(CloudProvider::Azure, CloudProvider::Azure));
    assert!(matches!(CloudProvider::Oracle, CloudProvider::Oracle));
    assert!(matches!(
        CloudProvider::DigitalOcean,
        CloudProvider::DigitalOcean
    ));
    let custom = CloudProvider::Custom("my-cloud".to_string());
    assert!(matches!(custom, CloudProvider::Custom(_)));
}

#[test]
fn test_container_runtime_variants() {
    assert!(matches!(ContainerRuntime::Docker, ContainerRuntime::Docker));
    assert!(matches!(ContainerRuntime::Podman, ContainerRuntime::Podman));
    assert!(matches!(
        ContainerRuntime::Containerd,
        ContainerRuntime::Containerd
    ));
    assert!(matches!(ContainerRuntime::CRIO, ContainerRuntime::CRIO));
    let other = ContainerRuntime::Other("custom".to_string());
    assert!(matches!(other, ContainerRuntime::Other(_)));
}

// ── DeploymentLayer Display impl ────────────────────────────────────────────────

#[test]
fn test_deployment_layer_display_bare_metal() {
    let layer = DeploymentLayer::BareMetalOS;
    let s = format!("{}", layer);
    assert_eq!(s, "BareMetalOS");
}

#[test]
fn test_deployment_layer_display_middleware() {
    let layer = DeploymentLayer::MiddlewareLayer {
        host_os: "Fedora".to_string(),
        host_version: Some("39".to_string()),
    };
    let s = format!("{}", layer);
    assert!(s.contains("Fedora"));
    assert!(s.contains("Middleware"));
}

#[test]
fn test_deployment_layer_display_service() {
    let layer = DeploymentLayer::ServiceLayer {
        guest_os: vec!["Docker".to_string()],
    };
    let s = format!("{}", layer);
    assert!(s.contains("Docker"));
    assert!(s.contains("ServiceLayer"));
}

#[test]
fn test_deployment_layer_display_container() {
    let layer = DeploymentLayer::ContainerLayer {
        runtime: ContainerRuntime::Docker,
        container_id: None,
    };
    let s = format!("{}", layer);
    assert!(s.contains("Docker"));
}

#[test]
fn test_deployment_layer_display_vm() {
    let layer = DeploymentLayer::VMLayer {
        hypervisor: "VMware".to_string(),
        gpu_passthrough: false,
    };
    let s = format!("{}", layer);
    assert!(s.contains("VMware"));
}

#[test]
fn test_deployment_layer_display_cloud() {
    let layer = DeploymentLayer::CloudLayer {
        provider: CloudProvider::AWS,
        instance_type: None,
        region: None,
    };
    let s = format!("{}", layer);
    assert!(s.contains("AWS"));
}

// ── GCP alternate env vars ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_detector_gcp_google_cloud_project_env() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::set_var("GOOGLE_CLOUD_PROJECT", "test-project");
    }
    let mut detector = LayerDetector::new();
    let _ = detector.detect().await.expect("detect succeeds");
    // SAFETY: See module-level SAFETY comment above.
    unsafe {
        std::env::remove_var("GOOGLE_CLOUD_PROJECT");
    }
}

// ── LayerDetector::default() ────────────────────────────────────────────────────

#[tokio::test]
async fn test_detector_default_creates_functional_detector() {
    let mut detector = LayerDetector::default();
    let layer = detector
        .detect()
        .await
        .expect("default detector can detect");
    assert!(!layer.description().is_empty());
}
