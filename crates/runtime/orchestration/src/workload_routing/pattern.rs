// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

/// Kinds of GPU compute patterns used by springs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadPattern {
    /// Aggregate reduction (sum, max, etc.).
    Reduction,
    /// Scatter write pattern.
    Scatter,
    /// Monte Carlo simulation.
    MonteCarlo,
    /// ODE batch integration.
    OdeBatch,
    /// NLME iteration.
    NlmeIteration,
    /// Matrix multiplication.
    MatMul,
    /// Fast Fourier transform.
    Fft,
    /// Sparse matrix-vector product.
    SpMV,
    /// Element-wise operations.
    ElementWise,
    /// Smith-Waterman alignment.
    SmithWaterman,
    /// Pairwise distance / similarity (N×N or N×M).
    /// neuralSpring: `PairwiseL2Gpu`, `PairwiseHammingGpu`.
    Pairwise,
    /// Batch fitness evaluation (population × genome).
    /// neuralSpring: `BatchFitnessGpu`, `SwarmNnGpu`.
    BatchFitness,
    /// HMM forward/backward (states × observations).
    /// neuralSpring/wetSpring: `HmmBatchForwardF64`.
    HmmBatch,
    /// Spatial game / lattice payoff computation.
    /// neuralSpring: `SpatialPayoffGpu`.
    SpatialPayoff,
    /// Stochastic population simulation (populations × loci).
    /// neuralSpring: `WrightFisherGpu`, `BatchedMultinomialGpu`.
    Stochastic,
    /// Population pharmacokinetics (subjects × timepoints).
    /// healthSpring: `PopulationPkGpu`, `NlmeDispatch`.
    PopulationPk,
    /// Dose-response sweep (concentrations × parameters).
    /// healthSpring: `HillDoseResponseGpu`.
    DoseResponse,
    /// Diversity index computation (samples × taxa).
    /// healthSpring/wetSpring: `ShannonGpu`, `SimpsonGpu`.
    DiversityIndex,
}

impl WorkloadPattern {
    /// Estimated GPU memory (VRAM) in bytes for a given problem size.
    ///
    /// Absorbed from healthSpring V19 `gpu_memory_estimate()` proposal.
    /// Estimates are conservative upper bounds, suitable for scheduling
    /// decisions (rejecting workloads that won't fit in available VRAM).
    ///
    /// The formula is pattern-dependent:
    /// - N×N patterns (Pairwise, `SpatialPayoff`): `8 * N²` (f64 matrix)
    /// - Batch patterns: `8 * N` per element (f64 vector)
    /// - FFT: `16 * N` (complex f64 in + out)
    /// - Sparse (`SpMV`): `24 * N` (CSR triple per row, pessimistic)
    #[must_use]
    pub const fn gpu_memory_estimate_bytes(self, problem_size: u64) -> u64 {
        match self {
            Self::Pairwise | Self::SpatialPayoff => {
                problem_size.saturating_mul(problem_size).saturating_mul(8)
            }
            Self::Fft => problem_size.saturating_mul(16),
            Self::SpMV => problem_size.saturating_mul(24),
            Self::Stochastic | Self::PopulationPk | Self::DoseResponse => {
                problem_size.saturating_mul(16)
            }
            _ => problem_size.saturating_mul(8),
        }
    }
}
