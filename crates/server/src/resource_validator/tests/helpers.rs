// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::time::Duration;

use crate::resource_estimator::ResourceEstimate;
use crate::resource_validator::SystemCapabilities;

pub(super) fn base_estimate() -> ResourceEstimate {
    ResourceEstimate {
        graph_id: "test-graph".to_string(),
        cpu_cores: 4,
        memory_bytes: 4 * 1024 * 1024 * 1024,
        gpu_memory_bytes: 0,
        storage_bytes: 1024,
        network_bandwidth_mbps: 100,
        estimated_duration: Duration::from_secs(1),
        max_parallelism: 1,
        critical_path_length: 1,
        node_estimates: HashMap::new(),
        warnings: vec![],
    }
}

pub(super) fn base_capabilities() -> SystemCapabilities {
    SystemCapabilities {
        total_cpu_cores: 16,
        available_cpu_cores: 16,
        total_memory_bytes: 32 * 1024 * 1024 * 1024,
        available_memory_bytes: 32 * 1024 * 1024 * 1024,
        total_gpu_memory_bytes: 8 * 1024 * 1024 * 1024,
        available_gpu_memory_bytes: 8 * 1024 * 1024 * 1024,
        total_storage_bytes: 1024 * 1024 * 1024 * 1024,
        available_storage_bytes: 1024 * 1024 * 1024 * 1024,
        network_bandwidth_mbps: 1000,
        gpu_count: 1,
        gpu_types: vec!["Test GPU".to_string()],
    }
}

pub(super) fn wgpu_safe_or_skip() -> bool {
    if toadstool_testing::gpu_guards::is_wgpu_safe() {
        return true;
    }
    eprintln!("{}", toadstool_testing::gpu_guards::wgpu_skip_reason());
    false
}
