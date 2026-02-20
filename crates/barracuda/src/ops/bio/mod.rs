// SPDX-License-Identifier: AGPL-3.0-only

//! Life-science and analytical-chemistry GPU primitives.
//!
//! Absorbed from wetSpring handoff v4 (Feb 20, 2026).
//!
//! | Module | Shader | Primitive |
//! |--------|--------|-----------|
//! | `smith_waterman` | `smith_waterman_banded_f64.wgsl` | Banded SW local alignment |
//! | `gillespie`      | `gillespie_ssa_f64.wgsl`         | Parallel Gillespie SSA |
//! | `tree_inference` | `tree_inference_f64.wgsl`        | Decision tree / RF inference |
//! | `felsenstein`    | `felsenstein_f64.wgsl`           | Felsenstein pruning likelihood |

pub mod felsenstein;
pub mod gillespie;
pub mod smith_waterman;
pub mod tree_inference;

pub use felsenstein::{FelsensteinGpu, FelsensteinResult, PhyloTree};
pub use gillespie::{GillespieConfig, GillespieGpu, GillespieResult};
pub use smith_waterman::{SmithWatermanGpu, SwConfig, SwResult};
pub use tree_inference::{FlatForest, TreeInferenceGpu};
