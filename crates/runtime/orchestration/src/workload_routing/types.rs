// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use super::pattern::WorkloadPattern;

/// Target compute substrate for workload execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubstrateTarget {
    /// CPU execution.
    Cpu,
    /// GPU execution.
    Gpu,
    /// NPU execution.
    Npu,
}

/// Routing threshold for a workload pattern, validated by cross-spring benchmarks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingThreshold {
    /// Workload pattern this threshold applies to.
    pub pattern: WorkloadPattern,
    /// Problem size (element count) below which CPU is faster.
    pub gpu_crossover_n: u64,
    /// Source spring and version that validated this threshold.
    pub provenance: &'static str,
}

/// Multi-GPU placement recommendation from topology analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiGpuPlacement {
    /// Recommended GPU card indices, ordered by interconnect affinity.
    pub gpu_indices: Vec<u32>,
    /// Whether all recommended GPUs share a `PCIe` switch (fast P2P).
    pub shared_switch: bool,
    /// Minimum effective inter-GPU bandwidth in bytes/sec.
    pub min_interconnect_bps: u64,
}
