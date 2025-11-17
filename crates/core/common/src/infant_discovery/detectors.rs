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

/// Kubernetes detector - detects if running in Kubernetes
pub struct KubernetesDetector;

impl KubernetesDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Check if we're in a Kubernetes environment
    fn is_kubernetes(&self) -> bool {
        // Check for Kubernetes service account
        Path::new("/var/run/secrets/kubernetes.io/serviceaccount").exists()
            || std::env::var("KUBERNETES_SERVICE_HOST").is_ok()
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
        let is_k8s = self.is_kubernetes();

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
    fn is_docker(&self) -> bool {
        // Check for /.dockerenv file
        Path::new("/.dockerenv").exists()
            // Or check cgroup
            || self.check_cgroup()
    }

    fn check_cgroup(&self) -> bool {
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
        let is_docker = self.is_docker();

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
    async fn is_consul_available(&self) -> bool {
        // Check environment variable
        let consul_addr = std::env::var("CONSUL_HTTP_ADDR")
            .unwrap_or_else(|_| "http://localhost:8500".to_string());

        // Try to connect to Consul API
        match reqwest::Client::new()
            .get(format!("{consul_addr}/v1/status/leader"))
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                tracing::debug!(consul_addr, "Successfully connected to Consul");
                true
            }
            Ok(response) => {
                tracing::debug!(
                    consul_addr,
                    status = %response.status(),
                    "Consul returned non-success status"
                );
                false
            }
            Err(e) => {
                tracing::trace!(consul_addr, error = %e, "Consul not available");
                false
            }
        }
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

impl CloudDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Detect which cloud provider (if any)
    fn detect_cloud_provider(&self) -> Option<String> {
        // Check for AWS
        if Path::new("/sys/hypervisor/uuid").exists() {
            if let Ok(content) = std::fs::read_to_string("/sys/hypervisor/uuid") {
                if content.to_lowercase().starts_with("ec2") {
                    return Some("aws".to_string());
                }
            }
        }

        // Check for AWS metadata service
        if std::env::var("AWS_REGION").is_ok() || std::env::var("AWS_DEFAULT_REGION").is_ok() {
            return Some("aws".to_string());
        }

        // Check for GCP
        if std::env::var("GCP_PROJECT").is_ok() || std::env::var("GOOGLE_CLOUD_PROJECT").is_ok() {
            return Some("gcp".to_string());
        }

        // Check for Azure
        if std::env::var("AZURE_SUBSCRIPTION_ID").is_ok() {
            return Some("azure".to_string());
        }

        None
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
        let provider = self.detect_cloud_provider();

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

    #[tokio::test]
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
}
