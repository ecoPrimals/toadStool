// SPDX-License-Identifier: AGPL-3.0-only
//! Workload-size-aware routing thresholds from cross-spring Kokkos parity benchmarks.
//!
//! Thresholds are absorbed from:
//! - **healthSpring V14.1**: `kokkos_reduction`, `kokkos_scatter`, `kokkos_monte_carlo`,
//!   `kokkos_ode_batch`, `kokkos_nlme_iteration`
//! - **neuralSpring S139**: `bench_kokkos_parity`, FFT, element-wise, BLAST pipeline
//! - **groundSpring V99** / **hotSpring v0.6.25**: spectral `SpMV`
//!
//! Each threshold defines a problem-size crossover below which CPU is faster than GPU.
//! The router uses these to select the optimal substrate (CPU vs GPU) for a given
//! workload pattern and problem size.

mod defaults;
mod multi_gpu;
mod pattern;
mod types;

pub use pattern::WorkloadPattern;
pub use types::{MultiGpuPlacement, RoutingThreshold, SubstrateTarget};

use defaults::default_thresholds;
use multi_gpu::{combinations, evaluate_group};

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
            .map_or(u64::MAX, |t| t.gpu_crossover_n);

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

    /// VRAM-aware routing: falls back to CPU if estimated GPU memory exceeds
    /// `available_vram_bytes`, even when problem size crosses the GPU threshold.
    ///
    /// Absorbed from healthSpring V19 scheduling proposal.
    #[must_use]
    pub fn route_with_vram(
        &self,
        pattern: WorkloadPattern,
        problem_size: u64,
        available_vram_bytes: u64,
    ) -> SubstrateTarget {
        let base = self.route(pattern, problem_size);
        if base == SubstrateTarget::Gpu
            && pattern.gpu_memory_estimate_bytes(problem_size) > available_vram_bytes
        {
            return SubstrateTarget::Cpu;
        }
        base
    }
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

impl Default for WorkloadRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::multi_gpu::combinations;
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
            combinations(&[0, 1, 2], 2),
            vec![vec![0, 1], vec![0, 2], vec![1, 2]]
        );
        assert_eq!(combinations(&[0, 1, 2], 1), vec![vec![0], vec![1], vec![2]]);
        assert_eq!(combinations(&[0, 1, 2], 3), vec![vec![0, 1, 2]]);
        assert!(combinations(&[0], 2).is_empty());
    }

    #[test]
    fn route_multi_gpu_single() {
        let router = WorkloadRouter::new();
        let topo = toadstool_sysmon::pcie_topology::PcieTopologyGraph::empty();
        let placement = router.route_multi_gpu(&[0], 1, &topo);
        assert!(placement.is_some());
        assert_eq!(placement.unwrap().gpu_indices, vec![0]);
    }

    #[test]
    fn route_multi_gpu_insufficient() {
        let router = WorkloadRouter::new();
        let topo = toadstool_sysmon::pcie_topology::PcieTopologyGraph::empty();
        assert!(router.route_multi_gpu(&[0], 2, &topo).is_none());
        assert!(router.route_multi_gpu(&[], 1, &topo).is_none());
    }

    #[test]
    fn route_pairwise_crossover() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::Pairwise, 100_000),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::Pairwise, 1_000_000),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn route_batch_fitness_crossover() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::BatchFitness, 10_000),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::BatchFitness, 100_000),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn route_hmm_batch_crossover() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::HmmBatch, 1_000),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::HmmBatch, 10_000),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn route_population_pk_low_threshold() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::PopulationPk, 50),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::PopulationPk, 200),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn route_dose_response_crossover() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::DoseResponse, 500),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::DoseResponse, 2_000),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn route_diversity_index_crossover() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::DiversityIndex, 100),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::DiversityIndex, 1_000),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn route_stochastic_crossover() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::Stochastic, 50_000),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::Stochastic, 200_000),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn route_spatial_payoff_crossover() {
        let router = WorkloadRouter::new();
        assert_eq!(
            router.route(WorkloadPattern::SpatialPayoff, 2_000),
            SubstrateTarget::Cpu
        );
        assert_eq!(
            router.route(WorkloadPattern::SpatialPayoff, 8_000),
            SubstrateTarget::Gpu
        );
    }

    #[test]
    fn gpu_memory_estimate_pairwise_quadratic() {
        let est = WorkloadPattern::Pairwise.gpu_memory_estimate_bytes(1_000);
        assert_eq!(est, 8_000_000, "1000×1000 f64 matrix = 8MB");
    }

    #[test]
    fn gpu_memory_estimate_fft_linear() {
        let est = WorkloadPattern::Fft.gpu_memory_estimate_bytes(1_000_000);
        assert_eq!(est, 16_000_000, "1M complex f64 = 16MB");
    }

    #[test]
    fn gpu_memory_estimate_reduction_linear() {
        let est = WorkloadPattern::Reduction.gpu_memory_estimate_bytes(1_000_000);
        assert_eq!(est, 8_000_000, "1M f64 elements = 8MB");
    }

    #[test]
    fn route_with_vram_falls_back() {
        let router = WorkloadRouter::new();
        let two_gb: u64 = 2 * 1024 * 1024 * 1024;
        assert_eq!(
            router.route_with_vram(WorkloadPattern::Reduction, 1_000_000, two_gb),
            SubstrateTarget::Gpu,
            "8MB fits in 2GB VRAM"
        );
        assert_eq!(
            router.route_with_vram(WorkloadPattern::Pairwise, 1_000_000, two_gb),
            SubstrateTarget::Cpu,
            "1M×1M f64 = 8TB, exceeds 2GB VRAM — fallback to CPU"
        );
    }

    #[test]
    fn all_patterns_have_default_thresholds() {
        let router = WorkloadRouter::new();
        let patterns = [
            WorkloadPattern::Reduction,
            WorkloadPattern::Scatter,
            WorkloadPattern::MonteCarlo,
            WorkloadPattern::OdeBatch,
            WorkloadPattern::NlmeIteration,
            WorkloadPattern::MatMul,
            WorkloadPattern::Fft,
            WorkloadPattern::SpMV,
            WorkloadPattern::ElementWise,
            WorkloadPattern::SmithWaterman,
            WorkloadPattern::Pairwise,
            WorkloadPattern::BatchFitness,
            WorkloadPattern::HmmBatch,
            WorkloadPattern::SpatialPayoff,
            WorkloadPattern::Stochastic,
            WorkloadPattern::PopulationPk,
            WorkloadPattern::DoseResponse,
            WorkloadPattern::DiversityIndex,
        ];
        for pattern in patterns {
            let result = router.route(pattern, 0);
            assert_eq!(
                result,
                SubstrateTarget::Cpu,
                "size 0 should route to CPU for {pattern:?}"
            );
        }
    }
}
