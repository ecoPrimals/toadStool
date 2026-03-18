// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Capability types for layer adaptation
//!
//! Compute, storage, and network capability structs and enums.

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

    /// Tensor operations (barraCuda)
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

    /// Supports tensor operations (barraCuda)
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
