// SPDX-License-Identifier: AGPL-3.0-only
//! Workload-size-aware routing thresholds from cross-spring Kokkos parity benchmarks.
//!
//! Thresholds are absorbed from:
//! - **healthSpring V14.1**: kokkos_reduction, kokkos_scatter, kokkos_monte_carlo,
//!   kokkos_ode_batch, kokkos_nlme_iteration
//! - **neuralSpring S139**: bench_kokkos_parity, FFT, element-wise, BLAST pipeline
//! - **groundSpring V99** / **hotSpring v0.6.25**: spectral SpMV
//!
//! Each threshold defines a problem-size crossover below which CPU is faster than GPU.
//! The router uses these to select the optimal substrate (CPU vs GPU) for a given
//! workload pattern and problem size.

use serde::{Deserialize, Serialize};

/// Kinds of GPU compute patterns used by springs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadPattern {
    Reduction,
    Scatter,
    MonteCarlo,
    OdeBatch,
    NlmeIteration,
    MatMul,
    Fft,
    SpMV,
    ElementWise,
    SmithWaterman,
}

/// Target compute substrate for workload execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubstrateTarget {
    Cpu,
    Gpu,
    Npu,
}

/// Routing threshold for a workload pattern, validated by cross-spring benchmarks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingThreshold {
    pub pattern: WorkloadPattern,
    /// Problem size (element count) below which CPU is faster.
    pub gpu_crossover_n: u64,
    /// Source spring and version that validated this threshold.
    pub provenance: &'static str,
}

/// Routes workloads to the best substrate based on problem size and pattern.
pub struct WorkloadRouter {
    thresholds: Vec<RoutingThreshold>,
}

impl WorkloadRouter {
    /// Create router with default thresholds from cross-spring benchmarks.
    #[must_use]
    pub fn new() -> Self {
        Self {
            thresholds: default_thresholds(),
        }
    }

    /// Route a workload to the best substrate based on problem size.
    ///
    /// Returns `SubstrateTarget::Gpu` when `problem_size > gpu_crossover_n`,
    /// otherwise `SubstrateTarget::Cpu`. Unknown patterns default to CPU.
    #[must_use]
    pub fn route(&self, pattern: WorkloadPattern, problem_size: u64) -> SubstrateTarget {
        let crossover = self
            .thresholds
            .iter()
            .find(|t| t.pattern == pattern)
            .map(|t| t.gpu_crossover_n)
            .unwrap_or(u64::MAX);

        if problem_size > crossover {
            SubstrateTarget::Gpu
        } else {
            SubstrateTarget::Cpu
        }
    }

    /// Register a custom threshold (e.g. from runtime calibration).
    ///
    /// Replaces any existing threshold for the same pattern.
    pub fn set_threshold(&mut self, threshold: RoutingThreshold) {
        self.thresholds.retain(|t| t.pattern != threshold.pattern);
        self.thresholds.push(threshold);
    }
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

impl WorkloadRouter {
    /// Place a cooperating multi-GPU workload on GPUs that share a `PCIe`
    /// switch for fast P2P communication (e.g. halo exchange in lattice QCD).
    ///
    /// `available_gpus` is the set of card indices to choose from.
    /// `gpu_count` is how many GPUs the workload needs.
    /// Returns a placement recommendation sorted by interconnect quality.
    #[must_use]
    pub fn route_multi_gpu(
        &self,
        available_gpus: &[u32],
        gpu_count: usize,
        topology: &toadstool_sysmon::pcie_topology::PcieTopologyGraph,
    ) -> Option<MultiGpuPlacement> {
        if available_gpus.len() < gpu_count || gpu_count == 0 {
            return None;
        }

        if gpu_count == 1 {
            return Some(MultiGpuPlacement {
                gpu_indices: vec![available_gpus[0]],
                shared_switch: true,
                min_interconnect_bps: u64::MAX,
            });
        }

        let mut best: Option<(Vec<u32>, bool, u64)> = None;

        for combo in combinations(available_gpus, gpu_count) {
            let (shared, min_bw) = evaluate_group(&combo, topology);
            let score = if shared { min_bw + 1 } else { min_bw };

            let is_better = best.as_ref().is_none_or(|(_, prev_shared, prev_bw)| {
                match (shared, *prev_shared) {
                    (true, false) => true,
                    (false, true) => false,
                    _ => score > *prev_bw + u64::from(*prev_shared),
                }
            });

            if is_better {
                best = Some((combo, shared, min_bw));
            }
        }

        best.map(|(indices, shared_switch, min_bw)| MultiGpuPlacement {
            gpu_indices: indices,
            shared_switch,
            min_interconnect_bps: min_bw,
        })
    }
}

fn evaluate_group(
    gpus: &[u32],
    topology: &toadstool_sysmon::pcie_topology::PcieTopologyGraph,
) -> (bool, u64) {
    let mut all_shared_switch = true;
    let mut min_bw = u64::MAX;

    for i in 0..gpus.len() {
        for j in (i + 1)..gpus.len() {
            let bw = topology.effective_bandwidth_bps(gpus[i], gpus[j]);
            min_bw = min_bw.min(bw);

            if let Some(pair) = topology.pair(gpus[i], gpus[j]) {
                if pair.common_bridge.is_none() || pair.hops > 1 {
                    all_shared_switch = false;
                }
            } else {
                all_shared_switch = false;
            }
        }
    }

    (all_shared_switch, min_bw)
}

fn combinations(items: &[u32], k: usize) -> Vec<Vec<u32>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }

    let mut result = Vec::new();
    for (i, &item) in items.iter().enumerate() {
        for mut rest in combinations(&items[i + 1..], k - 1) {
            rest.insert(0, item);
            result.push(rest);
        }
    }
    result
}

impl Default for WorkloadRouter {
    fn default() -> Self {
        Self::new()
    }
}

fn default_thresholds() -> Vec<RoutingThreshold> {
    vec![
        RoutingThreshold {
            pattern: WorkloadPattern::Reduction,
            gpu_crossover_n: 10_000,
            provenance: "healthSpring V14.1 kokkos_reduction",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::Scatter,
            gpu_crossover_n: 50_000,
            provenance: "healthSpring V14.1 kokkos_scatter",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::MonteCarlo,
            gpu_crossover_n: 100_000,
            provenance: "healthSpring V14.1 kokkos_monte_carlo",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::OdeBatch,
            gpu_crossover_n: 5_000,
            provenance: "healthSpring V14.1 kokkos_ode_batch",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::NlmeIteration,
            gpu_crossover_n: 100,
            provenance: "healthSpring V14.1 kokkos_nlme_iteration",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::MatMul,
            gpu_crossover_n: 256,
            provenance: "neuralSpring S139 bench_kokkos_parity",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::Fft,
            gpu_crossover_n: 4_096,
            provenance: "neuralSpring S139",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::SpMV,
            gpu_crossover_n: 1_000,
            provenance: "hotSpring v0.6.25 spectral",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::ElementWise,
            gpu_crossover_n: 100_000,
            provenance: "neuralSpring S139",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::SmithWaterman,
            gpu_crossover_n: 1_000,
            provenance: "neuralSpring S139 BLAST pipeline",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_reduction_below_crossover_uses_cpu() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::Reduction, 5_000),
            SubstrateTarget::Cpu
        );
    }

    #[test]
    fn route_reduction_above_crossover_uses_gpu() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::Reduction, 50_000),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn route_reduction_at_crossover_uses_cpu() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::Reduction, 10_000),
            SubstrateTarget::Cpu
        );
    }

    #[test]
    fn route_matmul_below_crossover_uses_cpu() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::MatMul, 128),
            SubstrateTarget::Cpu
        );
    }

    #[test]
    fn route_matmul_above_crossover_uses_gpu() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::MatMul, 512),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn route_nlme_iteration_low_threshold() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::NlmeIteration, 50),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::NlmeIteration, 200),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn custom_threshold_override() {
        let mut router = WorkloadRouter::new();
        router.set_threshold(RoutingThreshold {
            pattern: WorkloadPattern::Reduction,
            gpu_crossover_n: 1_000,
            provenance: "runtime calibration",
        });
        assert_eq!(
            router.route(WorkloadPattern::Reduction, 500),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::Reduction, 2_000),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn default_router_construction() {
        let router = WorkloadRouter::default();
        assert_eq!(
            router.route(WorkloadPattern::Fft, 8_192),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn combinations_basic() {
        assert_eq!(
            super::combinations(&[0, 1, 2], 2),
            vec![vec![0, 1], vec![0, 2], vec![1, 2]]
        );
        assert_eq!(
            super::combinations(&[0, 1, 2], 1),
            vec![vec![0], vec![1], vec![2]]
        );
        assert_eq!(super::combinations(&[0, 1, 2], 3), vec![vec![0, 1, 2]]);
        assert!(super::combinations(&[0], 2).is_empty());
    }

    #[test]
    fn route_multi_gpu_single() {
        let router = WorkloadRouter::new();
        let topo = toadstool_sysmon::pcie_topology::PcieTopologyGraph {
            gpus: Vec::new(),
            bridge_chains: std::collections::HashMap::new(),
            pairs: Vec::new(),
            bridge_fanout: std::collections::HashMap::new(),
        };
        let placement = router.route_multi_gpu(&[0], 1, &topo);
        assert!(placement.is_some());
        assert_eq!(placement.unwrap().gpu_indices, vec![0]);
    }

    #[test]
    fn route_multi_gpu_insufficient() {
        let router = WorkloadRouter::new();
        let topo = toadstool_sysmon::pcie_topology::PcieTopologyGraph {
            gpus: Vec::new(),
            bridge_chains: std::collections::HashMap::new(),
            pairs: Vec::new(),
            bridge_fanout: std::collections::HashMap::new(),
        };
        assert!(router.route_multi_gpu(&[0], 2, &topo).is_none());
        assert!(router.route_multi_gpu(&[], 1, &topo).is_none());
    }
}
