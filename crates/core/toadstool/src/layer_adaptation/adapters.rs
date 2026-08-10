// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! Layer-specific capability adapters
//!
//! Adapts capabilities based on deployment layer (bare metal, middleware,
//! service, container, VM, cloud).

use crate::deployment_layer::DeploymentLayer;
use std::collections::HashMap;

#[cfg(feature = "runtime")]
use super::detection::{
    detect_network_bandwidth, detect_storage_read_bandwidth, detect_storage_write_bandwidth,
    get_available_disk, get_total_memory,
};
use super::types::{
    AdaptedCapabilities, CapabilityMetadata, ComputeCapabilities, GpuAccess, NetworkAccess,
    NetworkCapabilities, StorageCapabilities, StorageType,
};

/// Layer capability adapter
///
/// Adapts capabilities based on deployment layer.
pub struct LayerCapabilityAdapter {
    /// The deployment layer
    layer: DeploymentLayer,
}

impl LayerCapabilityAdapter {
    /// Create a new adapter for a deployment layer
    pub const fn new(layer: DeploymentLayer) -> Self {
        Self { layer }
    }

    /// Get adapted capabilities for this layer (stub for no-runtime)
    #[cfg(not(feature = "runtime"))]
    pub async fn get_adapted_capabilities(&self) -> AdaptedCapabilities {
        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: GpuAccess::None,
                has_cpu: true,
                cpu_cores: None,
                memory_bytes: None,
                supports_tensor_ops: false,
                supports_nn_training: false,
                supports_nn_inference: false,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::HostFilesystem,
                available_bytes: None,
                read_bandwidth: None,
                write_bandwidth: None,
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::Direct,
                bandwidth: None,
                latency_ms: None,
                has_service_mesh: false,
            },
            metadata: CapabilityMetadata {
                layer: format!("{}", self.layer),
                host_os: None,
                cloud_provider: None,
                extra: std::collections::HashMap::new(),
            },
        }
    }
}

#[cfg(feature = "runtime")]
impl LayerCapabilityAdapter {
    /// Get adapted capabilities for this layer
    #[cfg(feature = "runtime")]
    pub async fn get_adapted_capabilities(&self) -> AdaptedCapabilities {
        match &self.layer {
            DeploymentLayer::BareMetalOS => self.adapt_bare_metal().await,
            DeploymentLayer::MiddlewareLayer { host_os, .. } => {
                self.adapt_middleware(host_os).await
            }
            DeploymentLayer::ServiceLayer { .. } => self.adapt_service_layer().await,
            DeploymentLayer::ContainerLayer { .. } => self.adapt_container().await,
            DeploymentLayer::VMLayer {
                gpu_passthrough, ..
            } => self.adapt_vm(*gpu_passthrough).await,
            DeploymentLayer::CloudLayer { provider, .. } => self.adapt_cloud(provider).await,
        }
    }

    /// Adapt capabilities for bare metal
    async fn adapt_bare_metal(&self) -> AdaptedCapabilities {
        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: GpuAccess::Direct,
                has_cpu: true,
                cpu_cores: Some(
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4),
                ),
                memory_bytes: get_total_memory().await,
                supports_tensor_ops: true,
                supports_nn_training: true,
                supports_nn_inference: true,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::DirectBlock,
                available_bytes: get_available_disk(),
                read_bandwidth: detect_storage_read_bandwidth(),
                write_bandwidth: detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::Direct,
                bandwidth: detect_network_bandwidth(),
                latency_ms: Some(1), // Local network
                has_service_mesh: false,
            },
            metadata: CapabilityMetadata {
                layer: "BareMetalOS".to_string(),
                host_os: None,
                cloud_provider: None,
                extra: HashMap::new(),
            },
        }
    }

    /// Adapt capabilities for middleware layer
    async fn adapt_middleware(&self, host_os: &str) -> AdaptedCapabilities {
        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: GpuAccess::ViaHost,
                has_cpu: true,
                cpu_cores: Some(
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4),
                ),
                memory_bytes: get_total_memory().await,
                supports_tensor_ops: true,
                supports_nn_training: true,
                supports_nn_inference: true,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::HostFilesystem,
                available_bytes: get_available_disk(),
                read_bandwidth: detect_storage_read_bandwidth(),
                write_bandwidth: detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::Direct,
                bandwidth: detect_network_bandwidth(),
                latency_ms: Some(1),
                has_service_mesh: false,
            },
            metadata: CapabilityMetadata {
                layer: "MiddlewareLayer".to_string(),
                host_os: Some(host_os.to_string()),
                cloud_provider: None,
                extra: HashMap::new(),
            },
        }
    }

    /// Adapt capabilities for service layer
    async fn adapt_service_layer(&self) -> AdaptedCapabilities {
        // Service layer exposes capabilities to guest OS
        // Similar to bare metal but may have resource limits
        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: GpuAccess::Direct,
                has_cpu: true,
                cpu_cores: Some(
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4),
                ),
                memory_bytes: get_total_memory().await,
                supports_tensor_ops: true,
                supports_nn_training: true,
                supports_nn_inference: true,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::DirectBlock,
                available_bytes: get_available_disk(),
                read_bandwidth: detect_storage_read_bandwidth(),
                write_bandwidth: detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::Direct,
                bandwidth: detect_network_bandwidth(),
                latency_ms: Some(1),
                has_service_mesh: true, // Often has service mesh for guests
            },
            metadata: CapabilityMetadata {
                layer: "ServiceLayer".to_string(),
                host_os: None,
                cloud_provider: None,
                extra: HashMap::new(),
            },
        }
    }

    /// Adapt capabilities for container
    async fn adapt_container(&self) -> AdaptedCapabilities {
        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: GpuAccess::ViaHost, // Can be Direct with nvidia-container-runtime
                has_cpu: true,
                cpu_cores: Some(
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4),
                ), // May be limited by cgroups
                memory_bytes: get_total_memory().await, // May be limited by cgroups
                supports_tensor_ops: true,
                supports_nn_training: true,
                supports_nn_inference: true,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::PersistentVolume,
                available_bytes: get_available_disk(),
                read_bandwidth: detect_storage_read_bandwidth(),
                write_bandwidth: detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::HostNamespace,
                bandwidth: detect_network_bandwidth(),
                latency_ms: Some(1),
                has_service_mesh: true, // Kubernetes service mesh
            },
            metadata: CapabilityMetadata {
                layer: "ContainerLayer".to_string(),
                host_os: None,
                cloud_provider: None,
                extra: HashMap::new(),
            },
        }
    }

    /// Adapt capabilities for VM
    async fn adapt_vm(&self, gpu_passthrough: bool) -> AdaptedCapabilities {
        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: if gpu_passthrough {
                    GpuAccess::Direct
                } else {
                    GpuAccess::None
                },
                has_cpu: true,
                cpu_cores: Some(
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4),
                ),
                memory_bytes: get_total_memory().await,
                supports_tensor_ops: gpu_passthrough,
                supports_nn_training: gpu_passthrough,
                supports_nn_inference: true, // CPU fallback available
            },
            storage: StorageCapabilities {
                storage_type: StorageType::DirectBlock,
                available_bytes: get_available_disk(),
                read_bandwidth: detect_storage_read_bandwidth(),
                write_bandwidth: detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::Direct,
                bandwidth: detect_network_bandwidth(),
                latency_ms: Some(2), // Slight overhead vs bare metal
                has_service_mesh: false,
            },
            metadata: CapabilityMetadata {
                layer: "VMLayer".to_string(),
                host_os: None,
                cloud_provider: None,
                extra: {
                    let mut extra = HashMap::new();
                    extra.insert("gpu_passthrough".to_string(), gpu_passthrough.to_string());
                    extra
                },
            },
        }
    }

    /// Adapt capabilities for cloud
    async fn adapt_cloud(
        &self,
        provider: &crate::deployment_layer::CloudProvider,
    ) -> AdaptedCapabilities {
        let provider_name = match provider {
            crate::deployment_layer::CloudProvider::AWS => "AWS",
            crate::deployment_layer::CloudProvider::GCP => "GCP",
            crate::deployment_layer::CloudProvider::Azure => "Azure",
            crate::deployment_layer::CloudProvider::Oracle => "Oracle",
            crate::deployment_layer::CloudProvider::DigitalOcean => "DigitalOcean",
            crate::deployment_layer::CloudProvider::Custom(name) => name,
        };

        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: GpuAccess::ViaCloud,
                has_cpu: true,
                cpu_cores: Some(
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4),
                ),
                memory_bytes: get_total_memory().await,
                supports_tensor_ops: true, // Cloud GPUs support this
                supports_nn_training: true,
                supports_nn_inference: true,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::CloudObject,
                available_bytes: None, // Effectively unlimited in cloud
                read_bandwidth: Some(1_000_000_000), // 1 GB/s typical
                write_bandwidth: Some(1_000_000_000),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::CloudVPC,
                bandwidth: Some(10_000_000_000), // 10 Gbps typical
                latency_ms: Some(10),            // Inter-region latency
                has_service_mesh: true,          // Cloud service mesh
            },
            metadata: CapabilityMetadata {
                layer: "CloudLayer".to_string(),
                host_os: None,
                cloud_provider: Some(provider_name.to_string()),
                extra: HashMap::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deployment_layer::{CloudProvider, ContainerRuntime, DeploymentLayer};
    use crate::layer_adaptation::{
        GpuAccess, NetworkAccess, StorageType, compute_capabilities, network_capabilities,
        storage_capabilities,
    };

    #[tokio::test]
    async fn test_bare_metal_adaptation() {
        let layer = DeploymentLayer::BareMetalOS;
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities().await;

        assert_eq!(caps.compute.gpu_access, GpuAccess::Direct);
        assert!(caps.compute.has_cpu);
        assert!(caps.compute.supports_tensor_ops);
        assert_eq!(caps.storage.storage_type, StorageType::DirectBlock);
        assert_eq!(caps.network.network_access, NetworkAccess::Direct);
        assert!(caps.has_direct_gpu_access());
        assert!(caps.has_gpu_access());
    }

    #[tokio::test]
    async fn test_middleware_adaptation() {
        let layer = DeploymentLayer::MiddlewareLayer {
            host_os: "Pop!_OS".to_string(),
            host_version: Some("22.04".to_string()),
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities().await;

        assert_eq!(caps.compute.gpu_access, GpuAccess::ViaHost);
        assert_eq!(caps.storage.storage_type, StorageType::HostFilesystem);
        assert_eq!(caps.metadata.host_os, Some("Pop!_OS".to_string()));
        assert!(!caps.has_direct_gpu_access());
        assert!(caps.has_gpu_access());
    }

    #[tokio::test]
    async fn test_vm_adaptation_with_passthrough() {
        let layer = DeploymentLayer::VMLayer {
            hypervisor: "QEMU/KVM".to_string(),
            gpu_passthrough: true,
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities().await;

        assert_eq!(caps.compute.gpu_access, GpuAccess::Direct);
        assert!(caps.compute.supports_tensor_ops);
        assert!(caps.has_direct_gpu_access());
    }

    #[tokio::test]
    async fn test_vm_adaptation_without_passthrough() {
        let layer = DeploymentLayer::VMLayer {
            hypervisor: "VirtualBox".to_string(),
            gpu_passthrough: false,
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities().await;

        assert_eq!(caps.compute.gpu_access, GpuAccess::None);
        assert!(!caps.compute.supports_tensor_ops);
        assert!(caps.compute.supports_nn_inference); // CPU fallback
        assert!(!caps.has_gpu_access());
    }

    #[tokio::test]
    async fn test_cloud_adaptation() {
        let layer = DeploymentLayer::CloudLayer {
            provider: CloudProvider::AWS,
            instance_type: Some("g5.4xlarge".to_string()),
            region: Some("us-east-1".to_string()),
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities().await;

        assert_eq!(caps.compute.gpu_access, GpuAccess::ViaCloud);
        assert_eq!(caps.storage.storage_type, StorageType::CloudObject);
        assert_eq!(caps.network.network_access, NetworkAccess::CloudVPC);
        assert_eq!(caps.metadata.cloud_provider, Some("AWS".to_string()));
        assert!(!caps.has_direct_gpu_access());
        assert!(caps.has_gpu_access());
    }

    #[tokio::test]
    async fn test_capability_list_generation() {
        let layer = DeploymentLayer::BareMetalOS;
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities().await;
        let cap_list = caps.to_capability_list();

        assert!(cap_list.contains(&compute_capabilities::GPU_COMPUTE_DIRECT.to_string()));
        assert!(cap_list.contains(&compute_capabilities::CPU_COMPUTE.to_string()));
        assert!(cap_list.contains(&compute_capabilities::TENSOR_OPS.to_string()));
        assert!(cap_list.contains(&storage_capabilities::BLOCK_STORAGE_DIRECT.to_string()));
        assert!(cap_list.contains(&network_capabilities::NETWORK_DIRECT.to_string()));
    }

    #[tokio::test]
    async fn test_container_adaptation() {
        let layer = DeploymentLayer::ContainerLayer {
            runtime: ContainerRuntime::Docker,
            container_id: Some("abc123".to_string()),
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities().await;

        assert_eq!(caps.compute.gpu_access, GpuAccess::ViaHost);
        assert_eq!(caps.storage.storage_type, StorageType::PersistentVolume);
        assert_eq!(caps.network.network_access, NetworkAccess::HostNamespace);
        assert!(caps.network.has_service_mesh);
    }
}
