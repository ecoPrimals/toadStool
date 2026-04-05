// SPDX-License-Identifier: AGPL-3.0-or-later
use toadstool::resources::{
    CpuRequirements as CoreCpuRequirements, GpuRequirements as CoreGpuRequirements,
    MemoryRequirements as CoreMemoryRequirements, NetworkRequirements as CoreNetworkRequirements,
    ResourceRequirements as CoreResourceRequirements,
    StorageRequirements as CoreStorageRequirements,
};

use super::requirements::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements,
    ResourceRequirements, StorageRequirements,
};

impl From<ResourceRequirements> for CoreResourceRequirements {
    fn from(distributed: ResourceRequirements) -> Self {
        Self {
            cpu: CoreCpuRequirements {
                min_cores: distributed.cpu.min_cores,
                max_cores: distributed.cpu.max_cores,
                architecture: None,
            },
            memory: CoreMemoryRequirements {
                min_bytes: distributed.memory.min_bytes,
                max_bytes: distributed.memory.max_bytes,
            },
            storage: CoreStorageRequirements {
                min_bytes: distributed.storage.min_bytes,
                max_bytes: distributed.storage.max_bytes,
                storage_type: None,
            },
            gpu: distributed.gpu.map(|gpu| CoreGpuRequirements {
                min_units: 1,
                max_units: None,
                gpu_type: gpu.compute_capability,
                min_memory_bytes: Some((gpu.min_memory_gb * 1024.0 * 1024.0 * 1024.0) as u64),
            }),
            network: CoreNetworkRequirements {
                min_bandwidth: distributed
                    .network
                    .bandwidth_mbps
                    .map(|mbps| mbps * 1024 * 1024),
                max_bandwidth: None,
                max_latency_ms: distributed.network.latency_ms,
            },
        }
    }
}

impl From<CoreResourceRequirements> for ResourceRequirements {
    fn from(core: CoreResourceRequirements) -> Self {
        Self {
            cpu: CpuRequirements {
                min_cores: core.cpu.min_cores,
                max_cores: core.cpu.max_cores,
            },
            memory: MemoryRequirements {
                min_bytes: core.memory.min_bytes,
                max_bytes: core.memory.max_bytes,
            },
            storage: StorageRequirements {
                min_bytes: core.storage.min_bytes,
                max_bytes: core.storage.max_bytes,
            },
            network: NetworkRequirements {
                bandwidth_mbps: core
                    .network
                    .min_bandwidth
                    .map(|bytes_per_sec| bytes_per_sec / (1024 * 1024)),
                latency_ms: core.network.max_latency_ms,
            },
            gpu: core.gpu.map(|gpu| GpuRequirements {
                min_memory_gb: gpu
                    .min_memory_bytes
                    .map(|bytes| bytes as f64 / (1024.0 * 1024.0 * 1024.0))
                    .unwrap_or(1.0),
                compute_capability: gpu.gpu_type,
            }),
        }
    }
}
