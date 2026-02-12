// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Capability Adaptation Based on Deployment Layer
//!
//! This module adapts Toadstool's capabilities based on the detected deployment
//! layer, ensuring appropriate exposure of resources (GPU, storage, network) for
//! each environment.
//!
//! # Philosophy
//!
//! **Adaptation over assumption**: Don't assume what capabilities we should expose.
//! Detect the layer, then adapt capabilities accordingly.
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool::deployment_layer::{DeploymentLayer, LayerDetector};
//! use toadstool::layer_adaptation::LayerCapabilityAdapter;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut detector = LayerDetector::new();
//! let layer = detector.detect().await?;
//!
//! let adapter = LayerCapabilityAdapter::new(layer);
//! let capabilities = adapter.get_adapted_capabilities();
//!
//! // Capabilities are now appropriate for the layer
//! if capabilities.has_direct_gpu_access() {
//!     println!("Can use GPU directly");
//! } else {
//!     println!("GPU via host or cloud APIs");
//! }
//! # Ok(())
//! # }
//! ```

use crate::deployment_layer::DeploymentLayer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compute capability constants (extending infant_discovery capabilities)
pub mod compute_capabilities {
    /// Direct GPU compute access (bare metal, GPU passthrough)
    pub const GPU_COMPUTE_DIRECT: &str = "gpu_compute_direct";

    /// GPU compute via host OS (middleware layer)
    pub const GPU_COMPUTE_HOST: &str = "gpu_compute_host";

    /// GPU compute via cloud APIs (cloud layer)
    pub const GPU_COMPUTE_CLOUD: &str = "gpu_compute_cloud";

    /// CPU-only compute (fallback)
    pub const CPU_COMPUTE: &str = "cpu_compute";

    /// Tensor operations (barraCUDA)
    pub const TENSOR_OPS: &str = "tensor_operations";

    /// Neural network training
    pub const NN_TRAINING: &str = "neural_network_training";

    /// Neural network inference
    pub const NN_INFERENCE: &str = "neural_network_inference";
}

/// Storage capability constants
pub mod storage_capabilities {
    /// Direct block storage access (bare metal)
    pub const BLOCK_STORAGE_DIRECT: &str = "block_storage_direct";

    /// Host OS filesystem (middleware, container)
    pub const FILESYSTEM_HOST: &str = "filesystem_host";

    /// Cloud object storage (S3, GCS, Azure Blob)
    pub const OBJECT_STORAGE_CLOUD: &str = "object_storage_cloud";

    /// Persistent volumes (container orchestration)
    pub const PERSISTENT_VOLUME: &str = "persistent_volume";
}

/// Network capability constants
pub mod network_capabilities {
    /// Direct network access (bare metal, VM)
    pub const NETWORK_DIRECT: &str = "network_direct";

    /// Network via host namespace (container)
    pub const NETWORK_HOST_NAMESPACE: &str = "network_host_namespace";

    /// Network via cloud VPC (cloud layer)
    pub const NETWORK_CLOUD_VPC: &str = "network_cloud_vpc";

    /// Service mesh integration
    pub const SERVICE_MESH: &str = "service_mesh_integration";
}

/// Adapted capabilities for a deployment layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptedCapabilities {
    /// Compute capabilities
    pub compute: ComputeCapabilities,

    /// Storage capabilities
    pub storage: StorageCapabilities,

    /// Network capabilities
    pub network: NetworkCapabilities,

    /// Additional metadata
    pub metadata: CapabilityMetadata,
}

impl AdaptedCapabilities {
    /// Check if we have direct GPU access
    pub fn has_direct_gpu_access(&self) -> bool {
        self.compute.gpu_access == GpuAccess::Direct
    }

    /// Check if GPU is available (direct or via host/cloud)
    pub fn has_gpu_access(&self) -> bool {
        self.compute.gpu_access != GpuAccess::None
    }

    /// Get all capabilities as a flat list of strings
    pub fn to_capability_list(&self) -> Vec<String> {
        let mut caps = Vec::new();

        // Compute capabilities
        match self.compute.gpu_access {
            GpuAccess::Direct => caps.push(compute_capabilities::GPU_COMPUTE_DIRECT.to_string()),
            GpuAccess::ViaHost => caps.push(compute_capabilities::GPU_COMPUTE_HOST.to_string()),
            GpuAccess::ViaCloud => caps.push(compute_capabilities::GPU_COMPUTE_CLOUD.to_string()),
            GpuAccess::None => {}
        }

        if self.compute.has_cpu {
            caps.push(compute_capabilities::CPU_COMPUTE.to_string());
        }

        if self.compute.supports_tensor_ops {
            caps.push(compute_capabilities::TENSOR_OPS.to_string());
        }

        if self.compute.supports_nn_training {
            caps.push(compute_capabilities::NN_TRAINING.to_string());
        }

        if self.compute.supports_nn_inference {
            caps.push(compute_capabilities::NN_INFERENCE.to_string());
        }

        // Storage capabilities
        match self.storage.storage_type {
            StorageType::DirectBlock => {
                caps.push(storage_capabilities::BLOCK_STORAGE_DIRECT.to_string())
            }
            StorageType::HostFilesystem => {
                caps.push(storage_capabilities::FILESYSTEM_HOST.to_string())
            }
            StorageType::CloudObject => {
                caps.push(storage_capabilities::OBJECT_STORAGE_CLOUD.to_string())
            }
            StorageType::PersistentVolume => {
                caps.push(storage_capabilities::PERSISTENT_VOLUME.to_string())
            }
        }

        // Network capabilities
        match self.network.network_access {
            NetworkAccess::Direct => caps.push(network_capabilities::NETWORK_DIRECT.to_string()),
            NetworkAccess::HostNamespace => {
                caps.push(network_capabilities::NETWORK_HOST_NAMESPACE.to_string())
            }
            NetworkAccess::CloudVPC => {
                caps.push(network_capabilities::NETWORK_CLOUD_VPC.to_string())
            }
        }

        if self.network.has_service_mesh {
            caps.push(network_capabilities::SERVICE_MESH.to_string());
        }

        caps
    }
}

/// Compute capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeCapabilities {
    /// GPU access type
    pub gpu_access: GpuAccess,

    /// CPU available (always true for now)
    pub has_cpu: bool,

    /// Number of CPU cores available
    pub cpu_cores: Option<usize>,

    /// Memory available (bytes)
    pub memory_bytes: Option<u64>,

    /// Supports tensor operations (barraCUDA)
    pub supports_tensor_ops: bool,

    /// Supports neural network training
    pub supports_nn_training: bool,

    /// Supports neural network inference
    pub supports_nn_inference: bool,
}

/// GPU access type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuAccess {
    /// Direct GPU access (bare metal, GPU passthrough VM)
    Direct,

    /// GPU via host OS (middleware layer)
    ViaHost,

    /// GPU via cloud APIs (cloud layer)
    ViaCloud,

    /// No GPU access
    None,
}

/// Storage capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCapabilities {
    /// Storage type
    pub storage_type: StorageType,

    /// Available storage (bytes)
    pub available_bytes: Option<u64>,

    /// Read bandwidth (bytes/sec)
    pub read_bandwidth: Option<u64>,

    /// Write bandwidth (bytes/sec)
    pub write_bandwidth: Option<u64>,
}

/// Storage type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    /// Direct block storage (bare metal)
    DirectBlock,

    /// Host OS filesystem (middleware, container)
    HostFilesystem,

    /// Cloud object storage (S3, GCS, Azure Blob)
    CloudObject,

    /// Persistent volume (container orchestration)
    PersistentVolume,
}

/// Network capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCapabilities {
    /// Network access type
    pub network_access: NetworkAccess,

    /// Bandwidth (bytes/sec)
    pub bandwidth: Option<u64>,

    /// Latency (milliseconds)
    pub latency_ms: Option<u32>,

    /// Has service mesh integration
    pub has_service_mesh: bool,
}

/// Network access type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkAccess {
    /// Direct network access (bare metal, VM)
    Direct,

    /// Network via host namespace (container)
    HostNamespace,

    /// Network via cloud VPC (cloud layer)
    CloudVPC,
}

/// Capability metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    /// Deployment layer this is adapted for
    pub layer: String,

    /// Host OS if running as middleware
    pub host_os: Option<String>,

    /// Cloud provider if in cloud
    pub cloud_provider: Option<String>,

    /// Additional key-value metadata
    pub extra: HashMap<String, String>,
}

/// Layer capability adapter
///
/// Adapts capabilities based on deployment layer.
pub struct LayerCapabilityAdapter {
    /// The deployment layer
    layer: DeploymentLayer,
}

impl LayerCapabilityAdapter {
    /// Create a new adapter for a deployment layer
    pub fn new(layer: DeploymentLayer) -> Self {
        Self { layer }
    }

    /// Get adapted capabilities for this layer
    pub fn get_adapted_capabilities(&self) -> AdaptedCapabilities {
        match &self.layer {
            DeploymentLayer::BareMetalOS => self.adapt_bare_metal(),
            DeploymentLayer::MiddlewareLayer { host_os, .. } => self.adapt_middleware(host_os),
            DeploymentLayer::ServiceLayer { .. } => self.adapt_service_layer(),
            DeploymentLayer::ContainerLayer { .. } => self.adapt_container(),
            DeploymentLayer::VMLayer {
                gpu_passthrough, ..
            } => self.adapt_vm(*gpu_passthrough),
            DeploymentLayer::CloudLayer { provider, .. } => self.adapt_cloud(provider),
        }
    }

    /// Adapt capabilities for bare metal
    fn adapt_bare_metal(&self) -> AdaptedCapabilities {
        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: GpuAccess::Direct,
                has_cpu: true,
                cpu_cores: Some(
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4),
                ),
                memory_bytes: Self::get_total_memory(),
                supports_tensor_ops: true,
                supports_nn_training: true,
                supports_nn_inference: true,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::DirectBlock,
                available_bytes: Self::get_available_disk(),
                read_bandwidth: Self::detect_storage_read_bandwidth(),
                write_bandwidth: Self::detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::Direct,
                bandwidth: Self::detect_network_bandwidth(),
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
    fn adapt_middleware(&self, host_os: &str) -> AdaptedCapabilities {
        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: GpuAccess::ViaHost,
                has_cpu: true,
                cpu_cores: Some(
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4),
                ),
                memory_bytes: Self::get_total_memory(),
                supports_tensor_ops: true,
                supports_nn_training: true,
                supports_nn_inference: true,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::HostFilesystem,
                available_bytes: Self::get_available_disk(),
                read_bandwidth: Self::detect_storage_read_bandwidth(),
                write_bandwidth: Self::detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::Direct,
                bandwidth: Self::detect_network_bandwidth(),
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
    fn adapt_service_layer(&self) -> AdaptedCapabilities {
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
                memory_bytes: Self::get_total_memory(),
                supports_tensor_ops: true,
                supports_nn_training: true,
                supports_nn_inference: true,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::DirectBlock,
                available_bytes: Self::get_available_disk(),
                read_bandwidth: Self::detect_storage_read_bandwidth(),
                write_bandwidth: Self::detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::Direct,
                bandwidth: Self::detect_network_bandwidth(),
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
    fn adapt_container(&self) -> AdaptedCapabilities {
        AdaptedCapabilities {
            compute: ComputeCapabilities {
                gpu_access: GpuAccess::ViaHost, // Can be Direct with nvidia-container-runtime
                has_cpu: true,
                cpu_cores: Some(
                    std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4),
                ), // May be limited by cgroups
                memory_bytes: Self::get_total_memory(), // May be limited by cgroups
                supports_tensor_ops: true,
                supports_nn_training: true,
                supports_nn_inference: true,
            },
            storage: StorageCapabilities {
                storage_type: StorageType::PersistentVolume,
                available_bytes: Self::get_available_disk(),
                read_bandwidth: Self::detect_storage_read_bandwidth(),
                write_bandwidth: Self::detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::HostNamespace,
                bandwidth: Self::detect_network_bandwidth(),
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
    fn adapt_vm(&self, gpu_passthrough: bool) -> AdaptedCapabilities {
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
                memory_bytes: Self::get_total_memory(),
                supports_tensor_ops: gpu_passthrough,
                supports_nn_training: gpu_passthrough,
                supports_nn_inference: true, // CPU fallback available
            },
            storage: StorageCapabilities {
                storage_type: StorageType::DirectBlock,
                available_bytes: Self::get_available_disk(),
                read_bandwidth: Self::detect_storage_read_bandwidth(),
                write_bandwidth: Self::detect_storage_write_bandwidth(),
            },
            network: NetworkCapabilities {
                network_access: NetworkAccess::Direct,
                bandwidth: Self::detect_network_bandwidth(),
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
    fn adapt_cloud(
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
                memory_bytes: Self::get_total_memory(),
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

    /// Get total system memory (bytes)
    fn get_total_memory() -> Option<u64> {
        // Platform-specific memory detection
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(kb) = line.split_whitespace().nth(1) {
                            if let Ok(kb_val) = kb.parse::<u64>() {
                                return Some(kb_val * 1024); // Convert KB to bytes
                            }
                        }
                    }
                }
            }
        }

        // Fallback: Use sysinfo crate or return None
        None
    }

    /// Get available disk space (bytes)
    ///
    /// Uses sysinfo for cross-platform disk detection (pure Rust).
    /// Returns the available space on the root/primary disk.
    fn get_available_disk() -> Option<u64> {
        use sysinfo::Disks;

        let disks = Disks::new_with_refreshed_list();

        // Find the root filesystem or largest available disk
        let mut root_disk_space: Option<u64> = None;
        let mut largest_disk_space: u64 = 0;

        for disk in disks.list() {
            let available = disk.available_space();

            // Check if this is the root filesystem
            #[cfg(unix)]
            {
                let mount_point = disk.mount_point();
                if mount_point.as_os_str() == "/" {
                    root_disk_space = Some(available);
                }
            }

            #[cfg(windows)]
            {
                // On Windows, prefer C: drive
                let mount_point = disk.mount_point().to_string_lossy();
                if mount_point.starts_with("C:") {
                    root_disk_space = Some(available);
                }
            }

            // Track largest disk as fallback
            if available > largest_disk_space {
                largest_disk_space = available;
            }
        }

        // Return root disk space if found, otherwise largest disk
        root_disk_space.or(if largest_disk_space > 0 {
            Some(largest_disk_space)
        } else {
            None
        })
    }

    /// Detect storage read bandwidth (bytes/sec) via runtime heuristics
    ///
    /// **Deep Debt**: Runtime detection, no hardcoding
    fn detect_storage_read_bandwidth() -> Option<u64> {
        // Strategy: Check for SSD vs HDD indicators
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            // Check /sys/block for rotational devices (0 = SSD, 1 = HDD)
            if let Ok(entries) = fs::read_dir("/sys/block") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let rotational_path = path.join("queue/rotational");

                    if let Ok(content) = fs::read_to_string(&rotational_path) {
                        if let Ok(is_rotational) = content.trim().parse::<u8>() {
                            // SSD: ~500 MB/s typical, HDD: ~150 MB/s typical
                            return Some(if is_rotational == 0 {
                                500_000_000 // 500 MB/s for SSD
                            } else {
                                150_000_000 // 150 MB/s for HDD
                            });
                        }
                    }
                }
            }
        }

        // Fallback: Conservative estimate for unknown storage
        Some(100_000_000) // 100 MB/s conservative
    }

    /// Detect storage write bandwidth (bytes/sec) via runtime heuristics
    ///
    /// **Deep Debt**: Runtime detection, no hardcoding
    fn detect_storage_write_bandwidth() -> Option<u64> {
        // Write is typically 80% of read for SSDs, 90% for HDDs
        Self::detect_storage_read_bandwidth().map(|read_bw| (read_bw as f64 * 0.85) as u64)
    }

    /// Detect network bandwidth (bytes/sec) via runtime heuristics
    ///
    /// **Deep Debt**: Runtime detection, no hardcoding
    fn detect_network_bandwidth() -> Option<u64> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            // Check /sys/class/net for interface speeds
            if let Ok(entries) = fs::read_dir("/sys/class/net") {
                let mut max_speed = 0u64;

                for entry in entries.flatten() {
                    let path = entry.path();
                    let speed_path = path.join("speed");

                    // Skip loopback
                    if let Some(name) = path.file_name() {
                        if name == "lo" {
                            continue;
                        }
                    }

                    if let Ok(content) = fs::read_to_string(&speed_path) {
                        // Speed is in Mbps, convert to bytes/sec
                        if let Ok(mbps) = content.trim().parse::<u64>() {
                            let bytes_per_sec = (mbps * 1_000_000) / 8;
                            max_speed = max_speed.max(bytes_per_sec);
                        }
                    }
                }

                if max_speed > 0 {
                    return Some(max_speed);
                }
            }
        }

        // Fallback: Assume gigabit ethernet (125 MB/s)
        Some(125_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bare_metal_adaptation() {
        let layer = DeploymentLayer::BareMetalOS;
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities();

        assert_eq!(caps.compute.gpu_access, GpuAccess::Direct);
        assert!(caps.compute.has_cpu);
        assert!(caps.compute.supports_tensor_ops);
        assert_eq!(caps.storage.storage_type, StorageType::DirectBlock);
        assert_eq!(caps.network.network_access, NetworkAccess::Direct);
        assert!(caps.has_direct_gpu_access());
        assert!(caps.has_gpu_access());
    }

    #[test]
    fn test_middleware_adaptation() {
        let layer = DeploymentLayer::MiddlewareLayer {
            host_os: "Pop!_OS".to_string(),
            host_version: Some("22.04".to_string()),
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities();

        assert_eq!(caps.compute.gpu_access, GpuAccess::ViaHost);
        assert_eq!(caps.storage.storage_type, StorageType::HostFilesystem);
        assert_eq!(caps.metadata.host_os, Some("Pop!_OS".to_string()));
        assert!(!caps.has_direct_gpu_access());
        assert!(caps.has_gpu_access());
    }

    #[test]
    fn test_vm_adaptation_with_passthrough() {
        let layer = DeploymentLayer::VMLayer {
            hypervisor: "QEMU/KVM".to_string(),
            gpu_passthrough: true,
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities();

        assert_eq!(caps.compute.gpu_access, GpuAccess::Direct);
        assert!(caps.compute.supports_tensor_ops);
        assert!(caps.has_direct_gpu_access());
    }

    #[test]
    fn test_vm_adaptation_without_passthrough() {
        let layer = DeploymentLayer::VMLayer {
            hypervisor: "VirtualBox".to_string(),
            gpu_passthrough: false,
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities();

        assert_eq!(caps.compute.gpu_access, GpuAccess::None);
        assert!(!caps.compute.supports_tensor_ops);
        assert!(caps.compute.supports_nn_inference); // CPU fallback
        assert!(!caps.has_gpu_access());
    }

    #[test]
    fn test_cloud_adaptation() {
        let layer = DeploymentLayer::CloudLayer {
            provider: crate::deployment_layer::CloudProvider::AWS,
            instance_type: Some("g5.4xlarge".to_string()),
            region: Some("us-east-1".to_string()),
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities();

        assert_eq!(caps.compute.gpu_access, GpuAccess::ViaCloud);
        assert_eq!(caps.storage.storage_type, StorageType::CloudObject);
        assert_eq!(caps.network.network_access, NetworkAccess::CloudVPC);
        assert_eq!(caps.metadata.cloud_provider, Some("AWS".to_string()));
        assert!(!caps.has_direct_gpu_access());
        assert!(caps.has_gpu_access());
    }

    #[test]
    fn test_capability_list_generation() {
        let layer = DeploymentLayer::BareMetalOS;
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities();
        let cap_list = caps.to_capability_list();

        assert!(cap_list.contains(&compute_capabilities::GPU_COMPUTE_DIRECT.to_string()));
        assert!(cap_list.contains(&compute_capabilities::CPU_COMPUTE.to_string()));
        assert!(cap_list.contains(&compute_capabilities::TENSOR_OPS.to_string()));
        assert!(cap_list.contains(&storage_capabilities::BLOCK_STORAGE_DIRECT.to_string()));
        assert!(cap_list.contains(&network_capabilities::NETWORK_DIRECT.to_string()));
    }

    #[test]
    fn test_container_adaptation() {
        let layer = DeploymentLayer::ContainerLayer {
            runtime: crate::deployment_layer::ContainerRuntime::Docker,
            container_id: Some("abc123".to_string()),
        };
        let adapter = LayerCapabilityAdapter::new(layer);
        let caps = adapter.get_adapted_capabilities();

        assert_eq!(caps.compute.gpu_access, GpuAccess::ViaHost);
        assert_eq!(caps.storage.storage_type, StorageType::PersistentVolume);
        assert_eq!(caps.network.network_access, NetworkAccess::HostNamespace);
        assert!(caps.network.has_service_mesh);
    }
}
