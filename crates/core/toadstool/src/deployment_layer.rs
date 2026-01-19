// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Deployment Layer Detection and Adaptation
//!
//! This module implements multi-layer OS support for Toadstool, enabling it to
//! work correctly whether running as:
//! - The base OS (bare metal)
//! - Middleware on another OS (e.g., Pop!_OS)
//! - Service provider to another OS (e.g., SteamOS on biomeOS)
//! - Inside a container (Docker/Podman)
//! - Inside a VM (QEMU/KVM)
//! - In the cloud (EC2/GCE/Azure)
//!
//! # Philosophy
//!
//! **Adaptation over assumption**: Don't assume where we're running,
//! detect it and adapt accordingly.
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool::deployment_layer::{DeploymentLayer, LayerDetector};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let detector = LayerDetector::new();
//! let layer = detector.detect().await?;
//!
//! match layer {
//!     DeploymentLayer::BareMetalOS => {
//!         // Running as the OS itself
//!         println!("biomeOS is the base OS");
//!     }
//!     DeploymentLayer::MiddlewareLayer => {
//!         // Running on another OS (e.g., Pop!_OS)
//!         println!("biomeOS is middleware on {}", layer.host_os().unwrap());
//!     }
//!     DeploymentLayer::ServiceLayer => {
//!         // Providing services to another OS (e.g., SteamOS)
//!         println!("biomeOS is providing services");
//!     }
//!     _ => {}
//! }
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// Deployment layer where Toadstool is running
///
/// This determines how Toadstool exposes capabilities and interacts
/// with other system components.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentLayer {
    /// Running as the base OS on bare metal
    ///
    /// Example: biomeOS directly on hardware
    /// Capabilities: Full hardware access, direct GPU, all system resources
    BareMetalOS,

    /// Running as middleware on another OS
    ///
    /// Example: biomeOS on Pop!_OS
    /// Capabilities: Exposed through host OS APIs, GPU via host drivers
    MiddlewareLayer {
        /// The host OS we're running on
        host_os: String,
        /// Host OS version
        host_version: Option<String>,
    },

    /// Providing services to another OS layer above
    ///
    /// Example: biomeOS providing GPU services to SteamOS
    /// Capabilities: Expose APIs for upper layers, manage shared resources
    ServiceLayer {
        /// The guest OS(s) we're serving
        guest_os: Vec<String>,
    },

    /// Running inside a container
    ///
    /// Example: biomeOS in Docker/Podman
    /// Capabilities: Limited by container runtime, namespace isolation
    ContainerLayer {
        /// Container runtime (docker, podman, etc.)
        runtime: ContainerRuntime,
        /// Container ID if available
        container_id: Option<String>,
    },

    /// Running inside a virtual machine
    ///
    /// Example: biomeOS in QEMU/KVM
    /// Capabilities: Virtual hardware, may have GPU passthrough
    VMLayer {
        /// Hypervisor type (QEMU, KVM, VMware, VirtualBox, etc.)
        hypervisor: String,
        /// Whether GPU is passed through
        gpu_passthrough: bool,
    },

    /// Running in a cloud environment
    ///
    /// Example: biomeOS in AWS EC2, GCE, Azure
    /// Capabilities: Cloud APIs, cloud GPUs, network constraints
    CloudLayer {
        /// Cloud provider
        provider: CloudProvider,
        /// Instance type/size
        instance_type: Option<String>,
        /// Region
        region: Option<String>,
    },
}

impl DeploymentLayer {
    /// Get a human-readable description of this layer
    pub fn description(&self) -> &'static str {
        match self {
            Self::BareMetalOS => "Base OS on bare metal",
            Self::MiddlewareLayer { .. } => "Middleware on host OS",
            Self::ServiceLayer { .. } => "Service provider to guest OS",
            Self::ContainerLayer { .. } => "Inside container",
            Self::VMLayer { .. } => "Inside virtual machine",
            Self::CloudLayer { .. } => "Cloud environment",
        }
    }

    /// Get the host OS if running as middleware
    pub fn host_os(&self) -> Option<&str> {
        match self {
            Self::MiddlewareLayer { host_os, .. } => Some(host_os),
            _ => None,
        }
    }

    /// Get guest OS(s) if providing services
    pub fn guest_os(&self) -> Option<&[String]> {
        match self {
            Self::ServiceLayer { guest_os } => Some(guest_os),
            _ => None,
        }
    }

    /// Check if running in a virtualized environment
    pub fn is_virtualized(&self) -> bool {
        matches!(
            self,
            Self::ContainerLayer { .. } | Self::VMLayer { .. } | Self::CloudLayer { .. }
        )
    }

    /// Check if we have direct hardware access
    pub fn has_direct_hardware_access(&self) -> bool {
        matches!(self, Self::BareMetalOS)
            || matches!(
                self,
                Self::VMLayer {
                    gpu_passthrough: true,
                    ..
                }
            )
    }
}

impl fmt::Display for DeploymentLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BareMetalOS => write!(f, "BareMetalOS"),
            Self::MiddlewareLayer { host_os, .. } => write!(f, "Middleware on {}", host_os),
            Self::ServiceLayer { guest_os } => {
                write!(f, "ServiceLayer (serving: {})", guest_os.join(", "))
            }
            Self::ContainerLayer { runtime, .. } => write!(f, "Container ({:?})", runtime),
            Self::VMLayer { hypervisor, .. } => write!(f, "VM ({})", hypervisor),
            Self::CloudLayer { provider, .. } => write!(f, "Cloud ({:?})", provider),
        }
    }
}

/// Container runtime types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContainerRuntime {
    /// Docker
    Docker,
    /// Podman
    Podman,
    /// containerd
    Containerd,
    /// CRI-O
    CRIO,
    /// Other/unknown
    Other(String),
}

/// Cloud provider types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CloudProvider {
    /// Amazon Web Services
    AWS,
    /// Google Cloud Platform
    GCP,
    /// Microsoft Azure
    Azure,
    /// Oracle Cloud
    Oracle,
    /// DigitalOcean
    DigitalOcean,
    /// Custom/unknown provider
    Custom(String),
}

/// Layer detector for identifying deployment environment
///
/// Uses multiple heuristics to determine where Toadstool is running.
pub struct LayerDetector {
    /// Cached detection result
    cached_layer: Option<DeploymentLayer>,
}

impl LayerDetector {
    /// Create a new layer detector
    pub fn new() -> Self {
        Self { cached_layer: None }
    }

    /// Detect the current deployment layer
    ///
    /// This performs various checks to determine the environment:
    /// - Checks for container indicators (/.dockerenv, /run/.containerenv)
    /// - Checks for VM indicators (DMI info, hypervisor CPU flags)
    /// - Checks for cloud metadata endpoints
    /// - Checks for host/guest OS relationships
    ///
    /// Results are cached for subsequent calls.
    pub async fn detect(&mut self) -> Result<DeploymentLayer, DetectionError> {
        // Return cached result if available
        if let Some(layer) = &self.cached_layer {
            return Ok(layer.clone());
        }

        // Detect layer through multiple checks
        let layer = self.detect_layer_internal().await?;

        // Cache result
        self.cached_layer = Some(layer.clone());

        Ok(layer)
    }

    /// Force re-detection (clears cache)
    pub fn reset(&mut self) {
        self.cached_layer = None;
    }

    /// Internal detection logic
    async fn detect_layer_internal(&self) -> Result<DeploymentLayer, DetectionError> {
        // Check for container first (most specific)
        if let Some(container) = self.detect_container().await? {
            return Ok(container);
        }

        // Check for cloud environment
        if let Some(cloud) = self.detect_cloud().await? {
            return Ok(cloud);
        }

        // Check for VM
        if let Some(vm) = self.detect_vm().await? {
            return Ok(vm);
        }

        // Check if we're middleware on another OS
        if let Some(middleware) = self.detect_middleware().await? {
            return Ok(middleware);
        }

        // Check if we're providing services to guest OS
        if let Some(service) = self.detect_service_layer().await? {
            return Ok(service);
        }

        // Default: assume bare metal
        Ok(DeploymentLayer::BareMetalOS)
    }

    /// Detect container environment
    async fn detect_container(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        // Check for Docker
        if Path::new("/.dockerenv").exists() {
            return Ok(Some(DeploymentLayer::ContainerLayer {
                runtime: ContainerRuntime::Docker,
                container_id: self.read_container_id().await.ok(),
            }));
        }

        // Check for Podman
        if Path::new("/run/.containerenv").exists() {
            return Ok(Some(DeploymentLayer::ContainerLayer {
                runtime: ContainerRuntime::Podman,
                container_id: self.read_container_id().await.ok(),
            }));
        }

        // Check cgroup for container indicators
        if let Ok(cgroup) = tokio::fs::read_to_string("/proc/1/cgroup").await {
            if cgroup.contains("docker") {
                return Ok(Some(DeploymentLayer::ContainerLayer {
                    runtime: ContainerRuntime::Docker,
                    container_id: self.read_container_id().await.ok(),
                }));
            } else if cgroup.contains("podman") {
                return Ok(Some(DeploymentLayer::ContainerLayer {
                    runtime: ContainerRuntime::Podman,
                    container_id: self.read_container_id().await.ok(),
                }));
            }
        }

        Ok(None)
    }

    /// Detect cloud environment
    async fn detect_cloud(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        // Check AWS metadata endpoint
        if self.check_aws_metadata().await {
            let instance_type = self.get_aws_instance_type().await.ok();
            let region = self.get_aws_region().await.ok();
            return Ok(Some(DeploymentLayer::CloudLayer {
                provider: CloudProvider::AWS,
                instance_type,
                region,
            }));
        }

        // Check GCP metadata endpoint
        if self.check_gcp_metadata().await {
            return Ok(Some(DeploymentLayer::CloudLayer {
                provider: CloudProvider::GCP,
                instance_type: self.get_gcp_instance_type().await.ok(),
                region: self.get_gcp_region().await.ok(),
            }));
        }

        // Check Azure metadata endpoint
        if self.check_azure_metadata().await {
            return Ok(Some(DeploymentLayer::CloudLayer {
                provider: CloudProvider::Azure,
                instance_type: self.get_azure_instance_type().await.ok(),
                region: self.get_azure_region().await.ok(),
            }));
        }

        Ok(None)
    }

    /// Detect VM environment
    async fn detect_vm(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        // Check DMI product name
        if let Ok(product) = tokio::fs::read_to_string("/sys/class/dmi/id/product_name").await {
            let product = product.trim().to_lowercase();

            if product.contains("virtualbox") {
                return Ok(Some(DeploymentLayer::VMLayer {
                    hypervisor: "VirtualBox".to_string(),
                    gpu_passthrough: self.detect_gpu_passthrough().await,
                }));
            } else if product.contains("vmware") {
                return Ok(Some(DeploymentLayer::VMLayer {
                    hypervisor: "VMware".to_string(),
                    gpu_passthrough: self.detect_gpu_passthrough().await,
                }));
            } else if product.contains("kvm") || product.contains("qemu") {
                return Ok(Some(DeploymentLayer::VMLayer {
                    hypervisor: "QEMU/KVM".to_string(),
                    gpu_passthrough: self.detect_gpu_passthrough().await,
                }));
            }
        }

        Ok(None)
    }

    /// Detect middleware layer (running on another OS)
    async fn detect_middleware(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        // Check if we're running on Pop!_OS, Ubuntu, etc.
        if let Ok(os_release) = tokio::fs::read_to_string("/etc/os-release").await {
            if !os_release.contains("biomeOS") && !os_release.contains("SteamOS") {
                // We're on a different OS - we're middleware
                let (host_os, host_version) = self.parse_os_release(&os_release);
                return Ok(Some(DeploymentLayer::MiddlewareLayer {
                    host_os,
                    host_version,
                }));
            }
        }

        Ok(None)
    }

    /// Detect service layer (providing to guest OS)
    ///
    /// **Deep Debt**: Runtime detection of managed guests, no hardcoding
    async fn detect_service_layer(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        let mut guest_os = Vec::new();

        // Strategy 1: Check for running QEMU/KVM instances
        if self.check_qemu_running().await {
            guest_os.push("QEMU/KVM guests".to_string());
        }

        // Strategy 2: Check for active Docker containers
        if self.check_docker_running().await {
            guest_os.push("Docker containers".to_string());
        }

        // Strategy 3: Check for Kubernetes/container orchestration
        if self.check_kubernetes_running().await {
            guest_os.push("Kubernetes pods".to_string());
        }

        // Return ServiceLayer if we're managing any guests
        if !guest_os.is_empty() {
            return Ok(Some(DeploymentLayer::ServiceLayer { guest_os }));
        }

        // No guests detected
        Ok(None)
    }

    /// Check if QEMU/KVM is running with guests
    async fn check_qemu_running(&self) -> bool {
        // Check for qemu processes
        if let Ok(output) = tokio::process::Command::new("pgrep")
            .arg("-x")
            .arg("qemu-system-x86_64")
            .output()
            .await
        {
            return output.status.success() && !output.stdout.is_empty();
        }

        // Alternative: Check /dev/kvm usage
        #[cfg(target_os = "linux")]
        {
            if let Ok(_) = tokio::fs::metadata("/dev/kvm").await {
                // KVM device exists, check if actively used
                if let Ok(output) = tokio::process::Command::new("lsof")
                    .arg("/dev/kvm")
                    .output()
                    .await
                {
                    return output.status.success() && !output.stdout.is_empty();
                }
            }
        }

        false
    }

    /// Check if Docker is running with containers
    async fn check_docker_running(&self) -> bool {
        // Check for Docker daemon and active containers
        if let Ok(output) = tokio::process::Command::new("docker")
            .arg("ps")
            .arg("-q")
            .output()
            .await
        {
            return output.status.success() && !output.stdout.is_empty();
        }

        false
    }

    /// Check if Kubernetes is running
    async fn check_kubernetes_running(&self) -> bool {
        // Check for kubelet process (indicates K8s node)
        if let Ok(output) = tokio::process::Command::new("pgrep")
            .arg("-x")
            .arg("kubelet")
            .output()
            .await
        {
            if output.status.success() && !output.stdout.is_empty() {
                return true;
            }
        }

        // Alternative: Check for K8s manifest directory
        if let Ok(_) = tokio::fs::metadata("/etc/kubernetes/manifests").await {
            return true;
        }

        false
    }

    /// Read container ID from cgroup
    async fn read_container_id(&self) -> Result<String, DetectionError> {
        let cgroup = tokio::fs::read_to_string("/proc/self/cgroup").await?;
        // Extract container ID from cgroup path
        // Example: 0::/docker/1234567890abcdef
        if let Some(line) = cgroup.lines().next() {
            if let Some(id) = line.split('/').last() {
                return Ok(id.to_string());
            }
        }
        Err(DetectionError::ContainerIdNotFound)
    }

    /// Check AWS metadata endpoint
    async fn check_aws_metadata(&self) -> bool {
        // PURE RUST: Use environment variables instead of HTTP
        // Use Songbird for external HTTP if needed
        std::env::var("AWS_EXECUTION_ENV").is_ok()
            || std::env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok()
            || std::env::var("ECS_CONTAINER_METADATA_URI").is_ok()
    }

    /// Get AWS instance type
    async fn get_aws_instance_type(&self) -> Result<String, DetectionError> {
        // PURE RUST: Use environment variable or return default
        // For detailed metadata, use Songbird for external HTTP
        std::env::var("AWS_INSTANCE_TYPE")
            .or_else(|_| std::env::var("EC2_INSTANCE_TYPE"))
            .or(Ok("unknown".to_string()))
    }

    /// Get AWS region
    async fn get_aws_region(&self) -> Result<String, DetectionError> {
        // PURE RUST: Use environment variable
        // For detailed metadata, use Songbird for external HTTP
        std::env::var("AWS_REGION")
            .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
            .or(Ok("us-east-1".to_string()))
    }

    /// Check GCP metadata endpoint
    async fn check_gcp_metadata(&self) -> bool {
        // PURE RUST: Use environment variables
        // Use Songbird for external HTTP if needed
        std::env::var("GCP_PROJECT").is_ok()
            || std::env::var("GOOGLE_CLOUD_PROJECT").is_ok()
            || std::env::var("GCLOUD_PROJECT").is_ok()
    }

    /// Get GCP instance type
    async fn get_gcp_instance_type(&self) -> Result<String, DetectionError> {
        // PURE RUST: Use environment variable or return default
        // For detailed metadata, use Songbird for external HTTP
        std::env::var("GCE_MACHINE_TYPE").or(Ok("unknown".to_string()))
    }

    /// Get GCP region
    async fn get_gcp_region(&self) -> Result<String, DetectionError> {
        // PURE RUST: Use environment variable
        // For detailed metadata, use Songbird for external HTTP
        std::env::var("GCE_ZONE")
            .or_else(|_| std::env::var("GOOGLE_CLOUD_ZONE"))
            .or(Ok("unknown".to_string()))
    }

    /// Check Azure metadata endpoint
    async fn check_azure_metadata(&self) -> bool {
        // PURE RUST: Use environment variables
        // Use Songbird for external HTTP if needed
        std::env::var("AZURE_SUBSCRIPTION_ID").is_ok()
            || std::env::var("WEBSITE_INSTANCE_ID").is_ok()
            || std::env::var("FUNCTIONS_WORKER_RUNTIME").is_ok()
    }

    /// Get Azure instance type
    async fn get_azure_instance_type(&self) -> Result<String, DetectionError> {
        // PURE RUST: Use environment variable or return default
        // For detailed metadata, use Songbird for external HTTP
        std::env::var("AZURE_VM_SIZE").or(Ok("unknown".to_string()))
    }

    /// Get Azure region
    async fn get_azure_region(&self) -> Result<String, DetectionError> {
        // PURE RUST: Use environment variable
        // For detailed metadata, use Songbird for external HTTP
        std::env::var("AZURE_LOCATION")
            .or_else(|_| std::env::var("AZURE_REGION"))
            .or(Ok("unknown".to_string()))
    }

    /// Detect GPU passthrough in VM
    async fn detect_gpu_passthrough(&self) -> bool {
        // Check if we have direct GPU access
        // This is a simplified check - real implementation would:
        // - Check PCI devices for GPU
        // - Verify IOMMU groups
        // - Check VFIO bindings
        Path::new("/dev/dri").exists()
    }

    /// Parse OS release file
    fn parse_os_release(&self, content: &str) -> (String, Option<String>) {
        let mut name = String::new();
        let mut version = None;

        for line in content.lines() {
            if let Some(val) = line.strip_prefix("NAME=") {
                name = val.trim_matches('"').to_string();
            } else if let Some(val) = line.strip_prefix("VERSION=") {
                version = Some(val.trim_matches('"').to_string());
            }
        }

        (name, version)
    }
}

impl Default for LayerDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Detection errors
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    /// I/O error during detection
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// External HTTP not available (use Songbird for external HTTP)
    #[error("External HTTP detection disabled - use Songbird for external HTTP")]
    ExternalHttpDisabled,

    /// Container ID not found
    #[error("Container ID not found")]
    ContainerIdNotFound,

    /// Detection failed
    #[error("Failed to detect deployment layer: {0}")]
    DetectionFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_layer_detector_creation() {
        let detector = LayerDetector::new();
        assert!(detector.cached_layer.is_none());
    }

    #[tokio::test]
    async fn test_deployment_layer_display() {
        let layer = DeploymentLayer::BareMetalOS;
        assert_eq!(format!("{}", layer), "BareMetalOS");

        let layer = DeploymentLayer::MiddlewareLayer {
            host_os: "Pop!_OS".to_string(),
            host_version: Some("22.04".to_string()),
        };
        assert_eq!(format!("{}", layer), "Middleware on Pop!_OS");
    }

    #[tokio::test]
    async fn test_deployment_layer_properties() {
        let layer = DeploymentLayer::BareMetalOS;
        assert!(layer.has_direct_hardware_access());
        assert!(!layer.is_virtualized());

        let layer = DeploymentLayer::ContainerLayer {
            runtime: ContainerRuntime::Docker,
            container_id: None,
        };
        assert!(!layer.has_direct_hardware_access());
        assert!(layer.is_virtualized());
    }
}
