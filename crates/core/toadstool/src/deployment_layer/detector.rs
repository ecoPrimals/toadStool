// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! Layer detection logic — identifies container, VM, cloud, and host environment.

use super::{CloudProvider, ContainerRuntime, DeploymentLayer, DetectionError};
use std::path::Path;
use toadstool_common::constants::platform_paths::{devfs, etc_paths, procfs, sysfs};
use toadstool_common::interned_strings::socket_env;

/// Layer detector for identifying deployment environment
///
/// Uses multiple heuristics to determine where Toadstool is running.
pub struct LayerDetector {
    pub(crate) cached_layer: Option<DeploymentLayer>,
}

impl LayerDetector {
    /// Create a new layer detector
    pub const fn new() -> Self {
        Self { cached_layer: None }
    }

    /// Detect the current deployment layer
    ///
    /// # Errors
    ///
    /// Returns error if filesystem or environment probing fails during detection.
    pub async fn detect(&mut self) -> Result<DeploymentLayer, DetectionError> {
        if let Some(layer) = &self.cached_layer {
            return Ok(layer.clone());
        }
        let layer = self.detect_layer_internal()?;
        self.cached_layer = Some(layer.clone());
        Ok(layer)
    }

    /// Force re-detection (clears cache)
    pub fn reset(&mut self) {
        self.cached_layer = None;
    }

    fn detect_layer_internal(&self) -> Result<DeploymentLayer, DetectionError> {
        if let Some(container) = self.detect_container()? {
            return Ok(container);
        }
        if let Some(cloud) = self.detect_cloud()? {
            return Ok(cloud);
        }
        if let Some(vm) = self.detect_vm()? {
            return Ok(vm);
        }
        if let Some(middleware) = self.detect_middleware()? {
            return Ok(middleware);
        }
        if let Some(service) = self.detect_service_layer()? {
            return Ok(service);
        }
        Ok(DeploymentLayer::BareMetalOS)
    }

    fn detect_container(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        if Path::new("/.dockerenv").exists() {
            return Ok(Some(DeploymentLayer::ContainerLayer {
                runtime: ContainerRuntime::Docker,
                container_id: self.read_container_id().ok(),
            }));
        }
        if Path::new("/run/.containerenv").exists() {
            return Ok(Some(DeploymentLayer::ContainerLayer {
                runtime: ContainerRuntime::Podman,
                container_id: self.read_container_id().ok(),
            }));
        }
        if let Ok(cgroup) = std::fs::read_to_string(procfs::PROC_INIT_CGROUP) {
            if cgroup.contains("docker") {
                return Ok(Some(DeploymentLayer::ContainerLayer {
                    runtime: ContainerRuntime::Docker,
                    container_id: self.read_container_id().ok(),
                }));
            } else if cgroup.contains("podman") {
                return Ok(Some(DeploymentLayer::ContainerLayer {
                    runtime: ContainerRuntime::Podman,
                    container_id: self.read_container_id().ok(),
                }));
            }
        }
        Ok(None)
    }

    fn detect_cloud(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        if self.check_aws_metadata() {
            return Ok(Some(DeploymentLayer::CloudLayer {
                provider: CloudProvider::AWS,
                instance_type: self.get_aws_instance_type().ok(),
                region: self.get_aws_region().ok(),
            }));
        }
        if self.check_gcp_metadata() {
            return Ok(Some(DeploymentLayer::CloudLayer {
                provider: CloudProvider::GCP,
                instance_type: self.get_gcp_instance_type().ok(),
                region: self.get_gcp_region().ok(),
            }));
        }
        if self.check_azure_metadata() {
            return Ok(Some(DeploymentLayer::CloudLayer {
                provider: CloudProvider::Azure,
                instance_type: self.get_azure_instance_type().ok(),
                region: self.get_azure_region().ok(),
            }));
        }
        Ok(None)
    }

    fn detect_vm(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        if let Ok(product) = std::fs::read_to_string(sysfs::CLASS_DMI_ID_PRODUCT_NAME) {
            let product = product.trim().to_lowercase();
            if product.contains("virtualbox") {
                return Ok(Some(DeploymentLayer::VMLayer {
                    hypervisor: "VirtualBox".to_string(),
                    gpu_passthrough: self.detect_gpu_passthrough(),
                }));
            }
            if product.contains("vmware") {
                return Ok(Some(DeploymentLayer::VMLayer {
                    hypervisor: "VMware".to_string(),
                    gpu_passthrough: self.detect_gpu_passthrough(),
                }));
            }
            if product.contains("kvm") || product.contains("qemu") {
                return Ok(Some(DeploymentLayer::VMLayer {
                    hypervisor: "QEMU/KVM".to_string(),
                    gpu_passthrough: self.detect_gpu_passthrough(),
                }));
            }
        }
        Ok(None)
    }

    fn detect_middleware(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        if let Ok(os_release) = std::fs::read_to_string(etc_paths::OS_RELEASE) {
            use toadstool_common::constants::ecosystem::os_identifiers;
            if !os_release.contains(os_identifiers::BIOMEOS)
                && !os_release.contains(os_identifiers::STEAMOS)
            {
                let (host_os, host_version) = self.parse_os_release(&os_release);
                return Ok(Some(DeploymentLayer::MiddlewareLayer {
                    host_os,
                    host_version,
                }));
            }
        }
        Ok(None)
    }

    fn detect_service_layer(&self) -> Result<Option<DeploymentLayer>, DetectionError> {
        let mut guest_os = Vec::new();
        if self.check_qemu_running() {
            guest_os.push("QEMU/KVM guests".to_string());
        }
        if self.check_docker_running() {
            guest_os.push("Docker containers".to_string());
        }
        if self.check_kubernetes_running() {
            guest_os.push("Kubernetes pods".to_string());
        }
        if !guest_os.is_empty() {
            return Ok(Some(DeploymentLayer::ServiceLayer { guest_os }));
        }
        Ok(None)
    }

    fn check_qemu_running(&self) -> bool {
        if let Ok(output) = std::process::Command::new("pgrep")
            .arg("-x")
            .arg("qemu-system-x86_64")
            .output()
        {
            return output.status.success() && !output.stdout.is_empty();
        }
        #[cfg(target_os = "linux")]
        if std::fs::metadata(devfs::KVM).is_ok() {
            if let Ok(output) = std::process::Command::new("lsof").arg(devfs::KVM).output() {
                return output.status.success() && !output.stdout.is_empty();
            }
        }
        false
    }

    fn check_docker_running(&self) -> bool {
        if let Ok(output) = std::process::Command::new("docker")
            .arg("ps")
            .arg("-q")
            .output()
        {
            return output.status.success() && !output.stdout.is_empty();
        }
        false
    }

    fn check_kubernetes_running(&self) -> bool {
        if let Ok(output) = std::process::Command::new("pgrep")
            .arg("-x")
            .arg("kubelet")
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                return true;
            }
        }
        std::fs::metadata(etc_paths::KUBERNETES_MANIFESTS).is_ok()
    }

    fn read_container_id(&self) -> Result<String, DetectionError> {
        let cgroup = std::fs::read_to_string(procfs::PROC_SELF_CGROUP)?;
        if let Some(line) = cgroup.lines().next() {
            if let Some(id) = line.split('/').next_back() {
                return Ok(id.to_string());
            }
        }
        Err(DetectionError::ContainerIdNotFound)
    }

    fn check_aws_metadata(&self) -> bool {
        std::env::var(socket_env::AWS_EXECUTION_ENV).is_ok()
            || std::env::var(socket_env::AWS_LAMBDA_FUNCTION_NAME).is_ok()
            || std::env::var(socket_env::ECS_CONTAINER_METADATA_URI).is_ok()
    }

    fn get_aws_instance_type(&self) -> Result<String, DetectionError> {
        std::env::var(socket_env::AWS_INSTANCE_TYPE)
            .or_else(|_| std::env::var(socket_env::EC2_INSTANCE_TYPE))
            .or_else(|_| Ok("unknown".to_string()))
    }

    fn get_aws_region(&self) -> Result<String, DetectionError> {
        std::env::var(socket_env::AWS_REGION)
            .or_else(|_| std::env::var(socket_env::AWS_DEFAULT_REGION))
            .or_else(|_| Ok("us-east-1".to_string()))
    }

    fn check_gcp_metadata(&self) -> bool {
        std::env::var(socket_env::GCP_PROJECT).is_ok()
            || std::env::var(socket_env::GOOGLE_CLOUD_PROJECT).is_ok()
            || std::env::var(socket_env::GCLOUD_PROJECT).is_ok()
    }

    fn get_gcp_instance_type(&self) -> Result<String, DetectionError> {
        std::env::var(socket_env::GCE_MACHINE_TYPE).or_else(|_| Ok("unknown".to_string()))
    }

    fn get_gcp_region(&self) -> Result<String, DetectionError> {
        std::env::var(socket_env::GCE_ZONE)
            .or_else(|_| std::env::var(socket_env::GOOGLE_CLOUD_ZONE))
            .or_else(|_| Ok("unknown".to_string()))
    }

    fn check_azure_metadata(&self) -> bool {
        std::env::var(socket_env::AZURE_SUBSCRIPTION_ID).is_ok()
            || std::env::var(socket_env::WEBSITE_INSTANCE_ID).is_ok()
            || std::env::var(socket_env::FUNCTIONS_WORKER_RUNTIME).is_ok()
    }

    fn get_azure_instance_type(&self) -> Result<String, DetectionError> {
        std::env::var(socket_env::AZURE_VM_SIZE).or_else(|_| Ok("unknown".to_string()))
    }

    fn get_azure_region(&self) -> Result<String, DetectionError> {
        std::env::var(socket_env::AZURE_LOCATION)
            .or_else(|_| std::env::var(socket_env::AZURE_REGION))
            .or_else(|_| Ok("unknown".to_string()))
    }

    fn detect_gpu_passthrough(&self) -> bool {
        Path::new(devfs::DRI_DIR).exists()
    }

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

#[cfg(test)]
#[path = "detector_tests.rs"]
mod tests;
