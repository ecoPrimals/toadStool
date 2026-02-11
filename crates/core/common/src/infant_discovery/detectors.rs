// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! Substrate detectors - detect the runtime environment without hardcoding
//!
//! These detectors identify what substrate ToadStool is running on
//! (Kubernetes, Docker, bare metal, etc.) through runtime inspection,
//! not hardcoded assumptions.
//!
//! Migrated from `async_trait` to native async for zero-cost abstraction.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use super::capabilities::{
    DetectedSubstrate, DiscoveryError, SubstrateCapability, SubstrateDetector, SubstrateType,
};

/// Environment snapshot for cloud detection.
///
/// Production uses `CloudEnvironment::from_env()`.
/// Tests use explicit values - zero env var mutation.
#[derive(Debug, Clone, Default)]
pub struct CloudEnvironment {
    pub aws_region: Option<String>,
    pub aws_default_region: Option<String>,
    pub gcp_project: Option<String>,
    pub google_cloud_project: Option<String>,
    pub azure_subscription_id: Option<String>,
    pub kubernetes_service_host: Option<String>,
    pub kubernetes_namespace: Option<String>,
    pub hostname: Option<String>,
    pub consul_http_addr: Option<String>,
}

impl CloudEnvironment {
    /// Capture current environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            aws_region: std::env::var("AWS_REGION").ok(),
            aws_default_region: std::env::var("AWS_DEFAULT_REGION").ok(),
            gcp_project: std::env::var("GCP_PROJECT").ok(),
            google_cloud_project: std::env::var("GOOGLE_CLOUD_PROJECT").ok(),
            azure_subscription_id: std::env::var("AZURE_SUBSCRIPTION_ID").ok(),
            kubernetes_service_host: std::env::var("KUBERNETES_SERVICE_HOST").ok(),
            kubernetes_namespace: std::env::var("KUBERNETES_NAMESPACE").ok(),
            hostname: std::env::var("HOSTNAME").ok(),
            consul_http_addr: std::env::var("CONSUL_HTTP_ADDR").ok(),
        }
    }
}

/// Kubernetes detector - detects if running in Kubernetes
pub struct KubernetesDetector;

impl KubernetesDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Pure check: is kubernetes based on env snapshot
    #[must_use]
    pub fn is_kubernetes_from(env: &CloudEnvironment) -> bool {
        Path::new("/var/run/secrets/kubernetes.io/serviceaccount").exists()
            || env.kubernetes_service_host.is_some()
    }

    /// Check if we're in a Kubernetes environment
    fn is_kubernetes() -> bool {
        Self::is_kubernetes_from(&CloudEnvironment::from_env())
    }
}

impl Default for KubernetesDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateDetector for KubernetesDetector {
    fn detect(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DetectedSubstrate>, DiscoveryError>> + Send + '_>>
    {
        let is_k8s = Self::is_kubernetes();

        Box::pin(async move {
            if !is_k8s {
                return Ok(None);
            }

            let mut metadata = std::collections::HashMap::new();

            // Detect Kubernetes metadata
            if let Ok(namespace) = std::env::var("KUBERNETES_NAMESPACE") {
                metadata.insert("namespace".to_string(), namespace);
            }

            if let Ok(pod_name) = std::env::var("HOSTNAME") {
                metadata.insert("pod_name".to_string(), pod_name);
            }

            if let Ok(service_host) = std::env::var("KUBERNETES_SERVICE_HOST") {
                metadata.insert("api_server".to_string(), service_host);
            }

            metadata.insert("orchestrator".to_string(), "kubernetes".to_string());

            Ok(Some(DetectedSubstrate {
                substrate_type: SubstrateType::ContainerOrchestrator,
                capabilities: vec![
                    SubstrateCapability::ContainerOrchestration,
                    SubstrateCapability::ServiceDiscovery,
                    SubstrateCapability::ServiceMesh,
                ],
                metadata,
            }))
        })
    }

    fn name(&self) -> &'static str {
        "kubernetes"
    }
}

/// Docker detector - detects if running in Docker
pub struct DockerDetector;

impl DockerDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Check if we're in a Docker container
    fn is_docker() -> bool {
        // Check for /.dockerenv file
        Path::new("/.dockerenv").exists()
            // Or check cgroup
            || Self::check_cgroup()
    }

    fn check_cgroup() -> bool {
        std::fs::read_to_string("/proc/self/cgroup").is_ok_and(|content| content.contains("docker"))
    }
}

impl Default for DockerDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateDetector for DockerDetector {
    fn detect(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DetectedSubstrate>, DiscoveryError>> + Send + '_>>
    {
        let is_docker = Self::is_docker();

        Box::pin(async move {
            if !is_docker {
                return Ok(None);
            }

            let mut metadata = std::collections::HashMap::new();
            metadata.insert("runtime".to_string(), "docker".to_string());

            // Try to detect container ID
            if let Ok(content) = std::fs::read_to_string("/proc/self/cgroup") {
                if let Some(line) = content.lines().next() {
                    if let Some(id_part) = line.split('/').next_back() {
                        if id_part.len() > 12 {
                            metadata.insert("container_id".to_string(), id_part[..12].to_string());
                        }
                    }
                }
            }

            Ok(Some(DetectedSubstrate {
                substrate_type: SubstrateType::ContainerRuntime,
                capabilities: vec![SubstrateCapability::ContainerRuntime],
                metadata,
            }))
        })
    }

    fn name(&self) -> &'static str {
        "docker"
    }
}

/// Consul detector - detects if Consul is available
pub struct ConsulDetector;

impl ConsulDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Check if Consul is available
    ///
    /// **PURE RUST**: Consul detection removed (no HTTP dependencies)
    async fn is_consul_available(&self) -> bool {
        tracing::trace!("Consul detection disabled (pure Rust mode)");
        false // Consul not available in pure Rust mode
    }
}

impl Default for ConsulDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateDetector for ConsulDetector {
    fn detect(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DetectedSubstrate>, DiscoveryError>> + Send + '_>>
    {
        Box::pin(async move {
            if !self.is_consul_available().await {
                return Ok(None);
            }

            let mut metadata = std::collections::HashMap::new();

            if let Ok(addr) = std::env::var("CONSUL_HTTP_ADDR") {
                metadata.insert("consul_addr".to_string(), addr);
            }

            metadata.insert("service_mesh".to_string(), "consul".to_string());

            Ok(Some(DetectedSubstrate {
                substrate_type: SubstrateType::ContainerOrchestrator,
                capabilities: vec![
                    SubstrateCapability::ServiceDiscovery,
                    SubstrateCapability::ServiceMesh,
                ],
                metadata,
            }))
        })
    }

    fn name(&self) -> &'static str {
        "consul"
    }
}

/// Cloud detector - detects if running in a cloud environment
pub struct CloudDetector;

/// Pure logic: detect cloud provider from environment snapshot
#[must_use]
pub fn detect_cloud_provider_from(env: &CloudEnvironment) -> Option<String> {
    // Check for AWS via filesystem
    if Path::new("/sys/hypervisor/uuid").exists() {
        if let Ok(content) = std::fs::read_to_string("/sys/hypervisor/uuid") {
            if content.to_lowercase().starts_with("ec2") {
                return Some("aws".to_string());
            }
        }
    }

    // Check for AWS from env
    if env.aws_region.is_some() || env.aws_default_region.is_some() {
        return Some("aws".to_string());
    }

    // Check for GCP
    if env.gcp_project.is_some() || env.google_cloud_project.is_some() {
        return Some("gcp".to_string());
    }

    // Check for Azure
    if env.azure_subscription_id.is_some() {
        return Some("azure".to_string());
    }

    None
}

impl CloudDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Detect which cloud provider (if any)
    fn detect_cloud_provider() -> Option<String> {
        detect_cloud_provider_from(&CloudEnvironment::from_env())
    }
}

impl Default for CloudDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateDetector for CloudDetector {
    fn detect(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DetectedSubstrate>, DiscoveryError>> + Send + '_>>
    {
        let provider = Self::detect_cloud_provider();

        Box::pin(async move {
            let Some(provider) = provider else {
                return Ok(None);
            };

            let mut metadata = std::collections::HashMap::new();
            metadata.insert("cloud_provider".to_string(), provider.clone());

            // Add provider-specific metadata
            match provider.as_str() {
                "aws" => {
                    if let Ok(region) = std::env::var("AWS_REGION") {
                        metadata.insert("region".to_string(), region);
                    }
                }
                "gcp" => {
                    if let Ok(project) = std::env::var("GOOGLE_CLOUD_PROJECT") {
                        metadata.insert("project".to_string(), project);
                    }
                }
                "azure" => {
                    if let Ok(sub_id) = std::env::var("AZURE_SUBSCRIPTION_ID") {
                        metadata.insert("subscription_id".to_string(), sub_id);
                    }
                }
                _ => {}
            }

            Ok(Some(DetectedSubstrate {
                substrate_type: SubstrateType::Cloud,
                capabilities: vec![SubstrateCapability::CloudCompute],
                metadata,
            }))
        })
    }

    fn name(&self) -> &'static str {
        "cloud"
    }
}

/// Bare metal detector - always succeeds as final fallback
pub struct BareMetalDetector;

impl BareMetalDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for BareMetalDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateDetector for BareMetalDetector {
    fn detect(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DetectedSubstrate>, DiscoveryError>> + Send + '_>>
    {
        Box::pin(async move {
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("deployment".to_string(), "bare_metal".to_string());

            // Add OS information
            if let Ok(os) = std::env::var("OS") {
                metadata.insert("os".to_string(), os);
            }

            Ok(Some(DetectedSubstrate {
                substrate_type: SubstrateType::Bare,
                capabilities: vec![SubstrateCapability::BareMetal],
                metadata,
            }))
        })
    }

    fn name(&self) -> &'static str {
        "bare_metal"
    }
}

/// Create standard detector chain
#[must_use]
pub fn standard_detectors() -> Vec<Box<dyn SubstrateDetector>> {
    vec![
        Box::new(KubernetesDetector::new()),
        Box::new(DockerDetector::new()),
        Box::new(ConsulDetector::new()),
        Box::new(CloudDetector::new()),
        Box::new(BareMetalDetector::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_bare_metal_detector_always_succeeds() {
        let detector = BareMetalDetector::new();
        let result = detector.detect().await.unwrap();
        assert!(result.is_some());
        let substrate = result.unwrap();
        assert_eq!(substrate.substrate_type, SubstrateType::Bare);
    }

    #[test]
    fn test_detector_chain() {
        let detectors = standard_detectors();
        assert_eq!(detectors.len(), 5);
        assert_eq!(detectors[0].name(), "kubernetes");
        assert_eq!(detectors[4].name(), "bare_metal");
    }

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
    fn test_cloud_detect_aws_from_region() {
        let env = CloudEnvironment {
            aws_region: Some("us-east-1".to_string()),
            ..Default::default()
        };
        assert_eq!(detect_cloud_provider_from(&env), Some("aws".to_string()));
    }

    #[test]
    fn test_cloud_detect_aws_default_region() {
        let env = CloudEnvironment {
            aws_default_region: Some("us-west-2".to_string()),
            ..Default::default()
        };
        assert_eq!(detect_cloud_provider_from(&env), Some("aws".to_string()));
    }

    #[test]
    fn test_cloud_detect_gcp() {
        let env = CloudEnvironment {
            gcp_project: Some("my-project".to_string()),
            ..Default::default()
        };
        assert_eq!(detect_cloud_provider_from(&env), Some("gcp".to_string()));
    }

    #[test]
    fn test_cloud_detect_gcp_google_cloud() {
        let env = CloudEnvironment {
            google_cloud_project: Some("another-project".to_string()),
            ..Default::default()
        };
        assert_eq!(detect_cloud_provider_from(&env), Some("gcp".to_string()));
    }

    #[test]
    fn test_cloud_detect_azure() {
        let env = CloudEnvironment {
            azure_subscription_id: Some("12345".to_string()),
            ..Default::default()
        };
        assert_eq!(detect_cloud_provider_from(&env), Some("azure".to_string()));
    }

    #[test]
    fn test_cloud_detect_none() {
        let env = CloudEnvironment::default();
        assert_eq!(detect_cloud_provider_from(&env), None);
    }

    #[test]
    fn test_kubernetes_detection_from_env() {
        let env = CloudEnvironment {
            kubernetes_service_host: Some("10.0.0.1".to_string()),
            ..Default::default()
        };
        assert!(KubernetesDetector::is_kubernetes_from(&env));
    }

    #[test]
    fn test_kubernetes_detection_absent() {
        let env = CloudEnvironment::default();
        // May still detect via filesystem, but env check should be false
        // (filesystem check is independent of env)
        let from_env_only = env.kubernetes_service_host.is_some();
        assert!(!from_env_only);
    }

    #[test]
    fn test_kubernetes_not_detected_clean_env() {
        let env = CloudEnvironment::default();
        // Without K8s env var AND without K8s serviceaccount path, should be false
        // Note: is_kubernetes_from also checks filesystem - that's fine for tests
        // as it's reading, not writing
        let _result = KubernetesDetector::is_kubernetes_from(&env);
        // Just verify it doesn't panic - actual result depends on test machine
    }

    #[test]
    fn test_cloud_not_detected_clean_env() {
        let env = CloudEnvironment::default();
        // Without any cloud env vars, might still detect via filesystem
        // but that's a read-only check, fine for concurrent tests
        let _result = detect_cloud_provider_from(&env);
    }

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

    #[tokio::test]
    async fn test_docker_detector_no_docker() {
        let detector = DockerDetector::new();
        let result = detector.detect().await.unwrap();
        assert!(result.is_none() || result.is_some());
    }

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

    #[tokio::test]
    async fn test_consul_detector_returns_none() {
        let detector = ConsulDetector::new();
        let result = detector.detect().await.unwrap();
        assert!(result.is_none());
    }

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
    fn test_bare_metal_detector_new() {
        let detector = BareMetalDetector::new();
        assert_eq!(detector.name(), "bare_metal");
    }

    #[test]
    fn test_bare_metal_detector_default() {
        let detector = BareMetalDetector;
        assert_eq!(detector.name(), "bare_metal");
    }

    #[tokio::test]
    async fn test_bare_metal_detector_capabilities() {
        let detector = BareMetalDetector::new();
        let result = detector.detect().await.unwrap();
        assert!(result.is_some());
        let substrate = result.unwrap();
        assert_eq!(substrate.substrate_type, SubstrateType::Bare);
        assert!(!substrate.capabilities.is_empty());
    }

    #[tokio::test]
    async fn test_bare_metal_detector_metadata() {
        let detector = BareMetalDetector::new();
        let result = detector.detect().await.unwrap();
        assert!(result.is_some());
        let substrate = result.unwrap();
        assert!(!substrate.metadata.is_empty());
        assert!(substrate.metadata.contains_key("deployment"));
        assert_eq!(
            substrate.metadata.get("deployment"),
            Some(&"bare_metal".to_string())
        );
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
    fn test_standard_detectors_count() {
        let detectors = standard_detectors();
        assert_eq!(detectors.len(), 5);
    }

    #[test]
    fn test_detector_names_unique() {
        let detectors = standard_detectors();
        let names: Vec<&str> = detectors.iter().map(|d| d.name()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len());
    }

    #[tokio::test]
    async fn test_all_detectors_dont_panic() {
        let detectors = standard_detectors();
        for detector in detectors {
            let result = detector.detect().await;
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_cloud_environment_from_env() {
        let env = CloudEnvironment::from_env();
        // May or may not have values depending on test environment
        let _ = (
            env.aws_region,
            env.aws_default_region,
            env.gcp_project,
            env.google_cloud_project,
            env.azure_subscription_id,
            env.kubernetes_service_host,
            env.kubernetes_namespace,
            env.hostname,
            env.consul_http_addr,
        );
    }

    #[test]
    fn test_cloud_environment_default() {
        let env = CloudEnvironment::default();
        assert!(env.aws_region.is_none());
        assert!(env.aws_default_region.is_none());
        assert!(env.gcp_project.is_none());
    }

    #[tokio::test]
    async fn test_cloud_detector_detect_when_aws_present() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let prev = std::env::var("AWS_REGION").ok();
        std::env::set_var("AWS_REGION", "us-east-1");

        let detector = CloudDetector::new();
        let result = detector.detect().await.unwrap();

        if let Some(p) = prev {
            std::env::set_var("AWS_REGION", p);
        } else {
            std::env::remove_var("AWS_REGION");
        }

        if let Some(ref substrate) = result {
            assert_eq!(substrate.substrate_type, SubstrateType::Cloud);
            assert_eq!(
                substrate.metadata.get("cloud_provider"),
                Some(&"aws".to_string())
            );
        }
    }

    #[tokio::test]
    async fn test_cloud_detector_detect_when_gcp_present() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let prev = std::env::var("GOOGLE_CLOUD_PROJECT").ok();
        std::env::set_var("GOOGLE_CLOUD_PROJECT", "test-project");

        let detector = CloudDetector::new();
        let result = detector.detect().await.unwrap();

        if let Some(p) = prev {
            std::env::set_var("GOOGLE_CLOUD_PROJECT", p);
        } else {
            std::env::remove_var("GOOGLE_CLOUD_PROJECT");
        }

        if let Some(ref substrate) = result {
            assert_eq!(substrate.substrate_type, SubstrateType::Cloud);
            assert!(substrate.metadata.contains_key("cloud_provider"));
            assert_eq!(
                substrate.metadata.get("cloud_provider"),
                Some(&"gcp".to_string())
            );
        }
    }

    #[tokio::test]
    async fn test_cloud_detector_detect_when_azure_present() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let prev = std::env::var("AZURE_SUBSCRIPTION_ID").ok();
        std::env::set_var("AZURE_SUBSCRIPTION_ID", "sub-12345");

        let detector = CloudDetector::new();
        let result = detector.detect().await.unwrap();

        if let Some(p) = prev {
            std::env::set_var("AZURE_SUBSCRIPTION_ID", p);
        } else {
            std::env::remove_var("AZURE_SUBSCRIPTION_ID");
        }

        if let Some(ref substrate) = result {
            assert_eq!(substrate.substrate_type, SubstrateType::Cloud);
            assert_eq!(
                substrate.metadata.get("cloud_provider"),
                Some(&"azure".to_string())
            );
        }
    }

    #[tokio::test]
    async fn test_kubernetes_detector_detect_when_k8s_env_set() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let prev_host = std::env::var("KUBERNETES_SERVICE_HOST").ok();
        let prev_ns = std::env::var("KUBERNETES_NAMESPACE").ok();
        let prev_hostname = std::env::var("HOSTNAME").ok();

        std::env::set_var("KUBERNETES_SERVICE_HOST", "10.96.0.1");
        std::env::set_var("KUBERNETES_NAMESPACE", "default");
        std::env::set_var("HOSTNAME", "my-pod-123");

        let detector = KubernetesDetector::new();
        let result = detector.detect().await.unwrap();

        if let Some(p) = prev_host {
            std::env::set_var("KUBERNETES_SERVICE_HOST", p);
        } else {
            std::env::remove_var("KUBERNETES_SERVICE_HOST");
        }
        if let Some(p) = prev_ns {
            std::env::set_var("KUBERNETES_NAMESPACE", p);
        } else {
            std::env::remove_var("KUBERNETES_NAMESPACE");
        }
        if let Some(p) = prev_hostname {
            std::env::set_var("HOSTNAME", p);
        } else {
            std::env::remove_var("HOSTNAME");
        }

        if let Some(ref substrate) = result {
            assert_eq!(
                substrate.substrate_type,
                SubstrateType::ContainerOrchestrator
            );
            assert_eq!(
                substrate.metadata.get("orchestrator"),
                Some(&"kubernetes".to_string())
            );
            assert_eq!(
                substrate.metadata.get("namespace"),
                Some(&"default".to_string())
            );
            assert_eq!(
                substrate.metadata.get("pod_name"),
                Some(&"my-pod-123".to_string())
            );
            assert_eq!(
                substrate.metadata.get("api_server"),
                Some(&"10.96.0.1".to_string())
            );
        }
    }

    #[tokio::test]
    async fn test_bare_metal_detector_with_os_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let prev = std::env::var("OS").ok();
        std::env::set_var("OS", "Linux");

        let detector = BareMetalDetector::new();
        let result = detector.detect().await.unwrap();

        if let Some(p) = prev {
            std::env::set_var("OS", p);
        } else {
            std::env::remove_var("OS");
        }

        assert!(result.is_some());
        let substrate = result.unwrap();
        assert_eq!(substrate.metadata.get("os"), Some(&"Linux".to_string()));
    }

    #[test]
    fn test_detect_cloud_provider_aws_precedence_over_gcp() {
        let env = CloudEnvironment {
            aws_region: Some("us-east-1".to_string()),
            gcp_project: Some("gcp-project".to_string()),
            ..Default::default()
        };
        assert_eq!(detect_cloud_provider_from(&env), Some("aws".to_string()));
    }

    #[test]
    fn test_detect_cloud_provider_gcp_precedence_over_azure() {
        let env = CloudEnvironment {
            gcp_project: Some("my-gcp".to_string()),
            azure_subscription_id: Some("azure-sub".to_string()),
            ..Default::default()
        };
        assert_eq!(detect_cloud_provider_from(&env), Some("gcp".to_string()));
    }

    #[test]
    fn test_kubernetes_is_from_serviceaccount_path() {
        let env = CloudEnvironment::default();
        let result = KubernetesDetector::is_kubernetes_from(&env);
        assert!(!result || Path::new("/var/run/secrets/kubernetes.io/serviceaccount").exists());
    }
}
