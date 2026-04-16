// SPDX-License-Identifier: AGPL-3.0-or-later

use super::pattern::WorkloadPattern;
use super::types::RoutingThreshold;

pub(crate) fn default_thresholds() -> Vec<RoutingThreshold> {
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
        RoutingThreshold {
            pattern: WorkloadPattern::Pairwise,
            gpu_crossover_n: 500_000,
            provenance: "neuralSpring S140 pairwise_substrate bench",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::BatchFitness,
            gpu_crossover_n: 50_000,
            provenance: "neuralSpring S140 batch_fitness_substrate bench",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::HmmBatch,
            gpu_crossover_n: 5_000,
            provenance: "neuralSpring S140 hmm_substrate bench",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::SpatialPayoff,
            gpu_crossover_n: 4_000,
            provenance: "neuralSpring S140 spatial_substrate bench",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::Stochastic,
            gpu_crossover_n: 100_000,
            provenance: "neuralSpring S140 stochastic_substrate bench",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::PopulationPk,
            gpu_crossover_n: 100,
            provenance: "healthSpring V14.1 metalForge parallel_gpu_min",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::DoseResponse,
            gpu_crossover_n: 1_000,
            provenance: "healthSpring V14.1 metalForge sweep_gpu_min",
        },
        RoutingThreshold {
            pattern: WorkloadPattern::DiversityIndex,
            gpu_crossover_n: 500,
            provenance: "healthSpring V14.1 metalForge reduce_gpu_min",
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::workload_routing::{RoutingThreshold, WorkloadPattern};

    use super::default_thresholds;

    #[test]
    fn default_thresholds_table_has_entry_per_pattern_variant() {
        let thresholds = default_thresholds();
        assert_eq!(thresholds.len(), 18);
        let patterns: Vec<WorkloadPattern> = thresholds.iter().map(|t| t.pattern).collect();
        let unique: std::collections::HashSet<_> = patterns.iter().copied().collect();
        assert_eq!(
            unique.len(),
            patterns.len(),
            "duplicate pattern rows in default_thresholds"
        );
    }

    #[test]
    fn default_matmul_and_reduction_crossovers_match_benchmark_table() {
        let thresholds: Vec<RoutingThreshold> = default_thresholds();
        let matmul = thresholds
            .iter()
            .find(|t| t.pattern == WorkloadPattern::MatMul)
            .expect("MatMul");
        let reduction = thresholds
            .iter()
            .find(|t| t.pattern == WorkloadPattern::Reduction)
            .expect("Reduction");
        assert_eq!(matmul.gpu_crossover_n, 256);
        assert_eq!(reduction.gpu_crossover_n, 10_000);
        for t in &thresholds {
            assert!(!t.provenance.is_empty());
        }
    }

    #[test]
    fn workload_router_default_uses_same_threshold_source() {
        let from_fn = default_thresholds();
        let router = crate::workload_routing::WorkloadRouter::default();
        for t in &from_fn {
            assert_eq!(
                router.route(t.pattern, t.gpu_crossover_n),
                crate::workload_routing::SubstrateTarget::Cpu
            );
            assert_eq!(
                router.route(t.pattern, t.gpu_crossover_n.saturating_add(1)),
                crate::workload_routing::SubstrateTarget::Gpu
            );
        }
    }
}
