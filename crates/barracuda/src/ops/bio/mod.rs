// SPDX-License-Identifier: AGPL-3.0-only

//! Life-science, evolutionary computation, and analytical-chemistry GPU primitives.
//!
//! Absorbed from wetSpring handoff v4–v6 and neuralSpring metalForge (Feb 2026).
//!
//! | Module | Shader | Primitive |
//! |--------|--------|-----------|
//! | `smith_waterman`    | `smith_waterman_banded_f64.wgsl` | Banded SW local alignment |
//! | `gillespie`         | `gillespie_ssa_f64.wgsl`         | Parallel Gillespie SSA |
//! | `tree_inference`    | `tree_inference_f64.wgsl`        | Decision tree / RF inference |
//! | `felsenstein`       | `felsenstein_f64.wgsl`           | Felsenstein pruning likelihood |
//! | `hmm`               | `hmm_forward_f64.wgsl`           | Batch HMM forward (f64) |
//! | `ani`               | `ani_batch_f64.wgsl`             | Pairwise ANI |
//! | `snp`               | `snp_calling_f64.wgsl`           | Position-parallel SNP calling |
//! | `dnds`              | `dnds_batch_f64.wgsl`            | Batch Nei-Gojobori dN/dS |
//! | `pangenome`         | `pangenome_classify.wgsl`        | Gene family classification |
//! | `quality_filter`    | `quality_filter.wgsl`            | Per-read quality trimming |
//! | `dada2`             | `dada2_e_step.wgsl`              | DADA2 E-step (batch log_p_error) |
//! | `locus_variance`    | `locus_variance.wgsl`            | Per-locus AF variance (FST) |
//! | `pairwise_hamming`  | `pairwise_hamming.wgsl`          | Pairwise Hamming distance |
//! | `pairwise_jaccard`  | `pairwise_jaccard.wgsl`          | Pairwise Jaccard distance |
//! | `spatial_payoff`    | `spatial_payoff.wgsl`            | Spatial PD payoff stencil |
//! | `batch_fitness`     | `batch_fitness_eval.wgsl`        | EA batch fitness evaluation |
//! | `rf_inference`      | `rf_batch_inference.wgsl`        | Batch RF inference (SoA f64) |

pub mod ani;
pub mod batch_fitness;
pub mod dada2;
pub mod dnds;
pub mod felsenstein;
pub mod gillespie;
pub mod hmm;
pub mod locus_variance;
pub mod pairwise_hamming;
pub mod pairwise_jaccard;
pub mod pangenome;
pub mod quality_filter;
pub mod rf_inference;
pub mod smith_waterman;
pub mod snp;
pub mod spatial_payoff;
pub mod tree_inference;

pub use ani::AniBatchF64;
pub use batch_fitness::BatchFitnessGpu;
pub use dada2::Dada2EStepGpu;
pub use dnds::DnDsBatchF64;
pub use felsenstein::{FelsensteinGpu, FelsensteinResult, PhyloTree};
pub use gillespie::{GillespieConfig, GillespieGpu, GillespieResult};
pub use hmm::HmmBatchForwardF64;
pub use locus_variance::LocusVarianceGpu;
pub use pairwise_hamming::PairwiseHammingGpu;
pub use pairwise_jaccard::PairwiseJaccardGpu;
pub use pangenome::PangenomeClassifyGpu;
pub use quality_filter::{QualityConfig, QualityFilterGpu};
pub use smith_waterman::{SmithWatermanGpu, SwConfig, SwResult};
pub use snp::SnpCallingF64;
pub use spatial_payoff::SpatialPayoffGpu;
pub use rf_inference::RfBatchInferenceGpu;
pub use tree_inference::{FlatForest, TreeInferenceGpu};
