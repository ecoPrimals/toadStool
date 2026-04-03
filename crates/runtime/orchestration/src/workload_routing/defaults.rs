// SPDX-License-Identifier: AGPL-3.0-only

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
