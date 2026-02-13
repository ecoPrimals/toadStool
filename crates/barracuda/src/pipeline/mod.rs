//! Pipeline orchestration for heterogeneous compute
//!
//! Provides declarative APIs for building multi-stage compute pipelines
//! that automatically route between CPU, GPU, and NPU based on workload
//! characteristics.
//!
//! # Cascade Pattern
//!
//! The cascade pattern (validated by hotSpring) progressively filters candidates:
//!
//! ```text
//! Candidates: 6000
//!     ↓
//! Tier 1: NMP pre-screen (CPU, ~1μs) → 79% rejected, 1260 pass
//!     ↓
//! Tier 2: SEMF proxy (CPU, ~0.1ms) → 13% rejected, 540 pass
//!     ↓
//! Tier 3: Classifier (CPU/NPU, ~10μs) → optional filtering
//!     ↓
//! Tier 4: Full HFB (CPU ∥, ~0.2s) → 488 evaluated
//! Result: 91.9% savings on expensive evaluations
//! ```
//!
//! # Example
//!
//! ```ignore
//! use barracuda::pipeline::{Pipeline, Stage, StageConfig};
//!
//! let pipeline = Pipeline::new()
//!     .stage(Stage::new("prescreen")
//!         .filter(|x| check_nmp_constraints(x))
//!         .target(Target::Cpu))
//!     .stage(Stage::new("proxy")
//!         .transform(|x| semf_objective(x))
//!         .filter(|_, y| y < threshold)
//!         .target(Target::Cpu))
//!     .stage(Stage::new("full")
//!         .transform(|x| hfb_objective(x))
//!         .target(Target::CpuParallel));
//!
//! let results = pipeline.run(&candidates)?;
//! ```
//!
//! # Reference
//!
//! hotSpring L2 heterogeneous pipeline: 7.2× speedup with precision-aware dispatch

pub mod cascade;
pub mod stage;

pub use cascade::{Cascade, CascadeBuilder, CascadeResult, FilterResult};
pub use stage::{Stage, StageConfig, StageResult, Target};
